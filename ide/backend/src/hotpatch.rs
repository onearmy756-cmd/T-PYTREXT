//! hotpatch.rs — MULTI-AGENT SWARM LIVE HOT-PATCHING (layer ya swarm).
//!
//! Ukwaruzo halisi wa patch bila KUSIMAMISHA programu:
//!   - **Unity ya Workspace (web content — html/css/js/json/md/...)**:
//!     faili inaandikwa kwenye workspace moja kwa moja; server ya akili
//!     (axum ServeDir + iframe preview) inaONEKANA kwa ombi lijilo —
//!     HAKUNA restart ya process, HAKUNA downtime.
//!   - **Binary/crate (rust na wenzake)**: patch inapimwa na selfcheck
//!     + kuhifadhiwa kwenye journal; процесu ya `.so`/binary update ina
//!     HITALIKA restart/HITL (kanuni ya usalama wa production) — tunasema
//!     uongo HATA MOJA ("needs_process_restart": true).
//!
//! Swarm hii inaitwa na agent WOTE kupitia HTTP (`POST /api/ide/hotpatch`)
//! — kila agent anayefanya kazi kwenye repo anaweza kuhofa patch iliyothibitishwa.
//! Kagua: lint yapite (`code_rules::lint`, kanuni 17 + security) KABLA
//! ya kuandika — hakuna patch yenye makosa inayotumia prod.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const JOURNAL_MAX: usize = 500;

/// Lugha zinazoonekana kama web-content (hot-patch halisi, zero-downtime).
pub fn is_web_content(path: &str) -> bool {
    matches!(
        path.rsplit('.').next().unwrap_or("").to_lowercase().as_str(),
        "html" | "htm" | "css" | "js" | "jsx" | "ts" | "tsx" | "mjs" | "json" | "md" | "svg"
    )
}

/// Lugha zinazopimwa na selfcheck (lint) — nenda only kwa hizi.
pub fn is_lintable(path: &str) -> bool {
    matches!(
        path.rsplit('.').next().unwrap_or("").to_lowercase().as_str(),
        "js" | "jsx" | "ts" | "tsx" | "rs" | "ex" | "exs" | "sql" | "html" | "go" | "c" | "h"
            | "cpp" | "hpp" | "java" | "cs" | "php" | "rb" | "kt" | "swift" | "dart"
    )
}

fn sha256_hex(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Journal append (best-effort) — `.hotpatch/journal.jsonl` ndani ya root.
pub fn journal_append(root: &Path, entry: &Value) {
    let d = root.join(".hotpatch");
    let _ = std::fs::create_dir_all(&d);
    let f = d.join("journal.jsonl");
    let existing = std::fs::read_to_string(&f).unwrap_or_default();
    let line = entry.to_string();
    let mut lines: Vec<&str> = existing.lines().collect();
    lines.push(line.as_str());
    let start = lines.len().saturating_sub(JOURNAL_MAX);
    let body = lines[start..].join("\n");
    let _ = std::fs::write(&f, if body.is_empty() { String::new() } else { format!("{body}\n") });
}

fn lint_for(path: &str, content: &str) -> Value {
    if !is_lintable(path) {
        return json!({ "ok": true, "skipped": true, "reason": "lugha haipimwi kwenye linter (web content safi)" });
    }
    crate::code_rules::lint(content, path)
}

/// Mzunguko wa KUHOT-PATCH: lint gate → stage → journal.
/// PATCH/KUU WANAYOHOFU kwenye repo (swarm-ready): iko swahili tu —
/// swarm inaita HTTP; seva inafanya mzunguko huu moja kwa moja.
pub fn apply(root: &Path, path: &str, content: &str) -> Value {
    // 1) SAFEGUARD: path ndani ya workspace (safeguard ile ile ya IDE)
    let Some(full) = crate::ide_api::safe_path_in_root(root, path) else {
        return json!({ "ok": false, "error": "path nje ya workspace — imekataliwa (hot-patch imezuiliwa)" });
    };

    // 2) LINT GATE: kosi zozote → kuzuia patch (usawa wa mtaalamu)
    let lint = lint_for(path, content);
    if lint["ok"].as_bool().unwrap_or(false) == false && is_lintable(path) {
        return json!({
            "ok": false,
            "rejected_by": "linter",
            "lint": lint,
            "note": "patch yenye makosa IMEZUIWA — rekebisha na rudia (ilani ya usalama)",
        });
    }

    // 3) STAGE: andika (zero-downtime kwa web content — ServeDir ina sehemu ile ile)
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bak: Option<String> = std::fs::read_to_string(&full).ok();
    let write_ok = std::fs::write(&full, content).is_ok();

    // 4) JOURNAL: sha256 + lint + zero-downtime flag
    let downtime = !is_web_content(path);
    let entry = json!({
        "t": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|x| x.as_millis() as u64)
            .unwrap_or(0),
        "path": path,
        "sha256": sha256_hex(content),
        "size": content.len(),
        "lint_ok": lint["ok"],
        "zero_downtime": !downtime,
        "previous_sha256": bak.as_deref().map(sha256_hex).unwrap_or_default(),
    });
    journal_append(root, &entry);

    json!({
        "ok": write_ok,
        "path": path,
        "kind": if downtime { "needs_process_restart" } else { "web_content_zero_downtime" },
        "lint": lint,
        "sha256": entry["sha256"],
        "note": if downtime {
            "binary/crate: tepu HOT-PATCH ya process inahitaji HITL/restart (usalama) — patch imeSTAGE kwenye journal"
        } else {
            "web content: inaonekana live kwa ombi lijilo — hakuna downtime"
        },
    })
}

// ---------------------------------------------------------------------------
// HTTP endpoint — POST /api/ide/hotpatch {path, content}
// ---------------------------------------------------------------------------

use axum::Json;

pub async fn api(Json(r): Json<Value>) -> Json<Value> {
    let Some(path) = r["path"].as_str().map(|s| s.to_string()) else {
        return Json(json!({ "ok": false, "error": "path ni lazima" }));
    };
    let Some(content) = r["content"].as_str() else {
        return Json(json!({ "ok": false, "error": "content ni lazima" }));
    };
    let root = crate::ide_api::ide_workspace_root();
    let mut out = apply(&root, &path, content);
    // TIME-TRAVEL: kila hot-patch inarekodiwa kwenye journal ya muda (inahusiana).
    let ok = out.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if ok {
        let _ = crate::timetravel::record(&root, &path, content, true);
    }
    if let Some(o) = out.as_object_mut() {
        o.insert("by".to_string(), json!("hotpatch:v1 — multi-agent swarm layer"));
    }
    Json(out)
}

// ---------------------------------------------------------------------------
// Tests — safi na deterministic
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tmproot(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fundi-hp-{tag}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn web_content_inatambuilika() {
        assert!(is_web_content("x/index.html"));
        assert!(is_web_content("style.CSS"));
        assert!(!is_web_content("src/main.rs"));
    }

    #[test]
    fn lintable_lugha_20() {
        assert!(is_lintable("app.ts"));
        assert!(is_lintable("m.rs"));
        assert!(!is_lintable("notes.md"));
    }

    #[test]
    fn path_nje_inekataliwa() {
        let root = tmproot("safe");
        let r = apply(&root, "../../etc/passwd", "root:x:0:0");
        assert_eq!(r["ok"], false);
        assert!(r["error"].as_str().unwrap().contains("imekataliwa"));
    }

    #[test]
    fn kodi_yenye_kosa_imezuiliwa_kabla_kuandika() {
        let root = tmproot("reject");
        // Kosi ya linter: eval imekatazwa (security_global)
        let bad = apply(&root, "app.js", "eval('x')");
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["rejected_by"], "linter");
        // File haijaandikwa (workspace lenyenye tu journal)
        assert!(!root.join("app.js").exists(), "patch ya kosi HAIJAWA andikwa");
    }
}
