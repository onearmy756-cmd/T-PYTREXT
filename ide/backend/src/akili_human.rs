//! akili_human.rs — DARAJA LA KIBINADAMU (Akili-V4 × Qwen3.8-27B)
//!
//! Hapa ndipo "muundo wa kibinadamu" unakamilika: tabaka 12 za `akili_v4`
//! (Attention, Perception, System1/2, Subconscious, Intuition, Imagination,
//! Emotions, Social, Ethics, Meaning, Identity) hufikiri KWANZA na kutoa
//! "hali ya ndani" (hisia, tumbo, dhamiri, hatua). Hali hiyo HUONGOZA injini
//! ya lugha (Qwen3.8-27B kwa llama-server) — kama ubongo wa mwanadamu
//! unavyoongoza mdomo. Fikira = akili_v4; lugha = Qwen3.8-27B.
//!
//! Endpoints (main.rs):
//!   GET  /api/akili/human/status → hali (LLM ipo? modeli? tabaka?)
//!   POST /api/akili/human/think  → { input } → hali ya ndani + jibu la lugha
//!
//! Env:
//!   FUNDI_HUMAN_LLM_URL   default http://127.0.0.1:8081  (llama-server)
//!   FUNDI_HUMAN_MODEL     default qwen3.8-27b
//!   FUNDI_HUMAN_BRIDGE    "1" huwasha lane hii kwenye chat (llm_ask)
//!
//! Haibadilishi muundo wa directory; ni nyongeza (add-only).

use axum::Json;
use serde_json::{json, Value};

/// Base ya llama-server inayobeba Qwen3.8-27B.
pub fn llm_base() -> String {
    std::env::var("FUNDI_HUMAN_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".into())
        .trim_end_matches('/')
        .to_string()
}

/// Jina la modeli ya lugha (llama-server huipuuza; ni kwa uoanifu wa OpenAI).
pub fn llm_model() -> String {
    std::env::var("FUNDI_HUMAN_MODEL").unwrap_or_else(|_| "qwen3.8-27b".into())
}

/// Lane ya kibinadamu imewashwa kwenye chat?
pub fn bridge_enabled() -> bool {
    matches!(
        std::env::var("FUNDI_HUMAN_BRIDGE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// llama-server inapatikana? (health check ya haraka).
pub async fn llm_alive() -> bool {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(format!("{}/health", llm_base()))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Hali ya ndani ya kibinadamu (tabaka 12) kwa swali/ombi.
/// Hii ni fikira PEKEE — haigusi mtandao; `akili_v4::decide` huitwa.
pub fn human_state(input: &str) -> Value {
    crate::akili_v4::decide(input)
}

/// Kiraka cha mfumo kinachompa lugha "hali ya ndani" ya mwanadamu.
/// Lugha HAITAJI kuiga hisia — inapewa hisia HALISI zilizohesabiwa na tabaka.
pub fn human_system_prompt(state: &Value, context: &str, len_instruction: &str) -> String {
    let d = &state["decision"];
    let action = d["action"].as_str().unwrap_or("");
    let used = d["used_system"].as_str().unwrap_or("system2");
    let conf = d["confidence"].as_f64().unwrap_or(0.5);
    let allowed = d["ethical_allowed"].as_bool().unwrap_or(true);
    let hitl = d["requires_hitl"].as_bool().unwrap_or(false);

    let emo = &state["emotion"];
    let emo_kind = emo["kind"].as_str().unwrap_or("Neutral");
    let emo_int = emo["intensity"].as_f64().unwrap_or(0.3);
    let mood = emo["mood"].as_str().unwrap_or("tulivu");

    let gut = &state["intuition"];
    let gut_sig = gut["gut_signal"].as_f64().unwrap_or(0.0);
    let gut_expl = gut["explanation"].as_str().unwrap_or("");

    let conscience = state["ethics"]["conscience"].as_str().unwrap_or("");
    let steps: Vec<String> = state["thought_steps"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let layers = d["layers_activated"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    format!(
        "Wewe ni akili ya MTECHOS yenye MUUNDO WA KIBINADAMU (Akili-V4). \
Fikira yako hupitia tabaka 12: attention, perception, emotions, intuition, \
system1/system2, imagination, social, ethics, meaning, subconscious, identity.\n\n\
HALI YAKO YA NDANI (imehesabiwa KABLA ya wewe kuandika — iheshimu):\n\
- Tabaka zilizoamka: {layers}\n\
- Hisia: {emo_kind} (kiwango {emo_int:.2}, hali: {mood})\n\
- Tumbo (intuition): {gut_sig:+.2} — {gut_expl}\n\
- Mfumo uliotumika: {used} (kujiamini {conf:.2})\n\
- Hatua iliyopangwa: {action}\n\
- Dhamiri (ethics): {conscience} (ruhusa={allowed}, HITL={hitl})\n\
- Mfululizo wa fikira (System2): {steps}\n\n\
JINSI YA KUTUMIA HALI HII: onyesha fikira ya kibinadamu — hisia zako, tumbo lako, \
dhamiri yako — kwa lugha ya asili. Si hisia za kubuni: hizi ni HALISI kutoka tabaka zako. \
Jibu kwa {len_instruction}. Jibu kwa maandishi safi ya Kiswahili.\n\n\
Muktadha (kumbukumbu za suluhisho za awali):\n{context}",
        steps = if steps.is_empty() {
            "(hakuna — System1 ya haraka)".to_string()
        } else {
            steps.join(" | ")
        },
    )
}

/// Fikiri kwa muundo wa kibinadamu, kisha amuru lugha (Qwen3.8-27B) kujibu.
/// Hurudisha (hali_ya_ndani, jibu_la_lugha). Jibu `None` kama LLM haipatikani.
pub async fn human_think(
    question: &str,
    context: &str,
    len_instruction: &str,
    max_tokens: usize,
    timeout_s: u64,
) -> (Value, Option<String>) {
    let state = human_state(question);
    let sys = human_system_prompt(&state, context, len_instruction);
    let base = llm_base();
    let url = format!("{}/v1/chat/completions", base);
    let body = json!({
        "model": llm_model(),
        "messages": [
            { "role": "system", "content": sys },
            { "role": "user", "content": question }
        ],
        "stream": false,
        "max_tokens": max_tokens,
        "temperature": 0.7,
        "chat_template_kwargs": { "enable_thinking": false }
    });
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_s))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (state, None),
    };
    let answer = match client.post(&url).json(&body).send().await {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) => v["choices"][0]["message"]["content"]
                .as_str()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            Err(_) => None,
        },
        _ => None,
    };
    (state, answer)
}

// ── Axum handlers ────────────────────────────────────────────────────────

/// GET /api/akili/human/status — LLM ipo? modeli? muundo wa kibinadamu?
pub async fn human_status() -> Json<Value> {
    let base = llm_base();
    let reachable = llm_alive().await;
    Json(json!({
        "ok": true,
        "version": "Akili-V4-Human",
        "model": "Qwen3.8-27B",
        "engine": "llama.cpp (llama-server)",
        "llm_url": base,
        "llm_model": llm_model(),
        "llm_reachable": reachable,
        "bridge_enabled": bridge_enabled(),
        "layers": [
            "1.Attention", "2.Perception", "3.System2", "4.System1",
            "5.Subconscious", "6.Intuition", "7.Imagination", "8.Emotions",
            "9.Social", "10.Ethics", "11.Meaning", "12.Identity"
        ],
        "note_sw": "Fikira = tabaka 12 za akili_v4; lugha = Qwen3.8-27B (llama-server). \
Hali ya ndani (hisia/tumbo/dhamiri) huongoza jibu la lugha."
    }))
}

/// POST { "input": "..." } — fikiri kibinadamu + jibu la lugha.
pub async fn human_live(Json(body): Json<Value>) -> Json<Value> {
    let input = body
        .get("input")
        .or_else(|| body.get("msg"))
        .or_else(|| body.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if input.trim().is_empty() {
        return Json(json!({"ok": false, "error": "input/msg ni lazima"}));
    }
    let context = body
        .get("context")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let max_tokens = body
        .get("max_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(256) as usize;
    let timeout_s = body
        .get("timeout_s")
        .and_then(|v| v.as_u64())
        .unwrap_or(600);

    let (state, answer) = human_think(&input, &context, "maelezo ya kati", max_tokens, timeout_s).await;
    let reachable = answer.is_some();
    Json(json!({
        "ok": true,
        "version": "Akili-V4-Human",
        "input": input,
        "human_state": state,
        "answer": answer,
        "llm_reachable": reachable,
        "note_sw": if reachable {
            "Fikira ya kibinadamu imeongoza jibu la Qwen3.8-27B."
        } else {
            "Lugha (llama-server) haipatikani — hali ya ndani pekee imerudishwa."
        }
    }))
}
