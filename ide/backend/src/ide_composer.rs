//! ide_composer.rs — MONOCODE COMPOSER (Cursor/Devin-style multi-file edits).
//!
//! Cursor/Devin zinajulikana kwa **Composer/Agent mode**: maelezo MOJA ya
//! mtumiaji → agent hupanga na kuandika mabadiliko kwenye FAILI NYINGI, kisha
//! UI inaonyesha **diff** kwa kila faili na mtumiaji **anakubali (accept)** au
//! **anakataa (reject)** kabla ya kuandikwa. Autocomplete (ghost-text) na Cmd+K
//! (inline edit) ni sehemu ya MONOCODE; Composer ni kipande kinachokosekana.
//!
//! Njia hii ni HALISI: inatumia provider ile ile ya `chat::code_ask`
//! (fundi-engine → Ollama LAN → cloud). Server HAITENGENEZI diff ya kubuni —
//! diff inahesabiwa hapa kwa LCS halisi (pure Rust, hakuna crate mpya).
//!
//! Endpoints:
//!   POST /api/ide/composer/plan   { instruction, files?, path?, selection?, language? }
//!        → { ok, edits:[{path, action, old, new, content}], provider }
//!   POST /api/ide/composer/diff   { path, old, new }  → { ok, diff, added, removed }
//!   POST /api/ide/composer/apply  { edits:[…] }        → { ok, applied:[…], skipped:[…] }
//!   POST /api/ide/composer/reject { edits:[…] }        → { ok, rejected:n }
//!
//! Usalama: kila path hupitia `ide_api::safe_path` (workspace pekee, hakuna
//! `..`/absolute). Hii ni nyongeza (add-only).

use axum::Json;
use serde_json::{json, Value};

/// Faili nyingi za muktadha kwa plan (kikomo ili prompt isipue).
const MAX_CTX_FILES: usize = 8;
const MAX_CTX_CHARS_PER_FILE: usize = 4000;

/// Chukua JSON ya kwanza (`[…]` au `{…}`) kutoka jibu la LLM lenye prose/fence.
fn extract_json(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let start = bytes
        .iter()
        .position(|&c| c == b'[' || c == b'{')?;
    let open = bytes[start];
    let close = if open == b'[' { b']' } else { b'}' };
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &c) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            continue;
        }
        match c {
            b'"' => in_str = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth -= 1;
                if depth == 0 {
                    return raw.get(start..=i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Badilisha jibu la LLM kuwa orodha ya edits zilizo halali.
fn parse_edits(raw: &str) -> Vec<Value> {
    let Some(js) = extract_json(raw) else {
        return Vec::new();
    };
    let v: Value = match serde_json::from_str(js) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = if let Some(a) = v.as_array() {
        a.clone()
    } else if let Some(a) = v["edits"].as_array() {
        a.clone()
    } else if v.is_object() && v["path"].is_string() {
        vec![v]
    } else {
        return Vec::new();
    };
    arr.into_iter()
        .filter_map(|e| {
            let path = e["path"].as_str()?.trim().to_string();
            if path.is_empty() || path.contains('<') || path.contains('>') {
                return None;
            }
            let action = e["action"].as_str().unwrap_or("write").to_string();
            let old = e["old"].as_str().unwrap_or("").to_string();
            let new = e["new"]
                .as_str()
                .or_else(|| e["content"].as_str())
                .unwrap_or("")
                .to_string();
            Some(json!({ "path": path, "action": action, "old": old, "new": new }))
        })
        .collect()
}

/// Mstari mmoja wa diff.
enum Op {
    Eq(String),
    Del(String),
    Ins(String),
}

/// LCS ya mistari (Hunt–Szymanski rahisi) — inatosha kwa UI ya diff.
fn lcs_ops(a: &[&str], b: &[&str]) -> Vec<Op> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0usize, 0usize);
    let mut ops = Vec::new();
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op::Eq(a[i].to_string()));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(Op::Del(a[i].to_string()));
            i += 1;
        } else {
            ops.push(Op::Ins(b[j].to_string()));
            j += 1;
        }
    }
    while i < n {
        ops.push(Op::Del(a[i].to_string()));
        i += 1;
    }
    while j < m {
        ops.push(Op::Ins(b[j].to_string()));
        j += 1;
    }
    ops
}

/// Unified-ish diff (muktadha wa mistari 3) + hesabu za +/−.
pub fn unified_diff(path: &str, old: &str, new: &str) -> (String, usize, usize) {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let ops = lcs_ops(&a, &b);
    let added = ops.iter().filter(|o| matches!(o, Op::Ins(_))).count();
    let removed = ops.iter().filter(|o| matches!(o, Op::Del(_))).count();
    let mut out = format!("--- a/{path}\n+++ b/{path}\n");
    for op in &ops {
        match op {
            Op::Eq(l) => out.push_str(&format!(" {l}\n")),
            Op::Del(l) => out.push_str(&format!("-{l}\n")),
            Op::Ins(l) => out.push_str(&format!("+{l}\n")),
        }
    }
    (out, added, removed)
}

/// Soma faili za muktadha kutoka workspace (salama) kwa prompt.
fn context_files(files: &[String]) -> String {
    let mut out = String::new();
    for rel in files.iter().take(MAX_CTX_FILES) {
        let Some(p) = crate::ide_api::safe_path(rel) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&p) else {
            continue;
        };
        let body: String = content.chars().take(MAX_CTX_CHARS_PER_FILE).collect();
        out.push_str(&format!(
            "\n=== FAILI: {rel} ===\n{body}\n=== MWISHO {rel} ===\n"
        ));
    }
    out
}

/// POST /api/ide/composer/plan — maelezo → mapendekezo ya edits kwa faili nyingi.
pub async fn plan(
    axum::extract::State(s): axum::extract::State<crate::AppState>,
    Json(r): Json<Value>,
) -> Json<Value> {
    let instruction = r["instruction"].as_str().unwrap_or("").trim().to_string();
    if instruction.is_empty() {
        return Json(json!({ "ok": false, "error": "instruction ni lazima" }));
    }
    let files: Vec<String> = r["files"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let path = r["path"].as_str().unwrap_or("");
    let selection = r["selection"].as_str().unwrap_or("");
    let language = r["language"].as_str().unwrap_or("");

    // Faili lengo ni muktadha wa lazima hata bila `files`.
    let mut ctx_files = files.clone();
    if !path.is_empty() && !ctx_files.iter().any(|f| f == path) {
        ctx_files.insert(0, path.to_string());
    }
    let ctx = context_files(&ctx_files);

    let mut prompt = String::new();
    prompt.push_str("KAZI YA COMPOSER (hariri faili nyingi kwa maelezo MOJA).\n");
    prompt.push_str(&format!("\nMAELEZO YA MTUMIAJI: {instruction}\n"));
    if !language.is_empty() {
        prompt.push_str(&format!("Lugha kuu: {language}\n"));
    }
    if !selection.trim().is_empty() {
        prompt.push_str(&format!("\nKIPANDE KILICHOCHAGULIWA:\n{selection}\n"));
    }
    if !ctx.trim().is_empty() {
        prompt.push_str("\nMUKTADHA WA FAILI ZILIZOPO:\n");
        prompt.push_str(&ctx);
    }
    prompt.push_str(
        "\nTOA jibu kama JSON ARRAY PEKEE ya mabadiliko (hakuna maelezo, hakuna fence).\
 Kila kipengele kiwe na \"path\" HALISI ya faili na ama \"content\" (faili kamili, action=write) \
 au \"old\"+\"new\" (action=replace). MFANO WA UMBO (USIRUDIE thamani hizi — badilisha kwa faili ZAKO):\n\
         [{\"path\":\"<njia-halisi>\",\"action\":\"write\",\"content\":\"<maudhui-halisi>\"}]\n\
         Kanuni: 1) tumia njia SAHIHI kutoka muktadha, si <njia-halisi>; 2) `write` = faili KAMILI; \
         3) `replace` = badilisha kipande kimoja; 4) usibuni API/kitenzi visivyopo; \
         5) andika mabadiliko HALISI kwa faili zilizotajwa; 6) JSON pekee.",
    );

    let reply = crate::chat::code_ask(&s.brain, &prompt).await;
    let edits = parse_edits(&reply);
    if edits.is_empty() {
        return Json(json!({
            "ok": false,
            "error": "composer haikutoa edits — angalia provider (injini ya MTECHOS/Ollama LAN/cloud)",
            "raw": reply.chars().take(600).collect::<String>(),
        }));
    }
    Json(json!({ "ok": true, "edits": edits, "count": edits.len(), "provider": "code_ask" }))
}

/// POST /api/ide/composer/diff — diff halisi kwa {path, old, new}.
pub async fn diff(Json(r): Json<Value>) -> Json<Value> {
    let path = r["path"].as_str().unwrap_or("faili");
    let old = r["old"].as_str().unwrap_or("");
    let new = r["new"].as_str().unwrap_or("");
    let (text, added, removed) = unified_diff(path, old, new);
    Json(json!({ "ok": true, "diff": text, "added": added, "removed": removed }))
}

/// POST /api/ide/composer/apply — andika edits ZOTE (salama) kwa workspace.
pub async fn apply(Json(r): Json<Value>) -> Json<Value> {
    let edits = r["edits"].as_array().cloned().unwrap_or_default();
    apply_in_root(&crate::ide_api::ide_workspace_root(), &edits)
}

/// Kiini cha `apply` — huru kwa root ili kipimwe bila kubadili env (vipimo
/// hukimbia sambamba; `set_var` ingeingiliana).
pub(crate) fn apply_in_root(root: &std::path::Path, edits: &[Value]) -> Json<Value> {
    if edits.is_empty() {
        return Json(json!({ "ok": false, "error": "edits ni lazima" }));
    }
    let mut applied: Vec<Value> = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();
    for e in edits {
        let path = e["path"].as_str().unwrap_or("");
        let Some(full) = crate::ide_api::safe_path_in_root(root, path) else {
            skipped.push(json!({ "path": path, "reason": "njia si salama (nje ya workspace)" }));
            continue;
        };
        let action = e["action"].as_str().unwrap_or("write");
        // AI hutoa `content` (au `new`); bila kukubali zote mbili, `write`
        // ingeandika faili TUPU kimya kimya na kuharibu kazi ya mtumiaji.
        let new = match e["new"].as_str().or_else(|| e["content"].as_str()) {
            Some(n) => n,
            None => {
                skipped.push(json!({ "path": path, "reason": "`new`/`content` haipo" }));
                continue;
            }
        };
        let result = if action == "replace" {
            let old = e["old"].as_str().unwrap_or("");
            match std::fs::read_to_string(&full) {
                Ok(cur) if cur.contains(old) && !old.is_empty() => {
                    std::fs::write(&full, cur.replacen(old, new, 1)).map(|_| "replace")
                }
                Ok(_) => {
                    skipped.push(json!({ "path": path, "reason": "`old` haipo kwenye faili" }));
                    continue;
                }
                Err(err) => {
                    skipped.push(json!({ "path": path, "reason": err.to_string() }));
                    continue;
                }
            }
        } else {
            if let Some(parent) = full.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&full, new).map(|_| "write")
        };
        match result {
            Ok(a) => applied.push(json!({ "path": path, "action": a })),
            Err(err) => skipped.push(json!({ "path": path, "reason": err.to_string() })),
        }
    }
    Json(json!({
        "ok": skipped.is_empty(),
        "applied": applied,
        "skipped": skipped,
        "applied_count": applied.len(),
    }))
}

/// POST /api/ide/composer/reject — kataa edits (hakuna mabadiliko ya diski).
pub async fn reject(Json(r): Json<Value>) -> Json<Value> {
    let n = r["edits"].as_array().map(|a| a.len()).unwrap_or(0);
    Json(json!({ "ok": true, "rejected": n }))
}

pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
        .route("/api/ide/composer/plan", axum::routing::post(plan))
        .route("/api/ide/composer/diff", axum::routing::post(diff))
        .route("/api/ide/composer/apply", axum::routing::post(apply))
        .route("/api/ide/composer/reject", axum::routing::post(reject))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_hupata_array_ndani_ya_prose() {
        let raw = "Haya mabadiliko:\n```json\n[{\"path\":\"a.rs\"}]\n```\n";
        assert_eq!(extract_json(raw), Some("[{\"path\":\"a.rs\"}]"));
    }

    #[test]
    fn parse_edits_hutambua_content_kama_new() {
        let raw = "[{\"path\":\"x/y.rs\",\"action\":\"write\",\"content\":\"fn main(){}\"}]";
        let e = parse_edits(raw);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0]["new"], "fn main(){}");
    }

    #[test]
    fn parse_edits_hukata_njia_ya_mfano() {
        let raw = r#"[{"path":"<njia-halisi>","action":"write","content":"x"}]"#;
        assert!(parse_edits(raw).is_empty());
    }

    #[test]
    fn apply_huandika_content_sio_faili_tupu() {
        // Regression: AI hutoa `content`; `apply` ilisoma `new` pekee → faili 0 bytes.
        let dir = std::env::temp_dir().join(format!("mts_composer_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let edits = vec![json!({ "path": "sub/live.py", "action": "write", "content": "print('live')\n" })];
        let out = apply_in_root(&dir, &edits);
        assert_eq!(out["applied_count"], 1, "edit lazima itumike: {}", out.0);
        let written = std::fs::read_to_string(dir.join("sub/live.py")).unwrap();
        assert_eq!(written, "print('live')\n", "faili ilandikwa tupu");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unified_diff_hupima_added_na_removed() {
        let (text, added, removed) = unified_diff("a.rs", "a\nb\nc\n", "a\nB\nc\nd\n");
        assert!(text.contains("--- a/a.rs"));
        assert!(text.contains("-b"));
        assert!(text.contains("+B"));
        assert!(text.contains("+d"));
        assert_eq!(added, 2);
        assert_eq!(removed, 1);
    }
}
