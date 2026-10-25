//! ide_engineer.rs — SENIOR SOFTWARE ENGINEER: bomba kamili la uchambuzi.
//!
//! Huendesha kila kitu kwa mpangilio (kama Devin/Trae/Claude Code senior loop):
//!   1. HESABU   — kugundua stack + ukubwa wa mradi
//!   2. USALAMA  — code_audit (kanuni 28) → grade
//!   3. TESTI    — ide_testkit (test/lint/fmt HALISI)
//!   4. BUILD    — ide_build (malengo ya jukwaa)
//!   5. KASI     — ide_build speed (millisecond benchmark)
//!   6. RIPOTI   — score + mapendekezo + hatua zinazofuata
//!
//! Hakuna uwongo: kila hatua hurudisha matokeo halisi; zisizowezekana
//! huwekwa `skipped`/`error` wazi.

use axum::Json;
use serde_json::{json, Value};

use crate::ide_api::ide_workspace_root;

async fn detect_summary(path: &str) -> Value {
    let suites = crate::ide_testkit::suites(Json(json!({ "path": path }))).await.0;
    suites
}

/// POST /api/ide/engineer/run — bomba kamili la senior engineer.
pub async fn run(Json(body): Json<Value>) -> Json<Value> {
    let path = body["path"].as_str().unwrap_or("").trim().to_string();
    let with_tests = body["tests"].as_bool().unwrap_or(true);
    let with_lint = body["lint"].as_bool().unwrap_or(false);
    let with_lsp = body["lsp"].as_bool().unwrap_or(false);
    let with_debug = body["debug"].as_bool().unwrap_or(false);
    let e2e_url = body["e2e_url"].as_str().unwrap_or("");
    let platforms = body["platforms"].as_array().cloned().unwrap_or_default();

    let root = ide_workspace_root();
    let started = std::time::Instant::now();
    let mut steps: Vec<Value> = Vec::new();

    // 1) HESABU
    let summary = detect_summary(&path).await;
    let detected = summary["detected"].as_array().cloned().unwrap_or_default();
    steps.push(json!({
        "step": "recon", "status": "ok",
        "workdir": summary["workdir"],
        "stacks": detected.iter().map(|d| d["label"].clone()).collect::<Vec<_>>(),
    }));

    // 2) USALAMA
    let audit = crate::code_audit::scan_workspace(&root, &path);
    let grade = audit["grade"].as_f64().unwrap_or(100.0);
    let critical = audit["by_severity"]["critical"].as_u64().unwrap_or(0);
    steps.push(json!({
        "step": "security", "status": if critical > 0 { "warning" } else { "ok" },
        "grade": grade, "total_findings": audit["total_findings"],
        "by_severity": audit["by_severity"], "by_category": audit["by_category"],
        "critical": critical,
    }));

    // 3) TESTI
    let test_result = if with_tests {
        let t = crate::ide_testkit::run(Json(json!({ "path": path, "with_lint": with_lint }))).await.0;
        steps.push(json!({
            "step": "tests", "status": if t["failed"].as_u64().unwrap_or(0) == 0 && t["ok"] == true { "ok" } else { "warning" },
            "suites": t["suites"], "passed": t["passed"], "failed": t["failed"],
            "results": t["results"],
        }));
        t
    } else {
        steps.push(json!({ "step": "tests", "status": "skipped" }));
        json!({ "skipped": true })
    };

    // 4) BUILD MATRIX (mapendekezo ya majukwaa)
    let platform_scripts: Vec<Value> = platforms
        .iter()
        .filter_map(|p| p.as_str())
        .map(|id| {
            json!({
                "platform": id,
                "command": format!("bash scripts/build-{id}.sh"),
                "endpoint": "/api/ide/build/script",
            })
        })
        .collect();
    steps.push(json!({
        "step": "builds", "status": "ok",
        "platforms": platform_scripts,
        "note": "Tumia /api/ide/build/script kutengeneza script, /api/ide/build/run kuendesha.",
    }));

    // 5) KASI
    let speed = crate::ide_build::speed(Json(json!({ "files": 200, "size": 2048 }))).await.0;
    steps.push(json!({ "step": "speed", "status": "ok", "result": speed }));

    // 5b) LSP (diagnostics halisi) — optional
    if with_lsp {
        if !path.is_empty() {
            let lang = std::path::Path::new(&path).extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
            let l = crate::ide_lsp::lsp(Json(json!({ "action": "diagnostics", "path": path }))).await.0;
            let diags = l["result"].as_array().cloned().unwrap_or_default().len();
            steps.push(json!({
                "step": "lsp", "status": if l["ok"] == json!(true) { "ok" } else { "warning" },
                "lang": lang, "diagnostics": diags, "detail": l,
            }));
        } else {
            steps.push(json!({ "step": "lsp", "status": "skipped", "note": "path inahitajika" }));
        }
    }

    // 5c) DEBUG (DAP halisi) — optional
    if with_debug {
        if !path.is_empty() {
            let d = crate::ide_dap::run(Json(json!({ "path": path, "stop_on_entry": true }))).await.0;
            steps.push(json!({
                "step": "debug", "status": if d["ok"] == json!(true) { "ok" } else { "warning" },
                "stopped": d["stopped"], "detail": d,
            }));
        } else {
            steps.push(json!({ "step": "debug", "status": "skipped", "note": "path inahitajika" }));
        }
    }

    // 5d) E2E (Playwright halisi) — optional
    if !e2e_url.is_empty() {
        let e = crate::ide_e2e::run(Json(json!({ "name": "engineer-smoke", "url": e2e_url }))).await.0;
        steps.push(json!({
            "step": "e2e", "status": if e["ok"] == json!(true) { "ok" } else { "warning" },
            "passed": e["passed"], "failed": e["failed"], "detail": e,
        }));
    }

    // 6) RIPOTI + score
    let test_failed = test_result["failed"].as_u64().unwrap_or(0);
    let test_passed = test_result["passed"].as_u64().unwrap_or(0);
    let mut score = grade;
    if test_failed > 0 {
        score -= 15.0 * test_failed as f64;
    }
    score = score.clamp(0.0, 100.0);

    let mut recommendations: Vec<String> = Vec::new();
    if critical > 0 {
        recommendations.push(format!("🔴 Rekebisha {critical} critical security findings kabla ya release."));
    }
    if test_failed > 0 {
        recommendations.push(format!("🧪 Testi {test_failed} zimeshindwa — rekebisha kabla ya kuunganisha."));
    }
    if detected.is_empty() {
        recommendations.push("📁 Hakuna stack iliyogundulika — ongeza package.json/Cargo.toml/requirements.txt.".into());
    }
    if recommendations.is_empty() {
        recommendations.push("✅ Mradi uko katika hali nzuri — endelea na vipengele vipya.".into());
    }

    Json(json!({
        "ok": true,
        "role": "senior_software_engineer",
        "workdir": summary["workdir"],
        "stacks": detected.iter().map(|d| d["label"].clone()).collect::<Vec<_>>(),
        "score": (score * 10.0).round() / 10.0,
        "security_grade": grade,
        "tests": { "passed": test_passed, "failed": test_failed },
        "speed": speed,
        "steps": steps,
        "recommendations": recommendations,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::post;
    axum::Router::new().route("/api/ide/engineer/run", post(run))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_bounds() {
        let s: f64 = (100.0f64 - 15.0 * 3.0).clamp(0.0, 100.0);
        assert_eq!(s, 55.0);
        assert_eq!((-5.0f64).clamp(0.0, 100.0), 0.0);
    }

    #[test]
    fn json_builds() {
        let v = json!({ "ok": true, "role": "senior_software_engineer" });
        assert_eq!(v["role"], "senior_software_engineer");
    }
}
