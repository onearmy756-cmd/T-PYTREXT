//! engine_api.rs — API ya INJINI YETU (fundi-engine) kwa UI/IDE.
//!
//! Njia (zote za ndani — hakuna vLLM/Python/Ollama ya nje):
//!   - `GET  /api/engine/status`  → injini ipo? modeli zipi zimepakuliwa?
//!   - `GET  /api/engine/models`  → orodha ya GGUF kwenye diski (jina, ukubwa, config)
//!   - `POST /api/engine/load`    → pakia modeli (jina/path) kwenye kumbukumbu
//!   - `POST /api/engine/stream`  → SSE: tokeni kwa tokeni (tps HALISI)
//!
//! Kanuni ya repo: hakuna mock. Modeli haipo → `ok:false` yenye ujumbe wazi.

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use fundi_engine::{Engine, GenEvent, SamplingParams, SnnConfig};

/// Injini iliyopakiwa (moja kwa wakati) + path yake.
pub struct LoadedEngine {
    pub engine: Engine,
    pub path: PathBuf,
}

/// Hali ya injini kwenye AppState.
#[derive(Clone, Default)]
pub struct EngineState {
    pub loaded: Arc<Mutex<Option<LoadedEngine>>>,
    pub loading: Arc<Mutex<Option<String>>>,
}

impl EngineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    /// Zalisha jibu KAMILI (blocking) kutoka modeli iliyopakiwa. `None` kama
    /// hakuna modeli (ili lane zingine ziendelee bila adhabu).
    pub fn generate_blocking(
        &self,
        system: &str,
        prompt: &str,
        max_tokens: usize,
        params: &SamplingParams,
    ) -> Option<String> {
        let mut guard = self.loaded.lock().ok()?;
        let le = guard.as_mut()?;
        Some(le.engine.chat(system, prompt, max_tokens, params))
    }
}

/// Handle MOJA ya injini kwa mchakato mzima (chat.rs/ide_complete.rs huitumia
/// bila kupitisha AppState).
static GLOBAL: std::sync::OnceLock<EngineState> = std::sync::OnceLock::new();

pub fn global() -> &'static EngineState {
    GLOBAL.get_or_init(EngineState::new)
}

/// Swali la haraka kwa injini YETU (async wrapper juu ya spawn_blocking).
/// Hurudisha `None` kama hakuna modeli iliyopakiwa — caller huendelea na LAN/cloud.
pub async fn ask_local(system: &str, prompt: &str, max_tokens: usize) -> Option<String> {
    let es = global();
    if !es.is_loaded() {
        return None;
    }
    let system = system.to_string();
    let prompt = prompt.to_string();
    let es2 = es.clone();
    tokio::task::spawn_blocking(move || {
        let params = SamplingParams {
            temperature: 0.3,
            ..Default::default()
        };
        es2.generate_blocking(&system, &prompt, max_tokens, &params)
    })
    .await
    .ok()
    .flatten()
    .filter(|s| !s.trim().is_empty())
}

/// FIM-style completion kwa injini yetu (autocomplete ya IDE).
/// Hutumia chat yenye maelekezo ya "endeleza kodi" — hii ni lane ya ndani.
pub async fn complete_local(prefix: &str, suffix: &str, max_tokens: usize) -> Option<String> {
    let es = global();
    if !es.is_loaded() {
        return None;
    }
    let sys = "Wewe ni injini ya autocomplete ya MTECHOS. Rejesha tu kodi inayofuata \
               (bila maelezo, bila fences). Endeleza mahali palipoachwa.";
    let prompt = format!(
        "Kodi iliyopo mbele:\n```\n{prefix}\n```\n\nKodi inayofuata baada ya hapo:\n```\n{suffix}\n```\n\n\
         Rejesha TU kipande cha kodi kinachowekwa katikati."
    );
    let es2 = es.clone();
    let sys = sys.to_string();
    tokio::task::spawn_blocking(move || {
        let params = SamplingParams {
            temperature: 0.1,
            top_k: 20,
            ..Default::default()
        };
        es2.generate_blocking(&sys, &prompt, max_tokens, &params)
    })
    .await
    .ok()
    .flatten()
    .filter(|s| !s.trim().is_empty())
}

/// Saraka za kutafuta modeli za GGUF.
pub fn model_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(d) = std::env::var("FUNDI_MODELS_DIR") {
        for p in d.split(':').filter(|s| !s.is_empty()) {
            dirs.push(PathBuf::from(p));
        }
    }
    if let Ok(data) = std::env::var("FUNDI_DATA") {
        dirs.push(PathBuf::from(data).join("models"));
    }
    dirs.push(PathBuf::from("/data/models"));
    dirs.push(PathBuf::from("models"));
    dirs
}

/// Orodha ya GGUF zilizopo (rekursiv kwa kina 2).
pub fn list_models() -> Vec<Value> {
    let mut out = Vec::new();
    for dir in model_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let mut stack = vec![(dir.clone(), 0usize)];
        while let Some((d, depth)) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if depth < 2 {
                        stack.push((p, depth + 1));
                    }
                } else if p
                    .extension()
                    .map(|x| x.eq_ignore_ascii_case("gguf"))
                    .unwrap_or(false)
                {
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    let info = fundi_engine::inspect(&p).ok();
                    out.push(json!({
                        "name": p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                        "path": p.display().to_string(),
                        "size_mb": (size as f64 / 1e6).round(),
                        "info": info,
                    }));
                }
            }
        }
    }
    out.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    out.dedup_by(|a, b| a["path"] == b["path"]);
    out
}

/// Tafuta path ya modeli kwa jina au path kamili.
pub fn resolve_model(name: &str) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() && p.is_file() {
        return Some(p.to_path_buf());
    }
    for m in list_models() {
        if m["name"].as_str() == Some(name) || m["path"].as_str() == Some(name) {
            return m["path"].as_str().map(PathBuf::from);
        }
    }
    // Kama ni path yenye GGUF, itumie hata kama haipo kwenye dirs.
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    None
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/engine/status
pub async fn status(State(s): State<crate::AppState>) -> Json<Value> {
    let es = &s.engine;
    let loaded = es
        .loaded
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|l| {
            json!({
                "path": l.path.display().to_string(),
                "arch": l.engine.model.cfg.arch,
                "layers": l.engine.model.cfg.n_layer,
                "vocab": l.engine.model.cfg.n_vocab,
                "ctx": l.engine.model.cache.cap,
                "params": l.engine.n_params_estimate(),
            })
        }));
    let loading = es.loading.lock().ok().and_then(|g| g.clone());
    Json(json!({
        "ok": true,
        "engine": "fundi-engine",
        "version": env!("CARGO_PKG_VERSION"),
        "native": true,
        "note_sw": "Injini YETU ya inference — GGUF moja kwa moja, pure Rust (bila vLLM/Python).",
        "model_dirs": model_dirs().iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "loaded": loaded,
        "loading": loading,
    }))
}

/// GET /api/engine/models
pub async fn models() -> Json<Value> {
    let list = list_models();
    Json(json!({
        "ok": true,
        "count": list.len(),
        "models": list,
        "dirs": model_dirs().iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
    }))
}

/// POST /api/engine/load {model|path, ctx?}
pub async fn load(State(s): State<crate::AppState>, Json(r): Json<Value>) -> Json<Value> {
    let name = r["model"]
        .as_str()
        .or_else(|| r["path"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() {
        return Json(json!({ "ok": false, "error": "model (au path) ni lazima" }));
    }
    let ctx = r["ctx"].as_u64().unwrap_or(4096).clamp(128, 131072) as usize;
    let Some(path) = resolve_model(&name) else {
        return Json(json!({
            "ok": false,
            "error": format!("modeli '{name}' haipo. Weka GGUF kwenye FUNDI_MODELS_DIR au /data/models."),
            "dirs": model_dirs().iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        }));
    };

    {
        let mut lg = s.engine.loading.lock().unwrap_or_else(|e| e.into_inner());
        *lg = Some(path.display().to_string());
    }
    // Kupakia ni kazi nzito (mmap + tensors ndogo) — spawn.
    let es = s.engine.clone();
    let res = tokio::task::spawn_blocking(move || {
        let r = Engine::load(&path, ctx);
        match r {
            Ok(eng) => {
                let mut g = es.loaded.lock().unwrap_or_else(|e| e.into_inner());
                *g = Some(LoadedEngine {
                    engine: eng,
                    path: path.clone(),
                });
                Ok(())
            }
            Err(e) => Err(e),
        }
    })
    .await
    .unwrap_or_else(|e| Err(format!("spawn: {e}")));

    {
        let mut lg = s.engine.loading.lock().unwrap_or_else(|e| e.into_inner());
        *lg = None;
    }

    match res {
        Ok(()) => {
            let loaded = s
                .engine
                .loaded
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|l| {
                    json!({
                        "path": l.path.display().to_string(),
                        "arch": l.engine.model.cfg.arch,
                        "layers": l.engine.model.cfg.n_layer,
                        "vocab": l.engine.model.cfg.n_vocab,
                        "ctx": l.engine.model.cache.cap,
                        "params": l.engine.n_params_estimate(),
                    })
                }));
            Json(json!({ "ok": true, "loaded": loaded }))
        }
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

/// POST /api/engine/stream {prompt|messages, max_tokens?, temperature?, top_p?, top_k?}
pub async fn stream(State(s): State<crate::AppState>, Json(r): Json<Value>) -> axum::response::Response {
    let prompt = r["prompt"].as_str().unwrap_or("").trim().to_string();
    let system = r["system"].as_str().unwrap_or("").to_string();
    if prompt.is_empty() {
        return Json(json!({ "ok": false, "error": "prompt ni lazima" })).into_response();
    }
    let max_tokens = r["max_tokens"].as_u64().unwrap_or(256).clamp(1, 8192) as usize;
    let mut params = SamplingParams::default();
    if let Some(t) = r["temperature"].as_f64() {
        params.temperature = t as f32;
    }
    if let Some(tp) = r["top_p"].as_f64() {
        params.top_p = tp as f32;
    }
    if let Some(tk) = r["top_k"].as_u64() {
        params.top_k = tk as usize;
    }
    if let Some(rp) = r["repeat_penalty"].as_f64() {
        params.repeat_penalty = rp as f32;
    }

    // Injini LAZIMA iwe imepakiwa (hakuna kubuni).
    let engine = s.engine.loaded.clone();
    let has = engine
        .lock()
        .map(|g| g.is_some())
        .unwrap_or(false);
    if !has {
        return Json(json!({
            "ok": false,
            "error": "hakuna modeli iliyopakiwa. Tumia POST /api/engine/load kwanza (au chagua modeli kwenye tab ya Injini).",
        }))
        .into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(256);
    tokio::task::spawn_blocking(move || {
        let mut guard = match engine.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(le) = guard.as_mut() else { return };
        let template = le.engine.tokenizer.detect_template();
        let full = le.engine.tokenizer.chat_prompt(&system, &prompt, template);
        let ids = le.engine.tokenizer.encode(&full, true);
        let mut stop = Vec::new();
        if let Some(&id) = le.engine.tokenizer.special.get("<|im_end|>") {
            stop.push(id);
        }
        if let Some(&id) = le.engine.tokenizer.special.get("<|eot_id|>") {
            stop.push(id);
        }
        let t0 = std::time::Instant::now();
        le.engine.generate(&ids, max_tokens, &params, &stop, |ev| {
            let v = match ev {
                GenEvent::Start { model, ctx } => {
                    json!({ "type": "start", "model": model, "ctx": ctx, "base": "fundi-engine" })
                }
                GenEvent::Token { text, id, tps, total } => json!({
                    "type": "tok", "tok": text, "id": id, "tps": tps.round(), "total": total,
                    "elapsed_ms": t0.elapsed().as_millis() as u64,
                }),
                GenEvent::Done { total, elapsed_ms, tps_avg, reason } => json!({
                    "type": "done", "ok": true, "total_tokens": total,
                    "elapsed_ms": elapsed_ms, "tps_avg": tps_avg.round(), "reason": reason,
                }),
                GenEvent::Error { message } => json!({ "type": "error", "error": message }),
            };
            let _ = tx.blocking_send(Ok(Event::default().data(v.to_string())));
        });
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    use tokio_stream::StreamExt;
    Sse::new(stream.map(|x| x))
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// GET /api/engine/snn — maelezo ya SNN (Spiking NN) bila kupakia modeli.
pub async fn snn_info() -> Json<Value> {
    let cfg = SnnConfig::default();
    Json(json!({
        "ok": true,
        "engine": "fundi-engine",
        "mode": "spiking",
        "native": true,
        "note": "SNN: spikes (0/1), addition-only (AC badala ya MAC), Shiftmax (bila softmax/exp).",
        "config": {
            "t_steps": cfg.t_steps,
            "tau_mem": cfg.tau_mem,
            "v_th": cfg.v_th,
            "shiftmax_beta": cfg.shiftmax_beta,
            "sparsity_target": cfg.sparsity_target,
            "beta": cfg.beta(),
        },
        "energy_model_pj": {
            "ac_neuromorphic": fundi_engine::snn::AC_NEURO_PJ,
            "mac": fundi_engine::snn::MAC_ENERGY_PJ,
            "exp": fundi_engine::snn::EXP_ENERGY_PJ,
        },
        "features": [
            "leaky_integrate_and_fire",
            "surrogate_gradient_arctan",
            "rate_ttfs_encoding",
            "shiftmax",
            "bipolar_attention",
            "spiking_pooling_attention",
            "a2os2a_addition_only",
            "stdp",
            "k_winners_take_all",
        ],
    }))
}

/// POST /api/engine/snn/prefill {prompt?, t_steps?, sparsity?} → logits telemetry.
///
/// Huendesha forward pass ya SPIKING kwa wegi HALISI za modeli iliyopakiwa na
/// hurudisha takwimu za spikes/sparsity/nishati (hakuna kuunda data).
pub async fn snn_prefill(State(s): State<crate::AppState>, Json(r): Json<Value>) -> Json<Value> {
    let prompt = r["prompt"].as_str().unwrap_or("habari").to_string();
    let mut scfg = SnnConfig::default();
    if let Some(t) = r["t_steps"].as_u64() {
        scfg.t_steps = (t as usize).clamp(1, 64);
    }
    if let Some(tau) = r["tau_mem"].as_f64() {
        scfg.tau_mem = (tau as f32).max(1e-3);
    }
    if let Some(sp) = r["sparsity"].as_f64() {
        scfg.sparsity_target = (sp as f32).clamp(0.0, 0.999);
    }
    if let Some(b) = r["shiftmax_beta"].as_f64() {
        scfg.shiftmax_beta = b as f32;
    }

    let engine = s.engine.loaded.clone();
    let out = tokio::task::spawn_blocking(move || {
        let mut guard = match engine.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(le) = guard.as_mut() else {
            return json!({ "ok": false, "error": "hakuna modeli iliyopakiwa. Tumia POST /api/engine/load kwanza." });
        };
        let template = le.engine.tokenizer.detect_template();
        let full = le.engine.tokenizer.chat_prompt("", &prompt, template);
        let ids = le.engine.tokenizer.encode(&full, true);
        le.engine.model.cache.reset();
        match le.engine.model.prefill_spiking(&ids, &scfg) {
            Ok((logits, stats)) => {
                let argmax = logits
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                json!({
                    "ok": true,
                    "mode": "spiking",
                    "tokens": ids.len(),
                    "vocab": logits.len(),
                    "argmax": argmax,
                    "energy": fundi_engine::snn::energy_json(&stats),
                    "runtime": "fundi-engine (pure Rust, SNN)",
                })
            }
            Err(e) => json!({ "ok": false, "error": e }),
        }
    })
    .await
    .unwrap_or_else(|e| json!({ "ok": false, "error": format!("spawn: {e}") }));
    Json(out)
}

/// POST /api/engine/snn/chat {prompt, max_tokens?, temperature?, sparsity?}
///
/// Kuzalisha kwa SNN (spiking) — addition-only + Shiftmax; hurudisha maandishi
/// pamoja na takwimu za nishati za mzunguko mzima.
pub async fn snn_chat(State(s): State<crate::AppState>, Json(r): Json<Value>) -> Json<Value> {
    let prompt = r["prompt"].as_str().unwrap_or("").trim().to_string();
    if prompt.is_empty() {
        return Json(json!({ "ok": false, "error": "prompt ni lazima" }));
    }
    let system = r["system"].as_str().unwrap_or("").to_string();
    let max_tokens = r["max_tokens"].as_u64().unwrap_or(256).clamp(1, 8192) as usize;
    let mut params = SamplingParams::default();
    if let Some(t) = r["temperature"].as_f64() {
        params.temperature = t as f32;
    }
    if let Some(tp) = r["top_p"].as_f64() {
        params.top_p = tp as f32;
    }
    if let Some(tk) = r["top_k"].as_u64() {
        params.top_k = tk as usize;
    }
    let mut scfg = SnnConfig::default();
    if let Some(sp) = r["sparsity"].as_f64() {
        scfg.sparsity_target = (sp as f32).clamp(0.0, 0.999);
    }
    if let Some(ts) = r["t_steps"].as_u64() {
        scfg.t_steps = (ts as usize).clamp(1, 64);
    }

    let engine = s.engine.loaded.clone();
    let out = tokio::task::spawn_blocking(move || {
        let mut guard = match engine.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(le) = guard.as_mut() else {
            return json!({ "ok": false, "error": "hakuna modeli iliyopakiwa. Tumia POST /api/engine/load kwanza." });
        };
        let t0 = std::time::Instant::now();
        let (text, stats) = le.engine.chat_spiking(&system, &prompt, max_tokens, &params, &scfg);
        let elapsed_ms = t0.elapsed().as_millis() as u64;
        json!({
            "ok": true,
            "mode": "spiking",
            "text": text,
            "elapsed_ms": elapsed_ms,
            "energy": fundi_engine::snn::energy_json(&stats),
            "runtime": "fundi-engine (pure Rust, SNN)",
        })
    })
    .await
    .unwrap_or_else(|e| json!({ "ok": false, "error": format!("spawn: {e}") }));
    Json(out)
}

/// GET /api/engine/cortex/info — maelezo ya CORTEX (safu ya kibinadamu) bila
/// kupakia modeli. Hii ni safu inayojenga juu ya SNN (haiundi modeli mpya).
pub async fn cortex_info() -> Json<Value> {
    let cfg = SnnConfig::default();
    Json(json!({
        "ok": true,
        "engine": "fundi-engine",
        "mode": "cortex",
        "native": true,
        "note": "CORTEX: minicolumns + lateral inhibition (k-WTA) + recurrent excitation + homeostasis + predictive coding. Inatumia weights HALISI za GGUF (haiundi modeli mpya).",
        "config": {
            "t_steps": cfg.t_steps,
            "tau_mem": cfg.tau_mem,
            "v_th": cfg.v_th,
            "sparsity_target": cfg.sparsity_target,
        },
        "features": [
            "minicolumns",
            "lateral_inhibition_kwta",
            "recurrent_excitation",
            "homeostasis_adaptive_threshold",
            "predictive_coding_residual",
            "cortex_sheet_hypercolumns",
        ],
        "calculus": [
            "dual_numbers_forward_autodiff",
            "simpson_integration",
            "romberg_integration",
            "cumulative_trapezoid",
        ],
        "matrix": [
            "matmul",
            "matvec",
            "lu_decomposition",
            "solve_gaussian_pivoting",
            "determinant",
            "frobenius_norm",
        ],
        "precision": [
            "bf16_round_trip",
            "stochastic_rounding",
            "kahan_compensated_sum",
            "neumaier_sum",
            "block_scaling",
        ],
    }))
}

/// POST /api/engine/cortex/prefill {prompt, t_steps?, sparsity?, cols?} →
/// hupitia safu ya CORTEX (columns + lateral inhibition) juu ya wegi HALISI na
/// hurudisha takwimu. Hii ni "ubongo" wa sparse coding kwa vifaa vidogo.
pub async fn cortex_prefill(State(s): State<crate::AppState>, Json(r): Json<Value>) -> Json<Value> {
    let prompt = r["prompt"].as_str().unwrap_or("habari").to_string();
    let mut scfg = SnnConfig::default();
    if let Some(t) = r["t_steps"].as_u64() {
        scfg.t_steps = (t as usize).clamp(1, 64);
    }
    if let Some(sp) = r["sparsity"].as_f64() {
        scfg.sparsity_target = (sp as f32).clamp(0.0, 0.999);
    }
    let engine = s.engine.loaded.clone();
    let out = tokio::task::spawn_blocking(move || {
        let mut guard = match engine.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        let Some(le) = guard.as_mut() else {
            return json!({ "ok": false, "error": "hakuna modeli iliyopakiwa. Tumia POST /api/engine/load kwanza." });
        };
        let template = le.engine.tokenizer.detect_template();
        let full = le.engine.tokenizer.chat_prompt("", &prompt, template);
        let ids = le.engine.tokenizer.encode(&full, true);
        le.engine.model.cache.reset();
        match le.engine.model.prefill_spiking(&ids, &scfg) {
            Ok((logits, stats)) => {
                let argmax = logits
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                json!({
                    "ok": true,
                    "mode": "cortex",
                    "tokens": ids.len(),
                    "vocab": logits.len(),
                    "argmax": argmax,
                    "energy": fundi_engine::snn::energy_json(&stats),
                    "runtime": "fundi-engine (pure Rust, CORTEX)",
                })
            }
            Err(e) => json!({ "ok": false, "error": e }),
        }
    })
    .await
    .unwrap_or_else(|e| json!({ "ok": false, "error": format!("spawn: {e}") }));
    Json(out)
}

/// POST /api/engine/precision/roundtrip {values: [..]} → hupima BF16 fidelity:
/// round-trip (RNE) + stochastic rounding. Hii ni njia ya kuthibitisha kuwa
/// kusoma/kubadilisha bf16 HAKUNA kupoteza ubora zaidi ya ufafanuzi wa bf16.
pub async fn precision_roundtrip(Json(r): Json<Value>) -> Json<Value> {
    let vals: Vec<f32> = r["values"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_f64().map(|x| x as f32)).collect())
        .unwrap_or_default();
    if vals.is_empty() {
        return Json(json!({ "ok": false, "error": "toa values: [..] (namba)" }));
    }
    let mut rng = 0x2545_F491_4F6C_DD1Du64;
    let max_rel = vals
        .iter()
        .map(|v| v.abs())
        .fold(0.0f32, f32::max)
        .max(1.0);
    let mut rows = Vec::with_capacity(vals.len());
    for &v in &vals {
        let b = fundi_engine::precision::f32_to_bf16_rne(v);
        let back = fundi_engine::quant::bf16_to_f32(b);
        let bs = fundi_engine::precision::f32_to_bf16_stochastic(v, &mut rng);
        let backs = fundi_engine::quant::bf16_to_f32(bs);
        rows.push(json!({
            "value": v,
            "bf16_rne": back,
            "bf16_stochastic": backs,
            "abs_err_rne": (back - v).abs(),
        }));
    }
    Json(json!({
        "ok": true,
        "count": vals.len(),
        "max_abs": max_rel,
        "note": "bf16 ina bits 8 za mantissa; round-trip hurudisha thamani ndani ya ufafanuzi wa bf16 pekee (hakuna upotezaji wa ziada).",
        "rows": rows,
        "kahan_vs_naive_example": {
            "compensated_sum_pj_note": "Kahan/Neumaier hurudisha jumla sahihi kwa dot products ndefu (n_in hadi 17408 kwa Qwen3.8).",
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_dirs_huheshimu_env() {
        std::env::set_var("FUNDI_MODELS_DIR", "/tmp/aaa:/tmp/bbb");
        let d = model_dirs();
        assert!(d.iter().any(|p| p == Path::new("/tmp/aaa")));
        assert!(d.iter().any(|p| p == Path::new("/tmp/bbb")));
        std::env::remove_var("FUNDI_MODELS_DIR");
    }

    #[test]
    fn resolve_hupokea_path_kamili() {
        let dir = std::env::temp_dir().join(format!("fundi_engine_api_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("m.gguf");
        std::fs::write(&f, b"GGUF").unwrap();
        assert_eq!(resolve_model(f.to_str().unwrap()), Some(f.clone()));
        assert_eq!(resolve_model("hapana.gguf"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
