//! teaching.rs — ELIMUPRO Awamu 2: ufundishaji.
//!
//! Notes/materiala kwa wanafunzi, assignments (individual/group) + AI-check,
//! groups + chat (WebSocket), na matangazo.
//!
//! Endpoints zote `/api/teaching/...`. AI-check hupiga LLM halisi kupitia
//! `education::llm_long`; kama LLM haipatikani hurudisha `ai_check.mode="pending"`
//! (mwalimu ahakiki) — SI kubuni alama.
//!
//! Tables: ep_notes ep_note_shares ep_assignments ep_submissions ep_groups
//!         ep_group_members ep_group_messages ep_announcements

use crate::AppState;
use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_notes (
            id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL,
            teacher_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'notes',   -- notes|slide|link|file
            url TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        // Share kwa level NZIMA (wote wa level hiyo huona).
        r#"CREATE TABLE IF NOT EXISTS ep_note_shares (
            id TEXT PRIMARY KEY,
            note_id TEXT NOT NULL,
            level_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(note_id, level_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_assignments (
            id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL,
            teacher_id TEXT NOT NULL,
            title TEXT NOT NULL,
            instructions TEXT NOT NULL DEFAULT '',
            mode TEXT NOT NULL DEFAULT 'individual',  -- individual|group
            marks REAL NOT NULL DEFAULT 20,
            due_at TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_submissions (
            id TEXT PRIMARY KEY,
            assignment_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            group_id TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL DEFAULT '',
            ai_score REAL NOT NULL DEFAULT -1,        -- -1 = haijapimwa
            ai_feedback TEXT NOT NULL DEFAULT '',
            ai_check TEXT NOT NULL DEFAULT '',        -- JSON kamili ya LLM
            score REAL NOT NULL DEFAULT -1,
            feedback TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'submitted', -- submitted|graded
            created_at TEXT NOT NULL,
            UNIQUE(assignment_id, student_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_groups (
            id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL,
            name TEXT NOT NULL,
            created_by TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_group_members (
            id TEXT PRIMARY KEY,
            group_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(group_id, student_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_group_messages (
            id TEXT PRIMARY KEY,
            group_id TEXT NOT NULL,
            author TEXT NOT NULL,
            author_type TEXT NOT NULL DEFAULT 'student', -- student|teacher
            body TEXT NOT NULL,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_announcements (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            target TEXT NOT NULL DEFAULT 'all',       -- all|course|level
            target_id TEXT NOT NULL DEFAULT '',
            author TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/teaching/status", get(status))
        // Notes
        .route("/api/teaching/note/add", post(note_add))
        .route("/api/teaching/note/share", post(note_share))
        .route("/api/teaching/notes", get(notes_list))
        // Assignments
        .route("/api/teaching/assignment/add", post(assignment_add))
        .route("/api/teaching/assignments", get(assignments_list))
        .route("/api/teaching/assignment/:id", get(assignment_get))
        .route("/api/teaching/assignment/:id/submit", post(submission_submit))
        .route("/api/teaching/assignment/:id/submissions", get(submissions_list))
        .route("/api/teaching/submission/:id/grade", post(submission_grade))
        // Groups + chat
        .route("/api/teaching/group/add", post(group_add))
        .route("/api/teaching/group/:id/member/add", post(group_member_add))
        .route("/api/teaching/groups", get(groups_list))
        .route("/api/teaching/group/:id", get(group_get))
        .route("/api/teaching/group/:id/message", post(group_message))
        .route("/api/teaching/group/:id/messages", get(group_messages))
        .route("/api/teaching/group/:id/ws", get(group_ws))
        // Matangazo
        .route("/api/teaching/announcement/add", post(announcement_add))
        .route("/api/teaching/announcements", get(announcements_list))
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

async fn table_count(db: &SqlitePool, table: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

// ============================================================
// BUS ya chat (kwa kila group)
// ============================================================

type Bus = Mutex<HashMap<String, broadcast::Sender<Value>>>;

fn bus() -> &'static Bus {
    static B: OnceLock<Bus> = OnceLock::new();
    B.get_or_init(|| Mutex::new(HashMap::new()))
}

fn subscribe(id: &str) -> broadcast::Receiver<Value> {
    let mut b = bus().lock().unwrap();
    b.entry(id.to_string())
        .or_insert_with(|| broadcast::channel(512).0)
        .subscribe()
}

fn publish(id: &str, v: Value) {
    let mut b = bus().lock().unwrap();
    let tx = b
        .entry(id.to_string())
        .or_insert_with(|| broadcast::channel(512).0);
    let _ = tx.send(v);
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "notes": table_count(&s.db, "ep_notes").await,
        "assignments": table_count(&s.db, "ep_assignments").await,
        "submissions": table_count(&s.db, "ep_submissions").await,
        "groups": table_count(&s.db, "ep_groups").await,
        "group_messages": table_count(&s.db, "ep_group_messages").await,
        "announcements": table_count(&s.db, "ep_announcements").await,
    }))
}

// ============================================================
// NOTES / MATERIALA
// ============================================================

#[derive(Deserialize)]
pub struct NoteIn {
    pub subject_id: String,
    #[serde(default)]
    pub teacher_id: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub url: String,
    /// level moja au zaidi za kushirikisha mara moja.
    #[serde(default)]
    pub level_ids: Vec<String>,
}

pub async fn note_add(State(s): State<AppState>, Json(b): Json<NoteIn>) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "kichwa cha notes linahitajika" }));
    }
    let kind = if b.kind.trim().is_empty() { "notes".to_string() } else { b.kind.trim().to_string() };
    let id = uid();
    let res = sqlx::query(
        "INSERT INTO ep_notes (id,subject_id,teacher_id,title,body,kind,url,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
    )
    .bind(&id)
    .bind(&b.subject_id)
    .bind(&b.teacher_id)
    .bind(b.title.trim())
    .bind(&b.body)
    .bind(&kind)
    .bind(&b.url)
    .bind(now())
    .execute(&s.db)
    .await;
    if let Err(e) = res {
        return Json(json!({ "ok": false, "error": format!("{e}") }));
    }
    let mut shared = 0;
    for lid in &b.level_ids {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ep_note_shares (id,note_id,level_id,created_at) VALUES (?1,?2,?3,?4)",
        )
        .bind(uid())
        .bind(&id)
        .bind(lid)
        .bind(now())
        .execute(&s.db)
        .await;
        shared += 1;
    }
    Json(json!({ "ok": true, "id": id, "shared_levels": shared }))
}

#[derive(Deserialize)]
pub struct NoteShareIn {
    pub note_id: String,
    pub level_id: String,
}

pub async fn note_share(State(s): State<AppState>, Json(b): Json<NoteShareIn>) -> Json<Value> {
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO ep_note_shares (id,note_id,level_id,created_at) VALUES (?1,?2,?3,?4)",
    )
    .bind(uid())
    .bind(&b.note_id)
    .bind(&b.level_id)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct NotesListQuery {
    #[serde(default)]
    pub subject_id: String,
    /// Kama level_id imetolewa → hurudisha notes zilizoshirikishwa level hiyo PEKEE
    /// (mtazamo wa mwanafunzi).
    #[serde(default)]
    pub level_id: String,
}

pub async fn notes_list(
    State(s): State<AppState>,
    Query(q): Query<NotesListQuery>,
) -> Json<Value> {
    let rows = if q.level_id.trim().is_empty() {
        sqlx::query_as::<_, (String, String, String, String, String, String, i64)>(
            "SELECT n.id,n.title,n.body,n.kind,n.url,n.subject_id,
                    COALESCE((SELECT COUNT(*) FROM ep_note_shares sh WHERE sh.note_id = n.id),0)
             FROM ep_notes n WHERE (?1 = '' OR n.subject_id = ?1) ORDER BY n.created_at DESC",
        )
        .bind(&q.subject_id)
        .fetch_all(&s.db)
        .await
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, String, i64)>(
            "SELECT n.id,n.title,n.body,n.kind,n.url,n.subject_id,
                    COALESCE((SELECT COUNT(*) FROM ep_note_shares sh WHERE sh.note_id = n.id),0)
             FROM ep_notes n
             JOIN ep_note_shares sh ON sh.note_id = n.id
             WHERE sh.level_id = ?1 AND (?2 = '' OR n.subject_id = ?2)
             ORDER BY n.created_at DESC",
        )
        .bind(&q.level_id)
        .bind(&q.subject_id)
        .fetch_all(&s.db)
        .await
    }
    .unwrap_or_default();
    let notes: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "title": r.1, "body": r.2, "kind": r.3, "url": r.4,
                "subject_id": r.5, "shared_levels": r.6
            })
        })
        .collect();
    Json(json!({ "ok": true, "notes": notes }))
}

// ============================================================
// ASSIGNMENTS
// ============================================================

#[derive(Deserialize)]
pub struct AssignmentIn {
    pub subject_id: String,
    #[serde(default)]
    pub teacher_id: String,
    pub title: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub marks: f64,
    #[serde(default)]
    pub due_at: String,
}

pub async fn assignment_add(
    State(s): State<AppState>,
    Json(b): Json<AssignmentIn>,
) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "kichwa cha kazi kinahitajika" }));
    }
    let mode = if b.mode == "group" { "group".to_string() } else { "individual".to_string() };
    let marks = if b.marks <= 0.0 { 20.0 } else { b.marks };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_assignments
         (id,subject_id,teacher_id,title,instructions,mode,marks,due_at,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&id)
    .bind(&b.subject_id)
    .bind(&b.teacher_id)
    .bind(b.title.trim())
    .bind(&b.instructions)
    .bind(&mode)
    .bind(marks)
    .bind(&b.due_at)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "mode": mode, "marks": marks }))
}

#[derive(Deserialize)]
pub struct AssignmentsQuery {
    #[serde(default)]
    pub subject_id: String,
}

pub async fn assignments_list(
    State(s): State<AppState>,
    Query(q): Query<AssignmentsQuery>,
) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, f64, String, String, i64)>(
        "SELECT a.id,a.title,a.instructions,a.mode,a.subject_id,a.teacher_id,a.marks,a.due_at,a.created_at,
                (SELECT COUNT(*) FROM ep_submissions sub WHERE sub.assignment_id = a.id)
         FROM ep_assignments a WHERE (?1 = '' OR a.subject_id = ?1) ORDER BY a.created_at DESC",
    )
    .bind(&q.subject_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "title": r.1, "instructions": r.2, "mode": r.3,
                "subject_id": r.4, "teacher_id": r.5, "marks": r.6, "due_at": r.7,
                "created_at": r.8, "submissions": r.9
            })
        })
        .collect();
    Json(json!({ "ok": true, "assignments": list }))
}

pub async fn assignment_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let a = sqlx::query_as::<_, (String, String, String, String, f64, String)>(
        "SELECT id,title,instructions,mode,marks,due_at FROM ep_assignments WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((aid, title, instructions, mode, marks, due)) = a else {
        return Json(json!({ "ok": false, "error": "kazi haijapatikana" }));
    };
    let subs = sqlx::query_as::<_, (String, String, String, f64, String, String)>(
        "SELECT sub.id,sub.student_id,sub.status,sub.score,COALESCE(st.full_name,sub.student_id),
                sub.created_at
         FROM ep_submissions sub LEFT JOIN ep_students st ON st.id = sub.student_id
         WHERE sub.assignment_id = ?1 ORDER BY sub.created_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "assignment": { "id": aid, "title": title, "instructions": instructions, "mode": mode, "marks": marks, "due_at": due },
        "submissions": subs.into_iter().map(|x| json!({
            "id": x.0, "student_id": x.1, "status": x.2, "score": x.3, "student": x.4, "at": x.5
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// SUBMISSIONS + AI-CHECK
// ============================================================

#[derive(Deserialize)]
pub struct SubmitIn {
    pub student_id: String,
    #[serde(default)]
    pub group_id: String,
    #[serde(default)]
    pub content: String,
}

/// AI-check: hupiga LLM ihakiki kazi dhidi ya maelekezo. Hurudisha `None`
/// kama LLM haipatikani → `mode="pending"` (mwalimu ahakiki).
async fn ai_check_submission(
    db: &SqlitePool,
    assignment_title: &str,
    instructions: &str,
    max_marks: f64,
    content: &str,
) -> Option<Value> {
    let prompt = format!(
        "Wewe ni mwalimu wa chuo nchini Tanzania. Hakiki kazi ya mwanafunzi kwa Kiswahili.\n\
         KAZI: {assignment_title}\nMAELEKEZO: {instructions}\nALAMA ZA JUU: {max_marks}\n\
         MAJIBU YA MWANAFUNZI:\n{content}\n\n\
         Rudisha JSON TU yenye: {{\"score\": namba 0-{max_marks}, \"feedback\": \"maoni kwa Kiswahili\", \
         \"ai_likelihood\": namba 0-100 (uwezekano kuwa AI iliandika), \"plagiarism_hint\": \"maoni\"}}"
    );
    let raw = crate::education::llm_long(db, &prompt, 120).await?;
    crate::education::extract_json(&raw)
}

pub async fn submission_submit(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<SubmitIn>,
) -> Json<Value> {
    if b.content.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "kazi haiwezi kuwa tupu" }));
    }
    let a = sqlx::query_as::<_, (String, String, f64)>(
        "SELECT title,instructions,marks FROM ep_assignments WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((title, instructions, marks)) = a else {
        return Json(json!({ "ok": false, "error": "kazi haijapatikana" }));
    };

    let checker = ai_check_submission(&s.db, &title, &instructions, marks, &b.content).await;
    let (ai_score, ai_feedback, ai_json, status) = match &checker {
        Some(v) => (
            v.get("score").and_then(|x| x.as_f64()).unwrap_or(-1.0),
            v.get("feedback").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            v.to_string(),
            "graded",
        ),
        None => (-1.0, String::new(), String::new(), "submitted"),
    };

    let sid = uid();
    // Upsert: mwanafunzi mmoja huweka jibu MOJA kwa kazi (hurudia = husahihisha).
    let res = sqlx::query(
        "INSERT INTO ep_submissions
         (id,assignment_id,student_id,group_id,content,ai_score,ai_feedback,ai_check,score,feedback,status,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(assignment_id, student_id) DO UPDATE SET
           content=excluded.content, group_id=excluded.group_id,
           ai_score=excluded.ai_score, ai_feedback=excluded.ai_feedback, ai_check=excluded.ai_check,
           score=excluded.score, feedback=excluded.feedback, status=excluded.status,
           created_at=excluded.created_at",
    )
    .bind(&sid)
    .bind(&id)
    .bind(&b.student_id)
    .bind(&b.group_id)
    .bind(&b.content)
    .bind(ai_score)
    .bind(&ai_feedback)
    .bind(&ai_json)
    .bind(if status == "graded" { ai_score } else { -1.0 })
    .bind(&ai_feedback)
    .bind(status)
    .bind(now())
    .execute(&s.db)
    .await;
    if let Err(e) = res {
        return Json(json!({ "ok": false, "error": format!("{e}") }));
    }
    Json(json!({
        "ok": true, "id": sid, "status": status,
        "ai_score": ai_score, "ai_feedback": ai_feedback,
        "ai_mode": if checker.is_some() { "graded" } else { "pending" }
    }))
}

pub async fn submissions_list(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, f64, String, f64, String, String)>(
        "SELECT sub.id,sub.student_id,COALESCE(st.full_name,sub.student_id),sub.ai_score,
                sub.ai_feedback,sub.score,sub.feedback,sub.status
         FROM ep_submissions sub LEFT JOIN ep_students st ON st.id = sub.student_id
         WHERE sub.assignment_id = ?1 ORDER BY sub.created_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "student_id": r.1, "student": r.2, "ai_score": r.3,
                "ai_feedback": r.4, "score": r.5, "feedback": r.6, "status": r.7
            })
        })
        .collect();
    Json(json!({ "ok": true, "submissions": list }))
}

#[derive(Deserialize)]
pub struct GradeIn {
    pub score: f64,
    #[serde(default)]
    pub feedback: String,
}

pub async fn submission_grade(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<GradeIn>,
) -> Json<Value> {
    let res = sqlx::query(
        "UPDATE ep_submissions SET score = ?1, feedback = ?2, status = 'graded' WHERE id = ?3",
    )
    .bind(b.score)
    .bind(&b.feedback)
    .bind(&id)
    .execute(&s.db)
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => Json(json!({ "ok": true, "score": b.score })),
        _ => Json(json!({ "ok": false, "error": "jibu haijapatikana" })),
    }
}

// ============================================================
// GROUPS + CHAT
// ============================================================

#[derive(Deserialize)]
pub struct GroupIn {
    pub subject_id: String,
    pub name: String,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub member_ids: Vec<String>,
}

pub async fn group_add(State(s): State<AppState>, Json(b): Json<GroupIn>) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la kikundi linahitajika" }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_groups (id,subject_id,name,created_by,created_at) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&id)
    .bind(&b.subject_id)
    .bind(b.name.trim())
    .bind(&b.created_by)
    .bind(now())
    .execute(&s.db)
    .await;
    let mut n = 0;
    for m in &b.member_ids {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ep_group_members (id,group_id,student_id,created_at) VALUES (?1,?2,?3,?4)",
        )
        .bind(uid())
        .bind(&id)
        .bind(m)
        .bind(now())
        .execute(&s.db)
        .await;
        n += 1;
    }
    Json(json!({ "ok": true, "id": id, "members": n }))
}

#[derive(Deserialize)]
pub struct MemberIn {
    pub student_id: String,
}

pub async fn group_member_add(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<MemberIn>,
) -> Json<Value> {
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO ep_group_members (id,group_id,student_id,created_at) VALUES (?1,?2,?3,?4)",
    )
    .bind(uid())
    .bind(&id)
    .bind(&b.student_id)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true }))
}

pub async fn groups_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT g.id,g.name,g.subject_id,
                (SELECT COUNT(*) FROM ep_group_members m WHERE m.group_id = g.id)
         FROM ep_groups g ORDER BY g.created_at",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "id": r.0, "name": r.1, "subject_id": r.2, "members": r.3 }))
        .collect();
    Json(json!({ "ok": true, "groups": list }))
}

pub async fn group_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let g = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,name,subject_id FROM ep_groups WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((gid, name, subject)) = g else {
        return Json(json!({ "ok": false, "error": "kikundi hakijapatikana" }));
    };
    let members = sqlx::query_as::<_, (String, String)>(
        "SELECT m.student_id,COALESCE(st.full_name,m.student_id)
         FROM ep_group_members m LEFT JOIN ep_students st ON st.id = m.student_id
         WHERE m.group_id = ?1 ORDER BY st.full_name",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "group": { "id": gid, "name": name, "subject_id": subject },
        "members": members.into_iter().map(|x| json!({ "id": x.0, "name": x.1 })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct MessageIn {
    pub author: String,
    #[serde(default)]
    pub author_type: String,
    pub body: String,
}

pub async fn group_message(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<MessageIn>,
) -> Json<Value> {
    if b.body.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "ujumbe hautumwa tupu" }));
    }
    let atype = if b.author_type == "teacher" { "teacher" } else { "student" };
    let mid = uid();
    let at = now();
    let _ = sqlx::query(
        "INSERT INTO ep_group_messages (id,group_id,author,author_type,body,created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&mid)
    .bind(&id)
    .bind(&b.author)
    .bind(atype)
    .bind(b.body.trim())
    .bind(&at)
    .execute(&s.db)
    .await;
    let v = json!({
        "type": "message", "id": mid, "group_id": id, "author": b.author,
        "author_type": atype, "body": b.body.trim(), "at": at
    });
    publish(&id, v.clone());
    Json(json!({ "ok": true, "message": v }))
}

pub async fn group_messages(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,author,author_type,body,created_at FROM ep_group_messages
         WHERE group_id = ?1 ORDER BY created_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "id": r.0, "author": r.1, "author_type": r.2, "body": r.3, "at": r.4 }))
        .collect();
    Json(json!({ "ok": true, "messages": list }))
}

pub async fn group_ws(ws: WebSocketUpgrade, Path(id): Path<String>) -> Response {
    ws.on_upgrade(move |socket| handle_group_ws(socket, id))
}

async fn handle_group_ws(mut socket: WebSocket, id: String) {
    let mut rx = subscribe(&id);
    let hello = json!({ "type": "hello", "group_id": id, "at": now() });
    if socket.send(WsMessage::Text(hello.to_string())).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Ok(v) => {
                    if socket.send(WsMessage::Text(v.to_string())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            },
            msg = socket.recv() => match msg {
                Some(Ok(WsMessage::Ping(p))) => {
                    if socket.send(WsMessage::Pong(p)).await.is_err() { break; }
                }
                Some(Ok(WsMessage::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            }
        }
    }
}

// ============================================================
// MATANGAZO
// ============================================================

#[derive(Deserialize)]
pub struct AnnouncementIn {
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub target_id: String,
    #[serde(default)]
    pub author: String,
}

/// Walengwa wa tangazo: wote, wanafunzi, walimu, level au course.
pub fn norm_announce_target(t: &str) -> &'static str {
    match t.trim().to_lowercase().as_str() {
        "students" | "wanafunzi" => "students",
        "teachers" | "walimu" => "teachers",
        "level" => "level",
        "course" => "course",
        _ => "all",
    }
}

/// Tangazo hili humfikia mlengwa huyu?
pub fn announce_reaches(target: &str, target_id: &str, audience: &str, ctx_id: &str) -> bool {
    match norm_announce_target(target) {
        "all" => true,
        "students" | "teachers" => norm_announce_target(audience) == norm_announce_target(target),
        t => t == norm_announce_target(target) && target_id == ctx_id,
    }
}

pub async fn announcement_add(
    State(s): State<AppState>,
    Json(b): Json<AnnouncementIn>,
) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "kichwa cha tangazo linahitajika" }));
    }
    let target = norm_announce_target(&b.target).to_string();
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_announcements (id,title,body,target,target_id,author,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(b.title.trim())
    .bind(&b.body)
    .bind(&target)
    .bind(&b.target_id)
    .bind(&b.author)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "target": target }))
}

#[derive(Deserialize)]
pub struct AnnouncementsQuery {
    /// Kama imetolewa → hurudisha matangazo ya `all` + yale ya level/course hiyo
    /// (mtazamo wa mwanafunzi).
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub course_id: String,
}

pub async fn announcements_list(
    State(s): State<AppState>,
    Query(q): Query<AnnouncementsQuery>,
) -> Json<Value> {
    let rows = if q.level_id.trim().is_empty() && q.course_id.trim().is_empty() {
        sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id,title,body,target,target_id,created_at FROM ep_announcements ORDER BY created_at DESC",
        )
        .fetch_all(&s.db)
        .await
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id,title,body,target,target_id,created_at FROM ep_announcements a
             WHERE a.target = 'all'
                OR (a.target = 'level' AND a.target_id = ?1)
                OR (a.target = 'course' AND a.target_id = ?2)
             ORDER BY a.created_at DESC",
        )
        .bind(&q.level_id)
        .bind(&q.course_id)
        .fetch_all(&s.db)
        .await
    }
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "title": r.1, "body": r.2, "target": r.3,
                "target_id": r.4, "created_at": r.5
            })
        })
        .collect();
    Json(json!({ "ok": true, "announcements": list }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("tp_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let url = format!("sqlite://{}?mode=rwc", dir.join("t.db").display());
        let db = SqlitePool::connect(&url).await.unwrap();
        init_tables(&db).await;
        db
    }

    #[tokio::test]
    async fn init_tables_huunda_majedwali_yote() {
        let db = test_db().await;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name LIKE 'ep_%'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(n, 8, "majedwali 8 ya ufundishaji yanatakiwa");
    }

    #[tokio::test]
    async fn notes_hufichwa_kwa_level_isiyoshirikishwa() {
        let db = test_db().await;
        sqlx::query(
            "INSERT INTO ep_notes (id,subject_id,teacher_id,title,body,kind,url,created_at)
             VALUES ('n1','s1','t1','Algebra','','notes','',?1)",
        )
        .bind(now())
        .execute(&db)
        .await
        .unwrap();
        sqlx::query("INSERT INTO ep_note_shares (id,note_id,level_id,created_at) VALUES ('sh1','n1','L1',?1)")
            .bind(now())
            .execute(&db)
            .await
            .unwrap();
        let (c2,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ep_notes n JOIN ep_note_shares sh ON sh.note_id = n.id WHERE sh.level_id = 'L2'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(c2, 0);
        let (c1,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ep_notes n JOIN ep_note_shares sh ON sh.note_id = n.id WHERE sh.level_id = 'L1'",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(c1, 1);
    }

    #[tokio::test]
    async fn submission_upsert_hurudia_kwa_mwanafunzi_mmoja() {
        let db = test_db().await;
        sqlx::query(
            "INSERT INTO ep_assignments (id,subject_id,teacher_id,title,instructions,mode,marks,due_at,created_at)
             VALUES ('a1','s1','t1','Kazi 1','','individual',20,'',?1)",
        )
        .bind(now())
        .execute(&db)
        .await
        .unwrap();
        for (id, content) in [("sub1", "jibu la kwanza"), ("sub2", "jibu la pili")] {
            sqlx::query(
                "INSERT INTO ep_submissions (id,assignment_id,student_id,content,created_at)
                 VALUES (?1,'a1','st1',?2,?3)
                 ON CONFLICT(assignment_id, student_id) DO UPDATE SET content=excluded.content",
            )
            .bind(id)
            .bind(content)
            .bind(now())
            .execute(&db)
            .await
            .unwrap();
        }
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_submissions WHERE assignment_id='a1'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n, 1, "mwanafunzi mmoja = jibu moja");
        let (c,): (String,) = sqlx::query_as("SELECT content FROM ep_submissions WHERE assignment_id='a1'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(c, "jibu la pili", "jibu la mwisho hushinda");
    }

    #[tokio::test]
    async fn matangazo_huchuja_kwa_level_na_all() {
        let db = test_db().await;
        for (id, target, tid) in [("an1", "all", ""), ("an2", "level", "L1"), ("an3", "level", "L2")] {
            sqlx::query(
                "INSERT INTO ep_announcements (id,title,body,target,target_id,author,created_at)
                 VALUES (?1,'T','',?2,?3,'admin',?4)",
            )
            .bind(id)
            .bind(target)
            .bind(tid)
            .bind(now())
            .execute(&db)
            .await
            .unwrap();
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ep_announcements a WHERE a.target='all'
             OR (a.target='level' AND a.target_id='L1')",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(n, 2);
    }

    #[tokio::test]
    async fn group_members_hawaongezwi_maradufu() {
        let db = test_db().await;
        sqlx::query("INSERT INTO ep_groups (id,subject_id,name,created_by,created_at) VALUES ('g1','s1','Kikundi A','t1',?1)")
            .bind(now())
            .execute(&db)
            .await
            .unwrap();
        for _ in 0..2 {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO ep_group_members (id,group_id,student_id,created_at) VALUES (?1,'g1','st1',?2)",
            )
            .bind(uid())
            .bind(now())
            .execute(&db)
            .await;
        }
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_group_members WHERE group_id='g1'")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn walengwa_wa_tangazo_hupangwa() {
        assert_eq!(norm_announce_target("wanafunzi"), "students");
        assert_eq!(norm_announce_target("walimu"), "teachers");
        assert_eq!(norm_announce_target("level"), "level");
        assert_eq!(norm_announce_target("course"), "course");
        assert_eq!(norm_announce_target("kitu kingine"), "all");
        assert_eq!(norm_announce_target(""), "all");
    }

    #[test]
    fn tangazo_humfikia_mlengwa_sahihi() {
        // `all` humfikia kila mtu.
        assert!(announce_reaches("all", "", "students", ""));
        assert!(announce_reaches("all", "", "teachers", ""));
        // Wanafunzi/walimu hutenganishwa.
        assert!(announce_reaches("students", "", "students", ""));
        assert!(!announce_reaches("students", "", "teachers", ""));
        assert!(announce_reaches("teachers", "", "teachers", ""));
        assert!(!announce_reaches("teachers", "", "students", ""));
        // Level/course hutegemea ID.
        assert!(announce_reaches("level", "L1", "", "L1"));
        assert!(!announce_reaches("level", "L1", "", "L2"));
        assert!(announce_reaches("course", "C1", "", "C1"));
    }
}
