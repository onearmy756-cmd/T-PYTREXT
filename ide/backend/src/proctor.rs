//! proctor.rs — ELIMUPRO Awamu 5: mitihani ya chuo + ulinzi mkali (moduli 11).
//!
//! Ulinzi (proctoring) wa kweli:
//!   - Picha za mwanafunzi kila baada ya sekunde (kutoka `getUserMedia`)
//!   - Kipimo cha sauti (mic level) — kelele za nje hurekodiwa
//!   - Kuzuia copy/paste + kuzuia DevTools + kuzuia kutoka fullscreen
//!   - Kutambua kutoka kwenye dirisha (focus loss) na kuangalia kando
//!   - Kuashiria (flag) matumizi ya AI — kwa hoja, si hukumu ya kiotomatiki
//!
//! Sehemu mbili (two-pane): maswali kushoto (kutoka PDF/benki), majibu kulia.
//! Swali tupu hujazwa dashi (`-`). Jibu lililoandikwa HUFUNGWA — mwanafunzi
//! hawezi kufuta au kukata.
//!
//! Kusahihisha: MCQ hupimwa kiotomatiki; code HUCOMPILE (IT); maandishi →
//! LLM halisi (au mwalimu). Bila LLM → `pending`, SI kubuni alama.
//!
//! Tables: ep_exams ep_exam_questions ep_exam_attempts ep_exam_answers
//!         ep_proctor_events

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
        r#"CREATE TABLE IF NOT EXISTS ep_exams (
            id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL DEFAULT '',
            level_id TEXT NOT NULL DEFAULT '',
            teacher_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL,
            instructions TEXT NOT NULL DEFAULT '',
            minutes INTEGER NOT NULL DEFAULT 60,
            status TEXT NOT NULL DEFAULT 'draft',  -- draft|published|closed
            proctor INTEGER NOT NULL DEFAULT 1,   -- ulinzi umewashwa?
            pass_mark REAL NOT NULL DEFAULT 50,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_exam_questions (
            id TEXT PRIMARY KEY,
            exam_id TEXT NOT NULL,
            n INTEGER NOT NULL DEFAULT 1,
            kind TEXT NOT NULL DEFAULT 'text',   -- mcq|text|code
            prompt TEXT NOT NULL,
            options TEXT NOT NULL DEFAULT '[]',
            answer_key TEXT NOT NULL DEFAULT '',
            language TEXT NOT NULL DEFAULT '',
            marks REAL NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_exam_attempts (
            id TEXT PRIMARY KEY,
            exam_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',  -- active|submitted|graded
            started_at TEXT NOT NULL,
            submitted_at TEXT NOT NULL DEFAULT '',
            score REAL NOT NULL DEFAULT -1,
            max_score REAL NOT NULL DEFAULT 0,
            violations INTEGER NOT NULL DEFAULT 0,
            grade_mode TEXT NOT NULL DEFAULT '',    -- auto|llm|teacher
            UNIQUE(exam_id, student_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_exam_answers (
            id TEXT PRIMARY KEY,
            attempt_id TEXT NOT NULL,
            question_id TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            locked INTEGER NOT NULL DEFAULT 0,      -- imefungwa baada ya kuandika
            auto_score REAL NOT NULL DEFAULT -1,
            ai_score REAL NOT NULL DEFAULT -1,
            ai_feedback TEXT NOT NULL DEFAULT '',
            score REAL NOT NULL DEFAULT -1,
            run_out TEXT NOT NULL DEFAULT '',        -- matokeo ya kucompile code
            saved_at TEXT NOT NULL,
            UNIQUE(attempt_id, question_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_proctor_events (
            id TEXT PRIMARY KEY,
            attempt_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            weight INTEGER NOT NULL DEFAULT 1,
            detail TEXT NOT NULL DEFAULT '',
            data TEXT NOT NULL DEFAULT '',
            at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/proctor/status", get(status))
        .route("/api/proctor/exam/create", post(exam_create))
        .route("/api/proctor/exams", get(exams_list))
        .route("/api/proctor/exam/:id", get(exam_get))
        .route("/api/proctor/exam/:id/question", post(question_add))
        .route("/api/proctor/exam/:id/questions", get(questions_list))
        .route("/api/proctor/exam/:id/publish", post(exam_publish))
        .route("/api/proctor/exam/:id/close", post(exam_close))
        .route("/api/proctor/exam/:id/attempts", get(exam_attempts))
        .route("/api/proctor/exam/:id/start", post(attempt_start))
        .route("/api/proctor/attempt/:aid", get(attempt_get))
        .route("/api/proctor/attempt/:aid/answer", post(answer_save))
        .route("/api/proctor/attempt/:aid/event", post(event_add))
        .route("/api/proctor/attempt/:aid/events", get(events_list))
        .route("/api/proctor/attempt/:aid/run", post(run_code_here))
        .route("/api/proctor/attempt/:aid/submit", post(attempt_submit))
        .route("/api/proctor/attempt/:aid/grade", post(answer_grade))
        .layer(axum::extract::DefaultBodyLimit::max(32 * 1024 * 1024))
}

// ============================================================
// HELPERS
// ============================================================

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Uzito wa kila kosa la ulinzi (kiasi cha "violations").
fn event_weight(kind: &str) -> i64 {
    match kind {
        "focus_lost" | "fullscreen_exit" => 1,
        "copy" | "paste" | "cut" => 2,
        "devtools" | "multi_face" => 3,
        "face_away" => 1,
        "ai_suspect" => 2,
        "photo" | "audio" => 0,
        _ => 0,
    }
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "exams": table_count(&s.db, "ep_exams").await,
        "published": sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ep_exams WHERE status='published'")
            .fetch_one(&s.db).await.map(|r| r.0).unwrap_or(0),
        "attempts": table_count(&s.db, "ep_exam_attempts").await,
        "active": sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ep_exam_attempts WHERE status='active'")
            .fetch_one(&s.db).await.map(|r| r.0).unwrap_or(0),
        "proctor_events": table_count(&s.db, "ep_proctor_events").await,
    }))
}

// ============================================================
// MITIHANI
// ============================================================

#[derive(Deserialize)]
pub struct ExamIn {
    pub title: String,
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub teacher_id: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub minutes: i64,
    #[serde(default)]
    pub proctor: Option<bool>,
    #[serde(default)]
    pub pass_mark: Option<f64>,
}

pub async fn exam_create(State(s): State<AppState>, Json(b): Json<ExamIn>) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "kichwa cha mtihani kinahitajika" }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_exams (id,subject_id,level_id,teacher_id,title,instructions,minutes,proctor,pass_mark,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )
    .bind(&id)
    .bind(&b.subject_id)
    .bind(&b.level_id)
    .bind(&b.teacher_id)
    .bind(b.title.trim())
    .bind(&b.instructions)
    .bind(if b.minutes > 0 { b.minutes } else { 60 })
    .bind(if b.proctor.unwrap_or(true) { 1 } else { 0 })
    .bind(b.pass_mark.unwrap_or(50.0))
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id }))
}

#[derive(Deserialize)]
pub struct ExamsQuery {
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub status: String,
}

pub async fn exams_list(State(s): State<AppState>, Query(q): Query<ExamsQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64)>(
        "SELECT id,title,status,created_at,minutes,
                (SELECT COUNT(*) FROM ep_exam_questions q WHERE q.exam_id = e.id),
                (SELECT COUNT(*) FROM ep_exam_attempts a WHERE a.exam_id = e.id)
         FROM ep_exams e WHERE (?1='' OR subject_id=?1) AND (?2='' OR status=?2)
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.subject_id)
    .bind(&q.status)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "exams": rows.into_iter().map(|r| json!({
            "id": r.0, "title": r.1, "status": r.2, "created_at": r.3,
            "minutes": r.4, "questions": r.5, "attempts": r.6
        })).collect::<Vec<_>>()
    }))
}

pub async fn exam_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64, i64, String, f64)>(
        "SELECT id,title,status,instructions,minutes,proctor,level_id,pass_mark FROM ep_exams WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({ "ok": false, "error": "mtihani haupo" }));
    };
    Json(json!({
        "ok": true,
        "exam": {
            "id": r.0, "title": r.1, "status": r.2, "instructions": r.3,
            "minutes": r.4, "proctor": r.5 == 1, "level_id": r.6, "pass_mark": r.7
        }
    }))
}

#[derive(Deserialize)]
pub struct QuestionIn {
    #[serde(default)]
    pub n: i64,
    #[serde(default)]
    pub kind: String,
    pub prompt: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub answer_key: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub marks: f64,
}

pub async fn question_add(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<QuestionIn>) -> Json<Value> {
    if b.prompt.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "swali linahitajika" }));
    }
    let kind = match b.kind.as_str() {
        "mcq" => "mcq",
        "code" => "code",
        _ => "text",
    };
    let n = if b.n > 0 {
        b.n
    } else {
        sqlx::query_as::<_, (i64,)>("SELECT COALESCE(MAX(n),0)+1 FROM ep_exam_questions WHERE exam_id=?1")
            .bind(&id)
            .fetch_one(&s.db)
            .await
            .map(|r| r.0)
            .unwrap_or(1)
    };
    let qid = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_exam_questions (id,exam_id,n,kind,prompt,options,answer_key,language,marks,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )
    .bind(&qid)
    .bind(&id)
    .bind(n)
    .bind(kind)
    .bind(b.prompt.trim())
    .bind(serde_json::to_string(&b.options).unwrap_or_else(|_| "[]".into()))
    .bind(&b.answer_key)
    .bind(&b.language)
    .bind(if b.marks > 0.0 { b.marks } else { 1.0 })
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": qid, "n": n, "kind": kind }))
}

#[derive(Deserialize)]
pub struct QuestionsQuery {
    #[serde(default)]
    pub with_key: Option<bool>,
}

pub async fn questions_list(State(s): State<AppState>, Path(id): Path<String>, Query(q): Query<QuestionsQuery>) -> Json<Value> {
    let with_key = q.with_key.unwrap_or(false);
    let rows = sqlx::query_as::<_, (String, i64, String, String, String, String, f64)>(
        "SELECT id,n,kind,prompt,options,answer_key,marks FROM ep_exam_questions WHERE exam_id=?1 ORDER BY n",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "questions": rows.into_iter().map(|r| {
            let mut v = json!({
                "id": r.0, "n": r.1, "kind": r.2, "prompt": r.3,
                "options": serde_json::from_str::<Value>(&r.4).unwrap_or(json!([])),
                "language": "", "marks": r.6
            });
            // Ufunguo wa majibu huonyeshwa kwa mwalimu PEKEE.
            if with_key { v["answer_key"] = json!(r.5); }
            v
        }).collect::<Vec<_>>()
    }))
}

pub async fn exam_publish(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let n = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ep_exam_questions WHERE exam_id=?1")
        .bind(&id)
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    if n == 0 {
        return Json(json!({ "ok": false, "error": "ongeza maswali kabla ya kuchapisha" }));
    }
    let _ = sqlx::query("UPDATE ep_exams SET status='published' WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "status": "published", "questions": n }))
}

pub async fn exam_close(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let _ = sqlx::query("UPDATE ep_exams SET status='closed' WHERE id=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "status": "closed" }))
}

pub async fn exam_attempts(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, f64, i64, String, String)>(
        "SELECT a.id,a.student_id,COALESCE(st.full_name,a.student_id),a.score,a.violations,a.status,a.grade_mode
         FROM ep_exam_attempts a LEFT JOIN ep_students st ON st.id = a.student_id
         WHERE a.exam_id=?1 ORDER BY a.started_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "attempts": rows.into_iter().map(|r| json!({
            "id": r.0, "student_id": r.1, "student": r.2, "score": r.3,
            "violations": r.4, "status": r.5, "grade_mode": r.6
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// KUANZA MTIHANI
// ============================================================

#[derive(Deserialize)]
pub struct StartIn {
    pub student_id: String,
}

pub async fn attempt_start(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<StartIn>) -> Json<Value> {
    let st = sqlx::query_as::<_, (String, i64)>("SELECT status,proctor FROM ep_exams WHERE id=?1")
        .bind(&id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((status, proctor)) = st else {
        return Json(json!({ "ok": false, "error": "mtihani haupo" }));
    };
    if status != "published" {
        return Json(json!({ "ok": false, "error": "mtihani haujachapishwa bado" }));
    }
    let existing = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM ep_exam_attempts WHERE exam_id=?1 AND student_id=?2",
    )
    .bind(&id)
    .bind(&b.student_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if let Some((aid, st)) = existing {
        if st != "active" {
            return Json(json!({ "ok": false, "error": "mtihani umekwisha kwa mwanafunzi huyu", "attempt_id": aid }));
        }
        return Json(json!({ "ok": true, "attempt_id": aid, "proctor": proctor == 1, "resumed": true }));
    }
    let aid = uid();
    let max: f64 = sqlx::query_as::<_, (f64,)>("SELECT COALESCE(SUM(marks),0) FROM ep_exam_questions WHERE exam_id=?1")
        .bind(&id)
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0.0);
    let _ = sqlx::query(
        "INSERT INTO ep_exam_attempts (id,exam_id,student_id,status,started_at,max_score) VALUES (?1,?2,?3,'active',?4,?5)",
    )
    .bind(&aid)
    .bind(&id)
    .bind(&b.student_id)
    .bind(now())
    .bind(max)
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "attempt_id": aid, "proctor": proctor == 1, "max_score": max }))
}

pub async fn attempt_get(State(s): State<AppState>, Path(aid): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, f64, i64, String)>(
        "SELECT exam_id,student_id,status,started_at,score,violations,grade_mode FROM ep_exam_attempts WHERE id=?1",
    )
    .bind(&aid)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({ "ok": false, "error": "attempt haipo" }));
    };
    let answers = sqlx::query_as::<_, (String, String, i64, f64, f64, String, String)>(
        "SELECT question_id,body,locked,auto_score,ai_score,ai_feedback,run_out FROM ep_exam_answers WHERE attempt_id=?1",
    )
    .bind(&aid)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "attempt": {
            "id": aid, "exam_id": r.0, "student_id": r.1, "status": r.2,
            "started_at": r.3, "score": r.4, "violations": r.5, "grade_mode": r.6
        },
        "answers": answers.into_iter().map(|a| json!({
            "question_id": a.0, "body": a.1, "locked": a.2 == 1,
            "auto_score": a.3, "ai_score": a.4, "ai_feedback": a.5, "run_out": a.6
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// MAJIBU (append-only: hufungwa baada ya kuandika)
// ============================================================

#[derive(Deserialize)]
pub struct AnswerIn {
    pub question_id: String,
    pub body: String,
}

pub async fn answer_save(State(s): State<AppState>, Path(aid): Path<String>, Json(b): Json<AnswerIn>) -> Json<Value> {
    let att = sqlx::query_as::<_, (String,)>("SELECT status FROM ep_exam_attempts WHERE id=?1")
        .bind(&aid)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    match att {
        Some((st,)) if st == "active" => {}
        Some(_) => return Json(json!({ "ok": false, "error": "mtihani umekwisha — jibu halibadiliki" })),
        None => return Json(json!({ "ok": false, "error": "attempt haipo" })),
    }
    let prev = sqlx::query_as::<_, (String, i64)>(
        "SELECT body,locked FROM ep_exam_answers WHERE attempt_id=?1 AND question_id=?2",
    )
    .bind(&aid)
    .bind(&b.question_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if let Some((old, locked)) = prev {
        // Jibu lililofungwa HALIBADILIKI — huzuia kufuta au kukata.
        if locked == 1 {
            return Json(json!({
                "ok": false, "locked": true,
                "error": "jibu hili limefungwa — huwezi kufuta au kukata",
                "body": old
            }));
        }
        if b.body.trim().is_empty() {
            return Json(json!({ "ok": false, "error": "jibu tupu haliwezi kuhifadhiwa" }));
        }
        let _ = sqlx::query(
            "UPDATE ep_exam_answers SET body=?1, locked=1, saved_at=?2 WHERE attempt_id=?3 AND question_id=?4",
        )
        .bind(b.body.trim())
        .bind(now())
        .bind(&aid)
        .bind(&b.question_id)
        .execute(&s.db)
        .await;
        return Json(json!({ "ok": true, "locked": true }));
    }
    if b.body.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jibu tupu haliwezi kuhifadhiwa" }));
    }
    let _ = sqlx::query(
        "INSERT INTO ep_exam_answers (id,attempt_id,question_id,body,locked,saved_at) VALUES (?1,?2,?3,?4,1,?5)",
    )
    .bind(uid())
    .bind(&aid)
    .bind(&b.question_id)
    .bind(b.body.trim())
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "locked": true }))
}

// ============================================================
// ULINZI (proctor events)
// ============================================================

#[derive(Deserialize)]
pub struct EventIn {
    pub kind: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub data: String,
}

pub async fn event_add(State(s): State<AppState>, Path(aid): Path<String>, Json(b): Json<EventIn>) -> Json<Value> {
    let w = event_weight(&b.kind);
    let _ = sqlx::query(
        "INSERT INTO ep_proctor_events (id,attempt_id,kind,weight,detail,data,at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(uid())
    .bind(&aid)
    .bind(&b.kind)
    .bind(w)
    .bind(&b.detail)
    .bind(&b.data)
    .bind(now())
    .execute(&s.db)
    .await;
    if w > 0 {
        let _ = sqlx::query("UPDATE ep_exam_attempts SET violations = violations + ?1 WHERE id=?2")
            .bind(w)
            .bind(&aid)
            .execute(&s.db)
            .await;
    }
    let total = sqlx::query_as::<_, (i64,)>("SELECT violations FROM ep_exam_attempts WHERE id=?1")
        .bind(&aid)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or(0);
    Json(json!({ "ok": true, "weight": w, "violations": total }))
}

pub async fn events_list(State(s): State<AppState>, Path(aid): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, i64, String, String, String)>(
        "SELECT kind,weight,detail,data,at FROM ep_proctor_events WHERE attempt_id=?1 ORDER BY at",
    )
    .bind(&aid)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "events": rows.into_iter().map(|r| json!({
            "kind": r.0, "weight": r.1, "detail": r.2, "data": r.3, "at": r.4
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// CODE (IT): kuendesha/kucompile moja kwa moja
// ============================================================

#[derive(Deserialize)]
pub struct RunIn {
    pub question_id: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub body: String,
}

pub async fn run_code_here(State(s): State<AppState>, Path(aid): Path<String>, Json(b): Json<RunIn>) -> Json<Value> {
    let body = if b.body.trim().is_empty() {
        sqlx::query_as::<_, (String,)>("SELECT body FROM ep_exam_answers WHERE attempt_id=?1 AND question_id=?2")
            .bind(&aid)
            .bind(&b.question_id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
            .map(|r| r.0)
            .unwrap_or_default()
    } else {
        b.body.clone()
    };
    if body.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "hakuna code ya kuendesha" }));
    }
    let lang = if b.language.trim().is_empty() { "python" } else { b.language.as_str() };
    let out = crate::ide_code_run::run_code(&body, lang, None).await;
    let _ = sqlx::query("UPDATE ep_exam_answers SET run_out=?1 WHERE attempt_id=?2 AND question_id=?3")
        .bind(out.to_string())
        .bind(&aid)
        .bind(&b.question_id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "run": out }))
}

// ============================================================
// KUSAHIHISHA
// ============================================================

/// Hupima jibu la maandishi kwa LLM halisi. Bila LLM → -1 (pending).
async fn ai_grade(db: &SqlitePool, prompt: &str, body: &str, marks: f64) -> (f64, String) {
    let p = format!(
        "Wewe ni mwalimu. Sahihisha jibu la mwanafunzi kwa Kiswahili.\n\
         SWALI: {prompt}\nJIBU: {body}\nAlama za juu: {marks}\n\n\
         Rudisha JSON TU: {{\"alama\":<0-{marks}>,\"maoni\":\"...\"}}"
    );
    match crate::education::llm_long(db, &p, 90).await {
        Some(t) => match crate::education::extract_json(&t) {
            Some(v) => {
                let s = v["alama"].as_f64().unwrap_or(-1.0).clamp(0.0, marks);
                (s, v["maoni"].as_str().unwrap_or("").to_string())
            }
            None => (-1.0, String::new()),
        },
        None => (-1.0, String::new()),
    }
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Hupima jibu la MCQ. `key` inaweza kuwa herufi ("B") au maandishi ya chaguo
/// ("O(n)") — zote zinakubalika. Hurudisha alama zilizopatikana.
pub fn mcq_score(options: &str, key: &str, body: &str, marks: f64) -> f64 {
    let opts: Vec<String> = serde_json::from_str(options).unwrap_or_default();
    let kt = normalize(key);
    let idx = if kt.len() == 1 && kt.chars().next().unwrap().is_ascii_alphabetic() {
        let i = kt.chars().next().unwrap() as u8 - b'a';
        if (i as usize) < opts.len() {
            Some(i)
        } else {
            None
        }
    } else {
        opts.iter().position(|o| normalize(o) == kt).map(|i| i as u8)
    };
    let given = body.trim().to_uppercase();
    if given.chars().next() == idx.map(|i| (b'A' + i) as char) {
        marks
    } else {
        0.0
    }
}

pub async fn attempt_submit(State(s): State<AppState>, Path(aid): Path<String>) -> Json<Value> {
    let att = sqlx::query_as::<_, (String, String)>("SELECT exam_id,status FROM ep_exam_attempts WHERE id=?1")
        .bind(&aid)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((exam_id, status)) = att else {
        return Json(json!({ "ok": false, "error": "attempt haipo" }));
    };
    if status != "active" {
        return Json(json!({ "ok": false, "error": "mtihani umekwisha kusubmitiwa" }));
    }
    let questions = sqlx::query_as::<_, (String, String, String, String, String, f64)>(
        "SELECT id,kind,prompt,options,answer_key,marks FROM ep_exam_questions WHERE exam_id=?1 ORDER BY n",
    )
    .bind(&exam_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut total = 0.0;
    let mut auto_all = true;
    let mut llm_used = false;
    let mut results: Vec<Value> = Vec::new();

    for (qid, kind, prompt, options, key, marks) in &questions {
        let body = sqlx::query_as::<_, (String,)>(
            "SELECT body FROM ep_exam_answers WHERE attempt_id=?1 AND question_id=?2",
        )
        .bind(&aid)
        .bind(qid)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_default();
        // Swali tupu → dashi.
        let body = if body.trim().is_empty() { "-".to_string() } else { body };

        let (auto, ai, fb): (f64, f64, String) = match kind.as_str() {
            "mcq" => (mcq_score(options, key, &body, *marks), -1.0, String::new()),
            "code" => {
                // Code: hupimwa kwa kuendesha — mwalimu/LLM huamua alama.
                (0.0, -1.0, "Code hukaguliwa kwa kuendesha".to_string())
            }
            _ => {
                if body == "-" {
                    (0.0, -1.0, String::new())
                } else {
                    let (sc, f) = ai_grade(&s.db, prompt, &body, *marks).await;
                    if sc < 0.0 {
                        auto_all = false;
                    } else {
                        llm_used = true;
                    }
                    (-1.0, sc, f)
                }
            }
        };
        let score = if auto >= 0.0 {
            auto
        } else if ai >= 0.0 {
            ai
        } else {
            -1.0
        };
        if score >= 0.0 {
            total += score;
        }
        let _ = sqlx::query(
            "UPDATE ep_exam_answers SET auto_score=?1, ai_score=?2, ai_feedback=?3, score=?4 WHERE attempt_id=?5 AND question_id=?6",
        )
        .bind(auto)
        .bind(ai)
        .bind(&fb)
        .bind(score)
        .bind(&aid)
        .bind(qid)
        .execute(&s.db)
        .await;
        results.push(json!({
            "question_id": qid, "n": 0, "kind": kind, "score": score,
            "auto_score": auto, "ai_score": ai, "ai_feedback": fb
        }));
    }

    let (max,): (f64,) = sqlx::query_as("SELECT COALESCE(SUM(marks),0) FROM ep_exam_questions WHERE exam_id=?1")
        .bind(&exam_id)
        .fetch_one(&s.db)
        .await
        .unwrap_or((0.0,));
    let mode = if auto_all && llm_used {
        "llm"
    } else if auto_all {
        "auto"
    } else {
        "pending"
    };
    let _ = sqlx::query(
        "UPDATE ep_exam_attempts SET status='submitted', submitted_at=?1, score=?2, max_score=?3, grade_mode=?4 WHERE id=?5",
    )
    .bind(now())
    .bind(total)
    .bind(max)
    .bind(mode)
    .bind(&aid)
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "status": "submitted", "score": total, "max_score": max,
        "grade_mode": mode, "results": results
    }))
}

#[derive(Deserialize)]
pub struct GradeIn {
    pub question_id: String,
    pub score: f64,
    #[serde(default)]
    pub feedback: String,
}

/// Mwalimu hupitisha (override) alama ya jibu moja, kisha hujumlisha upya.
pub async fn answer_grade(State(s): State<AppState>, Path(aid): Path<String>, Json(b): Json<GradeIn>) -> Json<Value> {
    let _ = sqlx::query(
        "UPDATE ep_exam_answers SET score=?1, ai_feedback=CASE WHEN ?2='' THEN ai_feedback ELSE ?2 END
         WHERE attempt_id=?3 AND question_id=?4",
    )
    .bind(b.score)
    .bind(&b.feedback)
    .bind(&aid)
    .bind(&b.question_id)
    .execute(&s.db)
    .await;
    let (total,): (f64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(CASE WHEN score < 0 THEN 0 ELSE score END),0) FROM ep_exam_answers WHERE attempt_id=?1",
    )
    .bind(&aid)
    .fetch_one(&s.db)
    .await
    .unwrap_or((0.0,));
    let _ = sqlx::query("UPDATE ep_exam_attempts SET score=?1, status='graded', grade_mode='teacher' WHERE id=?2")
        .bind(total)
        .bind(&aid)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "score": total }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    async fn exam(db: &SqlitePool) -> String {
        let id = uid();
        sqlx::query("INSERT INTO ep_exams (id,title,status,created_at) VALUES (?1,'Mtihani','published',?2)")
            .bind(&id)
            .bind(now())
            .execute(db)
            .await
            .unwrap();
        id
    }

    async fn question(db: &SqlitePool, exam_id: &str, kind: &str, key: &str, marks: f64) -> String {
        let id = uid();
        sqlx::query(
            "INSERT INTO ep_exam_questions (id,exam_id,n,kind,prompt,options,answer_key,marks,created_at)
             VALUES (?1,?2,1,?3,'Swali','[\"A\",\"B\",\"C\"]',?4,?5,?6)",
        )
        .bind(&id)
        .bind(exam_id)
        .bind(kind)
        .bind(key)
        .bind(marks)
        .bind(now())
        .execute(db)
        .await
        .unwrap();
        id
    }

    async fn attempt(db: &SqlitePool, exam_id: &str) -> String {
        let id = uid();
        sqlx::query(
            "INSERT INTO ep_exam_attempts (id,exam_id,student_id,status,started_at) VALUES (?1,?2,'s1','active',?3)",
        )
        .bind(&id)
        .bind(exam_id)
        .bind(now())
        .execute(db)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn init_tables_huunda_majedwali_yote() {
        let p = db().await;
        for t in [
            "ep_exams",
            "ep_exam_questions",
            "ep_exam_attempts",
            "ep_exam_answers",
            "ep_proctor_events",
        ] {
            assert_eq!(table_count(&p, t).await, 0, "{t} haipo");
        }
    }

    #[test]
    fn uzito_wa_ulinzi_ni_sahihi() {
        assert_eq!(event_weight("photo"), 0);
        assert_eq!(event_weight("focus_lost"), 1);
        assert_eq!(event_weight("copy"), 2);
        assert_eq!(event_weight("devtools"), 3);
        assert_eq!(event_weight("multi_face"), 3);
        assert_eq!(event_weight("kitu_kingine"), 0);
    }

    #[tokio::test]
    async fn jibu_hufungwa_baada_ya_kuandikwa_kwanza() {
        let p = db().await;
        let e = exam(&p).await;
        let q = question(&p, &e, "text", "", 10.0).await;
        let a = attempt(&p, &e).await;
        sqlx::query(
            "INSERT INTO ep_exam_answers (id,attempt_id,question_id,body,locked,saved_at) VALUES (?1,?2,?3,'jibu la kwanza',1,?4)",
        )
        .bind(uid()).bind(&a).bind(&q).bind(now()).execute(&p).await.unwrap();
        let (body, locked): (String, i64) = sqlx::query_as(
            "SELECT body,locked FROM ep_exam_answers WHERE attempt_id=?1 AND question_id=?2",
        ).bind(&a).bind(&q).fetch_one(&p).await.unwrap();
        assert_eq!(locked, 1);
        assert_eq!(body, "jibu la kwanza");
    }

    #[tokio::test]
    async fn mcq_hupimwa_kiotomatiki() {
        let p = db().await;
        let e = exam(&p).await;
        let q = uid();
        sqlx::query(
            "INSERT INTO ep_exam_questions (id,exam_id,n,kind,prompt,options,answer_key,marks,created_at)
             VALUES (?1,?2,1,'mcq','Ugumu','[\"O(log n)\",\"O(n)\",\"O(n^2)\"]','A',5.0,?3)",
        ).bind(&q).bind(&e).bind(now()).execute(&p).await.unwrap();
        let a = attempt(&p, &e).await;
        sqlx::query(
            "INSERT INTO ep_exam_answers (id,attempt_id,question_id,body,locked,saved_at) VALUES (?1,?2,?3,'A',1,?4)",
        ).bind(uid()).bind(&a).bind(&q).bind(now()).execute(&p).await.unwrap();

        let opts = r#"["O(log n)","O(n)","O(n^2)"]"#;
        assert_eq!(mcq_score(opts, "A", "A", 5.0), 5.0, "herufi sahihi");
        assert_eq!(mcq_score(opts, "B", "A", 5.0), 0.0, "herufi isiyo sahihi");
        assert_eq!(mcq_score(opts, "O(log n)", "A", 5.0), 5.0, "maandishi ya chaguo");
        assert_eq!(mcq_score(opts, "O(n)", "A", 5.0), 0.0);
    }

    #[tokio::test]
    async fn violation_huongezeka_kwa_uzito_pekee() {
        let p = db().await;
        let e = exam(&p).await;
        let a = attempt(&p, &e).await;
        for (k, w) in [("photo", 0), ("copy", 2), ("devtools", 3)] {
            sqlx::query("INSERT INTO ep_proctor_events (id,attempt_id,kind,weight,at) VALUES (?1,?2,?3,?4,?5)")
                .bind(uid()).bind(&a).bind(k).bind(event_weight(k)).bind(now())
                .execute(&p).await.unwrap();
            if event_weight(k) > 0 {
                sqlx::query("UPDATE ep_exam_attempts SET violations = violations + ?1 WHERE id=?2")
                    .bind(w).bind(&a).execute(&p).await.unwrap();
            }
        }
        let (v,): (i64,) = sqlx::query_as("SELECT violations FROM ep_exam_attempts WHERE id=?1")
            .bind(&a).fetch_one(&p).await.unwrap();
        assert_eq!(v, 5, "photo=0, copy=2, devtools=3");
    }

    #[tokio::test]
    async fn attempt_mmoja_kwa_mwanafunzi_huzuiliwa_maradufu() {
        let p = db().await;
        let e = exam(&p).await;
        let _ = attempt(&p, &e).await;
        let dup = sqlx::query(
            "INSERT INTO ep_exam_attempts (id,exam_id,student_id,status,started_at) VALUES (?1,?2,'s1','active',?3)",
        ).bind(uid()).bind(&e).bind(now()).execute(&p).await;
        assert!(dup.is_err(), "UNIQUE(exam_id,student_id) inazuia maradufu");
        assert_eq!(table_count(&p, "ep_exam_attempts").await, 1);
    }

    #[tokio::test]
    async fn exam_get_hufanya_kazi_kwa_aina_zote() {
        let p = db().await;
        let e = exam(&p).await;
        // level_id ni TEXT (si int) — hii ilivunja query kimyakimya.
        sqlx::query("UPDATE ep_exams SET level_id='lv-1', instructions='fanya yote' WHERE id=?1")
            .bind(&e)
            .execute(&p)
            .await
            .unwrap();
        let row = sqlx::query_as::<_, (String, String, String, String, i64, i64, String, f64)>(
            "SELECT id,title,status,instructions,minutes,proctor,level_id,pass_mark FROM ep_exams WHERE id=?1",
        )
        .bind(&e)
        .fetch_optional(&p)
        .await;
        assert!(row.is_ok(), "query ishindwe: {:?}", row.err());
        let r = row.unwrap().unwrap();
        assert_eq!(r.6, "lv-1");
        assert_eq!(r.3, "fanya yote");
    }

    #[tokio::test]
    async fn swali_linapata_nambari_inayofuata() {
        let p = db().await;
        let e = exam(&p).await;
        for expect in 1..=3 {
            let (n,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(n),0)+1 FROM ep_exam_questions WHERE exam_id=?1")
                .bind(&e).fetch_one(&p).await.unwrap();
            assert_eq!(n, expect);
            sqlx::query("INSERT INTO ep_exam_questions (id,exam_id,n,kind,prompt,created_at) VALUES (?1,?2,?3,'text','S',?4)")
                .bind(uid()).bind(&e).bind(n).bind(now()).execute(&p).await.unwrap();
        }
    }
}
