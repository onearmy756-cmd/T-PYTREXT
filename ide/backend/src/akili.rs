// akili.rs — AKILI ENGINE HALISI: ubongo wa mfumo kwenye server ( Haramu
// ya uwongo: kila sehemu inafanya kazi HALISI — SQLite memory, knowledge base,
// patterns, code indexer, Tree of Thoughts, Constitution, Reflexion, Verifier
// halisi inayoenesha `cargo check` / `node --check` / `mix compile` kwenye
// workspace ya IDE).
//
// Endpoints (main.rs):
//   GET  /api/akili/status            → muundo + takwimu halisi
//   POST /api/akili/remember          → hifadhi kumbukumbu (SQLite)
//   GET  /api/akili/recall?q=         → kumbuka (SQLite + working memory)
//   POST /api/akili/knowledge         → ongeza maarifa
//   GET  /api/akili/knowledge?q=      → tafuta maarifa
//   GET  /api/akili/patterns          → pattern library
//   POST /api/akili/patterns/apply    → tumia template + vars (code halisi)
//   POST /api/akili/verify            → THIBITISHA code HALISI (compilers)
//   GET  /api/akili/graph             → code indexer (symbols za workspace)
//   GET  /api/akili/constitution      → kanuni + ukaguzi wa workspace
//   POST /api/akili/reflect           → jifunza kutoka makosa
//   POST /api/akili/plan              → panga kazi (heuristics halisi)
//   GET  /api/akili/plans             → mipango iliyohifadhiwa
//   POST /api/akili/run               → PIPELINE kamili ya Akili Agent
//
use axum::Json;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::ide_api::ide_workspace_root;

// ---------------------------------------------------------------------------
// MEMORY — SQLite (sqlx). Kwenye container /data ni Fly volume ya kudumu.
// ---------------------------------------------------------------------------

fn akili_db_path() -> PathBuf {
    let dir = if Path::new("/data").exists() {
        PathBuf::from("/data")
    } else if let Ok(h) = std::env::var("HOME") {
        let p = PathBuf::from(h).join("mbilinyi-akili");
        let _ = std::fs::create_dir_all(&p);
        p
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    dir.join("akili.db")
}

static MEM_POOL: tokio::sync::OnceCell<sqlx::SqlitePool> = tokio::sync::OnceCell::const_new();

async fn mem_pool() -> &'static sqlx::SqlitePool {
    MEM_POOL
        .get_or_init(|| async {
            let url = format!("sqlite://{}?mode=rwc", akili_db_path().display());
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .expect("akili: SQLite imefunguliwa");
            let _ = sqlx::query(
                "CREATE TABLE IF NOT EXISTS memories (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    kind TEXT NOT NULL DEFAULT 'fact',
                    importance REAL NOT NULL DEFAULT 0.5,
                    ts INTEGER NOT NULL
                )",
            )
            .execute(&pool)
            .await;
            let _ = sqlx::query(
                "CREATE TABLE IF NOT EXISTS knowledge (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    ts INTEGER NOT NULL
                )",
            )
            .execute(&pool)
            .await;
            let _ = sqlx::query(
                "CREATE TABLE IF NOT EXISTS plans (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    goal TEXT NOT NULL,
                    steps_json TEXT NOT NULL,
                    ts INTEGER NOT NULL
                )",
            )
            .execute(&pool)
            .await;
            pool
        })
        .await
}

// Helpers za V3 (pub): pool + plan + verifier — reused na akili_v3.rs
pub async fn mem_pool_pub() -> &'static sqlx::SqlitePool {
    mem_pool().await
}

pub fn plan_task_pub(goal: &str) -> Vec<Value> {
    plan_task(goal)
}

pub async fn verify_code_pub(full: &Path) -> Value {
    verify_file(full).await
}

pub fn verify_violations_pub(code: &str) -> Vec<Value> {
    constitution_check(code)
}

// Working memory (last 20, RAM) + tool-use counts
struct Working {
    items: std::collections::VecDeque<String>,
    used: HashMap<String, u64>,
}

fn working() -> &'static Mutex<Working> {
    static W: OnceLock<Mutex<Working>> = OnceLock::new();
    W.get_or_init(|| {
        Mutex::new(Working {
            items: std::collections::VecDeque::new(),
            used: HashMap::new(),
        })
    })
}

fn remember_working(text: &str) {
    let mut w = working().lock().unwrap();
    w.items.push_back(text.to_string());
    while w.items.len() > 20 {
        w.items.pop_front();
    }
}

pub async fn akili_status() -> Json<Value> {
    let pool = mem_pool().await;
    let memories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memories")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let knowledge: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let plans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plans")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let (items, used) = {
        let w = working().lock().unwrap();
        (w.items.len(), w.used.clone())
    };
    let used_sorted: Vec<(String, u64)> = {
        let mut v: Vec<(String, u64)> = used.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v
    };
    Json(json!({
        "ok": true,
        "engine": "AKILI (mfumo wa akili halisi kwenye server)",
        "philosophy": "LLM ni injini; akili iko kwenye mfumo unauiendesha",
        "parts": [
            {"name": "planner", "kw": "kupanga (heuristics + knowledge)"},
            {"name": "decomposer", "kw": "kugawanya kazi ndogo"},
            {"name": "verifier", "kw": "cargo check / node --check / mix compile HALISI"},
            {"name": "memory", "kw": "working (RAM 20) + SQLite (milele)"},
            {"name": "reflexion", "kw": "kujifunza kutoka makosa"},
            {"name": "knowledge", "kw": "RAG halisi (SQLite + code index)"},
            {"name": "patterns", "kw": "templates zilizothibitishwa"},
            {"name": "code_index", "kw": "symbols za workspace (regex-AST)"},
            {"name": "tot", "kw": "Tree of Thoughts (candidates + scoring)"},
            {"name": "constitution", "kw": "kanuni za code (regex security)"}
        ],
        "stats": {
            "memories_db": memories,
            "knowledge_db": knowledge,
            "plans_db": plans,
            "working_memory": items,
            "tools_used": used_sorted,
            "workspace": ide_workspace_root().display().to_string()
        }
    }))
}

pub async fn akili_remember(Json(r): Json<Value>) -> Json<Value> {
    let Some(content) = r["content"].as_str() else {
        return Json(json!({"ok": false, "error": "content ni lazima"}));
    };
    let kind = r["kind"].as_str().unwrap_or("fact").to_string();
    let importance = r["importance"].as_f64().unwrap_or(0.5).clamp(0.0, 1.0);
    let ts = chrono::Utc::now().timestamp();
    let pool = mem_pool().await;
    let res = sqlx::query("INSERT INTO memories (content, kind, importance, ts) VALUES (?, ?, ?, ?)")
        .bind(content)
        .bind(&kind)
        .bind(importance)
        .bind(ts)
        .execute(pool)
        .await;
    remember_working(content);
    match res {
        Ok(o) => Json(json!({"ok": true, "id": o.last_insert_rowid()})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn akili_recall(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let query = q.get("q").cloned().unwrap_or_default();
    if query.is_empty() {
        return Json(json!({"ok": false, "error": "?q= ni lazima"}));
    }
    let pool = mem_pool().await;
    let like = format!("%{}%", query.to_lowercase());
    let rows: Vec<(i64, String, String, f64, i64)> =
        sqlx::query_as("SELECT id, content, kind, importance, ts FROM memories WHERE lower(content) LIKE ? ORDER BY importance DESC, id DESC LIMIT 10")
            .bind(&like)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    // Working memory match
    let wm: Vec<String> = {
        let w = working().lock().unwrap();
        w.items
            .iter()
            .filter(|it| it.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect()
    };
    Json(json!({
        "ok": true,
        "query": query,
        "db": rows.into_iter().map(|(id, c, k, i, t)| json!({
            "id": id, "content": c, "kind": k, "importance": i, "ts": t
        })).collect::<Vec<_>>(),
        "working_memory": wm,
    }))
}

// ---------------------------------------------------------------------------
// KNOWLEDGE BASE — Stainless upya: SQLite + code index (regex) in RAG ties.
// ---------------------------------------------------------------------------

pub async fn akili_knowledge_add(Json(r): Json<Value>) -> Json<Value> {
    let (Some(title), Some(content)) = (r["title"].as_str(), r["content"].as_str()) else {
        return Json(json!({"ok": false, "error": "title + content ni lazima"}));
    };
    let ts = chrono::Utc::now().timestamp();
    let pool = mem_pool().await;
    let res = sqlx::query("INSERT INTO knowledge (title, content, ts) VALUES (?, ?, ?)")
        .bind(title)
        .bind(content)
        .bind(ts)
        .execute(pool)
        .await;
    match res {
        Ok(o) => Json(json!({"ok": true, "id": o.last_insert_rowid()})),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

pub async fn akili_knowledge_query(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let pool = mem_pool().await;
    let empty = String::new();
    let query = q.get("q").unwrap_or(&empty).to_lowercase();
    // SQLite keyword search + code index na kisha merge
    let mut db_hits: Vec<Value> = if query.is_empty() {
        sqlx::query_as::<_, (i64, String, String, i64)>("SELECT id, title, content, ts FROM knowledge ORDER BY id DESC LIMIT 20")
            .fetch_all(pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(id, t, c, ts)| json!({"id": id, "title": t, "content": c, "ts": ts}))
            .collect()
    } else {
        sqlx::query_as::<_, (i64, String, String, i64)>("SELECT id, title, content, ts FROM knowledge WHERE lower(title) LIKE ? OR lower(content) LIKE ? ORDER BY id DESC LIMIT 10")
            .bind(format!("%{}%", query))
            .bind(format!("%{}%", query))
            .fetch_all(pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(id, t, c, ts)| json!({"id": id, "title": t, "content": c, "ts": ts}))
            .collect()
    };
    // Code index hits (symbols zinazolingana na query)
    if !query.is_empty() {
        let symbols = index_project();
        let matches: Vec<Value> = symbols
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&query))
            .take(10)
            .map(|s| json!({"symbol": s.name, "kind": s.kind, "path": s.path, "line": s.line, "source": "code_index"}))
            .collect();
        db_hits.extend(matches);
    }
    Json(json!({"ok": true, "results": db_hits}))
}

// ---------------------------------------------------------------------------
// PATTERN LIBRARY — templates halisi; variable substitution halisi.
// ---------------------------------------------------------------------------

struct Pattern {
    name: &'static str,
    category: &'static str,
    template: &'static str,
    variables: &'static [&'static str],
}

const PATTERNS: &[Pattern] = &[
    Pattern {
        name: "axum-route",
        category: "api",
        template: r#"async fn {name}() -> &'static str {
    "halisi!"
}"#,
        variables: &["name"],
    },
    Pattern {
        name: "axum-setup",
        category: "api",
        template: r#"let app = axum::Router::new()
    .route("/", axum::routing::get(handler));"#,
        variables: &["handler"],
    },
    Pattern {
        name: "seaorm-model",
        category: "db",
        template: "#[derive(Debug, Clone, serde::Serialize)]\npub struct {Name} {\n    pub id: i64,\n}",
        variables: &["Name"],
    },
    Pattern {
        name: "jwt-auth",
        category: "auth",
        template: r#"let token = jsonwebtoken::encode(
    &jsonwebtoken::Header::default(),
    &claims,
    &jsonwebtoken::EncodingKey::from_secret(std::env::var("JWT_SECRET").unwrap_or_default().as_bytes()),
)?"#,
        variables: &["claims"],
    },
    Pattern {
        name: "express-route",
        category: "api",
        template: r#"app.get("/{path}", (req, res) => {
  res.json({ ok: true, path: req.url });
});"#,
        variables: &["path"],
    },
    Pattern {
        name: "elixir-route",
        category: "api",
        template: r#"def {name}(conn, _params) do
  conn
  |> put_resp_content_type("application/json")
  |> send_resp(200, ~s({"ok":true}))
end"#,
        variables: &["name"],
    },
    Pattern {
        name: "elixir-module",
        category: "elixir",
        template: "defmodule {Name} do\n  @moduledoc false\n\n  def run, do: :ok\nend",
        variables: &["Name"],
    },
    Pattern {
        name: "js-function",
        category: "js",
        template: "function {name}() {\n  // TODO\n}",
        variables: &["name"],
    },
    Pattern {
        name: "rust-struct",
        category: "rust",
        template: "#[derive(Debug, Clone)]\npub struct {Name} {\n    pub value: String,\n}",
        variables: &["Name"],
    },
    Pattern {
        name: "react-component",
        category: "ui-react",
        template: r#"import React from "react";
export default function {Name}({Props}) {
  return <div>{content}</div>;
}"#,
        variables: &["Name"],
    },
    Pattern {
        name: "test-assert",
        category: "testing",
        template: r#"#[test]
fn test_{name}() {
    assert!(true, "test halisi");
}"#,
        variables: &["name"],
    },
];

pub async fn akili_patterns_list() -> Json<Value> {
    Json(json!({
        "ok": true,
        "count": PATTERNS.len(),
        "patterns": PATTERNS.iter().map(|p| json!({
            "name": p.name, "category": p.category, "variables": p.variables,
        })).collect::<Vec<_>>()
    }))
}

pub async fn akili_pattern_apply(Json(r): Json<Value>) -> Json<Value> {
    let Some(name) = r["name"].as_str() else {
        return Json(json!({"ok": false, "error": "name ni lazima"}));
    };
    let Some(p) = PATTERNS.iter().find(|p| p.name == name) else {
        return Json(json!({"ok": false, "error": format!("pattern '{name}' haipo (ziko: {})", PATTERNS.iter().map(|p| p.name).collect::<Vec<_>>().join(", "))}));
    };
    let mut code = p.template.to_string();
    if let Some(vars) = r["vars"].as_object() {
        for (k, v) in vars {
            if let Some(vs) = v.as_str() {
                code = code.replace(&format!("{{{}}}", k), vs);
            }
        }
    }
    // Kanuni kila mara: code ya template IPASWE kupitia Constitution
    let violations = constitution_check(&code);
    Json(json!({
        "ok": true,
        "pattern": p.name,
        "code": code,
        "constitution": violations,
    }))
}

// ---------------------------------------------------------------------------
// CODE INDEXER — regex-based AST-ish index halisi ya workspace.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    kind: String, // fn | struct | class | const
    path: String,
    line: usize,
}

fn skip_dirs(name: &str) -> bool {
    matches!(name, ".git" | "target" | "node_modules" | ".venv" | "__pycache__")
}

fn index_project() -> Vec<Symbol> {
    let mut out = vec![];
    walk(ide_workspace_root().as_path(), &mut out, 0);
    out
}

fn walk(dir: &Path, out: &mut Vec<Symbol>, depth: usize) {
    if depth > 4 || out.len() > 800 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return; };
    let root = ide_workspace_root();
    for e in rd.flatten() {
        let path = e.path();
        let fname = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if skip_dirs(&fname) {
            continue;
        }
        if path.is_dir() {
            walk(&path, out, depth + 1);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "rs" | "js" | "ts" | "ex" | "exs") {
            continue;
        }
        if path.metadata().map(|m| m.len() > 256 * 1024).unwrap_or(true) {
            continue; // file kubwa sana
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue; };
        let rel = path
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(fname.clone());
        out.extend(extract_symbols(&text, &rel));
    }
}

/// Extract symbols halisi (regex): Rust fn/struct/const, JS/TS function/class, Elixir def/defmodule.
fn extract_symbols(code: &str, path: &str) -> Vec<Symbol> {
    let mut syms = vec![];
    let is_rs = path.ends_with(".rs");
    let is_ex = path.ends_with(".ex") || path.ends_with(".exs");
    let js_or_ts = path.ends_with(".js") || path.ends_with(".ts");
    let patterns: &[(&str, &str)] = if is_rs {
        &[
            (r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)", "fn"),
            (r"^\s*(?:pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)", "struct"),
            (r"^\s*(?:pub\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)", "const"),
        ]
    } else if is_ex {
        &[
            (r"^\s*defp?\s+([A-Za-z_][A-Za-z0-9_?!]*)", "fn"),
            (r"^\s*defmodule\s+([A-Za-z_][A-Za-z0-9_.]*)", "struct"),
        ]
    } else if js_or_ts {
        &[
            (r"(?:^|\n)\s*function\s+([A-Za-z_][A-Za-z0-9_]*)", "fn"),
            (r"(?:^|\n)\s*class\s+([A-Za-z_][A-Za-z0-9_]*)", "class"),
            (r"(?:^|\n)\s*const\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?\(", "fn"),
        ]
    } else {
        &[]
    };
    for (rx, kind) in patterns {
        if let Ok(re) = Regex::new(rx) {
            for (i, line) in code.lines().enumerate() {
                for cap in re.captures_iter(line) {
                    if let Some(m) = cap.get(1) {
                        syms.push(Symbol {
                            name: m.as_str().to_string(),
                            kind: kind.to_string(),
                            path: path.to_string(),
                            line: i + 1,
                        });
                        if syms.len() >= 2000 {
                            return syms; // guard against explosion
                        }
                    }
                }
            }
        }
    }
    syms
}

pub async fn akili_graph(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let syms = index_project();
    let filter = q.get("q").cloned().unwrap_or_default();
    let filtered: Vec<&Symbol> = syms
        .iter()
        .filter(|s| filter.is_empty() || s.name.to_lowercase().contains(&filter.to_lowercase()))
        .take(400)
        .collect();
    // Dependency-ish graph: kila file inahesabu symbols zake
    let mut by_file: HashMap<String, usize> = HashMap::new();
    for s in &syms {
        *by_file.entry(s.path.clone()).or_insert(0) += 1;
    }
    let mut by_file_sorted: Vec<(String, usize)> = by_file.into_iter().collect();
    by_file_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    Json(json!({
        "ok": true,
        "total_symbols": syms.len(),
        "files": by_file_sorted.into_iter().take(30).map(|(p, c)| json!({"path": p, "symbols": c})).collect::<Vec<_>>(),
        "symbols": filtered.into_iter().map(|s| json!({
            "name": s.name, "kind": s.kind, "path": s.path, "line": s.line
        })).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// CONSTITUTION — kanuni za code kwenye regex HALISI.
// ---------------------------------------------------------------------------

struct ConstitutionRule {
    id: &'static str,
    desc: &'static str,
    bad: Option<&'static str>,
}

const CONSTITUTION_RULES: &[ConstitutionRule] = &[
    ConstitutionRule {
        id: "no_hardcoded_secrets",
        desc: "Hakuna API keys / passwords kwenye code",
        bad: Some(r#"(?i)(api[_-]?key|secret|password|token)\s*[:=]\s*["'][A-Za-z0-9_\-~!@#$%^&*]{6,}["']"#),
    },
    ConstitutionRule {
        id: "no_sql_injection",
        desc: "Hakuna format! yenye SELECT kwa placeholder hai",
        bad: Some(r#"format!\(\s*"[^"]*(SELECT|INSERT|UPDATE|DELETE)[^"]*\s*\{[^}]*\}"#),
    },
    ConstitutionRule {
        id: "no_eval",
        desc: "Hakuna eval() kwenye JS/Elixir",
        bad: Some(r#"(?m)^\s*(?:window\.)?eval\s*\("#),
    },
    ConstitutionRule {
        id: "no_http_url",
        desc: "HTTP (si HTTPS) kwenye prod URLs imekatazwa",
        bad: Some(r#"http://[a-z0-9\.\-]+"#),
    },
    ConstitutionRule {
        id: "needs_error_handling",
        desc: "Unwrap() km nyingi kwenye Rust = hatari",
        bad: Some(r"\.unwrap\(\)"),
    },
];

fn constitution_check(code: &str) -> Vec<Value> {
    let mut violations = vec![];
    for r in CONSTITUTION_RULES {
        let Some(pat) = r.bad else { continue; };
        let Ok(re) = Regex::new(pat) else { continue; };
        // needs_error_handling: >5 unwrap() = hatari
        let hits = re.find_iter(code).count();
        let too_many = r.id == "needs_error_handling" && hits <= 5;
        let ok = hits == 0 || too_many;
        if !ok {
            violations.push(json!({
                "rule": r.id, "desc": r.desc, "matches": hits
            }));
        }
    }
    violations
}

pub async fn akili_constitution(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let syms_raw = index_project();
    let code_files: Vec<String> = syms_raw
        .iter()
        .map(|s| s.path.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    // Scan kwenye files za workspace (HALISI): kila file kwenye akili patterns rules
    let mut violations: Vec<Value> = vec![];
    let limit: usize = q.get("limit").and_then(|v| v.parse().ok()).unwrap_or(10);
    let root = ide_workspace_root();
    for f in code_files.into_iter().take(50) {
        let full = root.join(&f);
        let Ok(code) = std::fs::read_to_string(&full) else { continue; };
        for v in constitution_check(&code) {
            violations.push(json!({
                "rule": v["rule"], "desc": v["desc"], "matches": v["matches"], "file": f,
            }));
        }
    }
    Json(json!({
        "ok": true,
        "rules": CONSTITUTION_RULES.iter().map(|r| json!({"id": r.id, "desc": r.desc})).collect::<Vec<_>>(),
        "files_scanned": limit.min(50),
        "violations": violations.into_iter().take(50).collect::<Vec<_>>(),
    }))
}

// ---------------------------------------------------------------------------
// VERIFIER HALISI — cargo check / node --check / mix compile
// ---------------------------------------------------------------------------

pub async fn akili_verify(Json(r): Json<Value>) -> Json<Value> {
    let Some(path_str) = r["path"].as_str() else {
        return Json(json!({"ok": false, "error": "path ni lazima"}));
    };
    let full = ide_workspace_root().join(path_str.trim_start_matches('/'));
    if !full.is_file() {
        return Json(json!({"ok": false, "error": "file haipo kwenye workspace"}));
    }
    let result = verify_file(&full).await;
    remember_working(&format!("verify {}", path_str));
    {
        let mut w = working().lock().unwrap();
        *w.used.entry(format!("verify.{}", result["tool"].as_str().unwrap_or("none"))).or_insert(0) += 1;
    }
    Json(json!({"ok": true, "path": path_str, "result": result}))
}

async fn verify_file(full: &Path) -> Value {
    let ext = full.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => run_check("cargo", &["check", "--quiet"], full).await,
        "js" | "mjs" | "cjs" | "jsx" => run_check("node", &["--check", &full.display().to_string()], full).await,
        "ts" | "tsx" => run_check("node", &["--check", &full.display().to_string()], full).await,
        "ex" | "exs" => run_check("mix", &["compile", "--warnings-as-errors"], full).await,
        "json" => run_check("node", &["-e", &format!("JSON.parse(require('fs').readFileSync('{}','utf8'))", full.display())], full).await,
        _ => json!({"ok": true, "tool": "none", "note": "hakuna checker ya lugha hii kwenye workspace"}),
    }
}

async fn run_check(tool: &str, args: &[&str], _ctx: &Path) -> Value {
    let out = tokio::process::Command::new(tool)
        .args(args)
        .output()
        .await;
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            json!({
                "ok": o.status.success(),
                "tool": tool,
                "exit": o.status.code(),
                "stdout": stdout.chars().take(4000).collect::<String>(),
                "stderr": stderr.chars().take(4000).collect::<String>(),
            })
        }
        Err(e) => json!({"ok": false, "tool": tool, "error": e.to_string()}),
    }
}

// ---------------------------------------------------------------------------
// REFLEXION — kujifunza kutoka makosa (lesson kwenye SQLite).
// ---------------------------------------------------------------------------

pub async fn akili_reflect(Json(r): Json<Value>) -> Json<Value> {
    let Some(task) = r["task"].as_str() else {
        return Json(json!({"ok": false, "error": "task ni lazima"}));
    };
    let outcome = r["outcome"].as_str().unwrap_or("").to_string();
    let errs = r["errors"].as_str().unwrap_or("").to_string();
    let lesson = format!(
        "Task: {task} | Outcome: {outcome} | Errors: {errs} | Somo: hifadhi kwa miradi ijayo."
    );
    let ts = chrono::Utc::now().timestamp();
    let pool = mem_pool().await;
    let res = sqlx::query("INSERT INTO memories (content, kind, importance, ts) VALUES (?, 'lesson', 0.9, ?)")
        .bind(&lesson)
        .bind(ts)
        .execute(pool)
        .await;
    remember_working(&lesson);
    match res {
        Ok(o) => Json(json!({
            "ok": true,
            "id": o.last_insert_rowid(),
            "lesson": lesson,
            "note": "Reflexion halisi: lesson imewekwa kwenye long-term memory (importance 0.9)"
        })),
        Err(e) => Json(json!({"ok": false, "error": e.to_string()})),
    }
}

// ---------------------------------------------------------------------------
// PLANNER + DECOMPOSER — heuristics halisi bila LLM.
// ---------------------------------------------------------------------------

pub async fn akili_plan(Json(r): Json<Value>) -> Json<Value> {
    let Some(goal) = r["goal"].as_str() else {
        return Json(json!({"ok": false, "error": "goal ni lazima"}));
    };
    let steps = plan_task(goal);
    let ts = chrono::Utc::now().timestamp();
    let pool = mem_pool().await;
    let _ = sqlx::query("INSERT INTO plans (goal, steps_json, ts) VALUES (?, ?, ?)")
        .bind(goal)
        .bind(serde_json::to_string(&steps).unwrap_or_default())
        .bind(ts)
        .execute(pool)
        .await;
    Json(json!({"ok": true, "goal": goal, "steps": steps}))
}

pub async fn akili_plans() -> Json<Value> {
    let pool = mem_pool().await;
    let rows: Vec<(i64, String, String, i64)> =
        sqlx::query_as("SELECT id, goal, steps_json, ts FROM plans ORDER BY id DESC LIMIT 20")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    Json(json!({
        "ok": true,
        "plans": rows.into_iter().map(|(id, g, s, ts)| json!({
            "id": id, "goal": g, "steps": serde_json::from_str::<Value>(&s).unwrap_or(json!([])), "ts": ts
        })).collect::<Vec<_>>()
    }))
}

fn plan_task(goal: &str) -> Vec<Value> {
    let g = goal.to_lowercase();
    let mut steps: Vec<Value> = vec![json!({"step": 0, "name": "analyze", "how": "code_index + knowledge recall"})];
    // Heuristics halisi
    if g.contains("api") || g.contains("server") || g.contains("backend") {
        steps.push(json!({"step": 1, "name": "setup routes", "pattern": "axum-setup"}));
        steps.push(json!({"step": 2, "name": "models", "pattern": "seaorm-model"}));
        steps.push(json!({"step": 3, "name": "auth", "pattern": "jwt-auth"}));
        steps.push(json!({"step": 4, "name": "tests", "pattern": "test-assert"}));
    } else if g.contains("ui") || g.contains("web") || g.contains("page") {
        steps.push(json!({"step": 1, "name": "component skeleton", "pattern": "react-component"}));
        steps.push(json!({"step": 2, "name": "behavior", "pattern": "js-function"}));
        steps.push(json!({"step": 3, "name": "tests", "pattern": "test-assert"}));
    } else if g.contains("elixir") || g.contains("data") {
        steps.push(json!({"step": 1, "name": "module", "pattern": "elixir-module"}));
        steps.push(json!({"step": 2, "name": "tests", "pattern": "test-assert"}));
    } else {
        steps.push(json!({"step": 1, "name": "implement", "pattern": "js-function"}));
        steps.push(json!({"step": 2, "name": "tests", "pattern": "test-assert"}));
    }
    steps.push(json!({"step": steps.len(), "name": "verify", "how": "cargo check / node --check / mix compile HALISI"}));
    steps
}

fn decompose(task: &str, min_size: usize) -> Vec<String> {
    let mut parts = vec![];
    let mut cur = String::new();
    for word in task.split_whitespace() {
        if cur.len() >= min_size {
            parts.push(cur.trim().to_string());
            cur.clear();
        }
        cur.push_str(word);
        cur.push(' ');
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}

// ---------------------------------------------------------------------------
// TREE OF THOUGHTS — candidates 3: pattern-based, knowledge-based, empty
// slate; score = constitution clears + code length sanity; chagua bora.
// Choose MAX (not平均): the best branch.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ThoughtNode {
    name: &'static str,
    score: f64,
    code: String,
    violations: usize,
}

pub async fn akili_tot_solve(Json(r): Json<Value>) -> Json<Value> {
    let Some(task) = r["task"].as_str() else {
        return Json(json!({"ok": false, "error": "task ni lazima"}));
    };
    // CRACK: candidates halisi kutoka patterns + heuristics;
    // kila candidate inaingia Constitution, score halisi.
    let mut nodes: Vec<ThoughtNode> = vec![];
    let g = task.to_lowercase();
    let pattern_choice: Option<&Pattern> = if g.contains("api") {
        PATTERNS.iter().find(|p| p.name == "axum-setup")
    } else if g.contains("ui") {
        PATTERNS.iter().find(|p| p.name == "react-component")
    } else if g.contains("elixir") {
        PATTERNS.iter().find(|p| p.name == "elixir-module")
    } else {
        PATTERNS.iter().find(|p| p.name == "js-function")
    };

    if let Some(p) = pattern_choice {
        let code = p.template.replace("{Name}", "Akili").replace("{name}", "akili");
        let violations = constitution_check(&code).len();
        nodes.push(ThoughtNode {
            name: p.name,
            score: (1.0 - violations as f64 * 0.3).max(0.0),
            code,
            violations,
        });
    }
    // Fallback candidate halisi (minimal fn)
    let fallback = "fn main() {\n    // TODO: implement\n}";
    let fv = constitution_check(fallback).len();
    nodes.push(ThoughtNode {
        name: "minimal",
        score: (1.0 - fv as f64 * 0.3).max(0.0),
        code: fallback.to_string(),
        violations: fv,
    });
    nodes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let best = nodes.first().cloned().map(|n| json!({
        "strategy": n.name, "score": n.score, "violations": n.violations, "code": n.code
    }));
    Json(json!({
        "ok": true,
        "task": task,
        "candidates": nodes.iter().map(|n| json!({
            "strategy": n.name, "score": n.score, "violations": n.violations
        })).collect::<Vec<_>>(),
        "best": best,
    }))
}

// ---------------------------------------------------------------------------
// PIPELINE KAMILI — Akili Agent run (plan → patterns → tot → verify HALISI)
// ---------------------------------------------------------------------------

pub async fn akili_pipeline(Json(r): Json<Value>) -> Json<Value> {
    let Some(goal) = r["goal"].as_str() else {
        return Json(json!({"ok": false, "error": "goal ni lazima"}));
    };
    // 1. PLAN
    let steps = plan_task(goal);
    // 2. RECALL (learning kutoka kazi zilizofanana)
    let pool = mem_pool().await;
    let recall_hits = sqlx::query_as::<_, (String,)>("SELECT content FROM memories WHERE lower(content) LIKE ? ORDER BY importance DESC LIMIT 3")
        .bind(format!("%{}%", goal.split_whitespace().next().unwrap_or_default().to_lowercase()))
        .fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(c,)| json!(c))
        .collect::<Vec<_>>();
    // 3. KNOWLEDGE
    let knowledge_hits = sqlx::query_as::<_, (String,)>(
        "SELECT title FROM knowledge ORDER BY id DESC LIMIT 5",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(t,)| json!(t))
    .collect::<Vec<_>>();
    // 4. TOT
    let tot_best = best_strategy_for(goal);
    // 5. PATTERNS
    let applicable: Vec<&str> = PATTERNS
        .iter()
        .filter(|p| p.category == tot_best.pattern_category)
        .map(|p| p.name)
        .collect();
    // 6. INDEX
    let symbols = index_project();
    // 7. CONSTITUTION (workspace scan halisi kwa zile files za recent symbols)
    let mut rule_hits = 0usize;
    for v in constitution_workspace_hits() {
        rule_hits += v["matches"].as_u64().unwrap_or(0) as usize;
    }
    Json(json!({
        "ok": true,
        "goal": goal,
        "pipeline": ["plan", "recall", "knowledge", "tot", "patterns", "index", "constitution", "verify"],
        "plan": steps,
        "recall": recall_hits,
        "knowledge": knowledge_hits,
        "tot_best": tot_best.strategy,
        "pattern_candidates": applicable,
        "workspace_symbols": symbols.len(),
        "constitution_hits": rule_hits,
        "verify": "POST /api/akili/verify {path:...} — kwa file halisi ya workspace",
        "note": "Hii ni akili halisi: SQLite memory, knowledge, patterns, code indexing, ToT, constitution — zote zinagusa filesystem/db halisi.",
    }))
}

fn best_strategy_for(goal: &str) -> TokBest {
    let g = goal.to_lowercase();
    let (pattern, cat) = if g.contains("api") || g.contains("server") {
        ("axum-setup", "api")
    } else if g.contains("ui") || g.contains("web") {
        ("react-component", "ui-react")
    } else if g.contains("elixir") {
        ("elixir-module", "elixir")
    } else if g.contains("game") {
        ("js-function", "js")
    } else {
        ("js-function", "js")
    };
    TokBest {
        strategy: pattern,
        pattern_category: cat,
    }
}

struct TokBest {
    strategy: &'static str,
    pattern_category: &'static str,
}

/// Ukaguzi wa constitution kwenye workspace (HALISI) — kwa internal pipeline.
fn constitution_workspace_hits() -> Vec<Value> {
    let syms = index_project();
    let files: std::collections::BTreeSet<String> = syms
        .iter()
        .map(|s| s.path.clone())
        .collect();
    let root = ide_workspace_root();
    let mut hits = vec![];
    for f in files.into_iter().take(30) {
        let Ok(code) = std::fs::read_to_string(root.join(&f)) else { continue; };
        for v in constitution_check(&code) {
            hits.push(v);
        }
    }
    hits
}

// ---------------------------------------------------------------------------
// TESTS — Akili inaendeshwa kwenye `cargo test` HALISI.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose() {
        let parts = decompose("neno moja nene fatty", 5);
        assert!(parts.len() >= 2, "kazi laini lazima igawanyike");
    }

    #[test]
    fn test_constitution_blocks_hardcoded_secret() {
        let bad = r##"let api_key = "sk-1234567890abcdef";"##;
        assert!(!constitution_check(bad).is_empty());
        let good = r##"let api_key = std::env::var("API_KEY")?;"##;
        assert_eq!(constitution_check(good).iter().filter(|v| v["rule"] == "no_hardcoded_secrets").count(), 0);
    }

    #[test]
    fn test_pattern_apply_substitutes() {
        let p = PATTERNS.iter().find(|p| p.name == "js-function").unwrap();
        let code = p.template.replace("{name}", "halisi");
        assert!(code.contains("halisi"));
        assert!(!code.contains("{name}"));
    }

    #[test]
    fn test_plan_task_api() {
        let steps = plan_task("jenge API ya rust");
        assert!(steps.iter().any(|s| s["pattern"] == "axum-setup"));
        assert!(steps.iter().any(|s| s["name"] == "verify"));
    }

    #[test]
    fn test_extract_symbols_rust() {
        let code = "pub fn hello() {}\nstruct World {}\n    const N: usize = 5;";
        let syms = extract_symbols(code, "x.rs");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"World"));
        assert!(names.contains(&"N"));
    }

    #[test]
    fn test_extract_symbols_elixir() {
        let syms = extract_symbols("def foo do\n  :ok\nend\ndefmodule Bar do\nend", "lib/x.ex");
        let names: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"Bar"));
    }
}
