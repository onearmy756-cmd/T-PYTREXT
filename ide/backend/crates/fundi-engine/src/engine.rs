//! engine.rs — ORCHESTRATOR ya injini YETU: chat + streaming (bila Python/vLLM).
//!
//! Inaunganisha: GGUF → tokenizer → forward → sampler → maandishi. Hutoa
//! callback kwa kila tokeni (SSE inairusha kwa UI) na hupima tok/s HALISI.

use crate::model::Model;
use crate::sampler::{sample, Rng, SamplingParams};
use crate::snn::{SnnConfig, SpikeStats};
use crate::tokenizer::Tokenizer;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Tukio la mstari mmoja (kwa SSE).
#[derive(Debug, Clone)]
pub enum GenEvent {
    /// tokeni moja imetolewa
    Token { text: String, id: u32, tps: f32, total: u32 },
    /// maelezo ya mwanzo
    Start { model: String, ctx: usize },
    /// mwisho
    Done { total: u32, elapsed_ms: u64, tps_avg: f32, reason: String },
    /// kosa
    Error { message: String },
}

/// Injini iliyopakiwa (modeli moja kwa wakati).
pub struct Engine {
    pub model: Model,
    pub tokenizer: Tokenizer,
    pub path: PathBuf,
}

impl Engine {
    pub fn load(path: &Path, ctx: usize) -> Result<Self, String> {
        let model = Model::load(path, ctx)?;
        let tokenizer = Tokenizer::from_gguf(&model.g)?;
        Ok(Engine {
            model,
            tokenizer,
            path: path.to_path_buf(),
        })
    }

    pub fn n_params_estimate(&self) -> u64 {
        self.model
            .g
            .tensors
            .iter()
            .map(|t| t.n_elements())
            .sum()
    }

    /// Zalisha tokeni mfululizo; `on_event` huitwa kwa kila tokeni.
    /// `stop_ids` = tokeni za kusimamisha (eos + chat special).
    pub fn generate<F: FnMut(GenEvent)>(
        &mut self,
        prompt_tokens: &[u32],
        max_tokens: usize,
        params: &SamplingParams,
        stop_ids: &[u32],
        mut on_event: F,
    ) {
        let cfg = &self.model.cfg;
        on_event(GenEvent::Start {
            model: cfg.arch.clone(),
            ctx: self.model.cache.cap,
        });

        // Prefill
        let t0 = Instant::now();
        let mut logits = match self.model.prefill(prompt_tokens) {
            Ok(l) => l,
            Err(e) => {
                on_event(GenEvent::Error { message: e });
                return;
            }
        };

        let mut history: Vec<u32> = prompt_tokens.to_vec();
        let mut rng = Rng::new(params.seed);
        let mut produced: u32 = 0;
        let mut t_first: Option<Instant> = None;
        let mut reason = "length".to_string();

        for _ in 0..max_tokens {
            let next = sample(&mut logits, params, &history, &mut rng);
            if stop_ids.contains(&next) || self.tokenizer.is_eos(next) {
                reason = "eos".to_string();
                break;
            }
            let text = self.tokenizer.decode_stream_piece(next);
            history.push(next);
            produced += 1;
            let now = Instant::now();
            if t_first.is_none() {
                t_first = Some(now);
            }
            let elapsed = t_first.unwrap().elapsed().as_secs_f32();
            let tps = if elapsed > 0.0 {
                produced as f32 / elapsed
            } else {
                0.0
            };
            on_event(GenEvent::Token {
                text,
                id: next,
                tps,
                total: produced,
            });
            if self.model.cache.len >= self.model.cache.cap {
                reason = "context".to_string();
                break;
            }
            let pos = self.model.cache.len;
            logits = match self.model.forward(next, pos) {
                Ok(l) => l,
                Err(e) => {
                    on_event(GenEvent::Error { message: e });
                    return;
                }
            };
        }

        let elapsed_ms = t0.elapsed().as_millis() as u64;
        let tps_avg = if elapsed_ms > 0 {
            produced as f32 / (elapsed_ms as f32 / 1000.0)
        } else {
            0.0
        };
        on_event(GenEvent::Done {
            total: produced,
            elapsed_ms,
            tps_avg,
            reason,
        });
    }

    /// Zalisha jibu kamili kwa prompt ya maandishi (chat moja).
    pub fn chat(&mut self, system: &str, user: &str, max_tokens: usize, params: &SamplingParams) -> String {
        let template = self.tokenizer.detect_template();
        let prompt = self.tokenizer.chat_prompt(system, user, template);
        let mut stop = Vec::new();
        if let Some(&id) = self.tokenizer.special.get("<|im_end|>") {
            stop.push(id);
        }
        if let Some(&id) = self.tokenizer.special.get("<|eot_id|>") {
            stop.push(id);
        }
        let ids = self.tokenizer.encode(&prompt, true);
        let mut out = String::new();
        self.generate(&ids, max_tokens, params, &stop, |ev| {
            if let GenEvent::Token { text, .. } = ev {
                out.push_str(&text);
            }
        });
        out
    }

    /// Zalisha kwa SNN (spiking) — addition-only + Shiftmax; hurudisha maandishi
    /// pamoja na takwimu za nishati za mzunguko mzima.
    ///
    /// Huhesabu prefill kwa spiking, kisha kila tokeni kwa `forward_spiking`.
    /// Hii ni "speed + ubora" ya SNN juu ya weights HALISI za GGUF.
    pub fn chat_spiking(
        &mut self,
        system: &str,
        user: &str,
        max_tokens: usize,
        params: &SamplingParams,
        scfg: &SnnConfig,
    ) -> (String, SpikeStats) {
        let template = self.tokenizer.detect_template();
        let prompt = self.tokenizer.chat_prompt(system, user, template);
        // Stop ids: eos + special tokens za "end"/"eot" (kama zipo).
        let mut stop = Vec::new();
        if let Some(e) = self.tokenizer.eos_id {
            stop.push(e);
        }
        for (name, &id) in self.tokenizer.special.iter() {
            let l = name.to_ascii_lowercase();
            if l.contains("end") || l.contains("eot") {
                stop.push(id);
            }
        }
        let ids = self.tokenizer.encode(&prompt, true);
        self.model.cache.reset();
        let (mut logits, mut total) = match self.model.prefill_spiking(&ids, scfg) {
            Ok(v) => v,
            Err(_) => return (String::new(), SpikeStats::default()),
        };
        let mut history: Vec<u32> = ids.clone();
        let mut rng = Rng::new(params.seed);
        let mut out = String::new();
        for _ in 0..max_tokens {
            let next = sample(&mut logits, params, &history, &mut rng);
            if stop.contains(&next) || self.tokenizer.is_eos(next) {
                break;
            }
            out.push_str(&self.tokenizer.decode_stream_piece(next));
            history.push(next);
            if self.model.cache.len >= self.model.cache.cap {
                break;
            }
            let pos = self.model.cache.len;
            match self.model.forward_spiking(next, pos, scfg) {
                Ok((l, s)) => {
                    logits = l;
                    total.absorb(&s);
                }
                Err(_) => break,
            }
        }
        (out, total)
    }
}

/// Taarifa za modeli kwa UI (bila kupakia).
pub fn inspect(path: &Path) -> Result<serde_json::Value, String> {
    let g = crate::gguf::GgufFile::open(path)?;
    let cfg = crate::model::ModelConfig::from_gguf(&g)?;
    let mut quants: std::collections::BTreeMap<&'static str, usize> = Default::default();
    for t in &g.tensors {
        *quants.entry(t.ty.label()).or_insert(0) += 1;
    }
    let total_elems: u64 = g.tensors.iter().map(|t| t.n_elements()).sum();
    Ok(serde_json::json!({
        "ok": true,
        "path": path.display().to_string(),
        "name": g.meta_str("general.name").unwrap_or("(bila jina)"),
        "arch": cfg.arch,
        "layers": cfg.n_layer,
        "embd": cfg.n_embd,
        "heads": cfg.n_head,
        "heads_kv": cfg.n_head_kv,
        "head_dim": cfg.head_dim,
        "ff": cfg.n_ff,
        "vocab": cfg.n_vocab,
        "rope_base": cfg.rope_base,
        "tied_embeddings": cfg.tie_embeddings,
        "tensors": g.tensors.len(),
        "params": total_elems,
        "quant_mix": quants,
        "size_mb": (g.tensors.iter().map(|t| t.n_bytes()).sum::<u64>() as f64 / 1e6).round(),
    }))
}
