//! MBILINYITECH IDE — Tauri 2 shell.
//! UI: `server/agent/static/boot-canvas.html` → MBILINYI Agent Canvas (`/ide`).
//! Rust = dirisha la desktop + kuunganisha na agent HTTP backend.

use serde::Serialize;
use std::net::ToSocketAddrs;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Clone)]
struct AppState {
    api_base: Arc<Mutex<String>>,
}

#[derive(Serialize)]
struct ApiBase {
    base: String,
    tauri: bool,
}

#[tauri::command]
fn get_api_base(state: State<'_, AppState>) -> ApiBase {
    let base = state
        .api_base
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| default_api_base());
    ApiBase {
        base,
        tauri: true,
    }
}

#[tauri::command]
fn set_api_base(state: State<'_, AppState>, base: String) -> ApiBase {
    let b = base.trim().trim_end_matches('/').to_string();
    if let Ok(mut g) = state.api_base.lock() {
        *g = b.clone();
    }
    ApiBase {
        base: b,
        tauri: true,
    }
}

#[tauri::command]
fn start_agent_if_needed(state: State<'_, AppState>) -> Result<String, String> {
    let base = state
        .api_base
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| default_api_base());
    try_start_agent(&base)
}

fn try_start_agent(base: &str) -> Result<String, String> {
    if agent_up(base) {
        return Ok(format!("agent already up at {base}"));
    }

    let bin = resolve_agent_bin();
    let Some(bin) = bin else {
        return Err(
            "agent binary haipatikani (fundi-deploy). Anzisha agent mwenyewe au weka MTECHOS_AGENT_BIN"
                .into(),
        );
    };

    Command::new(&bin)
        .spawn()
        .map_err(|e| format!("spawn agent fail: {e}"))?;

    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if agent_up(base) {
            return Ok(format!("agent started → {base}"));
        }
    }
    Ok(format!("agent spawned; health bado haijathibitishwa kwa {base}"))
}

/// Tafuta binary HALISI ya agent (`fundi-deploy` — jina la kweli kwenye Cargo).
/// MTECHOS_AGENT_BIN ndiyo yenye kipaumbele; kisha paths za release/debug.
fn resolve_agent_bin() -> Option<String> {
    if let Ok(p) = std::env::var("MTECHOS_AGENT_BIN") {
        if std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }
    let exe = if cfg!(windows) { "fundi-deploy.exe" } else { "fundi-deploy" };
    let candidates = [
        format!("server/agent/target/release/{exe}"),
        format!("../server/agent/target/release/{exe}"),
        format!("../../server/agent/target/release/{exe}"),
        format!("server/agent/target/debug/{exe}"),
        format!("../server/agent/target/debug/{exe}"),
        format!("../../server/agent/target/debug/{exe}"),
    ];
    candidates
        .into_iter()
        .find(|p| std::path::Path::new(p).exists())
}

fn default_api_base() -> String {
    std::env::var("MTECHOS_API_BASE").unwrap_or_else(|_| "http://127.0.0.1:8080".into())
}

fn agent_up(base: &str) -> bool {
    let url = base
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let hostport = url.split('/').next().unwrap_or("127.0.0.1:8080");
    let addr = match hostport.to_socket_addrs() {
        Ok(mut it) => match it.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(400)).is_ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let api = default_api_base();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            api_base: Arc::new(Mutex::new(api.clone())),
        })
        .invoke_handler(tauri::generate_handler![
            get_api_base,
            set_api_base,
            start_agent_if_needed
        ])
        .setup(move |_app| {
            let base = api.clone();
            std::thread::spawn(move || {
                let _ = try_start_agent(&base);
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running MBILINYITECH IDE (Tauri)");
}

#[cfg(test)]
mod tests {
    #[test]
    fn candidates_zina_jina_sahihi_la_binary() {
        // Regression: jina la binary ni `fundi-deploy` (sio `fundi-agent`).
        let exe = if cfg!(windows) { "fundi-deploy.exe" } else { "fundi-deploy" };
        assert!(exe.starts_with("fundi-deploy"));
        assert!(!exe.contains("fundi-agent"));
    }

    #[test]
    fn api_base_inasafishwa() {
        // set_api_base hutupa '/' ya mwisho — base + path isitoe '//'.
        let b = "http://127.0.0.1:8080/".trim_end_matches('/');
        assert_eq!(b, "http://127.0.0.1:8080");
    }
}
