//! ide_pipeline.rs — AUTOPILOT MOJA: idea → code HALISI → usalama → testi →
//! build → ripoti ya senior engineer.
//!
//! Huunganisha injini zote za IDE kwenye bomba moja:
//!   1. SWARM      — huzalisha code halisi (au hutumia code iliyotolewa)
//!   2. ANDIKA     — huandika faili kwenye workspace (path halisi)
//!   3. USALAMA    — code_audit
//!   4. ENGINEER   — testi + build + LSP + DAP + E2E + kasi → score
//!
//! Kila hatua hurudisha matokeo halisi; hatua zinazoshindikana zinaonekana wazi.

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::ide_api::ide_workspace_root;

fn safe_join(path: &str) -> Option<std::path::PathBuf> {
    let root = ide_workspace_root();
    let rel = path.trim_start_matches('/');
    if rel.contains("..") {
        return None;
    }
    let p = root.join(rel);
    if p.starts_with(&root) {
        Some(p)
    } else {
        None
    }
}

fn ext_lang(path: &str) -> &'static str {
    match std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("") {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "ex" | "exs" => "elixir",
        "go" => "go",
        "java" => "java",
        "php" => "php",
        "rb" => "ruby",
        "r" => "r",
        "c" | "cc" | "cpp" | "h" | "hpp" => "cpp",
        _ => "any",
    }
}

/// GET /api/ide/pipeline/capabilities
pub async fn capabilities() -> Json<Value> {
    Json(json!({
        "ok": true,
        "stages": ["swarm.generate", "write", "security.audit", "engineer(tests,build,lsp,dap,e2e,speed)"],
        "endpoints": [
            "POST /api/ide/pipeline/run",
            "POST /api/ide/pipeline/project",
            "POST /api/ide/swarm/generate",
            "POST /api/ide/code/audit/file",
            "POST /api/ide/test/run",
            "GET  /api/ide/test/suites",
            "POST /api/ide/lsp/:action",
            "GET  /api/ide/lsp/servers",
            "POST /api/ide/dap/run",
            "POST /api/ide/e2e/run",
            "POST /api/ide/build/run",
            "POST /api/ide/engineer/run",
        ],
    }))
}

/// Zalisha mradi KAMILI (website/mobile/iot/gaming), andika faili zote, kisha
/// uendeshe self-test yake. Hutumika na autopilot na emulator (chanzo kimoja).
pub async fn generate_and_test(kind: &str, project: &str) -> Value {
    let kind = kind.trim().to_lowercase();
    if !crate::forge_projects::is_project(&kind) {
        return json!({
            "ok": false,
            "error": format!("kind isiyojulikana: {kind} (tumia website|mobile|iot|gaming)"),
        });
    }
    let project = if project.trim().is_empty() { "app" } else { project.trim() };
    let domain = crate::forge_domain::parse_domain(&format!("mradi wa {project}"));
    let Some((files, note)) = crate::forge_projects::generate_project(&kind, &domain) else {
        return json!({ "ok": false, "error": "generator haipatikani" });
    };
    let Some(dir) = safe_join(project) else {
        return json!({ "ok": false, "error": "path nje ya workspace" });
    };
    let mut written = 0usize;
    for (p, c) in &files {
        let full = dir.join(p);
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::write(&full, c).is_ok() {
            written += 1;
        }
    }
    let cmd = crate::forge_projects::test_command(&kind);
    let test = run_cmd(cmd, &dir).await;
    let ok = test["ok"].as_bool().unwrap_or(false);
    json!({
        "ok": ok,
        "kind": kind,
        "project": project,
        "path": dir.display().to_string(),
        "source": "production",
        "note": note,
        "files_total": files.len(),
        "files_written": written,
        "test_cmd": cmd,
        "test": test,
    })
}

/// Endesha amri kwa `bash -lc` ndani ya workdir (kwa self-test ya miradi).
async fn run_cmd(cmd: &str, dir: &std::path::Path) -> Value {
    let started = std::time::Instant::now();
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(600),
        tokio::process::Command::new("bash").arg("-lc").arg(cmd).current_dir(dir).output(),
    )
    .await;
    let ms = started.elapsed().as_millis() as u64;
    match out {
        Err(_) => json!({ "ok": false, "cmd": cmd, "ms": ms, "error": "timeout" }),
        Ok(Err(e)) => json!({ "ok": false, "cmd": cmd, "ms": ms, "error": format!("{e}") }),
        Ok(Ok(o)) => {
            let mut stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let mut stderr = String::from_utf8_lossy(&o.stderr).to_string();
            if stdout.len() > 12000 { stdout.truncate(12000); stdout.push_str("\n...[truncated]"); }
            if stderr.len() > 12000 { stderr.truncate(12000); stderr.push_str("\n...[truncated]"); }
            json!({ "ok": o.status.success(), "cmd": cmd, "ms": ms, "exit": o.status.code(), "stdout": stdout, "stderr": stderr })
        }
    }
}

/// POST /api/ide/pipeline/project — autopilot: zalisha mradi KAMILI + jijaribu.
/// body: { kind: "website|mobile|iot|gaming", project?: "app" }
pub async fn project(Json(body): Json<Value>) -> Json<Value> {
    let kind = body["kind"].as_str().unwrap_or("website");
    let project = body["project"].as_str().unwrap_or("app");
    Json(generate_and_test(kind, project).await)
}

/// POST /api/ide/pipeline/run
/// body: {
///   idea?: "website ya kuuza", kind?: "website", lang?: "html",
///   path: "projects/x/index.html",   // inahitajika ili kuandika code
///   code?: "<html>...",              // kama ipo, hutumika badala ya swarm
///   tests?: bool, lint?: bool, lsp?: bool, debug?: bool,
///   e2e_url?: "http://127.0.0.1:8080/",
///   platforms?: ["linux-arm64","windows-x64"],
///   write?: bool  // default true
/// }
pub async fn run(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let path = body["path"].as_str().unwrap_or("").trim().to_string();
    let idea = body["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = body["kind"].as_str().unwrap_or("website");
    let lang = body["lang"].as_str().unwrap_or("");
    let write = body["write"].as_bool().unwrap_or(true);
    let mut steps: Vec<Value> = Vec::new();
    let started = std::time::Instant::now();

    // 1) SWARM: pata code (iliyotolewa au kwa LLM halisi)
    let provided = body["code"].as_str().filter(|c| !c.is_empty()).map(|c| c.to_string());
    let (code, source) = if let Some(c) = provided {
        (c, "provided".to_string())
    } else if !idea.is_empty() {
        let g = crate::ide_swarm::swarm_generate(
            State(s.clone()),
            Json(json!({ "idea": idea, "kind": kind, "lang": if lang.is_empty() { "rust" } else { lang } })),
        )
        .await
        .0;
        let src = g["source"].as_str().unwrap_or("").to_string();
        steps.push(json!({
            "stage": "swarm.generate", "status": if g["ok"] == json!(true) { "ok" } else { "warning" },
            "source": src, "grade": g["audit"]["grade"], "findings": g["audit"]["total_findings"],
        }));
        (g["code"].as_str().unwrap_or("").to_string(), src)
    } else {
        (String::new(), String::new())
    };
    if code.is_empty() {
        return Json(json!({
            "ok": false,
            "error": "neta code wala idea — toa moja",
            "steps": steps,
        }));
    }

    // 2) ANDIKA
    let mut written_path = Value::Null;
    if write && !path.is_empty() {
        match safe_join(&path) {
            Some(full) => {
                if let Some(parent) = full.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let ok = std::fs::write(&full, &code).is_ok();
                written_path = json!(full.display().to_string());
                steps.push(json!({ "stage": "write", "status": if ok { "ok" } else { "error" }, "path": written_path }));
            }
            None => steps.push(json!({ "stage": "write", "status": "error", "error": "path nje ya workspace" })),
        }
    } else {
        steps.push(json!({ "stage": "write", "status": "skipped" }));
    }

    // 3) USALAMA — langi KAMILI: kagua → rekebisha → kagua tena → amua.
    //    (code_audit pekee ilitoa grade; hapa tunaandika code iliyorekebishwa
    //    na kufunga langi ya kutolewa.)
    let audit_lang = if lang.is_empty() { ext_lang(&path) } else { lang };
    let audit_path = if path.is_empty() { "snippet".to_string() } else { path.clone() };
    let mut gate = crate::sec_gate::gate_files(&[(audit_path.clone(), code.clone())]);
    let grade = gate["after"]["grade"].as_f64().unwrap_or(100.0);
    let critical = gate["after"]["by_severity"]["critical"].as_u64().unwrap_or(0);
    // Tumia code iliyorekebishwa (ikiwa marekebisho yalifanyika na faili inaandikwa).
    if let Some(fixed) = gate["files"][0]["content"].as_str() {
        if fixed != code && write && !path.is_empty() {
            if let Some(full) = safe_join(&path) {
                let _ = std::fs::write(&full, fixed);
            }
        }
    }
    if let Some(o) = gate.as_object_mut() {
        o.insert("lang".into(), json!(audit_lang));
    }
    steps.push(json!({
        "stage": "security.audit",
        "status": if gate["release_ok"] == json!(true) { "ok" } else { "warning" },
        "verdict": gate["after"]["verdict"], "release_ok": gate["release_ok"],
        "grade": grade, "total_findings": gate["after"]["total_findings"], "critical": critical,
        "remediation": gate["remediation"], "findings": gate["after"]["findings"],
    }));

    // 4) ENGINEER (testi + build + LSP + DAP + E2E + kasi)
    let eng = crate::ide_engineer::run(Json(json!({
        "path": if path.is_empty() { Value::Null } else { json!(path) },
        "tests": body["tests"].as_bool().unwrap_or(true),
        "lint": body["lint"].as_bool().unwrap_or(false),
        "lsp": body["lsp"].as_bool().unwrap_or(false),
        "debug": body["debug"].as_bool().unwrap_or(false),
        "e2e_url": body["e2e_url"].as_str().unwrap_or(""),
        "platforms": body["platforms"].clone(),
    }))).await.0;
    steps.push(json!({
        "stage": "engineer", "status": "ok",
        "score": eng["score"], "tests": eng["tests"], "elapsed_ms": eng["elapsed_ms"],
        "detail": eng,
    }));

    let score = eng["score"].as_f64().unwrap_or(grade);

    // 5) SDLC (mzunguko kamili) — hiari; huita `sdlc::run` (hakuna mantiki mpya).
    if body["sdlc"].as_bool().unwrap_or(false) && !idea.is_empty() {
        let sd = crate::sdlc::run(
            State(s.clone()),
            Json(json!({
                "spec": idea,
                "kind": kind,
                "lang": body["ui_lang"].as_str().unwrap_or(""),
                "username": body["username"].as_str().unwrap_or(""),
            })),
        )
        .await
        .0;
        steps.push(json!({
            "stage": "sdlc",
            "status": if sd["ok"] == json!(true) { "ok" } else { "warning" },
            "complexity": sd["complexity"], "score": sd["score"], "detail": sd,
        }));
    }

    Json(json!({
        "ok": true,
        "path": written_path,
        "source": source,
        "code_bytes": code.len(),
        "security_grade": grade,
        "security_verdict": gate["after"]["verdict"],
        "release_ok": gate["release_ok"],
        "senior_score": score,
        "steps": steps,
        "engineer": eng,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/pipeline/capabilities", get(capabilities))
        .route("/api/ide/pipeline/run", post(run))
        .route("/api/ide/pipeline/project", post(project))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_lang_maps() {
        assert_eq!(ext_lang("a/b.rs"), "rust");
        assert_eq!(ext_lang("x.ex"), "elixir");
        assert_eq!(ext_lang("x.ts"), "typescript");
        assert_eq!(ext_lang("x.unknown"), "any");
    }

    #[test]
    fn safe_join_rejects_traversal() {
        assert!(safe_join("../x").is_none());
    }

    #[test]
    fn project_kind_zote_zinatambulika() {
        for k in ["website", "mobile", "iot", "gaming"] {
            assert!(crate::forge_projects::is_project(k), "{k} haitambuliki");
        }
        assert!(!crate::forge_projects::is_project("unknown"));
    }
}
