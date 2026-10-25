//! ide_complete.rs — AI INLINE AUTOCOMPLETE (FIM) kwa IDE.
//!
//! Inatumia fill-in-the-middle (FIM) kupitia provider halisi:
//!   1) Ollama LAN (http://127.0.0.1:11434/api/generate) — model ndogo, haraka
//!   2) Cloud (FUNDI_CLOUD_URL + FUNDI_AI_KEY) — ikiwa LAN haipo
//!
//! Ndinayo ya ndani: model ndogo inatosha kwa autocomplete — tunapunguza
//! prefix/suffix (context window), tunakata daraja, na tunazuia "
//! code-fence za markdown. Hakuna hallucination ya viten vya nje.

use axum::Json;
use serde_json::{json, Value};
use std::time::Duration;

const SYSTEM_COMPLETION: &str = "Ni 2NYI — SUPER SENIOR SOFTWARE ENGINEER, code-completer halisi. Jibu na CODE PEKEE ya kukamilika \
kutoka kwenye cursor (complementary ya FIM). HAKUNA maelezo, HAKUNA markdown, \
HAKUNA comments mpya endapo newline inafuata mstari wa kawaida. \
Endeleza ndani ya mstari mmoja au miwili tu bila kubadilisha code iliyopo. \
GROUNDING: kamilisha kwa API/ruzuku zilizopo kwenye muktadha — usibuni majina ya functions au vitegemezi visivyothibitishwa.";

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.trim().is_empty())
}

struct FinishConfig {
    lan_url: String,
    lan_model: String,
    cloud_url: String,
    cloud_model: String,
    cloud_key: Option<String>,
}

fn config() -> FinishConfig {
    FinishConfig {
        lan_url: env("FUNDI_LLM_URL").unwrap_or_else(|| "http://127.0.0.1:11434".into()),
        lan_model: env("FUNDI_COMPLETION_MODEL").unwrap_or_else(|| "qwen2.5-coder:1.5b".into()),
        cloud_url: env("FUNDI_CLOUD_URL").unwrap_or_else(|| "https://ollama.com/v1".into()),
        // Default: model BURE ya Ollama Cloud (qwen2.5-coder haipo kwenye free tier).
        cloud_model: env("FUNDI_COMPLETION_MODEL_CLOUD")
            .or_else(|| env("FUNDI_CLOUD_MODEL"))
            .unwrap_or_else(|| "gemma4:31b".into()),
        cloud_key: env("FUNDI_AI_KEY").or_else(|| env("OPENAI_API_KEY")),
    }
}

fn clean_completion(raw: &str, _prefix: &str) -> String {
    let mut s = raw.trim().to_string();
    // Safisha FIM/chat tokens zilizovuja (Qwen: <|fim_*|>, <|endoftext|>; CodeLlama: <fim_*>).
    for tok in [
        "<|fim_middle|>", "<|fim_suffix|>", "<|fim_prefix|>", "<|fim_pad|>",
        "<|endoftext|>", "<|im_end|>", "<|im_start|>",
        "<fim_middle>", "<fim_suffix>", "<fim_prefix>",
        "</fim_middle>", "</fim_suffix>", "</fim_prefix>",
    ] {
        if let Some(idx) = s.find(tok) {
            s = s[..idx].to_string();
        }
    }
    let s = s.trim();
    let mut s = s.to_string();
    // Kata fence za markdown
    if let Some(rest) = s.strip_prefix("```") {
        s = rest.to_string();
        if let Some(idx) = s.find('\n') {
            s = s[idx + 1..].to_string();
        }
        s = s
            .trim_end()
            .trim_end_matches("```")
            .trim_end()
            .to_string();
    }
    // Kata maelezo (kama jibu liza maelezo badala ya code):
    if (s.contains("Jibu:") || s.contains("Maelezo:") || s.contains("This code") || s.contains("Hii code"))
    {
        // kivunge: kata hadi mstari wa kwanza wenye code
        if let Some(line) = s.lines().find(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("//")
                && !t.starts_with("#")
                && !t.ends_with(':')
                && !t.contains("Jibu")
                && !t.contains("Maelezo")
                && !t.contains("code")
        }) {
            s = line.to_string();
        }
    }
    // Hakuna tafsiri ya mwisho endapo newline inaonyesha kengele: kata kwenye newline ya kati
    let take = s.chars().take(220).collect::<String>();
    let cut_at = match take.find('\n') {
        Some(i) if i > 2 => i + 1, // include newline
        _ => take.len(),
    };
    take[..cut_at].to_string()
}

async fn ollama_lan_fim(cfg: &FinishConfig, prefix: &str, suffix: &str, max_tokens: u32) -> Option<String> {
    // Ollama FIM: /api/generate, raw, na FIM tokens KAMILI za Qwen2.5-Coder
    // (<|fim_prefix|>…<|fim_suffix|>…<|fim_middle|>). Tokens za CodeLlama
    // (<fim_prefix>) HAZITAMBULIKI na Qwen — zikivuja kwenye completion.
    let prompt = format!("<|fim_prefix|>{prefix}<|fim_suffix|>{suffix}<|fim_middle|>");
    let body = json!({
        "model": cfg.lan_model,
        "prompt": prompt,
        "stream": false,
        "raw": true,
        "options": {
            "num_predict": max_tokens,
            "temperature": 0.15,
            "stop": ["<|fim_pad|>", "<|endoftext|>", "<|fim_prefix|>", "<|fim_suffix|>", "\n\n\n"]
        },
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .ok()?;
    let resp = client
        .post(format!("{}/api/generate", cfg.lan_url.trim_end_matches('/')))
        .json(&body)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    v["response"].as_str().map(|s| s.to_string())
}

/// Injini YETU (fundi-engine) FIM kwa autocomplete: prefix/suffix → kipande
/// kinachowekwa katikati. Offline, GGUF ya ndani, bila vLLM/Ollama.
async fn local_engine_completion(prefix: &str, suffix: &str, max_tokens: u32) -> Option<String> {
    crate::engine_api::complete_local(prefix, suffix, max_tokens as usize).await
}

async fn cloud_openai_completion(cfg: &FinishConfig, prefix: &str, suffix: &str, max_tokens: u32) -> Option<String> {
    let key = cfg.cloud_key.as_ref()?;
    // OpenAI-compatible models kwa FIM: tumia chat na prefix/suffix kama miswado
    let body = json!({
        "model": cfg.cloud_model,
        "messages": [
            { "role": "system", "content": SYSTEM_COMPLETION },
            { "role": "user", "content": format!(
                "Endeleza code ndani ya cursor. PREFIX (code kabla ya cursor):\n```\n{prefix}\n```\nSUFFIX (code baada ya cursor):\n```\n{suffix}\n```\nJibu na code pekee, hakuna maelezo."
            ) }
        ],
        "stream": false,
        "max_tokens": max_tokens,
        "temperature": 0.15,
        "stop": ["```"],
    });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .ok()?;
    let req = client
        .post(format!(
            "{}/chat/completions",
            cfg.cloud_url.trim_end_matches('/')
        ))
        .bearer_auth(key)
        .json(&body);
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = resp.json().await.ok()?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
}

/// POST /api/ide/complete  — { path, prefix, suffix, language, maxTokens } → { ok, completion }
pub async fn complete(Json(r): Json<Value>) -> Json<Value> {
    let prefix = r["prefix"].as_str().unwrap_or("").to_string();
    let suffix = r["suffix"].as_str().unwrap_or("").to_string();
    if prefix.trim().is_empty() && suffix.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "prefix/suffix tupu" }));
    }
    // Context window ya model ndogo: kata context (mwisho wa prefix, mwanzo wa suffix)
    const CONTEXT_CHARS: usize = 2400;
    let chars: Vec<char> = prefix.chars().collect();
    let start = chars.len().saturating_sub(CONTEXT_CHARS);
    let prefix_ctx: String = chars[start.min(chars.len())..].iter().collect();
    let suffix_ctx: String = suffix.chars().take(700).collect();
    let max_tokens = r["maxTokens"].as_u64().unwrap_or(48) as u32;

    let cfg = config();
    let mut attempts = Vec::new();

    // 0) Injini YETU (fundi-engine, GGUF ya ndani) — kwanza, offline.
    if crate::engine_api::global().is_loaded() {
        match local_engine_completion(&prefix_ctx, &suffix_ctx, max_tokens).await {
            Some(raw) if !raw.trim().is_empty() => {
                let completion = clean_completion(&raw, &prefix_ctx);
                if !completion.trim().is_empty() {
                    return Json(json!({
                        "ok": true,
                        "completion": completion,
                        "provider": "fundi_engine",
                        "model": "gguf-ndani",
                        "attempts": attempts,
                    }));
                }
            }
            Some(_) => attempts.push(json!({ "provider": "fundi_engine", "error": "jibu tupu" })),
            None => attempts.push(json!({ "provider": "fundi_engine", "error": "haipatikani" })),
        }
    }

    // 1) Ollama LAN FIM (haraka, model ndogo)
    match ollama_lan_fim(&cfg, &prefix_ctx, &suffix_ctx, max_tokens).await {
        Some(raw) if !raw.trim().is_empty() => {
            let completion = clean_completion(&raw, &prefix_ctx);
            if !completion.trim().is_empty() {
                return Json(json!({
                    "ok": true,
                    "completion": completion,
                    "provider": "ollama_lan",
                    "model": cfg.lan_model,
                    "attempts": attempts,
                }));
            }
        }
        Some(_) => attempts.push(json!({ "provider": "ollama_lan", "error": "jibu tupu" })),
        None => attempts.push(json!({ "provider": "ollama_lan", "error": "haipatikani" })),
    }

    // 2) Cloud (OpenAI-compatible) — haiitaji LAN
    match cloud_openai_completion(&cfg, &prefix_ctx, &suffix_ctx, max_tokens).await {
        Some(raw) if !raw.trim().is_empty() => {
            let completion = clean_completion(&raw, &prefix_ctx);
            if !completion.trim().is_empty() {
                return Json(json!({
                    "ok": true,
                    "completion": completion,
                    "provider": "cloud",
                    "model": cfg.cloud_model,
                    "attempts": attempts,
                }));
            }
        }
        Some(_) => attempts.push(json!({ "provider": "cloud", "error": "jibu tupu" })),
        None => attempts.push(json!({ "provider": "cloud", "error": "key haipo au cloud haipatikani" })),
    }

    Json(json!({
        "ok": false,
        "error": "completion haipatikani — Ollama LAN haipo na cloud inahitaji FUNDI_AI_KEY",
        "attempts": attempts,
    }))
}

/// GET /api/ide/complete/status — hali ya provider za autocomplete
pub async fn status() -> Json<Value> {
    let cfg = config();
    let lan_ok = ping_lan(&cfg.lan_url).await;
    Json(json!({
        "ok": true,
        "engine": {
            "name": "fundi-engine",
            "available": crate::engine_api::global().is_loaded(),
            "note_sw": "Injini YETU ya inference — GGUF moja kwa moja, pure Rust.",
        },
        "lan": { "url": cfg.lan_url, "model": cfg.lan_model, "available": lan_ok },
        "cloud": {
            "url": cfg.cloud_url,
            "model": cfg.cloud_model,
            "key": cfg.cloud_key.map(|_| "configured").unwrap_or_else(|| "missing".into()),
        },
    }))
}

async fn ping_lan(lan_url: &str) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client
        .get(format!("{}/api/tags", lan_url.trim_end_matches('/')))
        .send()
        .await
    {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// SYSTEM prompt ya INLINE EDIT (Cmd+K): coder mode inayobadilisha code
/// ILIYOCHAGULIWA kulingana na maelekezo ya mtumiaji. Hurudisha code pekee.
const SYSTEM_EDIT: &str = "Ni 2NYI — SUPER SENIOR SOFTWARE ENGINEER, mhariri wa code (inline edit). Utapokea maelekezo na \
kishazi cha code. Badilisha SEHEMU ILIYOCHAGULIWA kulingana na maelekezo; usibadilishe \
kitu kingine. Jibu na CODE PEKEE ya kipande kilichobadilishwa — HAKUNA maelezo, \
HAKUNA markdown, HAKUNA fence. GROUNDING: usibuni API/kitenzi; tumia tu kile kilichopo kwenye code.";

/// Kata fence za markdown na mistari ya maelezo kutoka jibu la inline edit.
fn clean_edit(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if let Some(rest) = s.strip_prefix("```") {
        // ondoa lugha kwenye mstari wa kwanza wa fence
        if let Some(idx) = rest.find('\n') {
            s = rest[idx + 1..].to_string();
        } else {
            s = rest.to_string();
        }
        s = s.trim_end().trim_end_matches("```").trim_end().to_string();
    }
    s
}

/// POST /api/ide/edit — INLINE AI EDIT (Cmd+K, kama Cursor).
/// Body: { selection, instruction, path?, language?, prefix?, suffix? }
///   → { ok, code, provider }
///
/// Hurudisha code MPYA ya kipande kilichochaguliwa. Mantiki ni `chat::code_ask`
/// (coder mode — huruhusu code fences) iliyowekwa wazi kwa UI ya editor.
pub async fn edit(
    axum::extract::State(s): axum::extract::State<crate::AppState>,
    Json(r): Json<Value>,
) -> Json<Value> {
    let selection = r["selection"].as_str().unwrap_or("").to_string();
    let instruction = r["instruction"].as_str().unwrap_or("").trim().to_string();
    if instruction.is_empty() {
        return Json(json!({ "ok": false, "error": "instruction ni lazima" }));
    }
    if selection.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "hakuna kipande kilichochaguliwa" }));
    }
    let path = r["path"].as_str().unwrap_or("");
    let language = r["language"].as_str().unwrap_or("");
    // Konteksti ya jirani (prefix/suffix) — hutusaidia code iliyoendana.
    let prefix: String = r["prefix"].as_str().unwrap_or("").chars().rev().take(1200).collect::<String>().chars().rev().collect();
    let suffix: String = r["suffix"].as_str().unwrap_or("").chars().take(600).collect();

    let mut prompt = String::new();
    if !path.is_empty() {
        prompt.push_str(&format!("Faili: {path}\n"));
    }
    if !language.is_empty() {
        prompt.push_str(&format!("Lugha: {language}\n"));
    }
    prompt.push_str(&format!("\nMAELEKEZO: {instruction}\n"));
    if !prefix.is_empty() {
        prompt.push_str(&format!("\nCODE KABLA YA KIPANDE:\n{prefix}\n"));
    }
    prompt.push_str(&format!("\nKIPANDE KILICHOCHAGULIWA (badilisha hiki):\n{selection}\n"));
    if !suffix.is_empty() {
        prompt.push_str(&format!("\nCODE BAADA YA KIPANDE:\n{suffix}\n"));
    }
    prompt.push_str(&format!("\nJibu na {SYSTEM_EDIT}"));

    let reply = crate::chat::code_ask(&s.brain, &prompt).await;
    let code = clean_edit(&reply);
    if code.trim().is_empty() {
        return Json(json!({
            "ok": false,
            "error": "inline edit haipatikani — hakuna provider (injini ya MTECHOS/Ollama LAN/cloud)",
        }));
    }
    Json(json!({ "ok": true, "code": code, "provider": "code_ask" }))
}

pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
        .route("/api/ide/complete", axum::routing::post(complete))
        .route("/api/ide/complete/status", axum::routing::get(status))
        .route("/api/ide/edit", axum::routing::post(edit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_edit_hutoa_fence_na_lugha() {
        let raw = "```rust\nfn add(a: i32, b: i32) -> i32 { a + b }\n```";
        assert_eq!(clean_edit(raw), "fn add(a: i32, b: i32) -> i32 { a + b }");
    }

    #[test]
    fn clean_edit_hurudisha_code_safi_bila_fence() {
        let raw = "let x = 1;";
        assert_eq!(clean_edit(raw), "let x = 1;");
    }

    #[test]
    fn clean_completion_hukata_fim_tokens_zilizovuja() {
        // Qwen2.5-Coder hutumia <|fim_*|>; tokens hizi zikivuja zisiingie kwenye code.
        let raw = "return a + b</fim_middle></fim_suffix></fim_prefix>```";
        assert_eq!(clean_completion(raw, ""), "return a + b");
    }

    #[test]
    fn clean_completion_hukata_qwen_pipe_tokens() {
        let raw = "a + b\n<|fim_pad|>junk";
        assert_eq!(clean_completion(raw, ""), "a + b");
    }
}
