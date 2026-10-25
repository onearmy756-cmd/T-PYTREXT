//! timetravel.rs — SEMANTIC TIME-TRAVEL CODE DEBUGGING.
//!
//! Kila mwandiko wa agent (au mtumiaji) unarekodi "video ya kimantiki":
//!   {ts, path, ok(lint), size, symbols[], sha256, content}
//! ndani ya `.timetravel/<path-encoded>.jsonl` (max 50 kwa file).
//!
//! Debugging kwa Time-Travel (bisect HALISI):
//!   1. Journal inaok-flags (lint kila snapshot).
//!   2. `bisect_from()` — FIRST failing step baada ya LAST passing step:
//!      hapo ndipo mantiki ilipoanza kupotoka (hatua halisi, si tiesto).
//!   3. `line_diff()` — tofauti ya mizunguzo (semantic diff, LCS safi).
//!   4. `revert()` — rudi kwenye snapshot (kodi ya nyuma inarudi moja kwa moja).
//!
//! Symbols map (~AST snapshot heuristic, uongo HAPANA): fn/struct/impl (rs),
//! function/class (js/ts), def/class (py) — ni marker zina-ONEKANA; kama
//! parser kamili inahitajika zaidi, matokeo yanasema "heuristic".

use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const MAX_SNAPSHOTS: usize = 50;
pub const MAX_CONTENT: usize = 100 * 1024; // 100KB guard

fn dir_for(root: &Path) -> PathBuf {
    root.join(".timetravel")
}

fn file_for(root: &Path, path: &str) -> PathBuf {
    let enc: String = path
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    dir_for(root).join(format!("{enc}.jsonl"))
}

/// Symbols halisi (regex heuristic — inaonyesha nini imebadilika kimantiki).
pub fn symbols_of(content: &str, path: &str) -> Vec<String> {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    let patterns: &[&str] = match ext.as_str() {
        "rs" => &["fn\\s+[A-Za-z0-9_]+", "struct\\s+[A-Za-z0-9_]+", "impl\\s+[A-Za-z0-9_]+"],
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "jsx" => &["function\\s+[A-Za-z0-9_]+", "class\\s+[A-Za-z0-9_]+", "const\\s+[A-Za-z0-9_]+\\s*=\\s*\\("],
        "ex" | "exs" => &["def\\s+[A-Za-z0-9_]+", "defmodule\\s+[A-Za-z0-9_.]+"],
        _ => &["[A-Za-z0-9_]+\\s*=\\s*function|[A-Za-z0-9_]+\\s*[:(].*=.*=>"],
    };
    let mut out: Vec<String> = Vec::new();
    for p in patterns {
        if let Ok(re) = Regex::new(p) {
            for m in re.find_iter(content).take(40) {
                let s = m.as_str().to_string();
                if !out.contains(&s) {
                    out.push(s);
                }
                if out.len() >= 40 {
                    return out;
                }
            }
        }
    }
    out
}

fn sha256_hex(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Dulumi: record snapshot (best-effort — kamwe isiharibu write kuu).
pub fn record(root: &Path, path: &str, content: &str, ok: bool) -> Value {
    let d = dir_for(root);
    let _ = std::fs::create_dir_all(&d);
    let f = file_for(root, path);
    let content_keep: &str = if content.len() > MAX_CONTENT { &content[..MAX_CONTENT] } else { content };
    let entry = json!({
        "t": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|x| x.as_millis() as u64)
            .unwrap_or(0),
        "path": path,
        "ok": ok,
        "size": content.len(),
        "sha256": sha256_hex(content),
        "symbols": symbols_of(content_keep, path),
        "content": content_keep,
    });
    let line = entry.to_string();
    let existing = std::fs::read_to_string(&f).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().collect();
    lines.push(line.as_str());
    let start = lines.len().saturating_sub(MAX_SNAPSHOTS);
    let body = lines[start..].join("\n");
    let _ = std::fs::write(&f, if body.is_empty() { String::new() } else { format!("{body}\n") });
    entry
}

/// Orodha ya snapshots (BILA content — metadata tu kwa UI).
pub fn list(root: &Path, path: &str) -> Vec<Value> {
    let f = file_for(root, path);
    let txt = std::fs::read_to_string(&f).unwrap_or_default();
    txt.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .enumerate()
        .map(|(i, mut v)| {
            if let Some(o) = v.as_object_mut() {
                o.remove("content");
                o.insert("index".into(), json!(i));
            }
            v
        })
        .collect()
}

/// Snapshot MOJA +content (kwa revert).
pub fn snapshot(root: &Path, path: &str, index: usize) -> Option<Value> {
    let f = file_for(root, path);
    let txt = std::fs::read_to_string(&f).ok()?;
    let lines: Vec<&str> = txt.lines().collect();
    let raw = lines.get(index)?;
    serde_json::from_str::<Value>(raw).ok()
}

/// SEMANTIC DIFF: tofauti ya mizunguzo (LCS, jibu hupunguzwa kwa 200).
pub fn line_diff(old: &str, new: &str) -> Value {
    let a: Vec<&str> = old.lines().take(500).collect();
    let b: Vec<&str> = new.lines().take(500).collect();
    let n = a.len();
    let m = b.len();
    // LCS DP (n,m ≤500 → 250k cells, cyber-safe)
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
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
    let (mut added, mut removed): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            removed.push(a[i].to_string());
            i += 1;
        } else {
            added.push(b[j].to_string());
            j += 1;
        }
    }
    while i < n {
        removed.push(a[i].to_string());
        i += 1;
    }
    while j < m {
        added.push(b[j].to_string());
        j += 1;
    }
    added.truncate(200);
    removed.truncate(200);
    json!({ "added": added, "removed": removed })
}

/// BISECT muhimu (dev): FIRST kosa baada ya LAST kazi — hatua ya kupotoka.
/// Ingizo: orodha ya ok-flags kwenye mpangilio wa mata; kosa linapatika kama
/// kuna ≥1 true ikifuatiwa na false. Jibu: index ya FIRST false.
pub fn bisect_from(flags: &[Option<bool>]) -> Option<usize> {
    let mut saw_ok: bool = false;
    let mut last_ok: Option<usize> = None;
    for (i, f) in flags.iter().enumerate() {
        match f {
            Some(true) => {
                saw_ok = true;
                last_ok = Some(i);
            }
            Some(false) => {
                if saw_ok {
                    return Some(i);
                }
            }
            None => {} // snapshot bila lint (lugha isiyopimwa) — haishukiwi
        }
    }
    #[allow(clippy::let_and_return)]
    let r = last_ok.map(|_| usize::MAX); // zote ok — hakuna kupotoka (signal)
    r.filter(|x| *x != usize::MAX)
}

// ---------------------------------------------------------------------------
// HTTP endpoints
// ---------------------------------------------------------------------------

use axum::Json;
use axum::extract::Query;
use std::collections::HashMap;

/// GET /api/ide/timetravel/list?path=... — metadata tu.
pub async fn api_list(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let Some(path) = q.get("path") else {
        return Json(json!({ "ok": false, "error": "path ni lazima" }));
    };
    let root = crate::ide_api::ide_workspace_root();
    Json(json!({ "ok": true, "path": path, "snapshots": list(&root, path), "max": MAX_SNAPSHOTS }))
}

/// POST /api/ide/timetravel/revert {path, index} — kodi ya nyuma inarudi.
pub async fn api_revert(Json(r): Json<Value>) -> Json<Value> {
    let (Some(path), Some(idx)) = (
        r["path"].as_str().map(|s| s.to_string()),
        r["index"].as_u64().map(|i| i as usize),
    ) else {
        return Json(json!({ "ok": false, "error": "path + index ni lazima" }));
    };
    let root = crate::ide_api::ide_workspace_root();
    let Some(snap) = snapshot(&root, &path, idx) else {
        return Json(json!({ "ok": false, "error": format!("snapshot {idx} haipo") }));
    };
    let Some(content) = snap["content"].as_str() else {
        return Json(json!({ "ok": false, "error": "snapshot haina content (metadata tu)" }));
    };
    // Ndani ya workspace pekee (safeguard ileile ya IDE).
    let Some(full) = crate::ide_api::safe_path(&path) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace — imekataliwa" }));
    };
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let cur = std::fs::read_to_string(&full).unwrap_or_default();
    let diff = line_diff(&cur, content);
    let pure = std::fs::write(&full, content).is_ok();
    // Snapshot ya revert inarekodiwa pia (historia haitupi mtu wako).
    let _ = record(&root, &path, content, snap["ok"].as_bool().unwrap_or(false));
    Json(json!({
        "ok": pure,
        "path": path,
        "reverted_to": idx,
        "t_ms": snap["t"]
        ,
        "diff_from_current": diff,
    }))
}

/// POST /api/ide/timetravel/bisect {path} — kama: hatua ya kupotoka.
pub async fn api_bisect(Json(r): Json<Value>) -> Json<Value> {
    let Some(path) = r["path"].as_str().map(|s| s.to_string()) else {
        return Json(json!({ "ok": false, "error": "path ni lazima" }));
    };
    let root = crate::ide_api::ide_workspace_root();
    let snaps = list(&root, &path);
    if snaps.is_empty() {
        return Json(json!({ "ok": true, "found": false, "reason": "journal tupu" }));
    }
    let flags: Vec<Option<bool>> = snaps.iter().map(|s| s["ok"].as_bool()).collect();
    match bisect_from(&flags) {
        Some(i) => {
            // Semantic diff: snapshot iliyofanya kazi (i-1) dhidi ya iliyoharibika (i)
            let good = if i > 0 { snapshot(&root, &path, i - 1) } else { None };
            let bad = snapshot(&root, &path, i);
            let diff = match (&good, &bad) {
                (Some(g), Some(b)) => line_diff(
                    g["content"].as_str().unwrap_or(""),
                    b["content"].as_str().unwrap_or(""),
                ),
                _ => json!({ "added": [], "removed": [] }),
            };
            Json(json!({
                "ok": true,
                "found": true,
                "broken_step": i,
                "last_ok_step": if i > 0 { Some(i - 1) } else { None },
                "symbols_breakage": bad.as_ref().and_then(|b| b.get("symbols")).cloned().unwrap_or(json!([])),
                "semantic_diff": diff,
                "hint": "Rudi kwenye last_ok_step (revert), patch kiulimbikizo mpya kwenye mada mbaya.",
            }))
        }
        None => Json(json!({
            "ok": true,
            "found": false,
            "reason": "zote ok (au hakuna mfululizo true→false) — kosa halianzi kwenye journal hii",
        })),
    }
}

// ---------------------------------------------------------------------------
// Tests — hesabu safi (deterministic), hakuna mocks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tmproot(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fundi-tt-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn symbols_hesabu_halisi() {
        let v = symbols_of("fn main() {}\nstruct Kilo;\nimpl Kilo {}", "m.rs");
        assert!(v.len() >= 3);
        let w = symbols_of("function Hello(){}\nclass Bot {}", "app.js");
        assert!(w.iter().any(|s| s.contains("function")));
        assert!(w.iter().any(|s| s.contains("class")));
    }

    #[test]
    fn record_ina_snapshot_metadata_na_content_guard() {
        let root = tmproot("rec");
        let e = record(&root, "src/a.rs", "fn main() {}", true);
        assert_eq!(e["ok"], true);
        assert!(e.get("sha256").and_then(|v| v.as_str()).map(|s| s.len() == 64).unwrap_or(false));
        let l = list(&root, "src/a.rs");
        assert_eq!(l.len(), 1);
        assert!(l[0].get("content").is_none(), "list haiitaji content");
        let s = snapshot(&root, "src/a.rs", 0).unwrap();
        assert_eq!(s.get("content").and_then(|v| v.as_str()), Some("fn main() {}"));
    }

    #[test]
    fn snapshot_cap_50() {
        let root = tmproot("cap");
        for i in 0..55 {
            record(&root, "x.js", &format!("var v = {i};"), true);
        }
        assert_eq!(list(&root, "x.js").len(), MAX_SNAPSHOTS);
    }

    #[test]
    fn line_diff_detects_added_removed() {
        let d = line_diff("fn a() {\n  let x = 1;\n}", "fn a() {\n  let x = 2;\n  let y = 3;\n}");
        let added: Vec<&str> = d["added"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
        let removed: Vec<&str> = d["removed"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
        assert!(added.iter().any(|l| l.contains("let y = 3")));
        assert!(removed.iter().any(|l| l.contains("let x = 1")));
    }

    #[test]
    fn bisect_first_false_baada_true() {
        // ok: T T F F → kupotoka kwenye index 2
        assert_eq!(bisect_from(&[Some(true), Some(true), Some(false), Some(false)]), Some(2));
        // zote false bila ok moja → hakuna signal
        assert_eq!(bisect_from(&[Some(false), Some(false)]), None);
        // zote true → hakuna kupotoka
        assert_eq!(bisect_from(&[Some(true), Some(true)]), None);
        // lint haitambui (None hazihujumuisha mfululizo)
        assert_eq!(bisect_from(&[Some(true), None, Some(false)]), Some(2));
    }
}
