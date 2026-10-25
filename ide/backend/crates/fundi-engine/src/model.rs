//! model.rs — TRANSFORMER FORWARD PASS (Llama / Qwen2 / Mistral) kwa pure Rust.
//!
//! Usanifu (kanuni ya llama.cpp):
//!   tokens → embedding → [ kwa kila layer:
//!       RMSNorm → QKV proj → RoPE → attention (KV cache) → O proj → residual
//!       RMSNorm → gate/up proj → SiLU(gate)*up → down proj → residual ]
//!   → final RMSNorm → output proj → logits
//!
//! RAM: weights KUBWA hazifunguliwi kabisa; matvec hufungua safu moja kwa wakati
//! kwenye scratch ndogo. Tensor NDOGO (norms + biases, 1-D) hupakiwa mara moja
//! kama f32 (ni KB chache) ili kuepuka kufungua kila hatua.

use crate::gguf::{GgmlType, GgufFile};
use crate::snn::{attention_out, SnnConfig, SpikeStats};
use crate::tensor::{rmsnorm, rope, silu, Tensor};
use std::collections::HashMap;

/// Config ya modeli (kutoka GGUF metadata).
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub arch: String,
    pub n_layer: usize,
    pub n_embd: usize,
    pub n_head: usize,
    pub n_head_kv: usize,
    pub head_dim: usize,
    pub n_ff: usize,
    pub n_vocab: usize,
    pub rms_eps: f32,
    pub rope_base: f32,
    /// Mpangilio wa jozi za RoPE: true = NEOX (Qwen2/Gemma/Phi3), false = NORM (Llama).
    pub rope_neox: bool,
    pub tie_embeddings: bool,
}

impl ModelConfig {
    pub fn from_gguf(g: &GgufFile) -> Result<Self, String> {
        let arch = g
            .meta_str("general.architecture")
            .unwrap_or("llama")
            .to_string();
        let p = |k: &str| format!("{arch}.{k}");
        let need = |k: &str| -> Result<u64, String> {
            g.meta_u64(&p(k)).ok_or_else(|| format!("GGUF haina {k}"))
        };
        let n_layer = need("block_count")? as usize;
        let n_embd = need("embedding_length")? as usize;
        let n_head = need("attention.head_count")? as usize;
        let n_head_kv = g
            .meta_u64(&p("attention.head_count_kv"))
            .map(|v| v as usize)
            .unwrap_or(n_head);
        let head_dim = g
            .meta_u64(&p("attention.key_length"))
            .map(|v| v as usize)
            .unwrap_or(n_embd / n_head);
        let n_ff = need("feed_forward_length")? as usize;
        let n_vocab = g
            .meta_array("tokenizer.ggml.tokens")
            .map(|a| a.len())
            .ok_or("GGUF haina vocab (tokenizer.ggml.tokens)")?;
        let rms_eps = g
            .meta_f32(&p("attention.layer_norm_rms_epsilon"))
            .unwrap_or(1e-5);
        let rope_base = g.meta_f32(&p("rope.freq_base")).unwrap_or(10000.0);
        let tie_embeddings = g.meta_bool("general.tie_word_embeddings").unwrap_or(false)
            || g.tensor("output.weight").is_none();
        // RoPE pair layout: Qwen2/Gemma/Phi3 = NEOX; Llama/Mistral = NORM.
        let rope_neox = matches!(arch.as_str(), "qwen2" | "qwen2moe" | "gemma" | "gemma2" | "phi3" | "phi2");
        if n_head == 0 || head_dim == 0 {
            return Err("config ya modeli si sahihi (heads/head_dim = 0)".into());
        }
        if n_head % n_head_kv != 0 {
            return Err("n_head haigawiki kwa n_head_kv (GQA)".into());
        }
        Ok(ModelConfig {
            arch,
            n_layer,
            n_embd,
            n_head,
            n_head_kv,
            head_dim,
            n_ff,
            n_vocab,
            rms_eps,
            rope_base,
            rope_neox,
            tie_embeddings,
        })
    }
}

/// KV cache (kwa layer zote).
pub struct KvCache {
    pub k: Vec<Vec<f32>>,
    pub v: Vec<Vec<f32>>,
    pub len: usize,
    pub cap: usize,
    pub kv_dim: usize,
}

impl KvCache {
    pub fn new(n_layer: usize, cap: usize, n_kv_head: usize, head_dim: usize) -> Self {
        let kv_dim = n_kv_head * head_dim;
        KvCache {
            k: vec![vec![0f32; cap * kv_dim]; n_layer],
            v: vec![vec![0f32; cap * kv_dim]; n_layer],
            len: 0,
            cap,
            kv_dim,
        }
    }
    pub fn reset(&mut self) {
        self.len = 0;
    }
}

/// Injini ya modeli iliyopakiwa (mmap + config + KV cache + tensors ndogo).
pub struct Model {
    pub g: GgufFile,
    pub cfg: ModelConfig,
    pub cache: KvCache,
    /// Tensors za 1-D (norms + biases) zilizofunguliwa mara moja: jina → f32.
    small: HashMap<String, Vec<f32>>,
}

impl Model {
    pub fn load(path: &std::path::Path, ctx: usize) -> Result<Self, String> {
        let g = GgufFile::open(path)?;
        let cfg = ModelConfig::from_gguf(&g)?;
        let mut small = HashMap::new();
        for t in &g.tensors {
            if t.dims.len() == 1 {
                let n = t.n_elements() as usize;
                let bytes = g.tensor_bytes(&t.name)?;
                let mut v = vec![0f32; n];
                crate::quant::dequant_row(t.ty, bytes, &mut v);
                small.insert(t.name.clone(), v);
            }
        }
        let cache = KvCache::new(cfg.n_layer, ctx, cfg.n_head_kv, cfg.head_dim);
        Ok(Model {
            g,
            cfg,
            cache,
            small,
        })
    }

    /// Tensor kubwa kutoka mmap (zero-copy).
    fn big(&self, name: &str) -> Result<Tensor<'_>, String> {
        let info = self
            .g
            .tensor(name)
            .cloned()
            .ok_or_else(|| format!("tensor '{name}' haipo"))?;
        let bytes = self.g.tensor_bytes(name)?;
        Ok(Tensor::new(info, bytes))
    }

    fn small(&self, name: &str) -> Option<&[f32]> {
        self.small.get(name).map(|v| v.as_slice())
    }

    /// Forward kwa tokeni MOJA kwenye nafasi `pos` (== cache.len). Hurudisha logits.
    pub fn forward(&mut self, token: u32, pos: usize) -> Result<Vec<f32>, String> {
        let n_embd = self.cfg.n_embd;
        let head_dim = self.cfg.head_dim;
        let n_head = self.cfg.n_head;
        let n_kv = self.cfg.n_head_kv;
        let kv_dim = n_kv * head_dim;
        let n_ff = self.cfg.n_ff;
        let eps = self.cfg.rms_eps;

        // 1) Embedding.
        let emb = self.big("token_embd.weight")?;
        let row = token as usize;
        if row >= emb.n_out() {
            return Err(format!("token {row} nje ya vocab"));
        }
        let mut x = vec![0f32; n_embd];
        emb.dequant_row(row, &mut x);

        let mut xb = vec![0f32; n_embd];
        let mut q = vec![0f32; n_head * head_dim];
        let mut k = vec![0f32; kv_dim];
        let mut v = vec![0f32; kv_dim];
        let mut att = vec![0f32; n_head * (pos + 1)];
        let mut gate = vec![0f32; n_ff];
        let mut up = vec![0f32; n_ff];
        let mut o = vec![0f32; n_embd];

        for l in 0..self.cfg.n_layer {
            let lp = format!("blk.{l}");

            // 2) Attention norm.
            let attn_norm = self
                .small(&format!("{lp}.attn_norm.weight"))
                .ok_or_else(|| format!("{lp}.attn_norm.weight haipo"))?;
            rmsnorm(&x, attn_norm, eps, &mut xb);

            // 3) QKV.
            self.big(&format!("{lp}.attn_q.weight"))?
                .matvec_par(&xb, &mut q);
            self.big(&format!("{lp}.attn_k.weight"))?
                .matvec_par(&xb, &mut k);
            self.big(&format!("{lp}.attn_v.weight"))?
                .matvec_par(&xb, &mut v);

            if let Some(bq) = self.small(&format!("{lp}.attn_q.bias")) {
                add_inplace(&mut q, bq);
            }
            if let Some(bk) = self.small(&format!("{lp}.attn_k.bias")) {
                add_inplace(&mut k, bk);
            }
            if let Some(bv) = self.small(&format!("{lp}.attn_v.bias")) {
                add_inplace(&mut v, bv);
            }

            // 4) RoPE.
            rope(
                &mut q,
                &mut k,
                n_head,
                n_kv,
                head_dim,
                pos,
                self.cfg.rope_base,
                self.cfg.rope_neox,
            );

            // 5) Andika K,V kwenye cache (cache.len == pos).
            let cl = self.cache.len;
            self.cache.k[l][cl * kv_dim..(cl + 1) * kv_dim].copy_from_slice(&k);
            self.cache.v[l][cl * kv_dim..(cl + 1) * kv_dim].copy_from_slice(&v);

            // 6) Attention kwa kila head.
            let scale = 1.0 / (head_dim as f32).sqrt();
            let npos = cl + 1;
            let ratio = n_head / n_kv;
            let kc = &self.cache.k[l];
            let vc = &self.cache.v[l];
            for h in 0..n_head {
                let hk = h / ratio;
                let qh = &q[h * head_dim..(h + 1) * head_dim];
                let arow = &mut att[h * npos..(h + 1) * npos];
                let mut maxs = f32::NEG_INFINITY;
                for t in 0..npos {
                    let kt = &kc[t * kv_dim + hk * head_dim..t * kv_dim + (hk + 1) * head_dim];
                    let mut dot = 0f32;
                    for d in 0..head_dim {
                        dot += qh[d] * kt[d];
                    }
                    dot *= scale;
                    arow[t] = dot;
                    if dot > maxs {
                        maxs = dot;
                    }
                }
                let mut sum = 0f32;
                for t in 0..npos {
                    let e = (arow[t] - maxs).exp();
                    arow[t] = e;
                    sum += e;
                }
                let inv = if sum > 0.0 { 1.0 / sum } else { 0.0 };
                for d in 0..head_dim {
                    let mut acc = 0f32;
                    for t in 0..npos {
                        let vt = &vc[t * kv_dim + hk * head_dim..t * kv_dim + (hk + 1) * head_dim];
                        acc += arow[t] * inv * vt[d];
                    }
                    xb[h * head_dim + d] = acc;
                }
            }

            // 7) O proj + residual.
            self.big(&format!("{lp}.attn_output.weight"))?
                .matvec_par(&xb, &mut o);
            if let Some(bo) = self.small(&format!("{lp}.attn_output.bias")) {
                add_inplace(&mut o, bo);
            }
            for i in 0..n_embd {
                x[i] += o[i];
            }

            // 8) FFN norm.
            let ffn_norm = self
                .small(&format!("{lp}.ffn_norm.weight"))
                .ok_or_else(|| format!("{lp}.ffn_norm.weight haipo"))?;
            rmsnorm(&x, ffn_norm, eps, &mut xb);

            // 9) gate/up → SiLU(gate)*up → down.
            self.big(&format!("{lp}.ffn_gate.weight"))?
                .matvec_par(&xb, &mut gate);
            self.big(&format!("{lp}.ffn_up.weight"))?
                .matvec_par(&xb, &mut up);
            for i in 0..n_ff {
                gate[i] = silu(gate[i]) * up[i];
            }
            self.big(&format!("{lp}.ffn_down.weight"))?
                .matvec_par(&gate, &mut o);
            for i in 0..n_embd {
                x[i] += o[i];
            }
        }

        // 10) Final norm.
        let out_norm = self
            .small("output_norm.weight")
            .ok_or("output_norm.weight haipo")?;
        rmsnorm(&x, out_norm, eps, &mut xb);

        // 11) Output proj (au tied embeddings).
        let out_name = if self.g.tensor("output.weight").is_some() {
            "output.weight"
        } else {
            "token_embd.weight"
        };
        let mut logits = vec![0f32; self.cfg.n_vocab];
        self.big(out_name)?.matvec_par(&xb, &mut logits);

        self.cache.len += 1;
        Ok(logits)
    }

    /// Endesha prompt (tokeni nyingi) → logits za tokeni ya mwisho.
    pub fn prefill(&mut self, tokens: &[u32]) -> Result<Vec<f32>, String> {
        let mut logits = Vec::new();
        let start = self.cache.len;
        for (i, &t) in tokens.iter().enumerate() {
            logits = self.forward(t, start + i)?;
        }
        Ok(logits)
    }

    /// Forward ya **SPIKING** (SNN) kwa tokeni MOJA kwenye nafasi `pos`.
    ///
    /// Hubadilisha matvec ghali (MAC) kuwa **addition-only** (AC) kwa kutumia
    /// spikes (0/1), na softmax ya attention kuwa **Shiftmax** (bila `exp`).
    /// Weights HALISI za GGUF zinatumika — hakuna kupakua modeli mpya.
    ///
    /// Hurudisha `(logits, stats)`; `stats` ina spikes/AC/MAC/energy kwa UI.
    pub fn forward_spiking(
        &mut self,
        token: u32,
        pos: usize,
        scfg: &SnnConfig,
    ) -> Result<(Vec<f32>, SpikeStats), String> {
        let n_embd = self.cfg.n_embd;
        let head_dim = self.cfg.head_dim;
        let n_head = self.cfg.n_head;
        let n_kv = self.cfg.n_head_kv;
        let kv_dim = n_kv * head_dim;
        let n_ff = self.cfg.n_ff;
        let eps = self.cfg.rms_eps;
        let mut stats = SpikeStats::default();

        // 1) Embedding.
        let emb = self.big("token_embd.weight")?;
        let row = token as usize;
        if row >= emb.n_out() {
            return Err(format!("token {row} nje ya vocab"));
        }
        let mut x = vec![0f32; n_embd];
        emb.dequant_row(row, &mut x);

        let mut xb = vec![0f32; n_embd];
        let mut spikes = vec![0f32; n_embd];
        let mut spikes_ff = vec![0f32; n_ff];
        let mut q = vec![0f32; n_head * head_dim];
        let mut k = vec![0f32; kv_dim];
        let mut v = vec![0f32; kv_dim];
        let mut gate = vec![0f32; n_ff];
        let mut up = vec![0f32; n_ff];
        let mut o = vec![0f32; n_embd];
        let mut wscratch = vec![0f32; n_embd.max(n_ff)];

        // LIF kwa pembejeo (rate coding) — pembejeo ∈ ℝ → spikes.
        let mut lif = crate::snn::LifLayer::new(n_embd, scfg.clone());
        let mut lif_ff = crate::snn::LifLayer::new(n_ff, scfg.clone());

        for l in 0..self.cfg.n_layer {
            let lp = format!("blk.{l}");

            // 2) Attention norm + encode kwa spikes.
            let attn_norm = self
                .small(&format!("{lp}.attn_norm.weight"))
                .ok_or_else(|| format!("{lp}.attn_norm.weight haipo"))?;
            rmsnorm(&x, attn_norm, eps, &mut xb);
            encode_to_spikes(&mut lif, &xb, scfg, &mut spikes, &mut stats);

            // 3) QKV — ADDITION-ONLY (AC) kwa spikes.
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.attn_q.weight"))?,
                &spikes,
                &mut q,
                &mut wscratch,
                &mut stats,
            );
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.attn_k.weight"))?,
                &spikes,
                &mut k,
                &mut wscratch,
                &mut stats,
            );
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.attn_v.weight"))?,
                &spikes,
                &mut v,
                &mut wscratch,
                &mut stats,
            );

            if let Some(bq) = self.small(&format!("{lp}.attn_q.bias")) {
                add_inplace(&mut q, bq);
            }
            if let Some(bk) = self.small(&format!("{lp}.attn_k.bias")) {
                add_inplace(&mut k, bk);
            }
            if let Some(bv) = self.small(&format!("{lp}.attn_v.bias")) {
                add_inplace(&mut v, bv);
            }

            // 4) RoPE.
            rope(
                &mut q,
                &mut k,
                n_head,
                n_kv,
                head_dim,
                pos,
                self.cfg.rope_base,
                self.cfg.rope_neox,
            );

            // 5) Andika K,V kwenye cache.
            let cl = self.cache.len;
            self.cache.k[l][cl * kv_dim..(cl + 1) * kv_dim].copy_from_slice(&k);
            self.cache.v[l][cl * kv_dim..(cl + 1) * kv_dim].copy_from_slice(&v);

            // 6) Attention ya bipolar: Q·K (MAC) → Shiftmax (bila exp) → ×V.
            let scale = 1.0 / (head_dim as f32).sqrt();
            let npos = cl + 1;
            let ratio = n_head / n_kv;
            let kc = &self.cache.k[l];
            let vc = &self.cache.v[l];
            for h in 0..n_head {
                let hk = h / ratio;
                let qh = &q[h * head_dim..(h + 1) * head_dim];
                // Tengeneza K na V za head h kwa positions zote.
                let mut kh = vec![0f32; npos * head_dim];
                let mut vh = vec![0f32; npos * head_dim];
                for t in 0..npos {
                    kh[t * head_dim..t * head_dim + head_dim]
                        .copy_from_slice(&kc[t * kv_dim + hk * head_dim..t * kv_dim + (hk + 1) * head_dim]);
                    vh[t * head_dim..t * head_dim + head_dim]
                        .copy_from_slice(&vc[t * kv_dim + hk * head_dim..t * kv_dim + (hk + 1) * head_dim]);
                }
                let out = attention_out(
                    qh,
                    &kh,
                    &vh,
                    npos,
                    head_dim,
                    scale,
                    scfg.shiftmax_beta,
                    &mut stats,
                );
                xb[h * head_dim..(h + 1) * head_dim].copy_from_slice(&out);
            }

            // 7) O proj — ADDITION-ONLY kwa spikes za attention output.
            encode_to_spikes(&mut lif, &xb, scfg, &mut spikes, &mut stats);
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.attn_output.weight"))?,
                &spikes,
                &mut o,
                &mut wscratch,
                &mut stats,
            );
            if let Some(bo) = self.small(&format!("{lp}.attn_output.bias")) {
                add_inplace(&mut o, bo);
            }
            for i in 0..n_embd {
                x[i] += o[i];
            }

            // 8) FFN norm + encode.
            let ffn_norm = self
                .small(&format!("{lp}.ffn_norm.weight"))
                .ok_or_else(|| format!("{lp}.ffn_norm.weight haipo"))?;
            rmsnorm(&x, ffn_norm, eps, &mut xb);
            encode_to_spikes(&mut lif, &xb, scfg, &mut spikes, &mut stats);

            // 9) gate/up — ADDITION-ONLY; SiLU(gate)*up (multiplication NDOGO).
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.ffn_gate.weight"))?,
                &spikes,
                &mut gate,
                &mut wscratch,
                &mut stats,
            );
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.ffn_up.weight"))?,
                &spikes,
                &mut up,
                &mut wscratch,
                &mut stats,
            );
            for i in 0..n_ff {
                gate[i] = silu(gate[i]) * up[i];
            }
            stats.macs += n_ff as u64; // SiLU*up ni multiplications (ndogo)
            // Down proj — encode gate→spikes (n_ff), ADDITION-ONLY.
            encode_to_spikes(&mut lif_ff, &gate, scfg, &mut spikes_ff, &mut stats);
            crate::snn::addition_only_matvec(
                &self.big(&format!("{lp}.ffn_down.weight"))?,
                &spikes_ff,
                &mut o,
                &mut wscratch,
                &mut stats,
            );
            for i in 0..n_embd {
                x[i] += o[i];
            }
        }

        // 10) Final norm.
        let out_norm = self
            .small("output_norm.weight")
            .ok_or("output_norm.weight haipo")?;
        rmsnorm(&x, out_norm, eps, &mut xb);

        // 11) Output proj (au tied embeddings) — ADDITION-ONLY kwa spikes.
        encode_to_spikes(&mut lif, &xb, scfg, &mut spikes, &mut stats);
        let out_name = if self.g.tensor("output.weight").is_some() {
            "output.weight"
        } else {
            "token_embd.weight"
        };
        let mut logits = vec![0f32; self.cfg.n_vocab];
        crate::snn::addition_only_matvec(
            &self.big(out_name)?,
            &spikes,
            &mut logits,
            &mut wscratch,
            &mut stats,
        );

        self.cache.len += 1;
        Ok((logits, stats))
    }

    /// Prefill ya spiking (prompt → logits + stats zilizojumlishwa).
    pub fn prefill_spiking(
        &mut self,
        tokens: &[u32],
        scfg: &SnnConfig,
    ) -> Result<(Vec<f32>, SpikeStats), String> {
        let mut logits = Vec::new();
        let mut total = SpikeStats::default();
        let start = self.cache.len;
        for (i, &t) in tokens.iter().enumerate() {
            let (l, s) = self.forward_spiking(t, start + i, scfg)?;
            logits = l;
            total.absorb(&s);
        }
        Ok((logits, total))
    }
}

/// Encode vekta ya f32 → spikes (0/1) kwa LIF + **k-WTA** (k-winners-take-all).
///
/// Rate coding inahitaji pembejeo ∈ [0,1]: kwanza ReLU (spikes ni chanya),
/// kisha normalization kwa max. LIF huendesha `t_steps` → rate ∈ [0,1].
/// k-WTA huchagua neurons `(1 - sparsity_target)·n` zenye rate kubwa zaidi —
/// hii inadhibiti sparsity (na hivyo nishati) kwa uhakika. Hii ni mbinu halisi
/// ya SNN (winner-take-all), si bandia.
fn encode_to_spikes(
    lif: &mut crate::snn::LifLayer,
    x: &[f32],
    scfg: &SnnConfig,
    out: &mut [f32],
    stats: &mut SpikeStats,
) {
    let n = x.len();
    for o in out.iter_mut() {
        *o = 0.0;
    }
    if n == 0 {
        return;
    }
    // ReLU + normalization (fail-safe: max=0 → spikes zote sifuri).
    let mut act = vec![0f32; n];
    let mut mx = 0f32;
    for i in 0..n {
        let v = x[i].max(0.0);
        act[i] = v;
        if v > mx {
            mx = v;
        }
    }
    if mx <= 1e-9 {
        return;
    }
    let inv = 1.0 / mx;
    for v in act.iter_mut() {
        *v *= inv;
    }

    if lif.len() != n {
        *lif = crate::snn::LifLayer::new(n, scfg.clone());
    }
    // run() huongeza spikes = idadi ya neurons zenye rate>0; tutaibadilisha kwa
    // idadi HALISI ya spikes za k-WTA (sparsity ndiyo inayodhibiti nishati).
    let before = stats.spikes;
    let rate = lif.run(&act, scfg.t_steps, stats).to_vec();

    // k-WTA: chagua k = (1 - sparsity)·n neurons zenye rate kubwa.
    let keep = (((1.0 - scfg.sparsity_target).clamp(0.01, 1.0) * n as f32).round() as usize)
        .clamp(1, n);
    let mut idx: Vec<usize> = (0..n).collect();
    // Sort kwa rate (descending); n ni ndogo kwa tabaka hizi (hundi elfu).
    idx.sort_unstable_by(|&a, &b| {
        rate[b]
            .partial_cmp(&rate[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for &i in idx.iter().take(keep) {
        if rate[i] > 0.0 {
            out[i] = 1.0;
        }
    }
    stats.spikes = before + out.iter().filter(|v| **v > 0.5).count() as u64;
}

fn add_inplace(x: &mut [f32], b: &[f32]) {
    for i in 0..x.len().min(b.len()) {
        x[i] += b[i];
    }
}

/// Chaguo la quantization linaungwa mkono.
pub fn supported_quant(t: GgmlType) -> bool {
    matches!(
        t,
        GgmlType::F32
            | GgmlType::F16
            | GgmlType::Bf16
            | GgmlType::Q4_0
            | GgmlType::Q4_1
            | GgmlType::Q5_0
            | GgmlType::Q5_1
            | GgmlType::Q8_0
            | GgmlType::Q8_1
            | GgmlType::Q2K
            | GgmlType::Q3K
            | GgmlType::Q4K
            | GgmlType::Q5K
            | GgmlType::Q6K
            | GgmlType::Q8K
    )
}
