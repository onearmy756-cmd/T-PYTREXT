//! ide_lsp.rs — LANGUAGE SERVER HALISI (LSP) kwa IntelliSense.
//!
//! Huzungumza JSON-RPC 2.0 kwa framing `Content-Length` juu ya stdio na
//! language server halisi: rust-analyzer, typescript-language-server,
//! pyright-langserver / pylsp, gopls, R languageserver.
//!
//! Inatoa: diagnostics (errors/warnings), hover, completion.
//! Kama server haipo kwenye `PATH`, hurudisha `available:false` + hint —
//! haifichi ukweli.

use axum::Json;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::ide_api::ide_workspace_root;

struct Server {
    lang: &'static str,
    language_id: &'static str,
    /// amri + args
    cmd: &'static [&'static str],
    install: &'static str,
    /// extensions
    exts: &'static [&'static str],
}

const SERVERS: &[Server] = &[
    Server { lang: "rust", language_id: "rust", cmd: &["rust-analyzer"], install: "rustup component add rust-analyzer", exts: &["rs"] },
    Server { lang: "typescript", language_id: "typescript", cmd: &["typescript-language-server", "--stdio"], install: "npm i -g typescript-language-server typescript", exts: &["ts", "tsx"] },
    Server { lang: "javascript", language_id: "javascript", cmd: &["typescript-language-server", "--stdio"], install: "npm i -g typescript-language-server typescript", exts: &["js", "jsx", "mjs"] },
    Server { lang: "elixir", language_id: "elixir", cmd: &["elixir-ls"], install: "git clone https://github.com/elixir-lsp/elixir-ls && mix deps.get", exts: &["ex", "exs"] },
    Server { lang: "go", language_id: "go", cmd: &["gopls"], install: "go install golang.org/x/tools/gopls@latest", exts: &["go"] },
    Server { lang: "r", language_id: "r", cmd: &["R", "--slave", "-e", "languageserver::run()"], install: "install.packages('languageserver')", exts: &["r", "R"] },
];

fn find_lang(lang: &str) -> Option<&'static Server> {
    SERVERS.iter().find(|s| s.lang == lang)
}

fn lang_for_path(path: &str) -> Option<&'static Server> {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    SERVERS.iter().find(|s| s.exts.iter().any(|x| x.eq_ignore_ascii_case(ext)))
}

fn safe_path(path: &str) -> Option<std::path::PathBuf> {
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

pub(crate) fn frame(body: &Value) -> Vec<u8> {
    let s = body.to_string();
    let mut out = format!("Content-Length: {}\r\n\r\n", s.len()).into_bytes();
    out.extend_from_slice(s.as_bytes());
    out
}

pub(crate) async fn read_message<R: AsyncReadExt + Unpin>(r: &mut R) -> Option<Value> {
    let mut header = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = r.read(&mut b).await.ok()?;
        if n == 0 {
            return None;
        }
        header.push(b[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
        if header.len() > 16384 {
            return None;
        }
    }
    let hs = String::from_utf8_lossy(&header);
    let len = hs
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length:"))
        .and_then(|v| v.trim().parse::<usize>().ok())?;
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).await.ok()?;
    serde_json::from_slice(&body).ok()
}

async fn has_bin(bin: &str) -> bool {
    let out = tokio::process::Command::new("bash")
        .arg("-lc")
        .arg(format!("command -v {bin}"))
        .output()
        .await;
    matches!(out, Ok(o) if o.status.success())
}

/// GET /api/ide/lsp/servers
pub async fn servers() -> Json<Value> {
    let mut list = Vec::new();
    for s in SERVERS {
        let available = has_bin(s.cmd[0]).await;
        list.push(json!({
            "lang": s.lang, "language_id": s.language_id,
            "command": s.cmd.join(" "), "available": available,
            "extensions": s.exts, "install": s.install,
        }));
    }
    let ready = list.iter().filter(|x| x["available"].as_bool().unwrap_or(false)).count();
    Json(json!({ "ok": true, "total": list.len(), "ready": ready, "servers": list }))
}

/// POST /api/ide/lsp/{diagnostics|hover|completion}
pub async fn lsp(Json(body): Json<Value>) -> Json<Value> {
    let action = body["action"].as_str().unwrap_or("diagnostics");
    let path = body["path"].as_str().unwrap_or("");
    let explicit_lang = body["language"].as_str().unwrap_or("");

    let srv = if !explicit_lang.is_empty() {
        find_lang(explicit_lang)
    } else {
        lang_for_path(path)
    };
    let Some(srv) = srv else {
        return Json(json!({ "ok": false, "error": "lugha haijulikani", "path": path }));
    };

    // Maudhui: code iliyotolewa, au faili halisi.
    let (uri_path, text) = if let Some(code) = body["code"].as_str().filter(|c| !c.is_empty()) {
        let p = if path.is_empty() { format!("untitled.{}", srv.exts[0]) } else { path.to_string() };
        (p, code.to_string())
    } else {
        if path.is_empty() {
            return Json(json!({ "ok": false, "error": "path au code inahitajika" }));
        }
        let Some(full) = safe_path(path) else {
            return Json(json!({ "ok": false, "error": "path nje ya workspace" }));
        };
        match std::fs::read_to_string(&full) {
            Ok(t) => (path.to_string(), t),
            Err(e) => return Json(json!({ "ok": false, "error": format!("faili haipatikani: {e}") })),
        }
    };

    if !has_bin(srv.cmd[0]).await {
        return Json(json!({
            "ok": false,
            "error": format!("server '{}' haipo kwenye PATH", srv.cmd[0]),
            "install": srv.install, "lang": srv.lang,
        }));
    }

    let root = ide_workspace_root();
    let uri = format!("file://{}", safe_path(&uri_path).unwrap_or(root.join(&uri_path)).display());
    let root_uri = format!("file://{}", root.display());
    let lang_id = srv.language_id;

    let mut cmd = tokio::process::Command::new(srv.cmd[0]);
    if srv.cmd.len() > 1 {
        cmd.args(&srv.cmd[1..]);
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

    // initialize handshake
    let init = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "processId": std::process::id(),
            "rootUri": root_uri, "capabilities": {},
            "workspaceFolders": [{ "uri": root_uri, "name": "workspace" }],
        }
    });
    if stdin.write_all(&frame(&init)).await.is_err() {
        return Json(json!({ "ok": false, "error": "write initialize imeshindikana" }));
    }
    // subiri response ya initialize
    let mut got_init = false;
    for _ in 0..20 {
        match tokio::time::timeout(Duration::from_secs(20), read_message(&mut stdout)).await {
            Ok(Some(m)) if m["id"] == json!(1) => { got_init = true; break; }
            Ok(Some(_)) => continue,
            _ => break,
        }
    }
    if !got_init {
        return Json(json!({ "ok": false, "error": "handshake ya LSP imeshindikana/timeout", "lang": srv.lang }));
    }
    let _ = stdin.write_all(&frame(&json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }))).await;

    let result = match action {
        "hover" => {
            let _ = stdin.write_all(&frame(&json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "languageId": lang_id, "version": 1, "text": text } }
            }))).await;
            let line = body["line"].as_u64().unwrap_or(0);
            let character = body["character"].as_u64().unwrap_or(0);
            let _ = stdin.write_all(&frame(&json!({
                "jsonrpc": "2.0", "id": 2, "method": "textDocument/hover",
                "params": { "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }
            }))).await;
            wait_response(&mut stdout, 2, Duration::from_secs(20)).await
        }
        "completion" => {
            let _ = stdin.write_all(&frame(&json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "languageId": lang_id, "version": 1, "text": text } }
            }))).await;
            let line = body["line"].as_u64().unwrap_or(0);
            let character = body["character"].as_u64().unwrap_or(0);
            let _ = stdin.write_all(&frame(&json!({
                "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion",
                "params": { "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }
            }))).await;
            wait_response(&mut stdout, 2, Duration::from_secs(20)).await
        }
        _ => {
            // diagnostics
            let _ = stdin.write_all(&frame(&json!({
                "jsonrpc": "2.0", "method": "textDocument/didOpen",
                "params": { "textDocument": { "uri": uri, "languageId": lang_id, "version": 1, "text": text } }
            }))).await;
            wait_diagnostics(&mut stdout, &uri, Duration::from_secs(30)).await
        }
    };

    let _ = child.start_kill();
    Json(json!({ "ok": true, "action": action, "lang": srv.lang, "uri": uri, "result": result }))
}

async fn wait_response<R: AsyncReadExt + Unpin>(r: &mut R, id: u64, t: Duration) -> Value {
    let deadline = tokio::time::Instant::now() + t;
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            return json!({ "timeout": true });
        }
        match tokio::time::timeout(left, read_message(r)).await {
            Ok(Some(m)) if m["id"] == json!(id) => return m["result"].clone(),
            Ok(Some(_)) => continue,
            _ => return json!({ "timeout": true }),
        }
    }
}

async fn wait_diagnostics<R: AsyncReadExt + Unpin>(r: &mut R, uri: &str, t: Duration) -> Value {
    let deadline = tokio::time::Instant::now() + t;
    loop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            return json!({ "diagnostics": [], "timeout": true });
        }
        match tokio::time::timeout(left, read_message(r)).await {
            Ok(Some(m)) => {
                if m["method"] == json!("textDocument/publishDiagnostics") {
                    let params = &m["params"];
                    if params["uri"].as_str() == Some(uri) || params["uri"].is_null() {
                        return params["diagnostics"].clone();
                    }
                }
            }
            _ => return json!({ "diagnostics": [], "timeout": true }),
        }
    }
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/lsp/servers", get(servers))
        .route("/api/ide/lsp/:action", post(lsp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_by_ext() {
        assert_eq!(lang_for_path("a/b.rs").unwrap().lang, "rust");
        assert_eq!(lang_for_path("x.ex").unwrap().lang, "elixir");
        assert_eq!(lang_for_path("t.ts").unwrap().lang, "typescript");
        assert!(lang_for_path("noext").is_none());
    }

    #[test]
    fn frame_has_content_length() {
        let f = frame(&json!({"a":1}));
        let s = String::from_utf8_lossy(&f);
        assert!(s.starts_with("Content-Length: "));
        assert!(s.contains("\r\n\r\n{\"a\":1}"));
    }

    #[test]
    fn safe_path_rejects_traversal() {
        assert!(safe_path("../x").is_none());
    }
}
