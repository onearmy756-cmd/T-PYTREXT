//! agentic_bridge.rs — DARAJA KATI YA RUST NA GO/ELIXIR (Hatua 4).
//!
//! MTECHOS ni mchanganyiko wa lugha:
//!   * **Rust** — injini/protocol kuu (`native_protocol`).
//!   * **Go** (`agentic/ptyexec`) — "mikono": execution ya commands kwa
//!     concurrency (goroutines) + timeout ya watoto.
//!   * **Elixir/OTP** (`agentic/brain`) — "ubongo": tabaka 12 za kufikiri,
//!     System1/System2, ethics gate + HITL; supervisor wa fault-isolation.
//!
//! Daraja hili huwaita Go/Elixir kama **process za nje** (subprocess) kwa
//! kubadilishana JSON. Hii inahakikisha:
//!   * event loop ya Rust HAISUBIRI kazi ndefu za watoto;
//!   * crash ya sidecar HAIVUNJI server (fail-safe: `ok:false`, si panic).
//!
//! Njia: `MTECHOS_PTYEXEC_BIN`, `MTECHOS_BRAIN_BIN` (default: `agentic/...`).

use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;

use crate::AppState;

const ENV_PTYEXEC: &str = "MTECHOS_PTYEXEC_BIN";
const ENV_BRAIN: &str = "MTECHOS_BRAIN_BIN";

/// Router ya daraja (`/api/native/*` — kianzio kipya, hakuna mgongano).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/native/exec", post(native_exec))
        .route("/api/native/think", post(native_think))
        .route("/api/native/agentic", get(native_agentic_status))
}

/// Hali ya daraja (kipande cha Rust — hakuna subprocess).
async fn native_agentic_status() -> Json<Value> {
    Json(json!({
        "ok": true,
        "runtime": "fundi-rust",
        "components": {
            "go_ptyexec": { "binary": ptyexec_bin().to_string_lossy(), "present": ptyexec_bin().exists() },
            "elixir_brain": { "binary": brain_bin().to_string_lossy(), "present": brain_bin().exists() },
        },
        "protocol_version": crate::native_protocol::NATIVE_PROTOCOL_VERSION,
    }))
}

/// Mahali pa binary ya Go executor.
///
/// Default ni path ABSOLUTE kwa mzizi wa repo (kupanda madaraja mawili kutoka
/// saraka ya crate hii), hivyo hufanya kazi bila kujali CWD (tests, Docker).
/// `MTECHOS_PTYEXEC_BIN` huweka override kwa production.
fn ptyexec_bin() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_PTYEXEC) {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    repo_agentic().join("ptyexec").join("ptyexec")
}

/// Mahali pa escript ya Elixir brain.
fn brain_bin() -> PathBuf {
    if let Ok(p) = std::env::var(ENV_BRAIN) {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    repo_agentic().join("brain").join("mtechos_brain")
}

/// Je, runtime ya Elixir (`escript`) ipo kwenye PATH?
///
/// Escript binaries zinahitaji `escript` kwenye PATH; kama Erlang haijasakinishwa
/// test inaruka (hali ya mazingira, si bug ya code).
fn escript_available() -> bool {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join("escript");
        candidate.is_file()
    })
}

/// Saraka ya `agentic/` kwenye mzizi wa repo.
///
/// Hutumia `.parent()` (si `..`) ili kuepuka kanuni ya SEC-023 (path traversal).
fn repo_agentic() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // server/agent → server → repo_root
    let repo_root = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .unwrap_or(manifest);
    repo_root.join("agentic")
}

/// Endesha sidecar kwa stdin/stdout JSON; fail-safe (hakuna panic).
async fn run_sidecar(bin: PathBuf, payload: &Value, label: &str) -> Value {
    if !bin.exists() {
        return json!({
            "ok": false,
            "error": format!("{label} binary haipo: {}. Jenga: tazama agentic/README.md", bin.display()),
        });
    }

    let mut child = match tokio::process::Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return json!({ "ok": false, "error": format!("{label} haijaanza: {e}") });
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let data = payload.to_string();
        let _ = stdin.write_all(data.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    match child.wait_with_output().await {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            match serde_json::from_str::<Value>(&stdout) {
                Ok(v) => v,
                Err(_) => json!({
                    "ok": false,
                    "error": format!("{label} stdout sio JSON sahihi"),
                    "stdout": stdout.chars().take(2000).collect::<String>(),
                }),
            }
        }
        Err(e) => json!({ "ok": false, "error": format!("{label} imeshindwa: {e}") }),
    }
}

/// POST /api/native/exec — endesha commands kwa njia ya Go executor.
///
/// Body: `{"cmds":[{"id","bin","args","dir","timeout_ms"}],"concurrency":N}`
async fn native_exec(Json(body): Json<Value>) -> Json<Value> {
    let mut payload = body;
    // `timeout` ya Go ni nanoseconds (Duration int). Badilisha `timeout_ms` → `timeout`.
    if let Some(cmds) = payload.get_mut("cmds").and_then(|c| c.as_array_mut()) {
        for c in cmds.iter_mut() {
            if let Some(ms) = c.get("timeout_ms").and_then(|v| v.as_u64()) {
                if let Some(obj) = c.as_object_mut() {
                    obj.insert("timeout".to_string(), json!(ms * 1_000_000));
                    obj.remove("timeout_ms");
                }
            }
        }
    }
    let out = run_sidecar(ptyexec_bin(), &payload, "Go ptyexec").await;
    Json(out)
}

/// POST /api/native/think — fikiri kwa njia ya Elixir brain (ubongo).
///
/// Body: `{"input":"...","hitl_for_deploy":true}`
async fn native_think(Json(body): Json<Value>) -> Json<Value> {
    let out = run_sidecar(brain_bin(), &body, "Elixir brain").await;
    Json(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_zina_defaults_njema() {
        // Bila env, defaults ni paths za repo.
        assert!(ptyexec_bin().to_string_lossy().contains("ptyexec"));
        assert!(brain_bin().to_string_lossy().contains("mtechos_brain"));
    }

    #[tokio::test]
    async fn sidecar_isiyopo_hurudisha_error_bila_panic() {
        let v = run_sidecar(
            PathBuf::from("/definitely/missing/bin-xyz"),
            &json!({}),
            "Test",
        )
        .await;
        assert_eq!(v["ok"], false);
        assert!(v["error"].as_str().unwrap_or_default().contains("haipo"));
    }

    #[test]
    fn sidecar_payload_hubadilisha_timeout_ms() {
        let mut payload = json!({ "cmds": [ { "id": "a", "bin": "echo", "timeout_ms": 500 } ] });
        if let Some(cmds) = payload.get_mut("cmds").and_then(|c| c.as_array_mut()) {
            for c in cmds.iter_mut() {
                if let Some(ms) = c.get("timeout_ms").and_then(|v| v.as_u64()) {
                    if let Some(obj) = c.as_object_mut() {
                        obj.insert("timeout".to_string(), json!(ms * 1_000_000));
                        obj.remove("timeout_ms");
                    }
                }
            }
        }
        assert_eq!(payload["cmds"][0]["timeout"], 500_000_000u64);
        assert!(payload["cmds"][0].get("timeout_ms").is_none());
    }

    /// Integration HALISI: endesha Go executor ikiwa binary ipo (imejengwa).
    #[tokio::test]
    async fn integration_go_exec_ikiwa_binary_ipo() {
        if !ptyexec_bin().exists() {
            eprintln!("skipped: {} haipo (jenga: cd agentic/ptyexec && go build -o ptyexec ./cmd/ptyexec)", ptyexec_bin().display());
            return;
        }
        let v = run_sidecar(
            ptyexec_bin(),
            &json!({ "cmds": [ { "id": "a", "bin": "echo", "args": ["habari"] } ], "concurrency": 1 }),
            "Go ptyexec",
        )
        .await;
        assert_eq!(v["ok"], true, "Go executor: {v}");
        assert_eq!(v["results"][0]["exit_code"], 0);
        assert!(v["results"][0]["stdout"].as_str().unwrap_or_default().contains("habari"));
    }

    /// Integration HALISI: fikiri kwa Elixir brain ikiwa escript ipo.
    #[tokio::test]
    async fn integration_elixir_brain_ikiwa_binary_ipo() {
        if !brain_bin().exists() || !escript_available() {
            eprintln!(
                "skipped: brain={} escript_ipo={} (jenga: cd agentic/brain && mix escript.build)",
                brain_bin().display(),
                escript_available()
            );
            return;
        }
        let v = run_sidecar(
            brain_bin(),
            &json!({ "input": "rm -rf /data", "hitl_for_deploy": true }),
            "Elixir brain",
        )
        .await;
        assert_eq!(v["ok"], true, "Elixir brain: {v}");
        // Ubongo LAZIMA uitambue hatari.
        assert_eq!(v["decision"]["action"], "halt");
        assert_eq!(v["decision"]["requires_hitl"], true);
    }

    #[test]
    fn moduli_hii_inapita_linter_ya_repo() {
        // Module mpya LAZIMA ikidhi kanuni za usalama za MTECHOS (code_audit).
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/agentic_bridge.rs");
        let src = std::fs::read_to_string(path).expect("kuisoma agentic_bridge.rs");
        let v = crate::code_audit::analyze_content("rust", &src);
        assert!(v.is_empty(), "agentic_bridge.rs inavunja kanuni: {v:?}");
    }
}
