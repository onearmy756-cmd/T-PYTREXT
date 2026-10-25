//! ide_dap.rs — DEBUGGER HALISI (DAP) kwa Rust na Go.
//!
//! Huzungumza Debug Adapter Protocol (JSON-RPC kwa `Content-Length`, kama LSP)
//! na adapters za lugha za MTECHOS ili kufuatilia programu halisi, kuweka
//! breakpoint, kuendelea, na kusoma stack frames + variables.
//!
//! MTECHOS ni Rust/Go/Elixir pekee — hakuna Python/debugpy. Kama adapter
//! inayolingana haipo, hurudisha `available:false` + hint.

use axum::Json;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::ide_api::ide_workspace_root;
use crate::ide_lsp::{frame, read_message};

/// `which` ya kwanza iliyopo kwenye PATH.
async fn which_any(names: &[&str]) -> Option<String> {
    for n in names {
        let out = tokio::process::Command::new("bash")
            .arg("-lc")
            .arg(format!("command -v {n} 2>/dev/null"))
            .output()
            .await;
        if let Ok(o) = out {
            if o.status.success() {
                let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !p.is_empty() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// (adapter_id, binary, language) kwa adapters zinazopatikana.
async fn detect() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if let Some(bin) = which_any(&["lldb-dap", "codelldb"]).await {
        out.push(("lldb".into(), bin, "rust".into()));
    }
    if let Some(bin) = which_any(&["dlv", "dlv-dap"]).await {
        out.push(("go".into(), bin, "go".into()));
    }
    out
}

fn safe_script(path: &str) -> Option<std::path::PathBuf> {
    let root = ide_workspace_root();
    let rel = path.trim_start_matches('/');
    if rel.contains("..") {
        return None;
    }
    let p = root.join(rel);
    if p.starts_with(&root) && p.is_file() {
        Some(p)
    } else {
        None
    }
}

async fn send<W: AsyncWriteExt + Unpin>(w: &mut W, m: &Value) -> bool {
    w.write_all(&frame(m)).await.is_ok()
}

async fn await_response<R: AsyncReadExt + Unpin>(r: &mut R, id: u64, t: Duration) -> Value {
    let deadline = tokio::time::Instant::now() + t;
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            return json!({ "timeout": true });
        }
        match tokio::time::timeout(left, read_message(r)).await {
            Ok(Some(m)) if m["id"] == json!(id) => {
                if m["success"] == json!(false) {
                    return json!({ "error": m["message"] });
                }
                return m["body"].clone();
            }
            Ok(Some(_)) => continue,
            _ => return json!({ "timeout": true }),
        }
    }
}

/// GET /api/ide/dap/adapters
pub async fn adapters() -> Json<Value> {
    let det = detect().await;
    let list: Vec<Value> = det
        .iter()
        .map(|(id, bin, lang)| {
            let install = match id.as_str() {
                "lldb" => "rustup component add llvm-tools; use CodeLLDB (codelldb)",
                "go" => "go install github.com/go-delve/delve/cmd/dlv@latest",
                _ => "",
            };
            json!({ "id": id, "label": format!("{lang} ({id})"), "language": lang, "available": true,
                    "command": bin, "install": install })
        })
        .collect();
    Json(json!({
        "ok": true,
        "adapters": list,
        "ready": det.len(),
        "note": "DAP kwa Rust/Go. Hakuna Python.",
    }))
}

/// POST /api/ide/dap/run — debug programu HALISI.
/// body: { path, adapter?, breakpoints?: [line...], steps?: N, stop_on_entry?: bool }
pub async fn run(Json(body): Json<Value>) -> Json<Value> {
    let det = detect().await;
    let want = body["adapter"].as_str().unwrap_or("");
    let picked = det
        .iter()
        .find(|(id, _, _)| id == want)
        .or_else(|| det.first());
    let Some((adapter_id, bin, _lang)) = picked else {
        return Json(json!({
            "ok": false,
            "error": "hakuna DAP adapter iliyopatikana (lldb-dap/codelldb/dlv)",
            "install": "rustup component add llvm-tools (Rust) au go install github.com/go-delve/delve/cmd/dlv@latest (Go)",
        }));
    };

    let path = body["path"].as_str().unwrap_or("");
    let Some(script) = safe_script(path) else {
        return Json(json!({ "ok": false, "error": "script haipatikani (au nje ya workspace)" }));
    };
    let breakpoints: Vec<u64> = body["breakpoints"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
    let steps = body["steps"].as_u64().unwrap_or(0).min(50);
    let stop_on_entry = body["stop_on_entry"].as_bool().unwrap_or(true);

    let root = ide_workspace_root();
    let mut cmd = tokio::process::Command::new(bin);
    if adapter_id == "go" {
        cmd.arg("dap");
    }
    let child = cmd
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return Json(json!({ "ok": false, "error": format!("spawn: {e}") })),
    };
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    // 1) initialize
    if !send(&mut stdin, &json!({
        "seq": 1, "type": "request", "command": "initialize",
        "arguments": { "adapterID": adapter_id, "pathFormat": "path", "linesStartAt1": true, "columnsStartAt1": true }
    })).await {
        return Json(json!({ "ok": false, "error": "write initialize" }));
    }
    let _init = await_response(&mut stdout, 1, Duration::from_secs(20)).await;

    // 2) launch — arguments hutofautiana kwa adapter
    let launch_args = if adapter_id == "go" {
        json!({ "request": "launch", "mode": "debug", "program": script.display().to_string(), "stopOnEntry": stop_on_entry })
    } else {
        json!({ "program": script.display().to_string(), "stopOnEntry": stop_on_entry, "console": "internalConsole" })
    };
    let _ = send(&mut stdin, &json!({
        "seq": 2, "type": "request", "command": "launch", "arguments": launch_args
    })).await;
    let _launch = await_response(&mut stdout, 2, Duration::from_secs(20)).await;

    // 3) set breakpoints (kama zimetolewa)
    if !breakpoints.is_empty() {
        let bps: Vec<Value> = breakpoints.iter().map(|l| json!({ "line": l })).collect();
        let _ = send(&mut stdin, &json!({
            "seq": 3, "type": "request", "command": "setBreakpoints",
            "arguments": { "source": { "path": script.display().to_string() }, "breakpoints": bps }
        })).await;
        let _ = await_response(&mut stdout, 3, Duration::from_secs(15)).await;
    }

    // 4) configurationDone
    let _ = send(&mut stdin, &json!({ "seq": 4, "type": "request", "command": "configurationDone", "arguments": {} })).await;

    // 5) subiri event ya kwanza ya 'stopped' au 'terminated'
    let mut events: Vec<Value> = Vec::new();
    let mut thread_id: i64 = 1;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() { break; }
        match tokio::time::timeout(left, read_message(&mut stdout)).await {
            Ok(Some(m)) => {
                if m["type"] == json!("event") {
                    let ev = m["event"].as_str().unwrap_or("");
                    events.push(json!({ "event": ev, "body": m["body"].clone() }));
                    if ev == "stopped" {
                        if let Some(t) = m["body"]["threadId"].as_i64() {
                            thread_id = t;
                        }
                        break;
                    }
                    if ev == "terminated" || ev == "exited" {
                        break;
                    }
                }
            }
            _ => break,
        }
    }

    // 6) stackTrace + variables HALISI
    let mut scopes: Value = json!([]);
    let mut variables: Value = json!([]);
    let mut seq: u64 = 100;

    let _ = send(&mut stdin, &json!({
        "seq": seq, "type": "request", "command": "stackTrace",
        "arguments": { "threadId": thread_id, "startFrame": 0, "levels": 10 }
    })).await;
    let stack = await_response(&mut stdout, seq, Duration::from_secs(15)).await;
    seq += 1;

    if let Some(frame0) = stack["stackFrames"].as_array().and_then(|a| a.first()) {
        if let Some(fid) = frame0["id"].as_i64() {
            let _ = send(&mut stdin, &json!({
                "seq": seq, "type": "request", "command": "scopes",
                "arguments": { "frameId": fid }
            })).await;
            scopes = await_response(&mut stdout, seq, Duration::from_secs(15)).await;
            seq += 1;
            if let Some(refv) = scopes["scopes"].as_array().and_then(|a| a.first()) {
                if let Some(vref) = refv["variablesReference"].as_i64() {
                    let _ = send(&mut stdin, &json!({
                        "seq": seq, "type": "request", "command": "variables",
                        "arguments": { "variablesReference": vref }
                    })).await;
                    variables = await_response(&mut stdout, seq, Duration::from_secs(15)).await;
                    seq += 1;
                }
            }
        }
    }

    // 7) hatua za debugging (optional)
    let mut step_events: Vec<Value> = Vec::new();
    for _ in 0..steps {
        let _ = send(&mut stdin, &json!({ "seq": seq, "type": "request", "command": "next", "arguments": { "threadId": thread_id, "singleThread": false } })).await;
        seq += 1;
        match tokio::time::timeout(Duration::from_secs(10), read_message(&mut stdout)).await {
            Ok(Some(m)) if m["type"] == json!("event") && m["event"] == json!("stopped") => {
                step_events.push(m["body"].clone());
            }
            _ => break,
        }
    }

    // 8) disconnect
    let _ = send(&mut stdin, &json!({ "seq": seq, "type": "request", "command": "disconnect", "arguments": { "terminateDebuggee": true } })).await;
    let _ = child.start_kill();

    Json(json!({
        "ok": true,
        "adapter": adapter_id,
        "program": script.display().to_string(),
        "stop_on_entry": stop_on_entry,
        "breakpoints": breakpoints,
        "stopped": events.iter().any(|e| e["event"] == json!("stopped")),
        "frames": stack["stackFrames"],
        "scopes": scopes["scopes"],
        "variables": variables["variables"],
        "step_events": step_events.len(),
        "events": events,
        "note": "DAP halisi kwa Rust/Go — frames/variables ni za mchakato halisi.",
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/dap/adapters", get(adapters))
        .route("/api/ide/dap/run", post(run))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_script_rejects_escape() {
        assert!(safe_script("../x.rs").is_none());
        assert!(safe_script("/etc/passwd").is_none());
    }
}
