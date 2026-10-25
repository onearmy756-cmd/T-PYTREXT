//! ittools.rs — ELIMUPRO Awamu 16: zana za IT (moduli 16).
//!
//! Wanafunzi wa IT hupata mazingira halisi ndani ya app:
//!   - Code editor + terminal: hukimbiza code KWELI (`ide_code_run::run_code`).
//!   - Cyber security lab: mazoezi ya usalama yenye jibu linalothibitishwa.
//!   - Database tools: SQL ya kusoma (SELECT) kwenye database ya mazoezi.
//!   - Ufuatiliaji: mwalimu huona vikao hai vya wanafunzi.
//!
//! Kanuni halisi:
//!   - Hakuna kubuni: code hukimbizwa, SQL hutekelezwa, jibu la lab hulinganishwa.
//!   - SQL ya mazoezi ni READ-ONLY: statement moja ya SELECT/WITH pekee;
//!     maneno hatari (DROP/DELETE/UPDATE/INSERT/ATTACH/PRAGMA) hukataliwa.
//!   - Vikao vya wanafunzi hufungwa kwa `end`; mwalimu huona vilivyo hai.
//!
//! Tables: ep_it_sessions ep_it_submissions ep_it_labs ep_it_lab_attempts

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_it_sessions (
            id TEXT PRIMARY KEY,
            student_id TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'editor', -- editor|terminal|db|cyber
            language TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'active', -- active|ended
            started_at TEXT NOT NULL,
            ended_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_it_submissions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL DEFAULT '',
            student_id TEXT NOT NULL,
            language TEXT NOT NULL DEFAULT '',
            code TEXT NOT NULL DEFAULT '',
            exit_code INTEGER NOT NULL DEFAULT 0,
            stdout TEXT NOT NULL DEFAULT '',
            stderr TEXT NOT NULL DEFAULT '',
            ok INTEGER NOT NULL DEFAULT 0,
            score INTEGER NOT NULL DEFAULT 0,
            feedback TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_it_labs (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT '',
            category TEXT NOT NULL DEFAULT 'cyber', -- cyber|network|forensics|crypto
            brief TEXT NOT NULL DEFAULT '',
            expected TEXT NOT NULL DEFAULT '',
            points INTEGER NOT NULL DEFAULT 10,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_it_lab_attempts (
            id TEXT PRIMARY KEY,
            lab_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            answer TEXT NOT NULL DEFAULT '',
            correct INTEGER NOT NULL DEFAULT 0,
            points INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ittools/status", get(status))
        .route("/api/ittools/session/start", post(session_start))
        .route("/api/ittools/sessions", get(sessions_list))
        .route("/api/ittools/session/:id/end", post(session_end))
        .route("/api/ittools/run", post(run))
        .route("/api/ittools/submit", post(submit))
        .route("/api/ittools/submissions", get(submissions_list))
        .route("/api/ittools/lab/add", post(lab_add))
        .route("/api/ittools/labs", get(labs_list))
        .route("/api/ittools/lab/attempt", post(lab_attempt))
        .route("/api/ittools/lab/attempts", get(lab_attempts))
        .route("/api/ittools/db/query", post(db_query))
        .route("/api/ittools/db/schema", get(db_schema))
        .route("/api/ittools/monitor", get(monitor))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

/// Aina za kikao zinazokubalika.
pub fn norm_kind(k: &str) -> &'static str {
    match k.trim().to_lowercase().as_str() {
        "terminal" => "terminal",
        "db" | "database" | "sql" => "db",
        "cyber" | "security" | "usalama" => "cyber",
        _ => "editor",
    }
}

/// Lugha zinazokimbizwa na zana hii.
pub const LANGS: [&str; 8] = ["rust", "node", "ts", "bash", "go", "elixir", "python", "r"];

pub fn lang_ok(l: &str) -> bool {
    let l = l.trim().to_ascii_lowercase();
    LANGS.contains(&l.as_str())
}

/// SQL ya mazoezi ni salama? (statement MOJA ya SELECT/WITH pekee.)
pub fn sql_is_readonly(q: &str) -> Result<(), String> {
    let t = q.trim().trim_end_matches(';').trim();
    if t.is_empty() {
        return Err("swali ni tupu".into());
    }
    // Statement moja pekee — hakuna `;` katikati.
    if t.contains(';') {
        return Err("statement moja pekee inaruhusiwa".into());
    }
    let upper = t.to_ascii_uppercase();
    let first = upper.split_whitespace().next().unwrap_or("");
    if first != "SELECT" && first != "WITH" {
        return Err("SELECT/WITH pekee inaruhusiwa".into());
    }
    for bad in [
        "DROP", "DELETE", "UPDATE", "INSERT", "ATTACH", "DETACH", "PRAGMA", "ALTER", "CREATE",
        "REPLACE", "VACUUM", "REINDEX",
    ] {
        if upper.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == bad) {
            return Err(format!("neno '{bad}' haliruhusiwa"));
        }
    }
    Ok(())
}

/// Hali ya kikao kutokana na hali iliyohifadhiwa.
pub fn session_live(status: &str) -> bool {
    status == "active"
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (active,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_it_sessions WHERE status='active'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (subs,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_it_submissions")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (passed,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_it_submissions WHERE ok=1")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "languages": LANGS,
        "sessions_active": active,
        "sessions": table_count(&s.db, "ep_it_sessions").await,
        "submissions": subs,
        "submissions_ok": passed,
        "labs": table_count(&s.db, "ep_it_labs").await,
        "attempts": table_count(&s.db, "ep_it_lab_attempts").await,
        "tools": {
            "code_editor": true,
            "terminal": true,
            "cyber_lab": true,
            "database": true,
            "monitor": true
        }
    }))
}

#[derive(Deserialize)]
pub struct SessionIn {
    pub student_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub language: String,
}

pub async fn session_start(State(s): State<AppState>, Json(b): Json<SessionIn>) -> Json<Value> {
    if b.student_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "student_id inahitajika" }));
    }
    let kind = norm_kind(&b.kind);
    if kind == "editor" && !b.language.trim().is_empty() && !lang_ok(&b.language) {
        return Json(json!({
            "ok": false,
            "error": format!("lugha '{}' haitumiki", b.language),
            "languages": LANGS
        }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_it_sessions (id,student_id,kind,language,status,started_at)
         VALUES (?1,?2,?3,?4,'active',?5)",
    )
    .bind(&id)
    .bind(b.student_id.trim())
    .bind(kind)
    .bind(&b.language)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "kind": kind, "status": "active" }))
}

#[derive(Deserialize)]
pub struct ItQuery {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub status: String,
}

pub async fn sessions_list(State(s): State<AppState>, Query(q): Query<ItQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>)>(
        "SELECT id,student_id,kind,language,status,started_at,ended_at FROM ep_it_sessions
         WHERE (?1='' OR student_id=?1) AND (?2='' OR kind=?2) AND (?3='' OR status=?3)
         ORDER BY started_at DESC LIMIT 200",
    )
    .bind(&q.student_id)
    .bind(&q.kind)
    .bind(&q.status)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "sessions": rows.into_iter().map(|r| json!({
            "id": r.0, "student_id": r.1, "kind": r.2, "language": r.3,
            "status": r.4, "started_at": r.5, "ended_at": r.6, "live": session_live(&r.4)
        })).collect::<Vec<_>>()
    }))
}

pub async fn session_end(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query("UPDATE ep_it_sessions SET status='ended',ended_at=?1 WHERE id=?2 AND status='active'")
        .bind(now())
        .bind(&id)
        .execute(&s.db)
        .await;
    match r {
        Ok(x) if x.rows_affected() > 0 => Json(json!({ "ok": true, "status": "ended" })),
        _ => Json(json!({ "ok": false, "error": "kikao hakipo au kimefungwa" })),
    }
}

#[derive(Deserialize)]
pub struct RunIn {
    pub code: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub student_id: String,
}

/// Hukimbiza code KWELI (terminal ya mwanafunzi). Huhifadhi kama ameomba.
pub async fn run(State(s): State<AppState>, Json(b): Json<RunIn>) -> Json<Value> {
    if b.code.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "code ni tupu" }));
    }
    let lang = if b.language.trim().is_empty() {
        "python"
    } else {
        b.language.trim()
    };
    if !lang_ok(lang) {
        return Json(json!({
            "ok": false, "error": format!("lugha '{lang}' haitumiki"), "languages": LANGS
        }));
    }
    let res = crate::ide_code_run::run_code(&b.code, lang, None).await;
    // Hifadhi kama kikao kimetolewa.
    if !b.session_id.trim().is_empty() || !b.student_id.trim().is_empty() {
        let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let exit = res.get("exit").and_then(|v| v.as_i64()).unwrap_or(-1);
        let stdout = res.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
        let stderr = res.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
        let _ = sqlx::query(
            "INSERT INTO ep_it_submissions (id,session_id,student_id,language,code,exit_code,stdout,stderr,ok,score,feedback,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,'',?10)",
        )
        .bind(uid())
        .bind(&b.session_id)
        .bind(&b.student_id)
        .bind(lang)
        .bind(&b.code)
        .bind(exit)
        .bind(stdout)
        .bind(stderr)
        .bind(if ok { 1 } else { 0 })
        .bind(now())
        .execute(&s.db)
        .await;
    }
    Json(json!({ "ok": true, "language": lang, "result": res }))
}

#[derive(Deserialize)]
pub struct SubmitIn {
    pub student_id: String,
    pub code: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub expect_stdout: String,
}

/// Huwasilisha kazi: hukimbiza code, hulinganisha matokeo na `expect_stdout`.
pub async fn submit(State(s): State<AppState>, Json(b): Json<SubmitIn>) -> Json<Value> {
    if b.student_id.trim().is_empty() || b.code.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "student_id na code zinahitajika" }));
    }
    let lang = if b.language.trim().is_empty() {
        "python"
    } else {
        b.language.trim()
    };
    if !lang_ok(lang) {
        return Json(json!({
            "ok": false, "error": format!("lugha '{lang}' haitumiki"), "languages": LANGS
        }));
    }
    let res = crate::ide_code_run::run_code(&b.code, lang, None).await;
    let ran_ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let exit = res.get("exit").and_then(|v| v.as_i64()).unwrap_or(-1);
    let stdout = res.get("stdout").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let stderr = res.get("stderr").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let expected = b.expect_stdout.trim();
    let (passed, feedback, score) = if !ran_ok {
        (false, "code haikukimbia".to_string(), 0)
    } else if expected.is_empty() {
        (true, "imekimbia bila hitilafu".to_string(), 70)
    } else if stdout.trim() == expected {
        (true, "matokeo yanalingana".to_string(), 100)
    } else {
        (
            false,
            format!("matokeo hayalingani (yalitarajiwa: {expected})"),
            0,
        )
    };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_it_submissions (id,session_id,student_id,language,code,exit_code,stdout,stderr,ok,score,feedback,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
    )
    .bind(&id)
    .bind(&b.session_id)
    .bind(b.student_id.trim())
    .bind(lang)
    .bind(&b.code)
    .bind(exit)
    .bind(&stdout)
    .bind(&stderr)
    .bind(if passed { 1 } else { 0 })
    .bind(score)
    .bind(&feedback)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "id": id, "passed": passed, "score": score,
        "feedback": feedback, "stdout": stdout, "stderr": stderr, "exit": exit
    }))
}

#[derive(Deserialize)]
pub struct SubQuery {
    #[serde(default)]
    pub student_id: String,
}

pub async fn submissions_list(State(s): State<AppState>, Query(q): Query<SubQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, i64, i64, String, String)>(
        "SELECT id,student_id,language,ok,score,feedback,created_at FROM ep_it_submissions
         WHERE (?1='' OR student_id=?1) ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.student_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "submissions": rows.into_iter().map(|r| json!({
            "id": r.0, "student_id": r.1, "language": r.2, "ok": r.3 == 1,
            "score": r.4, "feedback": r.5, "created_at": r.6
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct LabIn {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub brief: String,
    #[serde(default)]
    pub expected: String,
    #[serde(default)]
    pub points: Option<i64>,
}

pub async fn lab_add(State(s): State<AppState>, Json(b): Json<LabIn>) -> Json<Value> {
    if b.title.trim().is_empty() || b.expected.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "title na expected zinahitajika" }));
    }
    let cat = match b.category.trim().to_lowercase().as_str() {
        "network" => "network",
        "forensics" => "forensics",
        "crypto" => "crypto",
        _ => "cyber",
    };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_it_labs (id,title,category,brief,expected,points,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(b.title.trim())
    .bind(cat)
    .bind(&b.brief)
    .bind(b.expected.trim())
    .bind(b.points.unwrap_or(10))
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "category": cat }))
}

#[derive(Deserialize)]
pub struct LabQuery {
    #[serde(default)]
    pub category: String,
}

pub async fn labs_list(State(s): State<AppState>, Query(q): Query<LabQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id,title,category,brief,points FROM ep_it_labs
         WHERE (?1='' OR category=?1) ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.category)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "labs": rows.into_iter().map(|r| json!({
            "id": r.0, "title": r.1, "category": r.2, "brief": r.3, "points": r.4
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct AttemptIn {
    pub lab_id: String,
    pub student_id: String,
    #[serde(default)]
    pub answer: String,
}

/// Hulinganisha jibu na linalotarajiwa — bila kubuni alama.
pub fn grade_answer(expected: &str, answer: &str) -> bool {
    expected.trim().eq_ignore_ascii_case(answer.trim()) && !expected.trim().is_empty()
}

pub async fn lab_attempt(State(s): State<AppState>, Json(b): Json<AttemptIn>) -> Json<Value> {
    if b.lab_id.trim().is_empty() || b.student_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "lab_id na student_id zinahitajika" }));
    }
    let lab = sqlx::query_as::<_, (String, i64)>("SELECT expected,points FROM ep_it_labs WHERE id=?1")
        .bind(b.lab_id.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let (expected, points) = match lab {
        None => return Json(json!({ "ok": false, "error": "lab haipo" })),
        Some(x) => x,
    };
    let correct = grade_answer(&expected, &b.answer);
    let awarded = if correct { points } else { 0 };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_it_lab_attempts (id,lab_id,student_id,answer,correct,points,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(b.lab_id.trim())
    .bind(b.student_id.trim())
    .bind(&b.answer)
    .bind(if correct { 1 } else { 0 })
    .bind(awarded)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "correct": correct, "points": awarded }))
}

#[derive(Deserialize)]
pub struct AttemptQuery {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub lab_id: String,
}

pub async fn lab_attempts(State(s): State<AppState>, Query(q): Query<AttemptQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, i64, i64, String)>(
        "SELECT id,lab_id,student_id,correct,points,created_at FROM ep_it_lab_attempts
         WHERE (?1='' OR student_id=?1) AND (?2='' OR lab_id=?2)
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.student_id)
    .bind(&q.lab_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "attempts": rows.into_iter().map(|r| json!({
            "id": r.0, "lab_id": r.1, "student_id": r.2,
            "correct": r.3 == 1, "points": r.4, "created_at": r.5
        })).collect::<Vec<_>>()
    }))
}

/// Njia ya database ya mazoezi (read-only, huseed ikiwa haipo).
fn practice_db_path() -> std::path::PathBuf {
    std::env::temp_dir().join("mtech-ittools-practice.db")
}

async fn practice_pool() -> Result<SqlitePool, String> {
    let p = practice_db_path();
    let url = format!("sqlite://{}?mode=rwc", p.display());
    let pool = SqlitePool::connect(&url).await.map_err(|e| e.to_string())?;
    // Seed ikiwa tupu.
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS wanafunzi (id INTEGER PRIMARY KEY, jina TEXT, course TEXT, alama INTEGER)",
    )
    .execute(&pool)
    .await;
    let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM wanafunzi")
        .fetch_one(&pool)
        .await
        .unwrap_or((0,));
    if n == 0 {
        for (jina, course, alama) in [
            ("Neema Juma", "CS", 88),
            ("Juma Ally", "CS", 72),
            ("Asha Mwinyi", "IT", 91),
            ("Baraka Mushi", "IT", 65),
        ] {
            let _ = sqlx::query("INSERT INTO wanafunzi (jina,course,alama) VALUES (?1,?2,?3)")
                .bind(jina)
                .bind(course)
                .bind(alama)
                .execute(&pool)
                .await;
        }
    }
    Ok(pool)
}

#[derive(Deserialize)]
pub struct QueryIn {
    pub sql: String,
}

pub async fn db_query(Json(b): Json<QueryIn>) -> Json<Value> {
    if let Err(e) = sql_is_readonly(&b.sql) {
        return Json(json!({ "ok": false, "error": e }));
    }
    let pool = match practice_pool().await {
        Ok(p) => p,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let rows = match sqlx::query(&b.sql).fetch_all(&pool).await {
        Ok(r) => r,
        Err(e) => return Json(json!({ "ok": false, "error": format!("SQL: {e}") })),
    };
    use sqlx::{Column, Row, TypeInfo};
    let cols: Vec<String> = rows
        .first()
        .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
        .unwrap_or_default();
    let data: Vec<Vec<Value>> = rows
        .iter()
        .map(|r| {
            (0..r.columns().len())
                .map(|i| {
                    let ti = r.column(i).type_info().name().to_string();
                    match ti.as_str() {
                        "INTEGER" | "INT" | "BIGINT" => r
                            .try_get::<i64, _>(i)
                            .map(|v| json!(v))
                            .unwrap_or(Value::Null),
                        "REAL" | "FLOAT" | "DOUBLE" => r
                            .try_get::<f64, _>(i)
                            .map(|v| json!(v))
                            .unwrap_or(Value::Null),
                        _ => r
                            .try_get::<String, _>(i)
                            .map(|v| json!(v))
                            .unwrap_or(Value::Null),
                    }
                })
                .collect()
        })
        .collect();
    Json(json!({ "ok": true, "columns": cols, "rows": data, "count": data.len() }))
}

pub async fn db_schema() -> Json<Value> {
    let pool = match practice_pool().await {
        Ok(p) => p,
        Err(e) => return Json(json!({ "ok": false, "error": e })),
    };
    let tables = sqlx::query_as::<_, (String,)>(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    let mut out = Vec::new();
    for (t,) in tables {
        let cols = sqlx::query_as::<_, (i64, String, String, i64, Option<String>, i64)>(
            &format!("PRAGMA table_info({t})"),
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default();
        out.push(json!({
            "table": t,
            "columns": cols.into_iter().map(|(cid, name, ty, notnull, dflt, pk)| json!({
                "cid": cid, "name": name, "type": ty, "notnull": notnull == 1,
                "default": dflt, "pk": pk == 1
            })).collect::<Vec<_>>()
        }));
    }
    Json(json!({ "ok": true, "tables": out }))
}

/// Ufuatiliaji wa mwalimu: vikao hai + muhtasari wa kazi.
pub async fn monitor(State(s): State<AppState>) -> Json<Value> {
    let live = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,student_id,kind,language,started_at FROM ep_it_sessions
         WHERE status='active' ORDER BY started_at DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let recent = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT student_id,language,feedback,ok,score FROM ep_it_submissions
         ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "live": live.into_iter().map(|r| json!({
            "id": r.0, "student_id": r.1, "kind": r.2, "language": r.3, "started_at": r.4
        })).collect::<Vec<_>>(),
        "recent": recent.into_iter().map(|r| json!({
            "student_id": r.0, "language": r.1, "feedback": r.2, "ok": r.3 == 1, "score": r.4
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lugha_zinazokubalika() {
        assert!(lang_ok("rust"));
        assert!(lang_ok("Python"));
        assert!(lang_ok("node"));
        assert!(!lang_ok("cobol"));
        assert!(!lang_ok(""));
    }

    #[test]
    fn sql_ya_mazoezi_ni_readonly() {
        assert!(sql_is_readonly("SELECT * FROM wanafunzi").is_ok());
        assert!(sql_is_readonly("select jina from wanafunzi where alama > 80;").is_ok());
        assert!(sql_is_readonly("WITH t AS (SELECT 1 AS x) SELECT * FROM t").is_ok());

        assert!(sql_is_readonly("DROP TABLE wanafunzi").is_err());
        assert!(sql_is_readonly("DELETE FROM wanafunzi").is_err());
        assert!(sql_is_readonly("UPDATE wanafunzi SET alama=100").is_err());
        assert!(sql_is_readonly("INSERT INTO wanafunzi VALUES (9,'x','y',1)").is_err());
        assert!(sql_is_readonly("ATTACH DATABASE '/etc/passwd' AS p").is_err());
        assert!(sql_is_readonly("PRAGMA table_info(wanafunzi)").is_err());
        assert!(sql_is_readonly("SELECT 1; DROP TABLE wanafunzi").is_err());
        assert!(sql_is_readonly("").is_err());
        assert!(sql_is_readonly("   ").is_err());
    }

    #[test]
    fn majibu_ya_lab_hulinganishwa_bila_kujali_herufi_kubwa() {
        assert!(grade_answer("flag{abc}", "flag{abc}"));
        assert!(grade_answer("Flag{ABC}", "flag{abc}"));
        assert!(grade_answer("  jibu  ", "jibu"));
        assert!(!grade_answer("flag{abc}", "flag{xyz}"));
        assert!(!grade_answer("", ""));
    }

    #[test]
    fn aina_za_kikao_hupangwa() {
        assert_eq!(norm_kind("terminal"), "terminal");
        assert_eq!(norm_kind("DB"), "db");
        assert_eq!(norm_kind("sql"), "db");
        assert_eq!(norm_kind("cyber"), "cyber");
        assert_eq!(norm_kind("usalama"), "cyber");
        assert_eq!(norm_kind(""), "editor");
    }

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn kikao_huanza_hai_kisha_hufungwa() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_it_sessions (id,student_id,kind,language,status,started_at) VALUES ('s1','st1','editor','python','active',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_it_sessions WHERE id='s1'")
            .fetch_one(&p).await.unwrap();
        assert!(session_live(&st));
        sqlx::query("UPDATE ep_it_sessions SET status='ended',ended_at=?1 WHERE id='s1' AND status='active'")
            .bind(now()).execute(&p).await.unwrap();
        let (st2,): (String,) = sqlx::query_as("SELECT status FROM ep_it_sessions WHERE id='s1'")
            .fetch_one(&p).await.unwrap();
        assert!(!session_live(&st2));
    }

    #[tokio::test]
    async fn lab_hutoa_pointi_kwa_jibu_sahihi_pekee() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_it_labs (id,title,category,brief,expected,points,created_at) VALUES ('L1','SQLi','cyber','','flag{sqli}',10,?1)")
            .bind(now()).execute(&p).await.unwrap();
        let (expected, points): (String, i64) =
            sqlx::query_as("SELECT expected,points FROM ep_it_labs WHERE id='L1'")
                .fetch_one(&p).await.unwrap();
        assert!(grade_answer(&expected, "flag{sqli}"));
        let awarded = if grade_answer(&expected, "flag{sqli}") { points } else { 0 };
        assert_eq!(awarded, 10);
        let wrong = if grade_answer(&expected, "flag{no}") { points } else { 0 };
        assert_eq!(wrong, 0);
    }
}
