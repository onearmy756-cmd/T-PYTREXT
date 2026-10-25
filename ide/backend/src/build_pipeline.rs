//! build_pipeline.rs — MZUNGUZO WA ARUFA WA KUJENDESHA (THE CRITICAL LOOP).
//!
//! Mtiririko (kwa mujibu wa ramani ya usanifu halisi):
//!   Mtumiaji → ARCHITECT (orodha ya faili) → CODER (modeli ndogo/vLLM local —
//!   hakuna gharama) → LINTER HALISI (`code_rules::lint`) → FIX LOOP
//!   (kodi inarejea kwa coder mpaka `ok:true`, juu 3 mizunguko) →
//!   Kodi Safi → SSE live kwa Monaco Editor.
//!
//! Kanuni za repo:
//!   - Hakuna linter mpya: tunaita `code_rules::lint` (chandulizo kimoja).
//!   - Hakuna LLM mpya: tunaita `chat::ask` (provider zote zilizopo, vLLM
//!     ni provider mpya ya GATEWAY kwenye `fundi-agent/agent_builder.rs`).//! - HATUTAANDIKA faili moja kwa moja (HITL): kodi safi inatoka kwa SSE;
//!     mtumiaji anaidhinisha kuhifadhi (Ctrl+S) — server HAIANDIKI bila idhini.
//!   - Hesabu zote testable: `extract_code`, `extract_files`, `fix_prompt`,
//!     `code_prompt` ni vazi la saafu (pure functions + tests).

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

/// Juu ya mizunguko ya kujisahihisha kwa kila faili.
pub const MAX_FIX_ROUNDS: usize = 3;

// ---------------------------------------------------------------------------
// Vadu safi (pure helpers — zote zina tests)
// ---------------------------------------------------------------------------

/// Prompt ya CODER: acha kodi MOJA tu ndani ya fenced block.
pub fn code_prompt(spec: &str, file: &str) -> String {
    format!(
        "Wewe ni Rust Coder wa MBILINYI (modeli ndogo, uwezo mkuu). \
Andika FAILI MOJA tu: `{file}`. MALEZO ya mradi: {spec}.\n\n\
Kanuni:\n\
- Toa KODI TU ndani ya block moja: ```lugha ...```\n\
- Hakuna maelezo nje ya block; hakuna maelezo ya taka nje ya block.\n\
- Hakuna hard-coded keys/passwords, hakuna `eval`, SQL parameterized.\n\
- Fuata mwingiliano faili nyingine endapo tayari."
    )
}

/// Tolewa kodi kutoka kwenye jibu la LLM: fenced block ndefu kuliko yote.
pub fn extract_code(answer: &str) -> Option<String> {
    let mut best: Option<String> = None;
    let bytes = answer.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    while let Some(start) = answer[i..].find("```") {
        let open = i + start;
        let body_start = open + 3;
        let line_end = answer[body_start..].find('\n').map(|e| body_start + e).unwrap_or(n);
        if line_end >= n {
            break;
        }
        let close = answer[line_end + 1..].find("```").map(|e| line_end + 1 + e);
        let Some(close) = close else { break };
        let body = answer[line_end + 1..close].trim().to_string();
        if best.as_ref().is_none_or(|b| body.len() > b.len()) && !body.is_empty() {
            best = Some(body);
        }
        i = close + 3;
    }
    best
}

/// Prompt ya ARCHITECT: orodha ya faili (JSON array ya path).
pub fn architect_prompt(spec: &str) -> String {
    format!(
        "Wewe ni Rust Architect wa MBILINYI. Mradi: {spec}\n\
Chaguo: rudi orodha ya faili kwa JSON array moja tu, mfano:\n\
[\"src/main.rs\", \"src/auth.rs\", \"tests/auth.rs\"]\n\
- HAKUNA maelezo mengine; path ndogo ndogo zinazofanya kazi pekee.\n\
- Rudisha only JSON array, hakuna fences."
    )
}

/// Tolewa orodha ya faili kutoka kwenye jibu la architect (JSON au regex tokens).
pub fn extract_files(answer: &str) -> Vec<String> {
    // 1) JSON array moja kwa moja
    if let Ok(v) = serde_json::from_str::<Value>(answer.trim()) {
        if let Some(arr) = v.as_array() {
            return arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    // 2) Regex ya path (nafuu kwa modeli ndogo zisizotii JSON)
    let re = regex::Regex::new(r"[A-Za-z0-9_/\-\\.]+\.(rs|js|ts|py|go|java|cs|rb|kt|swift|dart|sh|html|css|toml|json|yml|yaml)").expect("regex halali");
    let mut out: Vec<String> = Vec::new();
    for m in re.find_iter(answer) {
        let s = m.as_str().to_string();
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// Prompt ya FIX LOOP: tuma kodi + matokeo ya linter kwa coder arekebishe.
pub fn fix_prompt(file: &str, code: &str, findings: &[Value]) -> String {
    let f = findings
        .iter()
        .map(|x| {
            let name = x["name"].as_str().unwrap_or("check");
            let msg = x["message"].as_str().unwrap_or("kosa");
            format!("- {name}: {msg}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Faili `{file}` lipo na kosari hizi kutoka LINTER:\n{f}\n\n\
Kodi iliyokosea:\n```\n{code}\n```\n\n\
KANUNI: Toa toleo LILILIREKEBISHWA tu ndani ya block moja: ``` ... ```\n\
Hakuna majibu mengine. Usibadilishe logic nyingine, rekebisha makosa tu."
    )
}

// ---------------------------------------------------------------------------
// SSE Handler — POST /api/ide/build-pipeline
// ---------------------------------------------------------------------------

/// Upload ya SSE: type= start | step | code | lint | done (moja kwa moja kwa
/// Monaco: `code` inavana content yote, `lint` inatoa cosmos ya selfcheck).
async fn emit(tx: &tokio::sync::mpsc::Sender<Result<Event, Infallible>>, ev: Value) {
    let _ = tx.send(Ok(Event::default().data(ev.to_string()))).await;
}

use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;

pub async fn run(
    State(s): State<crate::AppState>,
    Json(r): Json<Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use tokio_stream::StreamExt;

    let spec = r["spec"].as_str().unwrap_or("").trim().to_string();
    if spec.is_empty() {
        return Json(json!({ "ok": false, "error": "spec ni lazima" })).into_response();
    }
    let user_files: Vec<String> = r
        .get("files")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let brain = Arc::clone(&s.brain);
    let agent = s.agent.clone();

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    tokio::spawn(async move {
        let t0 = std::time::Instant::now();

        // ── 1) ARCHITECT: orodha ya faili (deterministic kama user anatolewa)
        let files = if !user_files.is_empty() {
            user_files
        } else {
            emit(&tx, json!({ "type": "step", "stage": "architect" })).await;
            let prompt = architect_prompt(&spec);
            let reply = crate::chat::ask(
                &brain,
                &prompt,
                true,
                crate::chat::Length::Short,
                agent.as_deref(),
                None,
            )
            .await;
            let files = extract_files(&reply.answer_sw);
            if files.is_empty() {
                emit(&tx, json!({ "type": "error", "error": "architect hakutoa faili zozote" })).await;
                return;
            }
            emit(&tx, json!({ "type": "plan", "files": files })).await;
            files
        };

        // ── 2) CODER + LINTER + FIX LOOP kwa kila faili
        let mut safe: Vec<String> = Vec::new();
        for file in &files {
            emit(&tx, json!({ "type": "step", "stage": "code", "file": file })).await;
            let prompt = code_prompt(&spec, file);
            let reply = crate::chat::ask(
                &brain,
                &prompt,
                true,
                crate::chat::Length::Medium,
                agent.as_deref(),
                None,
            )
            .await;
            let Some(mut code) = extract_code(&reply.answer_sw) else {
                emit(&tx, json!({ "type": "error", "file": file, "error": "coder hakutoa kodi" })).await;
                continue;
            };

            let mut ok = false;
            let mut rounds = 0usize;
            for _ in 0..=MAX_FIX_ROUNDS {
                emit(&tx, json!({ "type": "code", "file": file, "content": code, "round": rounds })).await;
                let report = crate::code_rules::lint(&code, file);
                emit(&tx, json!({ "type": "lint", "file": file, "report": report, "round": rounds })).await;
                if report["ok"].as_bool().unwrap_or(false) {
                    ok = true;
                    break;
                }
                rounds += 1;
                if rounds > MAX_FIX_ROUNDS {
                    break;
                }
                let findings: Vec<Value> = report["findings"].as_array().cloned().unwrap_or_default();
                emit(&tx, json!({ "type": "step", "stage": "fix", "file": file, "round": rounds })).await;
                let fp = fix_prompt(file, &code, &findings);
                let reply = crate::chat::ask(
                    &brain,
                    &fp,
                    true,
                    crate::chat::Length::Medium,
                    agent.as_deref(),
                    None,
                )
                .await;
                if let Some(mut c) = extract_code(&reply.answer_sw) {
                    std::mem::swap(&mut code, &mut c);
                } else {
                    emit(&tx, json!({ "type": "error", "file": file, "error": "fix hakutoa kodi", "round": rounds })).await;
                    break;
                }
            }
            if ok {
                safe.push(file.clone());
            }
        }

        // ── 3) DONE (HITL: FE kuhifadhi kwa `Ctrl+S` — server HAIANDIKA)
        emit(&tx, json!({
            "type": "done",
            "ok": true,
            "safe_files": safe,
            "files_total": files.len(),
            "elapsed_ms": t0.elapsed().as_millis() as u64,
        })).await;
    });

    Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

// ---------------------------------------------------------------------------
// Tests — hesabu halisi (deterministic), hakuna mocks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_code_inarudisha_block_ndefu() {
        let a = "maelezo yoyote\n```{\njson junk\n```\n```rust\nfn main() {}\n```\nnnyingine";
        let c = extract_code(a).expect("kodi lazima");
        assert!(c.contains("fn main() {}"));
        assert!(!c.contains("json junk"));
    }

    #[test]
    fn extract_code_hakuna_block_ni_jibu() {
        assert_eq!(extract_code("hakuna block hapa"), None);
    }

    #[test]
    fn extract_files_coder_json() {
        let v = extract_files("[\"src/main.rs\", \"src/auth.rs\"]");
        assert_eq!(v, vec!["src/main.rs".to_string(), "src/auth.rs".to_string()]);
    }

    #[test]
    fn extract_files_regex_fallback() {
        let v = extract_files("Unaandika src/main.rs na src/auth.rs pamoja");
        assert!(v.contains(&"src/main.rs".to_string()));
        assert!(v.contains(&"src/auth.rs".to_string()));
    }

    #[test]
    fn fix_prompt_apea_findings() {
        let f = vec![json!({ "name": "balanced", "message": "mabano hayajafungwa" })];
        let p = fix_prompt("src/auth.rs", "fn x() {", &f);
        assert!(p.contains("src/auth.rs"));
        assert!(p.contains("balanced"));
        assert!(p.contains("mabano hayajafungwa"));
    }

    #[test]
    fn code_prompt_anutumia_file_spec() {
        let p = code_prompt("app ya bank", "src/auth.rs");
        assert!(p.contains("src/auth.rs"));
        assert!(p.contains("app ya bank"));
        assert!(p.contains("block"), "coder lazima atumie fenced block — {p}");
    }

    #[test]
    fn max_fix_rounds_ni_tatu() {
        // Mpaka wa mzunguko wa kujisahihisha — SI chaguo, ni pango linalindwa.
        assert_eq!(MAX_FIX_ROUNDS, 3);
    }
}
