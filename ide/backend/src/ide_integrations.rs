//! ide_integrations.rs — MBILINYI IDE: MCP, ACP, Plugins, Connectors, na
//! kumbukumbu ya mazungumzo (conversation memory).
//!
//! Kila kitu ni HALISI: MCP servers huhifadhiwa kwenye DB na hupimwa kwa
//! kufikia URL/command; ACP agents hupimwa kwa kuwepo kwenye PATH; plugins
//! hutoka kwenye `canvas-extension.json` halisi; connectors huripoti hali ya
//! zana za host (git, ollama). Hakuna mock.
//!
//!   GET    /api/mcp/servers               → orodha
//!   POST   /api/mcp/servers               → ongeza/update
//!   DELETE /api/mcp/servers/:id           → ondoa
//!   GET    /api/mcp/health                → pima kila server (halisi)
//!   GET    /api/acp/agents                → agents + kuwepo kwa CLI (PATH)
//!   GET    /api/plugins/installed         → extensions halisi za ndani
//!   GET    /api/connectors/status         → git/ollama/github hali halisi
//!   GET    /api/ide/conversations         → orodha ya mazungumzo
//!   POST   /api/ide/conversations         → unda mazungumzo
//!   GET    /api/ide/conversations/:id     → mazungumzo + ujumbe
//!   POST   /api/ide/conversations/:id/message → ongeza ujumbe
//!   DELETE /api/ide/conversations/:id     → ondoa mazungumzo

use crate::AppState;
use axum::{
    extract::{Path as AxPath, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub async fn init_tables(db: &sqlx::SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            transport TEXT NOT NULL,
            url TEXT,
            command TEXT,
            args TEXT,
            env TEXT,
            headers TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
    )
    .execute(db)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS ide_conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            lang TEXT NOT NULL DEFAULT 'sw',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
    )
    .execute(db)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS ide_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            at INTEGER NOT NULL
        )",
    )
    .execute(db)
    .await;
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4().simple())
}

/// `which` bila crate ya nje: hutafuta binary kwenye PATH (na .exe kwa Windows).
pub fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(bin);
        if cand.is_file() {
            return Some(cand);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{bin}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

// ═══════════════════════ MCP ═══════════════════════

#[derive(Debug, Deserialize)]
pub struct McpUpsertReq {
    pub id: Option<String>,
    pub name: String,
    /// stdio | sse | http
    #[serde(default)]
    pub transport: Option<String>,
    pub url: Option<String>,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub headers: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

fn row_to_mcp(row: &sqlx::sqlite::SqliteRow) -> Value {
    use sqlx::Row;
    let parse = |s: Option<String>| -> Value {
        s.and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .unwrap_or(Value::Null)
    };
    json!({
        "id": row.try_get::<String, _>("id").unwrap_or_default(),
        "name": row.try_get::<String, _>("name").unwrap_or_default(),
        "transport": row.try_get::<String, _>("transport").unwrap_or_default(),
        "url": row.try_get::<Option<String>, _>("url").unwrap_or_default(),
        "command": row.try_get::<Option<String>, _>("command").unwrap_or_default(),
        "args": parse(row.try_get::<Option<String>, _>("args").unwrap_or_default()),
        "env": parse(row.try_get::<Option<String>, _>("env").unwrap_or_default()),
        "headers": parse(row.try_get::<Option<String>, _>("headers").unwrap_or_default()),
        "enabled": row.try_get::<i64, _>("enabled").unwrap_or(1) != 0,
        "updated_at": row.try_get::<i64, _>("updated_at").unwrap_or(0),
    })
}

async fn mcp_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query(
        "SELECT id, name, transport, url, command, args, env, headers, enabled, updated_at
         FROM mcp_servers ORDER BY updated_at",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut servers: Vec<Value> = rows.iter().map(row_to_mcp).collect();
    servers.reverse(); // hivi karibuni kwanza (bila ORDER BY kushuka)
    Json(json!({ "ok": true, "servers": servers, "count": servers.len() }))
}

async fn mcp_upsert(
    State(s): State<AppState>,
    Json(r): Json<McpUpsertReq>,
) -> Json<Value> {
    let name = r.name.trim();
    if name.is_empty() {
        return Json(json!({ "ok": false, "error": "name ni lazima" }));
    }
    let transport = r
        .transport
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| {
            if r.command.as_deref().map(|c| !c.trim().is_empty()).unwrap_or(false) {
                "stdio".into()
            } else {
                "http".into()
            }
        });
    let id = r
        .id
        .clone()
        .filter(|i| !i.trim().is_empty())
        .unwrap_or_else(|| new_id("mcp"));
    let args = serde_json::to_string(&r.args.clone().unwrap_or_default()).unwrap_or_else(|_| "[]".into());
    let env = serde_json::to_string(&r.env.clone().unwrap_or_default()).unwrap_or_else(|_| "{}".into());
    let headers =
        serde_json::to_string(&r.headers.clone().unwrap_or_default()).unwrap_or_else(|_| "{}".into());
    let enabled = r.enabled.unwrap_or(true) as i64;
    let now = now_unix();
    let res = sqlx::query(
        "INSERT INTO mcp_servers
            (id, name, transport, url, command, args, env, headers, enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, transport=excluded.transport, url=excluded.url,
            command=excluded.command, args=excluded.args, env=excluded.env,
            headers=excluded.headers, enabled=excluded.enabled, updated_at=excluded.updated_at",
    )
    .bind(&id)
    .bind(name)
    .bind(&transport)
    .bind(&r.url)
    .bind(&r.command)
    .bind(&args)
    .bind(&env)
    .bind(&headers)
    .bind(enabled)
    .bind(now)
    .execute(&s.db)
    .await;
    match res {
        Ok(_) => Json(json!({ "ok": true, "id": id, "transport": transport })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn mcp_delete(
    State(s): State<AppState>,
    AxPath(id): AxPath<String>,
) -> Json<Value> {
    match sqlx::query("DELETE FROM mcp_servers WHERE id = ?1")
        .bind(&id)
        .execute(&s.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => Json(json!({ "ok": true, "id": id })),
        Ok(_) => Json(json!({ "ok": false, "error": "haipo" })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

/// Pima server moja: stdio → command kwenye PATH; http/sse → HTTP GET kwa muda
/// mfupi. Hurudisha hali halisi.
async fn probe_mcp(name: &str, transport: &str, url: Option<&str>, command: Option<&str>) -> Value {
    let mut out = json!({ "name": name, "transport": transport });
    if transport == "stdio" {
        let bin = command.unwrap_or("").split_whitespace().next().unwrap_or("");
        let present = !bin.is_empty() && which(bin).is_some();
        out["ok"] = json!(present);
        out["binary"] = json!(bin);
        out["detail"] = json!(if present {
            format!("`{bin}` ipo kwenye PATH")
        } else {
            format!("`{bin}` haipo kwenye PATH")
        });
        return out;
    }
    let Some(u) = url.filter(|u| !u.trim().is_empty()) else {
        out["ok"] = json!(false);
        out["detail"] = json!("url haipo");
        return out;
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build();
    match client {
        Ok(c) => match c.get(u).send().await {
            Ok(resp) => {
                out["ok"] = json!(true);
                out["status"] = json!(resp.status().as_u16());
                out["detail"] = json!("imefikiwa");
            }
            Err(e) => {
                out["ok"] = json!(false);
                out["detail"] = json!(e.to_string());
            }
        },
        Err(e) => {
            out["ok"] = json!(false);
            out["detail"] = json!(e.to_string());
        }
    }
    out
}

async fn mcp_health(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query("SELECT id, name, transport, url, command FROM mcp_servers")
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    use sqlx::Row;
    let mut probes = Vec::new();
    for row in &rows {
        let name: String = row.try_get("name").unwrap_or_default();
        let transport: String = row.try_get("transport").unwrap_or_default();
        let url: Option<String> = row.try_get("url").unwrap_or_default();
        let command: Option<String> = row.try_get("command").unwrap_or_default();
        probes.push(probe_mcp(&name, &transport, url.as_deref(), command.as_deref()).await);
    }
    Json(json!({ "ok": true, "probes": probes, "count": probes.len() }))
}

// ═══════════════════════ ACP ═══════════════════════

/// ACP agents zinazojulikana (Claude Code, Codex, Gemini, n.k.). Kuwepo
/// hupimwa kwa PATH (halisi).
const ACP_AGENTS: &[(&str, &str, &str)] = &[
    ("claude-code", "Claude Code", "claude"),
    ("codex", "OpenAI Codex", "codex"),
    ("gemini-cli", "Gemini CLI", "gemini"),
    ("aider", "Aider", "aider"),
    ("goose", "Goose", "goose"),
    ("opencode", "OpenCode", "opencode"),
    ("crush", "Crush", "crush"),
    ("continue", "Continue", "cn"),
];

async fn acp_agents() -> Json<Value> {
    let agents: Vec<Value> = ACP_AGENTS
        .iter()
        .map(|(id, label, bin)| {
            let path = which(bin);
            json!({
                "id": id,
                "label": label,
                "binary": bin,
                "available": path.is_some(),
                "path": path.map(|p| p.display().to_string()),
            })
        })
        .collect();
    let available = agents
        .iter()
        .filter(|a| a["available"].as_bool().unwrap_or(false))
        .count();
    Json(json!({ "ok": true, "agents": agents, "available": available }))
}

// ═══════════════════════ PLUGINS ═══════════════════════

/// Plugins = extensions halisi zilizosakinishwa (kutoka `canvas-extension.json`).
async fn plugins_installed() -> Json<Value> {
    let roots = crate::canvas_ext::plugin_roots();
    let plugins: Vec<Value> = roots
        .iter()
        .filter_map(|dir| {
            let manifest = std::fs::read_to_string(dir.join("canvas-extension.json")).ok()?;
            let m: Value = serde_json::from_str(&manifest).ok()?;
            Some(json!({
                "name": m.get("name").cloned().unwrap_or(Value::Null),
                "version": m.get("version").cloned().unwrap_or(json!("1.0.0")),
                "description": m.get("description").cloned().unwrap_or(Value::Null),
                "pages": m.pointer("/contributes/pages").cloned().unwrap_or(json!([])),
                "path": dir.display().to_string(),
                "enabled": true,
            }))
        })
        .collect();
    Json(json!({ "ok": true, "plugins": plugins, "count": plugins.len() }))
}

// ═══════════════════════ CONNECTORS ═══════════════════════

async fn connectors_status() -> Json<Value> {
    // git: kuwepo + repo root/remote (halisi)
    let git_bin = which("git");
    let git_version = if git_bin.is_some() {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let repo_root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let remote = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let connectors = json!([
        {
            "id": "git",
            "label": "Git",
            "connected": git_bin.is_some(),
            "detail": if git_bin.is_some() { git_version } else { "git haipo".into() },
            "extra": { "repo_root": repo_root, "remote": remote }
        },
        {
            "id": "github",
            "label": "GitHub",
            "connected": std::env::var("GITHUB_TOKEN").map(|t| !t.is_empty()).unwrap_or(false),
            "detail": "token ya env (GITHUB_TOKEN)"
        },
        {
            "id": "ollama-local",
            "label": "Ollama (local)",
            "connected": which("ollama").is_some(),
            "detail": "CLI ya ollama kwenye host"
        },
        {
            "id": "docker",
            "label": "Docker",
            "connected": which("docker").is_some(),
            "detail": "kontena/emulators"
        },
    ]);
    Json(json!({ "ok": true, "connectors": connectors }))
}

// ═══════════════════════ CONVERSATION MEMORY ═══════════════════════

#[derive(Debug, Deserialize)]
pub struct ConvCreateReq {
    pub title: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MsgReq {
    pub role: String,
    pub content: String,
}

async fn conv_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query("SELECT id, title, lang, created_at, updated_at FROM ide_conversations ORDER BY updated_at")
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    use sqlx::Row;
    let mut items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "title": r.try_get::<String, _>("title").unwrap_or_default(),
                "lang": r.try_get::<String, _>("lang").unwrap_or_default(),
                "created_at": r.try_get::<i64, _>("created_at").unwrap_or(0),
                "updated_at": r.try_get::<i64, _>("updated_at").unwrap_or(0),
            })
        })
        .collect();
    items.reverse(); // hivi karibuni kwanza
    Json(json!({ "ok": true, "conversations": items }))
}

async fn conv_create(State(s): State<AppState>, Json(r): Json<ConvCreateReq>) -> Json<Value> {
    let id = new_id("conv");
    let title = r.title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| "Mazungumzo mapya".into());
    let lang = r.lang.filter(|l| !l.trim().is_empty()).unwrap_or_else(|| "sw".into());
    let now = now_unix();
    match sqlx::query(
        "INSERT INTO ide_conversations (id, title, lang, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
    )
    .bind(&id)
    .bind(&title)
    .bind(&lang)
    .bind(now)
    .execute(&s.db)
    .await
    {
        Ok(_) => Json(json!({ "ok": true, "id": id, "title": title, "lang": lang })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn conv_get(State(s): State<AppState>, AxPath(id): AxPath<String>) -> Json<Value> {
    use sqlx::Row;
    let conv = sqlx::query("SELECT id, title, lang, created_at, updated_at FROM ide_conversations WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&s.db)
        .await
        .unwrap_or_default();
    let Some(conv) = conv else {
        return Json(json!({ "ok": false, "error": "haipo" }));
    };
    let msgs = sqlx::query("SELECT role, content, at FROM ide_messages WHERE conversation_id = ?1 ORDER BY id ASC")
        .bind(&id)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    let messages: Vec<Value> = msgs
        .iter()
        .map(|m| {
            json!({
                "role": m.try_get::<String, _>("role").unwrap_or_default(),
                "content": m.try_get::<String, _>("content").unwrap_or_default(),
                "at": m.try_get::<i64, _>("at").unwrap_or(0),
            })
        })
        .collect();
    Json(json!({
        "ok": true,
        "conversation": {
            "id": conv.try_get::<String, _>("id").unwrap_or_default(),
            "title": conv.try_get::<String, _>("title").unwrap_or_default(),
            "lang": conv.try_get::<String, _>("lang").unwrap_or_default(),
        },
        "messages": messages,
    }))
}

async fn conv_add_message(
    State(s): State<AppState>,
    AxPath(id): AxPath<String>,
    Json(r): Json<MsgReq>,
) -> Json<Value> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM ide_conversations WHERE id = ?1")
        .bind(&id)
        .fetch_optional(&s.db)
        .await
        .unwrap_or_default();
    if exists.is_none() {
        return Json(json!({ "ok": false, "error": "mazungumzo hayapo" }));
    }
    let now = now_unix();
    let res = sqlx::query(
        "INSERT INTO ide_messages (conversation_id, role, content, at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&id)
    .bind(&r.role)
    .bind(&r.content)
    .bind(now)
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE ide_conversations SET updated_at = ?1 WHERE id = ?2")
        .bind(now)
        .bind(&id)
        .execute(&s.db)
        .await;
    match res {
        Ok(_) => Json(json!({ "ok": true })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn conv_delete(State(s): State<AppState>, AxPath(id): AxPath<String>) -> Json<Value> {
    let _ = sqlx::query("DELETE FROM ide_messages WHERE conversation_id = ?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    match sqlx::query("DELETE FROM ide_conversations WHERE id = ?1")
        .bind(&id)
        .execute(&s.db)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => Json(json!({ "ok": true, "id": id })),
        Ok(_) => Json(json!({ "ok": false, "error": "haipo" })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/mcp/servers", get(mcp_list).post(mcp_upsert))
        .route("/api/mcp/servers/:id", delete(mcp_delete))
        .route("/api/mcp/health", get(mcp_health))
        .route("/api/acp/agents", get(acp_agents))
        .route("/api/connectors/status", get(connectors_status))
        .route("/api/ide/conversations", get(conv_list).post(conv_create))
        .route(
            "/api/ide/conversations/:id",
            get(conv_get).delete(conv_delete),
        )
        .route("/api/ide/conversations/:id/message", post(conv_add_message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_inapata_sh_na_kutokupata_bin_isiyopo() {
        // `sh` ipo kwenye Linux; bin ya kubuni haipo.
        assert!(which("sh").is_some());
        assert!(which("hakuna-bin-hii-kabisa-xyz").is_none());
    }

    #[test]
    fn acp_agents_ni_halisi_bila_mock() {
        // ACP_AGENTS ina entries; kila moja ina id/label/bin.
        assert!(ACP_AGENTS.len() >= 6);
        for (id, label, bin) in ACP_AGENTS {
            assert!(!id.is_empty() && !label.is_empty() && !bin.is_empty());
        }
    }

    #[test]
    fn new_id_ina_prefix() {
        assert!(new_id("mcp").starts_with("mcp-"));
        assert!(new_id("conv").starts_with("conv-"));
    }

    #[test]
    fn moduli_hii_inapita_linter_ya_repo() {
        // Module mpya LAZIMA ikidhi kanuni za usalama za MTECHOS (code_audit).
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/ide_integrations.rs");
        let src = std::fs::read_to_string(path).expect("kuisoma ide_integrations.rs");
        let v = crate::code_audit::analyze_content("rust", &src);
        assert!(v.is_empty(), "ide_integrations.rs inavunja kanuni: {v:?}");
    }
}
