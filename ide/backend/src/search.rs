//! search.rs — UTAFUTAJI halisi wa ndani (local-first).
//!
//! Badala ya Elasticsearch (server ya nje, RAM kubwa, si local-first), MTECHOS
//! hutumia **SQLite FTS5** — injini ya utafutaji iliyojengwa ndani ya SQLite.
//! Faida:
//!   - Hakuna server ya nje, hakuna port, hakuna RAM ya ziada.
//!   - Data haitoki kwenye mashine (local-first, kama kanuni ya MTECHOS).
//!   - Inasaidia Kiswahili kwa `unicode61` tokenizer + `remove_diacritics`.
//!   - BM25 ranking halisi (kama Elasticsearch) + snippet/highlight.
//!
//! Tuna index: code, notes, docs, na maudhui yoyote ya mtumiaji. Kila kitu
//! hukaa kwenye DB moja ya SQLite — inayosafiri na mtumiaji.

use crate::AppState;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/search/status", get(status))
        .route("/api/search/query", get(query))
        .route("/api/search/index", post(index_doc))
        .route("/api/search/index/bulk", post(index_bulk))
        .route("/api/search/doc/:id/remove", post(remove_doc))
        .route("/api/search/reindex", post(reindex))
}

/// Jedwali la FTS5 lenye `external content` (huhifadhi content mara moja pekee).
/// `search_docs` ni chanzo; `search_fts` ni index.
pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS search_docs (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL DEFAULT 'note',
            title TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            path TEXT NOT NULL DEFAULT '',
            lang TEXT NOT NULL DEFAULT 'sw',
            meta TEXT NOT NULL DEFAULT '{}',
            updated_at TEXT NOT NULL
        )"#,
        // FTS5: unicode61 + diacritics off (Kiswahili haina diacritics, lakini
        // hii hufanya maneno ya kigeni yalingane). `content=` inaifanya kuwa
        // external-content table → hakuna kunakili data maradufu.
        r#"CREATE VIRTUAL TABLE IF NOT EXISTS search_fts USING fts5(
            title, body, path,
            content='search_docs',
            content_rowid='rowid',
            tokenize="unicode61 remove_diacritics 2"
        )"#,
        // Triggers: hufanya index kuwa sambamba na jedwali la chanzo.
        r#"CREATE TRIGGER IF NOT EXISTS search_docs_ai AFTER INSERT ON search_docs BEGIN
            INSERT INTO search_fts(rowid, title, body, path)
            VALUES (new.rowid, new.title, new.body, new.path);
        END"#,
        r#"CREATE TRIGGER IF NOT EXISTS search_docs_ad AFTER DELETE ON search_docs BEGIN
            INSERT INTO search_fts(search_fts, rowid, title, body, path)
            VALUES ('delete', old.rowid, old.title, old.body, old.path);
        END"#,
        r#"CREATE TRIGGER IF NOT EXISTS search_docs_au AFTER UPDATE ON search_docs BEGIN
            INSERT INTO search_fts(search_fts, rowid, title, body, path)
            VALUES ('delete', old.rowid, old.title, old.body, old.path);
            INSERT INTO search_fts(rowid, title, body, path)
            VALUES (new.rowid, new.title, new.body, new.path);
        END"#,
    ];
    for d in ddl {
        if let Err(e) = sqlx::query(d).execute(db).await {
            tracing::warn!("search FTS5 DDL: {e}");
        }
    }
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let fts5: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='search_fts'",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or(0);
    let docs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM search_docs")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    let indexed: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM search_fts")
        .fetch_one(&s.db)
        .await
        .unwrap_or(0);
    Json(json!({
        "ok": true,
        "module": "search",
        "engine": "SQLite FTS5 (BM25)",
        "fts5_ready": fts5 > 0,
        "docs": docs,
        "indexed": indexed,
        "features": ["bm25_ranking", "snippet", "highlight", "prefix", "phrase", "boolean"],
        "kinds": ["code", "note", "doc", "lesson", "transcript"],
        "privacy": "index hukaa kwenye SQLite ya mtumiaji — hakuna server ya nje",
        "note": "Hii inachukua nafasi ya Elasticsearch: local-first, hakuna RAM ya ziada.",
    }))
}

#[derive(Deserialize)]
struct QueryIn {
    q: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    /// `1` → tumia FTS5 query syntax (phrase, AND/OR, prefix*).
    #[serde(default)]
    raw: Option<bool>,
}

/// Tafuta kwa BM25. Hukurudisha snippet yenye alama za `<b>`.
async fn query(State(s): State<AppState>, Query(p): Query<QueryIn>) -> Json<Value> {
    let q = p.q.trim();
    if q.is_empty() {
        return Json(json!({"ok": false, "error": "q inahitajika"}));
    }
    let limit = p.limit.unwrap_or(20).clamp(1, 100);
    let raw = p.raw.unwrap_or(false);
    // Maneno ya Kiswahili yana herufi za kawaida; kwa "raw" tunaacha syntax.
    let match_expr = if raw { q.to_string() } else { safe_match(q) };
    if match_expr.is_empty() {
        return Json(json!({"ok": false, "error": "swali tupu baada ya kusafisha"}));
    }

    let kind_filter = p.kind.clone().filter(|k| !k.trim().is_empty());
    let sql = if kind_filter.is_some() {
        "SELECT d.id, d.kind, d.title, d.path, d.lang, d.updated_at,
                snippet(search_fts, 1, '<b>', '</b>', '…', 20) AS snip,
                bm25(search_fts, 8.0, 1.0, 2.0) AS rank
         FROM search_fts f
         JOIN search_docs d ON d.rowid = f.rowid
         WHERE search_fts MATCH ?1 AND d.kind = ?2
         ORDER BY rank LIMIT ?3"
    } else {
        "SELECT d.id, d.kind, d.title, d.path, d.lang, d.updated_at,
                snippet(search_fts, 1, '<b>', '</b>', '…', 20) AS snip,
                bm25(search_fts, 8.0, 1.0, 2.0) AS rank
         FROM search_fts f
         JOIN search_docs d ON d.rowid = f.rowid
         WHERE search_fts MATCH ?1
         ORDER BY rank LIMIT ?2"
    };

    let rows = if let Some(kind) = kind_filter {
        sqlx::query(sql)
            .bind(&match_expr)
            .bind(&kind)
            .bind(limit as i64)
            .fetch_all(&s.db)
            .await
    } else {
        sqlx::query(sql)
            .bind(&match_expr)
            .bind(limit as i64)
            .fetch_all(&s.db)
            .await
    };

    let rows = match rows {
        Ok(r) => r,
        Err(e) => {
            return Json(json!({
                "ok": false,
                "error": format!("FTS5 query imeshindwa: {e}"),
                "match": match_expr,
            }))
        }
    };

    let hits: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<String, _>("id"),
                "kind": r.get::<String, _>("kind"),
                "title": r.get::<String, _>("title"),
                "path": r.get::<String, _>("path"),
                "lang": r.get::<String, _>("lang"),
                "updated_at": r.get::<String, _>("updated_at"),
                "snippet": r.get::<String, _>("snip"),
                "score": r.get::<f64, _>("rank"),
            })
        })
        .collect();

    Json(json!({
        "ok": true,
        "engine": "SQLite FTS5 (BM25)",
        "match": match_expr,
        "count": hits.len(),
        "hits": hits,
    }))
}

/// Husafisha swali la mtumiaji → FTS5 query salama: kila neno limewekwa kwenye
/// quotes (kuzuia sintaksia mbaya), maneno yote yanahitajika (AND ya FTS5 ni
/// nafasi). Hii hufanya "habari za usalama" → `"habari" "za" "usalama"`.
fn safe_match(q: &str) -> String {
    let cleaned: String = q
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() || c == '_' { c } else { ' ' })
        .collect();
    let terms: Vec<String> = cleaned
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t.to_lowercase()))
        .collect();
    terms.join(" ")
}

#[derive(Deserialize)]
struct DocIn {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    meta: Option<Value>,
}

async fn index_doc(State(s): State<AppState>, Json(b): Json<DocIn>) -> Json<Value> {
    if b.body.trim().is_empty() && b.title.as_deref().unwrap_or("").trim().is_empty() {
        return Json(json!({"ok": false, "error": "title au body inahitajika"}));
    }
    let id = b.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let kind = b.kind.clone().unwrap_or_else(|| "note".into());
    let title = b.title.clone().unwrap_or_default();
    let path = b.path.clone().unwrap_or_default();
    let lang = b.lang.clone().unwrap_or_else(|| "sw".into());
    let meta = b.meta.clone().unwrap_or_else(|| json!({})).to_string();

    let res = sqlx::query(
        "INSERT INTO search_docs (id, kind, title, body, path, lang, meta, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(id) DO UPDATE SET
            kind=excluded.kind, title=excluded.title, body=excluded.body,
            path=excluded.path, lang=excluded.lang, meta=excluded.meta,
            updated_at=excluded.updated_at",
    )
    .bind(&id)
    .bind(&kind)
    .bind(&title)
    .bind(&b.body)
    .bind(&path)
    .bind(&lang)
    .bind(&meta)
    .bind(now())
    .execute(&s.db)
    .await;

    match res {
        Ok(_) => Json(json!({"ok": true, "id": id, "kind": kind, "indexed": true})),
        Err(e) => Json(json!({"ok": false, "error": format!("kuweka index: {e}")})),
    }
}

#[derive(Deserialize)]
struct BulkIn {
    docs: Vec<DocIn>,
}

async fn index_bulk(State(s): State<AppState>, Json(b): Json<BulkIn>) -> Json<Value> {
    let mut ok = 0usize;
    let mut failed = 0usize;
    for d in b.docs {
        let id = d.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let res = sqlx::query(
            "INSERT INTO search_docs (id, kind, title, body, path, lang, meta, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET
                kind=excluded.kind, title=excluded.title, body=excluded.body,
                path=excluded.path, lang=excluded.lang, meta=excluded.meta,
                updated_at=excluded.updated_at",
        )
        .bind(&id)
        .bind(d.kind.unwrap_or_else(|| "note".into()))
        .bind(d.title.unwrap_or_default())
        .bind(&d.body)
        .bind(d.path.unwrap_or_default())
        .bind(d.lang.unwrap_or_else(|| "sw".into()))
        .bind(d.meta.unwrap_or_else(|| json!({})).to_string())
        .bind(now())
        .execute(&s.db)
        .await;
        if res.is_ok() {
            ok += 1;
        } else {
            failed += 1;
        }
    }
    Json(json!({"ok": true, "indexed": ok, "failed": failed}))
}

async fn remove_doc(State(s): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    let n = sqlx::query("DELETE FROM search_docs WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);
    Json(json!({"ok": n > 0, "removed": n, "id": id}))
}

/// Hujenga upya index yote (kwa `INSERT INTO search_fts(search_fts) VALUES('rebuild')`).
async fn reindex(State(s): State<AppState>) -> Json<Value> {
    match sqlx::query("INSERT INTO search_fts(search_fts) VALUES('rebuild')")
        .execute(&s.db)
        .await
    {
        Ok(_) => {
            let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM search_fts")
                .fetch_one(&s.db)
                .await
                .unwrap_or(0);
            Json(json!({"ok": true, "indexed": n}))
        }
        Err(e) => Json(json!({"ok": false, "error": format!("rebuild: {e}")})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_match_huweka_quotes() {
        assert_eq!(safe_match("habari za usalama"), "\"habari\" \"za\" \"usalama\"");
    }

    #[test]
    fn safe_match_huondoa_sintaksia_mbaya() {
        // Alama za FTS5 (", *, NEAR, -) hazipaswi kupita.
        assert_eq!(safe_match("usala*ma \"x\""), "\"usala\" \"ma\" \"x\"");
        assert_eq!(safe_match("a OR b"), "\"a\" \"or\" \"b\"");
    }

    #[test]
    fn safe_match_huheshimu_underscore() {
        assert_eq!(safe_match("fn main_loop"), "\"fn\" \"main_loop\"");
    }

    #[test]
    fn safe_match_swali_tupu() {
        assert_eq!(safe_match("   ***   "), "");
        assert_eq!(safe_match(""), "");
    }

    #[test]
    fn safe_match_lowercase() {
        assert_eq!(safe_match("MAMBO"), "\"mambo\"");
    }
}
