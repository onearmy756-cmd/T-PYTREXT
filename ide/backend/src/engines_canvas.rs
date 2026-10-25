//! engines_canvas.rs — kiunganishi kati ya IDE na MBILINYI Agent Canvas (vendored, v1.26.0).
//!
//! MBILINYI ENGINE imevendored kwenye `engines/mbilinyi/` (MIT, upstream OpenHands/OpenHands)
//! kama source reference. Module hii hutoa API ndogo ya backend inayosoma snapshot
//! hiyo na kuripoti hali yake kwa UI: metadata (kutoka manifest + package.json),
//! path halisi za modules muhimu, na ukamilishaji wa config ya startup.
//!
//! Kanuni za repo: hakuna mock kwenye njia za production — kila thamani inatoka
//! kwenye faili halisi la vendored source (`engines/mbilinyi`).

use crate::AppState;
use axum::{routing::get, Json, Router};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Chanzo kimoja cha uhakika: vendored snapshot ya MBILINYI ENGINE.
fn vendor_root() -> PathBuf {
    PathBuf::from("engines").join("mbilinyi")
}

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Meta data ya vendored OpenHands (package.json = chanzo halisi la version).
async fn status() -> Json<Value> {
    let root = vendor_root();
    let pkg_path = root.join("package.json");
    let pkg = read_json(&pkg_path).unwrap_or(Value::Null);
    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown)")
        .to_string();
    let pkg_name = pkg
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown)")
        .to_string();

    // Paths halisi zinazohusiana na Agent Canvas stack.
    let entry_cli = root.join("bin").join("agent-canvas.mjs");
    let defaults = root.join("config").join("defaults.json");
    let defaults_cfg = read_json(&defaults).unwrap_or(Value::Null);
    let node_engine = pkg
        .get("engines")
        .and_then(|e| e.get("node"))
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown)")
        .to_string();

    Json(json!({
        "ok": true,
        "engine": "MBILINYI IDE",
        "upstream": "OpenHands/OpenHands",
        "license": "MIT",
        "mode": "vendored source reference — haiendeshwi (hakuna mock)",
        "package": pkg_name,
        "version": version,
        "node_required": node_engine,
        "root": root.display().to_string(),
        "paths": {
            "package_json": pkg_path.display().to_string(),
            "cli_entry": entry_cli.display().to_string(),
            "defaults": defaults.display().to_string(),
        },
        "cli_entry_exists": entry_cli.exists(),
        "defaults_json_exists": defaults.exists(),
        "defaults_stack": {
            "agent_server": defaults_cfg.get("versions").and_then(|v| v.get("agentServer")).cloned().unwrap_or(Value::Null),
            "automation": defaults_cfg.get("versions").and_then(|v| v.get("automation")).cloned().unwrap_or(Value::Null),
        },
    }))
}

/// Ramani ya modules muhimu za Agent Canvas na src/ (HALISI — directory travel).
async fn modules() -> Json<Value> {
    let root = vendor_root();

    let top_dirs = [
        "src", "bin", "config", "docs", "docker", "helm", "electron", "scripts",
        "specs", "tools", "examples", "public", "tests", "__tests__",
    ];
    let mut present = Vec::new();
    let mut missing = Vec::new();
    for d in top_dirs {
        let p = root.join(d);
        if p.is_dir() {
            present.push(json!({"name": d, "path": p.display().to_string()}));
        } else {
            missing.push(d.to_string());
        }
    }

    let src_files = [
        "src/entry.client.tsx",
        "src/root.tsx",
        "src/routes/_index.tsx",
    ];
    let mut src_paths = Vec::new();
    for f in src_files {
        let p = root.join(f);
        src_paths.push(json!({
            "name": f,
            "exists": p.is_file(),
            "path": p.display().to_string(),
        }));
    }

    let has_pkg = root.join("package.json").is_file();
    let has_lockfile = root.join("package-lock.json").is_file();
    Json(json!({
        "ok": true,
        "engine": "MBILINYI IDE",
        "root": root.display().to_string(),
        "has_package_json": has_pkg,
        "has_lockfile": has_lockfile,
        "top_dirs_present": present,
        "top_dirs_missing": missing,
        "key_files": src_paths,
    }))
}

/// Config ya startup ya OpenHands Agent Canvas (DELETE kwa UI ya IDE).
async fn setup() -> Json<Value> {
    let root = vendor_root();
    let pkg = read_json(&root.join("package.json")).unwrap_or(Value::Null);
    let version = pkg
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown)");
    let cli = "cd engines/mbilinyi && npm install && npm i -g . && agent-canvas --info";
    Json(json!({
        "ok": true,
        "engine": "MBILINYI IDE",
        "version": version,
        "node_required": ">=24",
        "install_hint": cli,
        "dev_hint": "cd engines/mbilinyi && npm install && npm run dev",
        "check_hint": "agent-canvas --version",
        "info": "Agent Canvas inaendeshwa na uvx kwa agent-server + automation backend — angalia bin/agent-canvas.mjs.",
    }))
}

/// Endpoint ya Canvas Extension ya MBILINYI IDE.
/// Inarudisha URI ile ile ya `/ide` ili kupachika Agent Canvas kama page moja
/// (extension.js inaita).
async fn ide_shell() -> Json<Value> {
    Json(json!({
        "ok": true,
        "url": "/ide",
        "note": "MBILINYI IDE — route halisi GET /ide → MBILINYI Agent Canvas (React SPA), kupachikwa ndani ya Agent Canvas kama Canvas Extension. Hakuna ukurasa mpya."
    }))
}

/// Ollama kuendeleza OpenHands — config HALISI kutoka MTECHOS AI setup.
/// OpenHands inatumia provider-connections (base_url + api_key); Ollama za LAN
/// (127.0.0.1:11434) ni OpenAI-compatible, hivyo base_url = http://127.0.0.1:11434/v1.
/// Hii inataa models halisi kutoka MTECHOS (llm_setup::models_list) + mteja,
/// haitumi model bandia.
async fn ollama_provider() -> Json<Value> {
    // Models halisi za MTECHOS (llm_setup::models_list — source of truth ya ndani)
    let mtechos = crate::llm_setup::models_list();
    let oh_base = std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11434/v1".into());
    Json(json!({
        "ok": true,
        "provider": "ollama",
        "canvas_connection": {
            "display_name": "Ollama (MTECHOS)",
            "provider": "ollama",
            "base_url": oh_base,
            "api_key": "ollama",
            "note": "Paste kwenye MBILINYI IDE: Customize > LLM > Add connection — base_url ni OpenAI-compatible endpoint"
        },
        "recommended_models": ["qwen2.5vl:3b", "devstral:latest", "qwen2.5-coder:7b"],
        "mtechos_models": mtechos,
        "hint": format!("Kama Ollama inaendesha, curl {}/models — model zote za ndani zitaripotiwa; MBILINYI IDE inazichukua kwa ACP/openai-compatible", oh_base)
    }))
}

/// Routes ya MBILINYI Agent Canvas bridge (merge kwenye router kuu).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/engines/canvas/status", get(status))
        .route("/api/engines/canvas/modules", get(modules))
        .route("/api/engines/canvas/setup", get(setup))
        .route("/api/engines/canvas/ide-shell", get(ide_shell))
        .route("/api/engines/canvas/ollama-provider", get(ollama_provider))
}
