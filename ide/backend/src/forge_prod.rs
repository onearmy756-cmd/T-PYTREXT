//! forge_prod.rs — GENERATORS ZA PRODUCTION (sio scaffolding).
//!
//! Tofauti na `forge_generators.rs` (stubs), hapa tunazalisha kodi KAMILI
//! inayoendeshwa: models, validation, error handling, pagination, migrations,
//! na TESTI halisi. Hupokea `Domain` (kutoka `forge_domain`) — hivyo inajua
//! entities na fields zake, na hutoa business logic ya kweli.
//!
//! Kila mradi unaozalishwa hapa LAZIMA upite testi zake (cargo test / node --test).

use crate::forge_domain::{to_pascal, to_snake, Domain, Entity, FieldType};

pub fn rs_ident(name: &str) -> String {
    let mut s = to_snake(name);
    if s.is_empty() {
        s = "field".into();
    }
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s = format!("f_{}", s);
    }
    match s.as_str() {
        "type" | "match" | "move" | "ref" | "self" | "super" | "crate" | "loop" | "impl"
        | "fn" | "let" | "mut" | "pub" | "use" | "mod" | "struct" | "enum" | "trait"
        | "where" | "async" | "await" | "dyn" | "box" | "static" | "const" => s.push('_'),
        _ => {}
    }
    s
}

fn is_managed(name: &str) -> bool {
    matches!(name, "id" | "created_at" | "updated_at")
}

// ─────────────────────────────────────────────────────────────────────────────
// RUST / Axum (SQLx + tower + tests) — production
// ─────────────────────────────────────────────────────────────────────────────

/// Aina ya Rust.
fn rs_type(t: FieldType) -> &'static str {
    match t {
        FieldType::Int => "i64",
        FieldType::Float => "f64",
        FieldType::Text => "String",
        FieldType::Bool => "bool",
        FieldType::Timestamp => "chrono::DateTime<chrono::Utc>",
        FieldType::Email => "String",
        FieldType::Uuid => "String",
    }
}

pub fn generate_rust(domain: &Domain) -> (Vec<(String, String)>, String) {
    let mut files: Vec<(String, String)> = Vec::new();
    files.push(("src/lib.rs".into(), rs_lib(domain)));
    files.push(("src/db.rs".into(), rs_db()));
    files.push(("src/error.rs".into(), rs_error()));
    files.push(("src/models.rs".into(), rs_models(domain)));
    for e in &domain.entities {
        files.push((format!("src/{}.rs", e.name), rs_module(e)));
    }
    files.push(("src/main.rs".into(), rs_main(domain)));
    files.push(("Cargo.toml".into(), rs_cargo(domain)));
    files.push(("src/schema.sql".into(), rs_migrations(domain)));
    (files, format!("rust-api (axum+sqlx) entities {} (production)", domain.entities.len()))
}

fn rs_lib(domain: &Domain) -> String {
    let mut s = String::from("//! Library crate (production).\n\npub mod db;\npub mod error;\npub mod models;\n");
    for e in &domain.entities {
        s.push_str(&format!("pub mod {};\n", e.name));
    }
    s
}

fn rs_cargo(domain: &Domain) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = {{ version = "1", features = ["full"] }}
tower-http = {{ version = "0.5", features = ["trace"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
sqlx = {{ version = "0.7", features = ["runtime-tokio", "sqlite", "chrono", "uuid", "macros"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
uuid = {{ version = "1", features = ["v4", "serde"] }}
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
tower = {{ version = "0.4", features = ["util"] }}
http-body-util = "0.1"
"#,
        to_snake(&domain.project)
    )
}

fn rs_db() -> String {
    r#"//! Database pool (SQLx, SQLite).
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

pub type Db = SqlitePool;

/// Unda pool (in-memory kwa testi, faili kwa production).
pub async fn connect(url: &str) -> Result<Db, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await?;
    Ok(pool)
}

/// Endesha migrations zote zilizoandikwa (idempotent).
/// `sqlx::query` hutekeleza statement MOJA; tunagawanya kwa `;`.
pub async fn migrate(pool: &Db) -> Result<(), sqlx::Error> {
    let sql = include_str!("schema.sql");
    for stmt in sql.split(';') {
        let s = stmt.trim();
        if s.is_empty() {
            continue;
        }
        sqlx::query(s).execute(pool).await?;
    }
    Ok(())
}
"#
    .to_string()
}

fn rs_error() -> String {
    r#"//! Unified error type + HTTP mapping.
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database error".to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
"#
    .to_string()
}

fn rs_models(domain: &Domain) -> String {
    let mut s = String::from("//! ORM models (SQLx FromRow).\nuse serde::{Deserialize, Serialize};\n\n");
    for e in &domain.entities {
        let cls = to_pascal(&e.name);
        s.push_str(&format!(
            "#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]\npub struct {} {{\n    pub id: String,\n",
            cls
        ));
        for f in &e.fields {
            if is_managed(&f.name) {
                continue;
            }
            let name = rs_ident(&f.name);
            let ty = rs_type(f.ty);
            let ty = if f.required { ty.to_string() } else { format!("Option<{}>", ty) };
            s.push_str(&format!("    pub {}: {},\n", name, ty));
        }
        s.push_str("    pub created_at: chrono::DateTime<chrono::Utc>,\n    pub updated_at: chrono::DateTime<chrono::Utc>,\n}\n\n");

        // Create struct
        s.push_str(&format!("#[derive(Debug, Deserialize)]\npub struct {}Create {{\n", cls));
        for f in &e.fields {
            if is_managed(&f.name) {
                continue;
            }
            let name = rs_ident(&f.name);
            let ty = rs_type(f.ty);
            let ty = if f.required { ty.to_string() } else { format!("Option<{}>", ty) };
            s.push_str(&format!("    pub {}: {},\n", name, ty));
        }
        s.push_str("}\n\n");

        // Update struct (all optional)
        s.push_str(&format!("#[derive(Debug, Deserialize, Default)]\npub struct {}Update {{\n", cls));
        for f in &e.fields {
            if is_managed(&f.name) {
                continue;
            }
            let name = rs_ident(&f.name);
            s.push_str(&format!("    pub {}: Option<{}>,\n", name, rs_type(f.ty)));
        }
        s.push_str("}\n\n");

        // Page
        s.push_str(&format!(
            "#[derive(Debug, Serialize)]\npub struct {}Page {{\n    pub items: Vec<{}>,\n    pub total: i64,\n}}\n\n",
            cls, cls
        ));
    }
    s
}

fn rs_module(e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let m = &e.name;
    let mut create_cols: Vec<String> = vec!["id".into()];
    let mut placeholders: Vec<String> = vec!["?".into()];
    let mut binds: Vec<String> = vec!["id.clone()".into()];
    let mut idx = 1;
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = rs_ident(&f.name);
        create_cols.push(n.clone());
        placeholders.push("?".into());
        binds.push(format!("payload.{}", n));
        idx += 1;
    }
    create_cols.push("created_at".into());
    create_cols.push("updated_at".into());
    placeholders.push("?".into());
    placeholders.push("?".into());
    binds.push("now".into());
    binds.push("now".into());
    let _ = idx;

    format!(
        r#"//! {cls} — repository + HTTP handlers (production).
use axum::{{extract::{{Path, Query, State}}, routing::get, Json, Router}};
use serde::Deserialize;

use crate::db::Db;
use crate::error::{{AppError, AppResult}};
use crate::models::{{{cls}, {cls}Create, {cls}Page, {cls}Update}};

#[derive(Debug, Deserialize)]
pub struct PageParams {{
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}}

fn default_limit() -> i64 {{
    50
}}

pub fn router() -> Router<Db> {{
    Router::new()
        .route("/{m}", get(list).post(create))
        .route("/{m}/:id", get(get_one).put(update).delete(remove))
}}

pub async fn create(State(db): State<Db>, Json(payload): Json<{cls}Create>) -> AppResult<Json<{cls}>> {{
    validate_create(&payload)?;
    let now = chrono::Utc::now();
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO {table} ({cols}) VALUES ({ph})")
        {binds_block}
        .execute(&db)
        .await?;
    let row = get_one(State(db), Path(id)).await?;
    Ok(row)
}}

pub async fn list(State(db): State<Db>, Query(p): Query<PageParams>) -> AppResult<Json<{cls}Page>> {{
    let limit = p.limit.clamp(1, 200);
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM {table}")
        .fetch_one(&db)
        .await?;
    let items = sqlx::query_as::<_, {cls}>(
        "SELECT * FROM {table} ORDER BY created_at LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(p.offset.max(0))
    .fetch_all(&db)
    .await?;
    Ok(Json({cls}Page {{ items, total }}))
}}

pub async fn get_one(State(db): State<Db>, Path(id): Path<String>) -> AppResult<Json<{cls}>> {{
    let row = sqlx::query_as::<_, {cls}>("SELECT * FROM {table} WHERE id = ?")
        .bind(&id)
        .fetch_optional(&db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}}

pub async fn update(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(payload): Json<{cls}Update>,
) -> AppResult<Json<{cls}>> {{
    let existing = sqlx::query_as::<_, {cls}>("SELECT * FROM {table} WHERE id = ?")
        .bind(&id)
        .fetch_optional(&db)
        .await?
        .ok_or(AppError::NotFound)?;
{update_block}
    let row = sqlx::query_as::<_, {cls}>("SELECT * FROM {table} WHERE id = ?")
        .bind(&id)
        .fetch_one(&db)
        .await?;
    let _ = existing;
    Ok(Json(row))
}}

pub async fn remove(State(db): State<Db>, Path(id): Path<String>) -> AppResult<axum::http::StatusCode> {{
    let affected = sqlx::query("DELETE FROM {table} WHERE id = ?")
        .bind(&id)
        .execute(&db)
        .await?
        .rows_affected();
    if affected == 0 {{
        return Err(AppError::NotFound);
    }}
    Ok(axum::http::StatusCode::NO_CONTENT)
}}

fn validate_create(payload: &{cls}Create) -> AppResult<()> {{
{validation_block}    Ok(())
}}

#[cfg(test)]
mod tests {{
    use super::*;
    use crate::db;

    async fn setup() -> Db {{
        let pool = db::connect("sqlite::memory:").await.expect("connect");
        db::migrate(&pool).await.expect("migrate");
        pool
    }}

    fn sample() -> {cls}Create {{
        {cls}Create {{
{sample_block}        }}
    }}

    #[tokio::test]
    async fn create_then_get() {{
        let pool = setup().await;
        let created = create(State(pool.clone()), Json(sample())).await.expect("create");
        let id = created.0.id.clone();
        let fetched = get_one(State(pool.clone()), Path(id.clone())).await.expect("get");
        assert_eq!(fetched.0.id, id);
    }}

    #[tokio::test]
    async fn list_pagination() {{
        let pool = setup().await;
        for _ in 0..3 {{
            create(State(pool.clone()), Json(sample())).await.expect("create");
        }}
        let page = list(State(pool.clone()), Query(PageParams {{ offset: 0, limit: 2 }})).await.expect("list");
        assert_eq!(page.0.items.len(), 2);
        assert!(page.0.total >= 3);
    }}

    #[tokio::test]
    async fn update_changes_row() {{
        let pool = setup().await;
        let created = create(State(pool.clone()), Json(sample())).await.expect("create");
        let id = created.0.id.clone();
        let updated = update(State(pool.clone()), Path(id.clone()), Json({cls}Update::default())).await.expect("update");
        assert_eq!(updated.0.id, id);
    }}

    #[tokio::test]
    async fn delete_then_missing() {{
        let pool = setup().await;
        let created = create(State(pool.clone()), Json(sample())).await.expect("create");
        let id = created.0.id.clone();
        assert_eq!(remove(State(pool.clone()), Path(id.clone())).await.expect("remove"), axum::http::StatusCode::NO_CONTENT);
        assert!(matches!(get_one(State(pool), Path(id)).await, Err(AppError::NotFound)));
    }}
}}
"#,
        cls = cls,
        m = m,
        table = e.table(),
        cols = create_cols.join(", "),
        ph = placeholders.join(", "),
        binds_block = binds
            .iter()
            .map(|b| format!("        .bind({})", b))
            .collect::<Vec<_>>()
            .join("\n"),
        update_block = rs_update_block(e),
        validation_block = rs_validation(e),
        sample_block = rs_sample(e),
    )
}

fn rs_update_block(e: &Entity) -> String {
    let mut s = String::new();
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = rs_ident(&f.name);
        s.push_str(&format!(
            "    if let Some(v) = payload.{} {{\n        sqlx::query(\"UPDATE {table} SET {n} = ?, updated_at = ? WHERE id = ?\")\n            .bind(v).bind(chrono::Utc::now()).bind(&id)\n            .execute(&db).await?;\n    }}\n",
            n,
            table = e.table(),
            n = n
        ));
    }
    s
}

fn rs_validation(e: &Entity) -> String {
    let mut s = String::new();
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = rs_ident(&f.name);
        if f.required && f.ty == FieldType::Text {
            s.push_str(&format!(
                "    if payload.{n}.trim().is_empty() {{\n        return Err(AppError::Validation(\"{n} ni lazima\".into()));\n    }}\n",
                n = n
            ));
        }
    }
    s
}

fn rs_sample(e: &Entity) -> String {
    let mut s = String::new();
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = rs_ident(&f.name);
        let val = match f.ty {
            FieldType::Int => "1".to_string(),
            FieldType::Float => "1.5".to_string(),
            FieldType::Bool => "true".to_string(),
            FieldType::Timestamp => "chrono::Utc::now()".to_string(),
            FieldType::Text | FieldType::Email | FieldType::Uuid => "\"sample\".to_string()".to_string(),
        };
        let val = if f.required { val } else { format!("Some({})", val) };
        s.push_str(&format!("            {}: {},\n", n, val));
    }
    s
}

fn rs_main(domain: &Domain) -> String {
    let crate_name = to_snake(&domain.project);
    let mut s = String::from(
        "//! Server entrypoint (production).\nuse axum::Router;\nuse tower_http::trace::TraceLayer;\n\n",
    );
    s.push_str(&format!("use {}::db;\n", crate_name));
    for e in &domain.entities {
        s.push_str(&format!("use {}::{};\n", crate_name, e.name));
    }
    s.push_str("\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n");
    s.push_str("    tracing_subscriber::fmt::init();\n    let url = std::env::var(\"DATABASE_URL\").unwrap_or_else(|_| \"sqlite:mradi.db?mode=rwc\".to_string());\n    let pool = db::connect(&url).await?;\n    db::migrate(&pool).await?;\n\n    let app = Router::new()\n        .route(\"/health\", axum::routing::get(health))\n");
    for e in &domain.entities {
        s.push_str(&format!("        .merge({}::router())\n", e.name));
    }
    s.push_str("        .layer(TraceLayer::new_for_http())\n        .with_state(pool);\n\n    let addr = std::env::var(\"BIND_ADDR\").unwrap_or_else(|_| \"0.0.0.0:8080\".to_string());\n    let listener = tokio::net::TcpListener::bind(&addr).await?;\n    tracing::info!(\"listening on {addr}\");\n    axum::serve(listener, app).await?;\n    Ok(())\n}\n\nasync fn health() -> axum::Json<serde_json::Value> {\n    axum::Json(serde_json::json!({ \"ok\": true }))\n}\n");
    s
}

fn rs_migrations(domain: &Domain) -> String {
    let mut s = String::from("-- Initial schema (production).\n");
    for e in &domain.entities {
        s.push_str(&format!("CREATE TABLE IF NOT EXISTS {} (\n    id TEXT PRIMARY KEY,\n", e.table()));
        for f in &e.fields {
            if is_managed(&f.name) {
                continue;
            }
            let n = rs_ident(&f.name);
            let nn = if f.required { " NOT NULL" } else { "" };
            s.push_str(&format!("    {} {}{},\n", n, f.ty.sql(), nn));
        }
        s.push_str("    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,\n    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP\n);\n\n");
    }
    s
}

/// Je, kind hii ina generator ya PRODUCTION (domain-driven)?
pub fn is_production(kind: &str) -> bool {
    matches!(kind, "rust-api" | "node-api")
}

// ─────────────────────────────────────────────────────────────────────────────
// NODE / Express (node:sqlite built-in + node:test) — production, bila deps
// ─────────────────────────────────────────────────────────────────────────────

/// Aina ya JS column ya node:sqlite (dynamic typing).
fn js_sample(t: FieldType) -> &'static str {
    match t {
        FieldType::Int => "1",
        FieldType::Float => "1.5",
        FieldType::Bool => "true",
        FieldType::Timestamp => "new Date().toISOString()",
        FieldType::Text | FieldType::Email | FieldType::Uuid => "'sample'",
    }
}

fn js_payload(e: &Entity) -> String {
    let mut s = String::from("{\n");
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        s.push_str(&format!("      {}: {},\n", to_snake(&f.name), js_sample(f.ty)));
    }
    s.push_str("    }");
    s
}

fn js_store(e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let mut cols = vec!["id".to_string()];
    let mut binds = vec!["id".to_string()];
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = to_snake(&f.name);
        cols.push(n.clone());
        binds.push(format!("data.{}", n));
    }
    cols.push("created_at".into());
    cols.push("updated_at".into());
    binds.push("now".into());
    binds.push("now".into());
    let cols_sql = cols.join(", ");
    let ph = cols.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    format!(
        r#"import {{ randomUUID }} from "node:crypto";

// Store ya {cls} (production, node:sqlite).
export function create{cls}Store(db) {{
  const insert = db.prepare("INSERT INTO {table} ({cols_sql}) VALUES ({ph})");
  const byId = db.prepare("SELECT * FROM {table} WHERE id = ?");
  const count = db.prepare("SELECT COUNT(*) AS total FROM {table}");
  const page = db.prepare("SELECT * FROM {table} ORDER BY created_at LIMIT ? OFFSET ?");
  const remove = db.prepare("DELETE FROM {table} WHERE id = ?");
  const setCol = (col) => db.prepare(`UPDATE {table} SET ${{col}} = ?, updated_at = ? WHERE id = ?`);

  return {{
    create(data) {{
      validate(data);
      const id = randomUUID();
      const now = new Date().toISOString();
      insert.run({binds});
      return byId.get(id);
    }},
    get(id) {{
      return byId.get(id);
    }},
    list({{ offset = 0, limit = 50 }} = {{}}) {{
      const lim = Math.min(Math.max(limit, 1), 200);
      return {{ items: page.all(lim, Math.max(offset, 0)), total: Number(count.get().total) }};
    }},
    update(id, data) {{
      if (!byId.get(id)) return null;
      const now = new Date().toISOString();
      for (const [col, value] of Object.entries(data)) {{
        if (value === undefined) continue;
        setCol(col).run(value, now, id);
      }}
      return byId.get(id);
    }},
    remove(id) {{
      return remove.run(id).changes > 0;
    }},
  }};
}}

export function validate(data) {{
  if (!data || typeof data !== "object") {{
    throw Object.assign(new Error("data si sahihi"), {{ status: 422 }});
  }}
{validation_block}}}

export function schema() {{
  return `CREATE TABLE IF NOT EXISTS {table} (
    id TEXT PRIMARY KEY,
    {schema_cols}
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
  );`;
}}
"#,
        cls = cls,
        table = e.table(),
        cols_sql = cols_sql,
        ph = ph,
        binds = binds.join(", "),
        validation_block = js_validation(e),
        schema_cols = js_schema_cols(e),
    )
}

fn js_schema_cols(e: &Entity) -> String {
    let mut s = String::new();
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = to_snake(&f.name);
        let nn = if f.required { " NOT NULL" } else { "" };
        s.push_str(&format!("    {} TEXT{},\n", n, nn));
    }
    s
}

fn js_validation(e: &Entity) -> String {
    let mut s = String::new();
    for f in &e.fields {
        if is_managed(&f.name) {
            continue;
        }
        let n = to_snake(&f.name);
        if f.required && f.ty == FieldType::Text {
            s.push_str(&format!(
                "  if (typeof data.{n} !== \"string\" || data.{n}.trim() === \"\") {{\n    throw Object.assign(new Error(\"{n} ni lazima\"), {{ status: 422 }});\n  }}\n",
                n = n
            ));
        }
    }
    s
}

fn js_routes(e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let m = &e.name;
    format!(
        r#"import {{ Router }} from "express";
import {{ create{cls}Store }} from "./{m}.store.js";

export function {m}Router(db) {{
  const store = create{cls}Store(db);
  const router = Router();

  router.post("/", (req, res) => {{
    try {{
      res.status(201).json(store.create(req.body));
    }} catch (err) {{
      res.status(err.status || 500).json({{ error: err.message }});
    }}
  }});

  router.get("/", (req, res) => {{
    const offset = Number(req.query.offset ?? 0);
    const limit = Number(req.query.limit ?? 50);
    res.json(store.list({{ offset, limit }}));
  }});

  router.get("/:id", (req, res) => {{
    const row = store.get(req.params.id);
    if (!row) return res.status(404).json({{ error: "{m} haipo" }});
    res.json(row);
  }});

  router.put("/:id", (req, res) => {{
    const row = store.update(req.params.id, req.body ?? {{}});
    if (!row) return res.status(404).json({{ error: "{m} haipo" }});
    res.json(row);
  }});

  router.delete("/:id", (req, res) => {{
    if (!store.remove(req.params.id)) return res.status(404).json({{ error: "{m} haipo" }});
    res.status(204).end();
  }});

  return router;
}}
"#,
        cls = cls,
        m = m
    )
}

fn js_app(domain: &Domain) -> String {
    let mut s = String::from(
        "// Express app (production).\nimport express from \"express\";\nimport { DatabaseSync } from \"node:sqlite\";\n",
    );
    for e in &domain.entities {
        s.push_str(&format!(
            "import {{ {}Router }} from \"./{}.routes.js\";\nimport {{ schema as {}Schema }} from \"./{}.store.js\";\n",
            e.name, e.name, e.name, e.name
        ));
    }
    s.push_str("\nexport function createApp(db = new DatabaseSync(\":memory:\")) {\n");
    for e in &domain.entities {
        s.push_str(&format!("  db.exec({}Schema());\n", e.name));
    }
    s.push_str("\n  const app = express();\n  app.use(express.json());\n\n  app.get(\"/health\", (_req, res) => res.json({ ok: true }));\n");
    for e in &domain.entities {
        s.push_str(&format!("  app.use(\"/{}\", {}Router(db));\n", e.name, e.name));
    }
    s.push_str("\n  app.use((err, _req, res, _next) => {\n    res.status(err.status || 500).json({ error: err.message });\n  });\n\n  return { app, db };\n}\n\nif (import.meta.url === `file://${process.argv[1]}`) {\n  const port = Number(process.env.PORT ?? 3000);\n  const { app } = createApp(new DatabaseSync(process.env.DB_PATH ?? \":memory:\"));\n  app.listen(port, () => console.error(`listening on ${port}`));\n}\n");
    s
}

fn js_test(e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let m = &e.name;
    format!(
        r#"import {{ test }} from "node:test";
import assert from "node:assert/strict";
import {{ createApp }} from "./src/app.js";

function withApp() {{
  const {{ app }} = createApp();
  const server = app.listen(0);
  const port = server.address().port;
  return {{ server, base: `http://127.0.0.1:${{port}}` }};
}}

const payload = {payload};

test("health inajibu ok", async () => {{
  const {{ server, base }} = withApp();
  const res = await fetch(`${{base}}/health`);
  assert.equal(res.status, 200);
  assert.equal((await res.json()).ok, true);
  server.close();
}});

test("create + get", async () => {{
  const {{ server, base }} = withApp();
  const created = await fetch(`${{base}}/{m}`, {{
    method: "POST",
    headers: {{ "content-type": "application/json" }},
    body: JSON.stringify(payload),
  }});
  assert.equal(created.status, 201);
  const body = await created.json();
  const got = await fetch(`${{base}}/{m}/${{body.id}}`);
  assert.equal(got.status, 200);
  assert.equal((await got.json()).id, body.id);
  server.close();
}});

test("pagination", async () => {{
  const {{ server, base }} = withApp();
  for (let i = 0; i < 3; i++) {{
    await fetch(`${{base}}/{m}`, {{
      method: "POST",
      headers: {{ "content-type": "application/json" }},
      body: JSON.stringify(payload),
    }});
  }}
  const res = await fetch(`${{base}}/{m}?offset=0&limit=2`);
  const data = await res.json();
  assert.equal(data.items.length, 2);
  assert.ok(data.total >= 3);
  server.close();
}});

test("delete + 404", async () => {{
  const {{ server, base }} = withApp();
  const created = await fetch(`${{base}}/{m}`, {{
    method: "POST",
    headers: {{ "content-type": "application/json" }},
    body: JSON.stringify(payload),
  }});
  const body = await created.json();
  const del = await fetch(`${{base}}/{m}/${{body.id}}`, {{ method: "DELETE" }});
  assert.equal(del.status, 204);
  const missing = await fetch(`${{base}}/{m}/${{body.id}}`);
  assert.equal(missing.status, 404);
  server.close();
}});

test("validation inakataa body tupu", async () => {{
  const {{ server, base }} = withApp();
  const res = await fetch(`${{base}}/{m}`, {{
    method: "POST",
    headers: {{ "content-type": "application/json" }},
    body: JSON.stringify({{}}),
  }});
  assert.equal(res.status, 422);
  server.close();
}});
"#,
        m = m,
        payload = js_payload(e)
    )
}

pub fn generate_node(domain: &Domain) -> (Vec<(String, String)>, String) {
    let mut files: Vec<(String, String)> = Vec::new();
    files.push(("package.json".into(), node_package_json(domain)));
    files.push(("src/app.js".into(), js_app(domain)));
    for e in &domain.entities {
        files.push((format!("src/{}.store.js", e.name), js_store(e)));
        files.push((format!("src/{}.routes.js", e.name), js_routes(e)));
        files.push((format!("{}.test.js", e.name), js_test(e)));
    }
    (files, format!("node-api (express+node:sqlite) entities {} (production)", domain.entities.len()))
}

fn node_package_json(domain: &Domain) -> String {
    format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "engines": {{ "node": ">=22.5" }},
  "dependencies": {{
    "express": "^4.19.2"
  }},
  "scripts": {{
    "start": "node src/app.js",
    "test": "node --test"
  }}
}}
"#,
        to_snake(&domain.project)
    )
}

/// Zalisha kodi ya PRODUCTION kutoka domain (files, note). `None` kama kind haina.
pub fn generate_production(kind: &str, domain: &Domain) -> Option<(Vec<(String, String)>, String)> {
    match kind {
        "rust-api" => Some(generate_rust(domain)),
        "node-api" => Some(generate_node(domain)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge_domain::parse_domain;

    fn write_project(files: &[(String, String)], dir: &std::path::Path) {
        for (path, content) in files {
            let full = dir.join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&full, content).unwrap();
        }
    }

    #[test]
    fn rust_production_inazalisha_nyaraka_zote() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, note) = generate_rust(&domain);
        assert!(note.contains("production"));
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        for want in [
            "src/lib.rs", "src/db.rs", "src/error.rs", "src/models.rs", "src/bidhaa.rs",
            "src/main.rs", "Cargo.toml", "src/schema.sql",
        ] {
            assert!(paths.contains(&want), "haipo: {want}");
        }
        let module = files.iter().find(|(p, _)| p == "src/bidhaa.rs").unwrap();
        assert!(module.1.contains("sqlx::query(\"INSERT INTO bidhaas"));
        assert!(module.1.contains("#[tokio::test]"));
        assert!(module.1.contains("AppError::NotFound"));
        assert!(module.1.contains("LIMIT ? OFFSET ?"));
        let mig = files.iter().find(|(p, _)| p == "src/schema.sql").unwrap();
        assert!(mig.1.contains("CREATE TABLE IF NOT EXISTS bidhaas"));
    }

    /// Uthibitisho HALISI: mradi wa Rust unacompile + testi zinapita.
    /// Inarukwa kwa default (inahitaji network kwa crates); endesha kwa --ignored.
    #[test]
    #[ignore = "inahitaji cargo + network; endesha kwa --ignored"]
    fn rust_production_unacompile_na_kupita_tests() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, _) = generate_rust(&domain);
        let dir = std::env::temp_dir().join(format!("forge_rs_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_project(&files, &dir);
        let out = std::process::Command::new("cargo")
            .args(["test", "--quiet"])
            .current_dir(&dir)
            .output()
            .expect("cargo haipo");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        println!("CARGO STDOUT:\n{stdout}\nCARGO STDERR:\n{stderr}");
        assert!(out.status.success(), "cargo test imeshindwa");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn production_hupita_linter_ya_repo() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi), Wateja(email, jina)");
        let mut violations = 0;
        for kind in ["rust-api", "node-api"] {
            let (files, _) = generate_production(kind, &domain).unwrap();
            for (p, c) in &files {
                let lang = crate::code_audit::lang_for(p);
                let v = crate::code_audit::analyze_content(lang, c);
                if !v.is_empty() {
                    violations += v.len();
                    println!("VIOLATION kind={kind} path={p} lang={lang} -> {v:?}");
                }
            }
        }
        assert_eq!(violations, 0, "kodi ya production inavunja kanuni za repo");
    }

    #[test]
    fn production_inashughulikia_entities_nyingi() {
        let domain = parse_domain("mradi wa Shule: Wanafunzi(jina, umri), Walimu(jina, email)");
        let (rs, _) = generate_rust(&domain);
        let rpaths: Vec<&str> = rs.iter().map(|(p, _)| p.as_str()).collect();
        assert!(rpaths.contains(&"src/wanafunzi.rs"));
        assert!(rpaths.contains(&"src/walimu.rs"));
    }

    #[test]
    fn node_production_inazalisha_nyaraka_zote() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, note) = generate_node(&domain);
        assert!(note.contains("production"));
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        for want in [
            "package.json", "src/app.js", "src/bidhaa.store.js", "src/bidhaa.routes.js",
            "bidhaa.test.js",
        ] {
            assert!(paths.contains(&want), "haipo: {want}");
        }
        let store = files.iter().find(|(p, _)| p == "src/bidhaa.store.js").unwrap();
        assert!(store.1.contains("INSERT INTO bidhaas"));
        assert!(store.1.contains("node:crypto"));
        let app = files.iter().find(|(p, _)| p == "src/app.js").unwrap();
        assert!(app.1.contains("node:sqlite"));
    }

    /// Uthibitisho HALISI: mradi wa Node unapita `node --test` (bila npm install —
    /// store ni node:sqlite built-in; app.js inahitaji express).
    #[test]
    #[ignore = "inahitaji node + npm install express; endesha kwa --ignored"]
    fn node_production_unapita_node_test() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, _) = generate_node(&domain);
        let dir = std::env::temp_dir().join(format!("forge_node_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_project(&files, &dir);
        let install = std::process::Command::new("npm")
            .args(["install", "--no-audit", "--no-fund", "--silent"])
            .current_dir(&dir)
            .output()
            .expect("npm haipo");
        assert!(install.status.success(), "npm install imeshindwa");
        let out = std::process::Command::new("node")
            .args(["--test"])
            .current_dir(&dir)
            .output()
            .expect("node haipo");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        println!("NODE STDOUT:\n{stdout}\nNODE STDERR:\n{stderr}");
        assert!(out.status.success(), "node --test imeshindwa");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
