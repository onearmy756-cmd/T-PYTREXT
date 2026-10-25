//! live.rs — ELIMUPRO Awamu 3 & 4: madarasa ya moja kwa moja na uwasilishaji.
//!
//! Moduli 9 (Live Classes) na 10 (Presentations):
//!   - Kikao (live|presentation) + washiriki (presence halisi)
//!   - WebRTC signaling (offer/answer/ICE) — relay kwa WS + polling fallback
//!   - AI Scribe: transcript ya kila anayesema (kutoka Web Speech API upande wa UI)
//!   - Muhtasari wa somo (LLM halisi; haipo → `pending`, SI kubuni)
//!   - Slides za presentation + broadcast ya slide ya sasa
//!   - Recording: blob ya sauti/video huhifadhiwa kwenye disk
//!   - Alama + maoni ya presentation kwa mwanafunzi
//!
//! Tables: ep_live_sessions ep_live_participants ep_live_signals ep_live_transcript
//!         ep_live_slides ep_live_summary ep_live_marks

use crate::AppState;
use axum::{
    body::Bytes,
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
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_live_sessions (
            id TEXT PRIMARY KEY,
            subject_id TEXT NOT NULL DEFAULT '',
            teacher_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'live',        -- live|presentation
            status TEXT NOT NULL DEFAULT 'scheduled', -- scheduled|live|ended
            presenter TEXT NOT NULL DEFAULT '',
            current_slide INTEGER NOT NULL DEFAULT 0,
            recording TEXT NOT NULL DEFAULT '',
            started_at TEXT NOT NULL DEFAULT '',
            ended_at TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_live_participants (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            peer TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'student',   -- teacher|student
            student_id TEXT NOT NULL DEFAULT '',
            joined_at TEXT NOT NULL,
            last_seen TEXT NOT NULL,
            UNIQUE(session_id, peer)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_live_signals (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            from_peer TEXT NOT NULL,
            to_peer TEXT NOT NULL,
            kind TEXT NOT NULL,
            payload TEXT NOT NULL,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_live_transcript (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            speaker TEXT NOT NULL,
            text TEXT NOT NULL,
            at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_live_slides (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            page INTEGER NOT NULL DEFAULT 0,
            title TEXT NOT NULL DEFAULT '',
            data TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_live_summary (
            session_id TEXT PRIMARY KEY,
            summary TEXT NOT NULL DEFAULT '',
            mode TEXT NOT NULL DEFAULT 'pending',  -- llm|pending
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_live_marks (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            score REAL NOT NULL DEFAULT -1,
            feedback TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            UNIQUE(session_id, student_id)
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/live/status", get(status))
        .route("/api/live/session/create", post(session_create))
        .route("/api/live/sessions", get(sessions_list))
        .route("/api/live/session/:id", get(session_get))
        .route("/api/live/session/:id/start", post(session_start))
        .route("/api/live/session/:id/end", post(session_end))
        .route("/api/live/session/:id/join", post(session_join))
        .route("/api/live/session/:id/leave", post(session_leave))
        .route("/api/live/session/:id/peers", get(session_peers))
        .route("/api/live/session/:id/signal", post(signal_post))
        .route("/api/live/session/:id/signals", get(signals_drain))
        .route("/api/live/session/:id/transcript", post(transcript_add))
        .route("/api/live/session/:id/transcript/list", get(transcript_list))
        .route("/api/live/session/:id/summary", post(summary_make))
        .route("/api/live/session/:id/summary/get", get(summary_get))
        .route("/api/live/session/:id/slide", post(slide_add))
        .route("/api/live/session/:id/slides", get(slides_list))
        .route("/api/live/session/:id/slide/go", post(slide_go))
        .route("/api/live/session/:id/mark", post(mark_set))
        .route("/api/live/session/:id/marks", get(marks_list))
        .route("/api/live/session/:id/record", post(record_save))
        .route("/api/live/session/:id/recording", get(record_get))
        .route("/api/live/session/:id/ws", get(session_ws))
        .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))
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

fn live_dir() -> PathBuf {
    let d = crate::data_io::data_dir().join("live");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn peer_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..10].to_string()
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

// ============================================================
// BUS
// ============================================================

type Bus = Mutex<HashMap<String, broadcast::Sender<Value>>>;

fn bus() -> &'static Bus {
    static B: OnceLock<Bus> = OnceLock::new();
    B.get_or_init(|| Mutex::new(HashMap::new()))
}

fn subscribe(id: &str) -> broadcast::Receiver<Value> {
    let mut b = bus().lock().unwrap();
    b.entry(id.to_string())
        .or_insert_with(|| broadcast::channel(1024).0)
        .subscribe()
}

fn publish(id: &str, v: Value) {
    let mut b = bus().lock().unwrap();
    let tx = b
        .entry(id.to_string())
        .or_insert_with(|| broadcast::channel(1024).0);
    let _ = tx.send(v);
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "sessions": table_count(&s.db, "ep_live_sessions").await,
        "live_now": sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ep_live_sessions WHERE status='live'")
            .fetch_one(&s.db).await.map(|r| r.0).unwrap_or(0),
        "participants": table_count(&s.db, "ep_live_participants").await,
        "transcript_lines": table_count(&s.db, "ep_live_transcript").await,
        "slides": table_count(&s.db, "ep_live_slides").await,
    }))
}

// ============================================================
// SESSIONS
// ============================================================

#[derive(Deserialize)]
pub struct SessionIn {
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub teacher_id: String,
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub presenter: String,
}

pub async fn session_create(State(s): State<AppState>, Json(b): Json<SessionIn>) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "kichwa cha kikao kinahitajika" }));
    }
    let kind = if b.kind == "presentation" { "presentation" } else { "live" };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_live_sessions (id,subject_id,teacher_id,title,kind,status,presenter,created_at)
         VALUES (?1,?2,?3,?4,?5,'scheduled',?6,?7)",
    )
    .bind(&id)
    .bind(&b.subject_id)
    .bind(&b.teacher_id)
    .bind(b.title.trim())
    .bind(kind)
    .bind(&b.presenter)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "kind": kind }))
}

#[derive(Deserialize)]
pub struct SessionsQuery {
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub kind: String,
}

pub async fn sessions_list(
    State(s): State<AppState>,
    Query(q): Query<SessionsQuery>,
) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, i64)>(
        "SELECT id,title,kind,status,subject_id,started_at,
                (SELECT COUNT(*) FROM ep_live_participants p WHERE p.session_id = s.id)
         FROM ep_live_sessions s
         WHERE (?1 = '' OR subject_id = ?1) AND (?2 = '' OR kind = ?2)
         ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.subject_id)
    .bind(&q.kind)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "title": r.1, "kind": r.2, "status": r.3,
                "subject_id": r.4, "started_at": r.5, "participants": r.6
            })
        })
        .collect();
    Json(json!({ "ok": true, "sessions": list }))
}

pub async fn session_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, i64, String)>(
        "SELECT id,title,kind,status,subject_id,presenter,started_at,ended_at,recording,current_slide,
                COALESCE((SELECT mode FROM ep_live_summary m WHERE m.session_id = s.id),'')
         FROM ep_live_sessions s WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({ "ok": false, "error": "kikao hakijapatikana" }));
    };
    let peers = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT peer,name,role,student_id FROM ep_live_participants WHERE session_id = ?1 ORDER BY joined_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "session": {
            "id": r.0, "title": r.1, "kind": r.2, "status": r.3, "subject_id": r.4,
            "presenter": r.5, "started_at": r.6, "ended_at": r.7, "recording": r.8,
            "current_slide": r.9, "summary_mode": r.10
        },
        "peers": peers.into_iter().map(|p| json!({
            "peer": p.0, "name": p.1, "role": p.2, "student_id": p.3
        })).collect::<Vec<_>>()
    }))
}

pub async fn session_start(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let at = now();
    let r = sqlx::query("UPDATE ep_live_sessions SET status='live', started_at=?1 WHERE id=?2")
        .bind(&at)
        .bind(&id)
        .execute(&s.db)
        .await;
    match r {
        Ok(x) if x.rows_affected() > 0 => {
            publish(&id, json!({ "type": "status", "status": "live", "at": at }));
            Json(json!({ "ok": true, "status": "live", "at": at }))
        }
        _ => Json(json!({ "ok": false, "error": "kikao hakijapatikana" })),
    }
}

/// Kumaliza kikao: huzalisha muhtasari wa AI kutoka transcript.
pub async fn session_end(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let at = now();
    let _ = sqlx::query("UPDATE ep_live_sessions SET status='ended', ended_at=?1 WHERE id=?2")
        .bind(&at)
        .bind(&id)
        .execute(&s.db)
        .await;
    let sum = build_summary(&s.db, &id).await;
    publish(&id, json!({ "type": "status", "status": "ended", "at": at }));
    Json(json!({ "ok": true, "status": "ended", "summary": sum }))
}

// ============================================================
// PRESENCE
// ============================================================

#[derive(Deserialize)]
pub struct JoinIn {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub student_id: String,
}

pub async fn session_join(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<JoinIn>) -> Json<Value> {
    let exists = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ep_live_sessions WHERE id=?1")
        .bind(&id)
        .fetch_one(&s.db)
        .await
        .map(|r| r.0)
        .unwrap_or(0);
    if exists == 0 {
        return Json(json!({ "ok": false, "error": "kikao hakijapatikana" }));
    }
    let peer = peer_id();
    let role = if b.role == "teacher" { "teacher" } else { "student" };
    let name = if b.name.trim().is_empty() { "Mshiriki".to_string() } else { b.name.trim().to_string() };
    let at = now();
    let _ = sqlx::query(
        "INSERT INTO ep_live_participants (id,session_id,peer,name,role,student_id,joined_at,last_seen)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7)",
    )
    .bind(uid())
    .bind(&id)
    .bind(&peer)
    .bind(&name)
    .bind(role)
    .bind(&b.student_id)
    .bind(&at)
    .execute(&s.db)
    .await;
    publish(&id, json!({
        "type": "peer_join", "peer": peer, "name": name, "role": role, "at": at
    }));
    Json(json!({ "ok": true, "peer": peer, "role": role, "name": name }))
}

#[derive(Deserialize)]
pub struct PeerIn {
    pub peer: String,
}

pub async fn session_leave(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<PeerIn>) -> Json<Value> {
    let _ = sqlx::query("DELETE FROM ep_live_participants WHERE session_id=?1 AND peer=?2")
        .bind(&id)
        .bind(&b.peer)
        .execute(&s.db)
        .await;
    publish(&id, json!({ "type": "peer_leave", "peer": b.peer, "at": now() }));
    Json(json!({ "ok": true }))
}

pub async fn session_peers(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT peer,name,role FROM ep_live_participants WHERE session_id=?1 ORDER BY joined_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "peers": rows.into_iter().map(|r| json!({ "peer": r.0, "name": r.1, "role": r.2 })).collect::<Vec<_>>()
    }))
}

// ============================================================
// WEBRTC SIGNALING
// ============================================================

#[derive(Deserialize)]
pub struct SignalIn {
    pub from_peer: String,
    pub to_peer: String,
    pub kind: String,
    pub payload: Value,
}

pub async fn signal_post(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<SignalIn>) -> Json<Value> {
    let sid = uid();
    let v = json!({
        "type": "signal", "id": sid, "from": b.from_peer, "to": b.to_peer,
        "kind": b.kind, "payload": b.payload, "at": now()
    });
    let _ = sqlx::query(
        "INSERT INTO ep_live_signals (id,session_id,from_peer,to_peer,kind,payload,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&sid)
    .bind(&id)
    .bind(&b.from_peer)
    .bind(&b.to_peer)
    .bind(&b.kind)
    .bind(b.payload.to_string())
    .bind(now())
    .execute(&s.db)
    .await;
    // Broadcast kwa kila mtu; mteja huchagua zile za `to` yake.
    publish(&id, v);
    Json(json!({ "ok": true, "id": sid }))
}

#[derive(Deserialize)]
pub struct SignalsQuery {
    pub peer: String,
}

/// Polling fallback: hurudisha signaling zilizoelekezwa kwa `peer`, kisha
/// huzifuta (drain) ili zisirudi tena.
pub async fn signals_drain(State(s): State<AppState>, Path(id): Path<String>, Query(q): Query<SignalsQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,from_peer,kind,payload,created_at FROM ep_live_signals
         WHERE session_id=?1 AND to_peer=?2 ORDER BY created_at",
    )
    .bind(&id)
    .bind(&q.peer)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let list: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "from": r.1, "kind": r.2,
                "payload": serde_json::from_str::<Value>(&r.3).unwrap_or(Value::Null),
                "at": r.4
            })
        })
        .collect();
    let _ = sqlx::query("DELETE FROM ep_live_signals WHERE session_id=?1 AND to_peer=?2")
        .bind(&id)
        .bind(&q.peer)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "signals": list }))
}

// ============================================================
// AI SCRIBE (transcript) + MUHTASARI
// ============================================================

#[derive(Deserialize)]
pub struct TranscriptIn {
    pub speaker: String,
    pub text: String,
}

pub async fn transcript_add(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<TranscriptIn>) -> Json<Value> {
    if b.text.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "maandishi matupu" }));
    }
    let tid = uid();
    let at = now();
    let _ = sqlx::query(
        "INSERT INTO ep_live_transcript (id,session_id,speaker,text,at) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&tid)
    .bind(&id)
    .bind(&b.speaker)
    .bind(b.text.trim())
    .bind(&at)
    .execute(&s.db)
    .await;
    let v = json!({ "type": "transcript", "id": tid, "speaker": b.speaker, "text": b.text.trim(), "at": at });
    publish(&id, v.clone());
    Json(json!({ "ok": true, "line": v }))
}

pub async fn transcript_list(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT speaker,text,at FROM ep_live_transcript WHERE session_id=?1 ORDER BY at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "lines": rows.into_iter().map(|r| json!({ "speaker": r.0, "text": r.1, "at": r.2 })).collect::<Vec<_>>()
    }))
}

/// Huunda muhtasari kutoka transcript kwa LLM halisi. Bila LLM → mode `pending`
/// (mwalimu aandike), SI kubuni muhtasari.
async fn build_summary(db: &SqlitePool, session_id: &str) -> Value {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT speaker,text FROM ep_live_transcript WHERE session_id=?1 ORDER BY at",
    )
    .bind(session_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let transcript: String = rows
        .iter()
        .map(|r| format!("{}: {}", r.0, r.1))
        .collect::<Vec<_>>()
        .join("\n");
    if transcript.trim().is_empty() {
        return json!({ "ok": false, "mode": "pending", "error": "hakuna transcript bado" });
    }
    let prompt = format!(
        "Haya ni maneno ya somo la darasa. Andika muhtasari mfupi wa Kiswahili wenye \
         hoja kuu 5-8 na maswali 3 ya mazoezi.\n\nTRANSCRIPT:\n{transcript}\n\n\
         Rudisha JSON TU: {{\"muhtasari\":\"...\",\"hoja\":[\"...\"],\"maswali\":[\"...\"]}}"
    );
    let out = crate::education::llm_long(db, &prompt, 120).await;
    let parsed = out.as_deref().and_then(crate::education::extract_json);
    match parsed {
        Some(v) => {
            let _ = sqlx::query(
                "INSERT INTO ep_live_summary (session_id,summary,mode,created_at) VALUES (?1,?2,'llm',?3)
                 ON CONFLICT(session_id) DO UPDATE SET summary=excluded.summary, mode='llm', created_at=excluded.created_at",
            )
            .bind(session_id)
            .bind(v.to_string())
            .bind(now())
            .execute(db)
            .await;
            json!({ "ok": true, "mode": "llm", "summary": v })
        }
        None => {
            let _ = sqlx::query(
                "INSERT INTO ep_live_summary (session_id,summary,mode,created_at) VALUES (?1,'','pending',?2)
                 ON CONFLICT(session_id) DO UPDATE SET mode='pending', created_at=excluded.created_at",
            )
            .bind(session_id)
            .bind(now())
            .execute(db)
            .await;
            json!({ "ok": false, "mode": "pending", "error": "LLM haikujibu — mwalimu aandike muhtasari" })
        }
    }
}

pub async fn summary_make(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    Json(build_summary(&s.db, &id).await)
}

pub async fn summary_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT summary,mode,created_at FROM ep_live_summary WHERE session_id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match row {
        Some((summary, mode, at)) => Json(json!({
            "ok": true, "mode": mode, "at": at,
            "summary": serde_json::from_str::<Value>(&summary).unwrap_or(Value::Null)
        })),
        None => Json(json!({ "ok": false, "error": "hakuna muhtasari bado" })),
    }
}

// ============================================================
// SLIDES (PRESENTATION)
// ============================================================

#[derive(Deserialize)]
pub struct SlideIn {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub data: String,
}

pub async fn slide_add(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<SlideIn>) -> Json<Value> {
    let (n,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(page),0)+1 FROM ep_live_slides WHERE session_id=?1")
        .bind(&id)
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let sid = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_live_slides (id,session_id,page,title,data,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&sid)
    .bind(&id)
    .bind(n)
    .bind(&b.title)
    .bind(&b.data)
    .bind(now())
    .execute(&s.db)
    .await;
    publish(&id, json!({ "type": "slide_add", "page": n, "title": b.title }));
    Json(json!({ "ok": true, "id": sid, "page": n }))
}

pub async fn slides_list(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT page,title,data FROM ep_live_slides WHERE session_id=?1 ORDER BY page",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "slides": rows.into_iter().map(|r| json!({ "page": r.0, "title": r.1, "data": r.2 })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct SlideGoIn {
    pub page: i64,
}

pub async fn slide_go(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<SlideGoIn>) -> Json<Value> {
    let _ = sqlx::query("UPDATE ep_live_sessions SET current_slide=?1 WHERE id=?2")
        .bind(b.page)
        .bind(&id)
        .execute(&s.db)
        .await;
    publish(&id, json!({ "type": "slide_go", "page": b.page, "at": now() }));
    Json(json!({ "ok": true, "page": b.page }))
}

// ============================================================
// ALAMA ZA PRESENTATION
// ============================================================

#[derive(Deserialize)]
pub struct MarkIn {
    pub student_id: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub feedback: String,
}

pub async fn mark_set(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<MarkIn>) -> Json<Value> {
    let _ = sqlx::query(
        "INSERT INTO ep_live_marks (id,session_id,student_id,score,feedback,created_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(session_id, student_id) DO UPDATE SET score=excluded.score, feedback=excluded.feedback",
    )
    .bind(uid())
    .bind(&id)
    .bind(&b.student_id)
    .bind(b.score)
    .bind(&b.feedback)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true }))
}

pub async fn marks_list(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, f64, String, String)>(
        "SELECT m.student_id,m.score,m.feedback,COALESCE(st.full_name,m.student_id)
         FROM ep_live_marks m LEFT JOIN ep_students st ON st.id = m.student_id
         WHERE m.session_id=?1",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "marks": rows.into_iter().map(|r| json!({
            "student_id": r.0, "score": r.1, "feedback": r.2, "student": r.3
        })).collect::<Vec<_>>()
    }))
}

// ============================================================
// RECORDING
// ============================================================

pub async fn record_save(State(s): State<AppState>, Path(id): Path<String>, body: Bytes) -> Json<Value> {
    if body.is_empty() {
        return Json(json!({ "ok": false, "error": "recording tupu" }));
    }
    let name = format!("{id}.webm");
    let path = live_dir().join(&name);
    if std::fs::write(&path, &body).is_err() {
        return Json(json!({ "ok": false, "error": "imeshindwa kuandika recording" }));
    }
    let _ = sqlx::query("UPDATE ep_live_sessions SET recording=?1 WHERE id=?2")
        .bind(&name)
        .bind(&id)
        .execute(&s.db)
        .await;
    publish(&id, json!({ "type": "recording", "name": name, "bytes": body.len() }));
    Json(json!({ "ok": true, "name": name, "bytes": body.len() }))
}

pub async fn record_get(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    use axum::http::{header, StatusCode};
    let name = sqlx::query_as::<_, (String,)>("SELECT recording FROM ep_live_sessions WHERE id=?1")
        .bind(&id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_default();
    if name.is_empty() || name.contains('/') || name.contains("..") {
        return (StatusCode::NOT_FOUND, "hakuna recording").into_response();
    }
    let Ok(bytes) = std::fs::read(live_dir().join(&name)) else {
        return (StatusCode::NOT_FOUND, "faili haipo").into_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "video/webm")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(axum::body::Body::from(bytes))
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "kosa").into_response())
}

// ============================================================
// WS
// ============================================================

pub async fn session_ws(ws: WebSocketUpgrade, Path(id): Path<String>) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, id))
}

async fn handle_ws(mut socket: WebSocket, id: String) {
    let mut rx = subscribe(&id);
    let hello = json!({ "type": "hello", "session_id": id, "at": now() });
    if socket.send(WsMessage::Text(hello.to_string())).await.is_err() {
        return;
    }
    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Ok(v) => {
                    if socket.send(WsMessage::Text(v.to_string())).await.is_err() { break; }
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
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    async fn session(db: &SqlitePool, kind: &str) -> String {
        let id = uid();
        sqlx::query(
            "INSERT INTO ep_live_sessions (id,title,kind,status,created_at) VALUES (?1,'Somo',?2,'scheduled',?3)",
        )
        .bind(&id)
        .bind(kind)
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
            "ep_live_sessions",
            "ep_live_participants",
            "ep_live_signals",
            "ep_live_transcript",
            "ep_live_slides",
            "ep_live_summary",
            "ep_live_marks",
        ] {
            let n = table_count(&p, t).await;
            assert_eq!(n, 0, "{t} haipo");
        }
    }

    #[tokio::test]
    async fn transcript_huheshimu_mfululizo_wa_muda() {
        let p = db().await;
        let sid = session(&p, "live").await;
        for (i, t) in ["Kwanza", "Pili", "Tatu"].iter().enumerate() {
            sqlx::query("INSERT INTO ep_live_transcript (id,session_id,speaker,text,at) VALUES (?1,?2,'Mwalimu',?3,?4)")
                .bind(uid())
                .bind(&sid)
                .bind(*t)
                .bind(format!("2026-10-21T10:0{i}:00+00:00"))
                .execute(&p)
                .await
                .unwrap();
        }
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT text FROM ep_live_transcript WHERE session_id=?1 ORDER BY at",
        )
        .bind(&sid)
        .fetch_all(&p)
        .await
        .unwrap();
        assert_eq!(rows[0].0, "Kwanza");
        assert_eq!(rows[2].0, "Tatu");
    }

    #[tokio::test]
    async fn slide_mpya_hupata_page_inayofuata() {
        let p = db().await;
        let sid = session(&p, "presentation").await;
        for expect in 1..=3 {
            let (n,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(page),0)+1 FROM ep_live_slides WHERE session_id=?1")
                .bind(&sid)
                .fetch_one(&p)
                .await
                .unwrap();
            assert_eq!(n, expect);
            sqlx::query("INSERT INTO ep_live_slides (id,session_id,page,title,data,created_at) VALUES (?1,?2,?3,'S','d',?4)")
                .bind(uid())
                .bind(&sid)
                .bind(n)
                .bind(now())
                .execute(&p)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn mshiriki_mmoja_huzuiliwa_kujiunga_maradufu() {
        let p = db().await;
        let sid = session(&p, "live").await;
        let peer = "peer-1";
        for _ in 0..2 {
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO ep_live_participants (id,session_id,peer,name,role,student_id,joined_at,last_seen)
                 VALUES (?1,?2,?3,'A','student','',?4,?4)",
            )
            .bind(uid())
            .bind(&sid)
            .bind(peer)
            .bind(now())
            .execute(&p)
            .await;
        }
        assert_eq!(table_count(&p, "ep_live_participants").await, 1);
    }

    #[tokio::test]
    async fn alama_ya_presentation_husasishwa_si_kurudia() {
        let p = db().await;
        let sid = session(&p, "presentation").await;
        for score in [7.0, 9.5] {
            sqlx::query(
                "INSERT INTO ep_live_marks (id,session_id,student_id,score,feedback,created_at)
                 VALUES (?1,?2,'stu1',?3,'f',?4)
                 ON CONFLICT(session_id, student_id) DO UPDATE SET score=excluded.score",
            )
            .bind(uid())
            .bind(&sid)
            .bind(score)
            .bind(now())
            .execute(&p)
            .await
            .unwrap();
        }
        let (s,): (f64,) = sqlx::query_as("SELECT score FROM ep_live_marks WHERE session_id=?1")
            .bind(&sid)
            .fetch_one(&p)
            .await
            .unwrap();
        assert_eq!(s, 9.5);
    }

    #[tokio::test]
    async fn signaling_huchukuliwa_kwa_peer_pekee_kisha_kufutwa() {
        let p = db().await;
        let sid = session(&p, "live").await;
        for to in ["a", "b"] {
            sqlx::query(
                "INSERT INTO ep_live_signals (id,session_id,from_peer,to_peer,kind,payload,created_at)
                 VALUES (?1,?2,'t',?3,'offer','{}',?4)",
            )
            .bind(uid())
            .bind(&sid)
            .bind(to)
            .bind(now())
            .execute(&p)
            .await
            .unwrap();
        }
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM ep_live_signals WHERE session_id=?1 AND to_peer='a'",
        )
        .bind(&sid)
        .fetch_all(&p)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1, "peer 'a' anapata yake pekee");
        let _ = sqlx::query("DELETE FROM ep_live_signals WHERE session_id=?1 AND to_peer='a'")
            .bind(&sid)
            .execute(&p)
            .await;
        assert_eq!(table_count(&p, "ep_live_signals").await, 1, "ya 'b' inabaki");
    }
}
