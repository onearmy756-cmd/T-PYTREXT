//! voip.rs — ELIMUPRO Awamu 13: mawasiliano ya LAN (moduli 13).
//!
//! Simu (sauti), video, SMS na kutuma faili — ndani ya LAN ya chuo, bila
//! internet. Sauti/video husafiri moja kwa moja kati ya vifaa (WebRTC); seva
//! hushughulikia tu ishara (signaling) na ujumbe wa maandishi.
//!
//! Kanuni halisi:
//!   - Mtumiaji hawezi kujipigia simu mwenyewe.
//!   - Simu huanza `ringing`; kupokea → `active`; kukataa/kukata → `ended`.
//!   - Ujumbe wa maandishi hufika kwa mpokeaji (`read` hupimwa).
//!   - Faili hupitishwa kwa metadata + njia ya LAN (hakuna cloud).
//!
//! Tables: ep_voip_users ep_calls ep_voip_messages ep_file_transfers

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
        r#"CREATE TABLE IF NOT EXISTS ep_voip_users (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'phone',  -- phone|desktop|tablet|web
            ip TEXT NOT NULL DEFAULT '',
            extension TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'offline', -- online|offline|busy
            last_seen TEXT NOT NULL,
            UNIQUE(user_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_calls (
            id TEXT PRIMARY KEY,
            caller_id TEXT NOT NULL,
            callee_id TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'audio',  -- audio|video
            status TEXT NOT NULL DEFAULT 'ringing', -- ringing|active|ended|missed|rejected
            started_at TEXT NOT NULL,
            answered_at TEXT,
            ended_at TEXT,
            duration_s INTEGER NOT NULL DEFAULT 0
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_voip_messages (
            id TEXT PRIMARY KEY,
            from_id TEXT NOT NULL,
            to_id TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'sms',    -- sms|file
            file_name TEXT NOT NULL DEFAULT '',
            file_size INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'sent', -- sent|delivered|read
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_file_transfers (
            id TEXT PRIMARY KEY,
            from_id TEXT NOT NULL,
            to_id TEXT NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            size INTEGER NOT NULL DEFAULT 0,
            mime TEXT NOT NULL DEFAULT '',
            lan_url TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'offered', -- offered|accepted|done|declined
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/voip/status", get(status))
        .route("/api/voip/presence", post(presence))
        .route("/api/voip/users", get(users_list))
        .route("/api/voip/call", post(call_start))
        .route("/api/voip/call/:id/answer", post(call_answer))
        .route("/api/voip/call/:id/reject", post(call_reject))
        .route("/api/voip/call/:id/end", post(call_end))
        .route("/api/voip/calls", get(calls_list))
        .route("/api/voip/message", post(message_send))
        .route("/api/voip/messages", get(messages_list))
        .route("/api/voip/message/:id/read", post(message_read))
        .route("/api/voip/file", post(file_offer))
        .route("/api/voip/files", get(files_list))
        .route("/api/voip/file/:id/accept", post(file_accept))
        .route("/api/voip/file/:id/done", post(file_done))
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

/// Aina za simu zinazokubalika.
pub fn norm_call_kind(k: &str) -> &'static str {
    match k.trim().to_lowercase().as_str() {
        "video" | "video_call" | "kamera" => "video",
        _ => "audio",
    }
}

/// Mabadiliko ya hali ya simu yanaruhusiwa?
pub fn call_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("ringing", "active")
            | ("ringing", "rejected")
            | ("ringing", "missed")
            | ("ringing", "ended")
            | ("active", "ended")
    )
}

/// Ujumbe unaweza kuwekwa `read`? (si `sent` tena.)
pub fn can_mark_read(status: &str) -> bool {
    matches!(status.trim().to_lowercase().as_str(), "sent" | "delivered")
}

/// Ukubwa wa faili unaokubalika kwa LAN (hadi 200 MB).
pub const MAX_FILE_BYTES: i64 = 200 * 1024 * 1024;

pub fn file_ok(size: i64) -> bool {
    size > 0 && size <= MAX_FILE_BYTES
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (online,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_voip_users WHERE status='online'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (ringing,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_calls WHERE status='ringing'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (active,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_calls WHERE status='active'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (unread,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_voip_messages WHERE status!='read'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "mode": "lan",
        "users": table_count(&s.db, "ep_voip_users").await,
        "online": online,
        "ringing": ringing,
        "active": active,
        "unread": unread,
        "messages": table_count(&s.db, "ep_voip_messages").await,
        "files": table_count(&s.db, "ep_file_transfers").await,
        "max_file_bytes": MAX_FILE_BYTES
    }))
}

#[derive(Deserialize)]
pub struct PresenceIn {
    pub user_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ip: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub extension: String,
    #[serde(default)]
    pub status: String,
}

pub async fn presence(State(s): State<AppState>, Json(b): Json<PresenceIn>) -> Json<Value> {
    if b.user_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "user_id inahitajika" }));
    }
    let kind = match b.kind.trim().to_lowercase().as_str() {
        "desktop" => "desktop",
        "tablet" => "tablet",
        "web" => "web",
        _ => "phone",
    };
    let status = match b.status.trim().to_lowercase().as_str() {
        "offline" => "offline",
        "busy" => "busy",
        _ => "online",
    };
    let existing = sqlx::query_as::<_, (String,)>("SELECT id FROM ep_voip_users WHERE user_id=?1")
        .bind(b.user_id.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let id = match existing {
        Some((id,)) => {
            let _ = sqlx::query(
                "UPDATE ep_voip_users SET name=?1,kind=?2,ip=?3,extension=?4,status=?5,last_seen=?6 WHERE id=?7",
            )
            .bind(&b.name)
            .bind(kind)
            .bind(&b.ip)
            .bind(&b.extension)
            .bind(status)
            .bind(now())
            .bind(&id)
            .execute(&s.db)
            .await;
            id
        }
        None => {
            let id = uid();
            let _ = sqlx::query(
                "INSERT INTO ep_voip_users (id,user_id,name,kind,ip,extension,status,last_seen)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            )
            .bind(&id)
            .bind(b.user_id.trim())
            .bind(&b.name)
            .bind(kind)
            .bind(&b.ip)
            .bind(&b.extension)
            .bind(status)
            .bind(now())
            .execute(&s.db)
            .await;
            id
        }
    };
    Json(json!({ "ok": true, "id": id, "status": status }))
}

pub async fn users_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT id,user_id,name,kind,ip,extension,status FROM ep_voip_users ORDER BY name",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "users": rows.into_iter().map(|r| json!({
            "id": r.0, "user_id": r.1, "name": r.2, "kind": r.3,
            "ip": r.4, "extension": r.5, "status": r.6
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct CallIn {
    pub caller_id: String,
    pub callee_id: String,
    #[serde(default)]
    pub kind: String,
}

pub async fn call_start(State(s): State<AppState>, Json(b): Json<CallIn>) -> Json<Value> {
    let caller = b.caller_id.trim();
    let callee = b.callee_id.trim();
    if caller.is_empty() || callee.is_empty() {
        return Json(json!({ "ok": false, "error": "caller_id na callee_id zinahitajika" }));
    }
    if caller == callee {
        return Json(json!({ "ok": false, "error": "hauwezi kujipigia simu mwenyewe" }));
    }
    let id = uid();
    let kind = norm_call_kind(&b.kind);
    let _ = sqlx::query(
        "INSERT INTO ep_calls (id,caller_id,callee_id,kind,status,started_at,duration_s)
         VALUES (?1,?2,?3,?4,'ringing',?5,0)",
    )
    .bind(&id)
    .bind(caller)
    .bind(callee)
    .bind(kind)
    .bind(now())
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE ep_voip_users SET status='busy' WHERE user_id IN (?1,?2)")
        .bind(caller)
        .bind(callee)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "id": id, "status": "ringing", "kind": kind }))
}

async fn call_set_status(
    s: &AppState,
    id: &str,
    from_ok: &[&str],
    to: &str,
) -> Result<(), String> {
    let cur = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT status,caller_id,callee_id,started_at FROM ep_calls WHERE id=?1",
    )
    .bind(id)
    .fetch_optional(&s.db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "simu haipo".to_string())?;
    if !from_ok.contains(&cur.0.as_str()) {
        return Err(format!("hali ya sasa '{}' hairuhusu", cur.0));
    }
    if !call_transition(&cur.0, to) {
        return Err(format!("mabadiliko {}→{} hayaruhusiwi", cur.0, to));
    }
    let (sql, duration) = match to {
        "active" => (
            "UPDATE ep_calls SET status='active',answered_at=?1 WHERE id=?2".to_string(),
            0,
        ),
        "ended" => {
            let dur = chrono::DateTime::parse_from_rfc3339(&cur.3)
                .map(|t| (chrono::Local::now() - t.with_timezone(&chrono::Local)).num_seconds().max(0))
                .unwrap_or(0);
            (
                format!("UPDATE ep_calls SET status='ended',ended_at=?1,duration_s={dur} WHERE id=?2"),
                0,
            )
        }
        other => (format!("UPDATE ep_calls SET status='{other}',ended_at=?1 WHERE id=?2"), 0),
    };
    let _ = duration;
    sqlx::query(&sql)
        .bind(now())
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(|e| e.to_string())?;
    if to != "active" {
        let _ = sqlx::query("UPDATE ep_voip_users SET status='online' WHERE user_id IN (?1,?2)")
            .bind(&cur.1)
            .bind(&cur.2)
            .execute(&s.db)
            .await;
    }
    Ok(())
}

pub async fn call_answer(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match call_set_status(&s, &id, &["ringing"], "active").await {
        Ok(()) => Json(json!({ "ok": true, "status": "active" })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

pub async fn call_reject(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match call_set_status(&s, &id, &["ringing"], "rejected").await {
        Ok(()) => Json(json!({ "ok": true, "status": "rejected" })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

pub async fn call_end(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match call_set_status(&s, &id, &["ringing", "active"], "ended").await {
        Ok(()) => Json(json!({ "ok": true, "status": "ended" })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

#[derive(Deserialize)]
pub struct CallsQuery {
    #[serde(default)]
    pub user_id: String,
}

pub async fn calls_list(State(s): State<AppState>, Query(q): Query<CallsQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, Option<String>, i64)>(
        "SELECT id,caller_id,callee_id,kind,status,started_at,answered_at,ended_at,duration_s
         FROM ep_calls WHERE (?1='' OR caller_id=?1 OR callee_id=?1)
         ORDER BY started_at DESC LIMIT 200",
    )
    .bind(&q.user_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "calls": rows.into_iter().map(|r| json!({
            "id": r.0, "caller_id": r.1, "callee_id": r.2, "kind": r.3, "status": r.4,
            "started_at": r.5, "answered_at": r.6, "ended_at": r.7, "duration_s": r.8
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct MessageIn {
    pub from_id: String,
    pub to_id: String,
    #[serde(default)]
    pub body: String,
}

pub async fn message_send(State(s): State<AppState>, Json(b): Json<MessageIn>) -> Json<Value> {
    if b.from_id.trim().is_empty() || b.to_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "from_id na to_id zinahitajika" }));
    }
    if b.body.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "ujumbe hauwezi kuwa tupu" }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_voip_messages (id,from_id,to_id,body,kind,status,created_at)
         VALUES (?1,?2,?3,?4,'sms','sent',?5)",
    )
    .bind(&id)
    .bind(b.from_id.trim())
    .bind(b.to_id.trim())
    .bind(b.body.trim())
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "status": "sent" }))
}

#[derive(Deserialize)]
pub struct MsgQuery {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub with_id: String,
}

pub async fn messages_list(State(s): State<AppState>, Query(q): Query<MsgQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT id,from_id,to_id,body,status,created_at FROM ep_voip_messages
         WHERE (?1='' OR from_id=?1 OR to_id=?1)
           AND (?2='' OR from_id=?2 OR to_id=?2)
         ORDER BY created_at DESC LIMIT 300",
    )
    .bind(&q.user_id)
    .bind(&q.with_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "messages": rows.into_iter().map(|r| json!({
            "id": r.0, "from_id": r.1, "to_id": r.2, "body": r.3, "status": r.4, "created_at": r.5
        })).collect::<Vec<_>>()
    }))
}

pub async fn message_read(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let cur = sqlx::query_as::<_, (String,)>("SELECT status FROM ep_voip_messages WHERE id=?1")
        .bind(&id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    match cur {
        None => Json(json!({ "ok": false, "error": "ujumbe haupo" })),
        Some((st,)) if !can_mark_read(&st) => {
            Json(json!({ "ok": false, "error": format!("hali '{st}' haiwezi kuwa read") }))
        }
        Some(_) => {
            let _ = sqlx::query("UPDATE ep_voip_messages SET status='read' WHERE id=?1")
                .bind(&id)
                .execute(&s.db)
                .await;
            Json(json!({ "ok": true, "status": "read" }))
        }
    }
}

#[derive(Deserialize)]
pub struct FileIn {
    pub from_id: String,
    pub to_id: String,
    pub name: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub lan_url: String,
}

pub async fn file_offer(State(s): State<AppState>, Json(b): Json<FileIn>) -> Json<Value> {
    if b.from_id.trim().is_empty() || b.to_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "from_id na to_id zinahitajika" }));
    }
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la faili linahitajika" }));
    }
    if !file_ok(b.size) {
        return Json(json!({
            "ok": false,
            "error": format!("ukubwa si sahihi (1..{MAX_FILE_BYTES} bytes)")
        }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_file_transfers (id,from_id,to_id,name,size,mime,lan_url,status,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,'offered',?8)",
    )
    .bind(&id)
    .bind(b.from_id.trim())
    .bind(b.to_id.trim())
    .bind(b.name.trim())
    .bind(b.size)
    .bind(&b.mime)
    .bind(&b.lan_url)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "status": "offered" }))
}

#[derive(Deserialize)]
pub struct FileQuery {
    #[serde(default)]
    pub user_id: String,
}

pub async fn files_list(State(s): State<AppState>, Query(q): Query<FileQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, String, String, String)>(
        "SELECT id,from_id,to_id,name,size,mime,lan_url,status,created_at FROM ep_file_transfers
         WHERE (?1='' OR from_id=?1 OR to_id=?1) ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&q.user_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "files": rows.into_iter().map(|r| json!({
            "id": r.0, "from_id": r.1, "to_id": r.2, "name": r.3, "size": r.4,
            "mime": r.5, "lan_url": r.6, "status": r.7, "created_at": r.8
        })).collect::<Vec<_>>()
    }))
}

async fn file_set(s: &AppState, id: &str, from: &[&str], to: &str) -> Result<(), String> {
    let cur = sqlx::query_as::<_, (String,)>("SELECT status FROM ep_file_transfers WHERE id=?1")
        .bind(id)
        .fetch_optional(&s.db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "faili haipo".to_string())?;
    if !from.contains(&cur.0.as_str()) {
        return Err(format!("hali '{}' hairuhusu", cur.0));
    }
    sqlx::query("UPDATE ep_file_transfers SET status=?1 WHERE id=?2")
        .bind(to)
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn file_accept(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match file_set(&s, &id, &["offered"], "accepted").await {
        Ok(()) => Json(json!({ "ok": true, "status": "accepted" })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

pub async fn file_done(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match file_set(&s, &id, &["accepted"], "done").await {
        Ok(()) => Json(json!({ "ok": true, "status": "done" })),
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aina_ya_simu_hupangwa() {
        assert_eq!(norm_call_kind("video"), "video");
        assert_eq!(norm_call_kind("VIDEO"), "video");
        assert_eq!(norm_call_kind("audio"), "audio");
        assert_eq!(norm_call_kind(""), "audio");
        assert_eq!(norm_call_kind("kamera"), "video");
    }

    #[test]
    fn mabadiliko_ya_simu_huruhusiwa_kwa_mpangilio() {
        assert!(call_transition("ringing", "active"));
        assert!(call_transition("ringing", "rejected"));
        assert!(call_transition("ringing", "missed"));
        assert!(call_transition("active", "ended"));
        // Haramu:
        assert!(!call_transition("active", "ringing"));
        assert!(!call_transition("ended", "active"));
        assert!(!call_transition("rejected", "active"));
        assert!(!call_transition("ringing", "ringing"));
    }

    #[test]
    fn ujumbe_unaweza_kuwa_read_mara_moja() {
        assert!(can_mark_read("sent"));
        assert!(can_mark_read("delivered"));
        assert!(!can_mark_read("read"));
    }

    #[test]
    fn ukubwa_wa_faili_hukaguliwa() {
        assert!(file_ok(1));
        assert!(file_ok(MAX_FILE_BYTES));
        assert!(!file_ok(0));
        assert!(!file_ok(-5));
        assert!(!file_ok(MAX_FILE_BYTES + 1));
    }

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn simu_huanza_ringing_na_huisha_ended() {
        let p = db().await;
        let id = "c1";
        sqlx::query("INSERT INTO ep_calls (id,caller_id,callee_id,kind,status,started_at,duration_s) VALUES (?1,'u1','u2','audio','ringing',?2,0)")
            .bind(id).bind(now()).execute(&p).await.unwrap();
        sqlx::query("UPDATE ep_calls SET status='active',answered_at=?1 WHERE id=?2")
            .bind(now()).bind(id).execute(&p).await.unwrap();
        sqlx::query("UPDATE ep_calls SET status='ended',ended_at=?1 WHERE id=?2")
            .bind(now()).bind(id).execute(&p).await.unwrap();
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_calls WHERE id=?1")
            .bind(id).fetch_one(&p).await.unwrap();
        assert_eq!(st, "ended");
    }

    #[tokio::test]
    async fn ujumbe_huanza_sent_kisha_read() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_voip_messages (id,from_id,to_id,body,kind,status,created_at) VALUES ('m1','u1','u2','habari','sms','sent',?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("UPDATE ep_voip_messages SET status='read' WHERE id='m1'")
            .execute(&p).await.unwrap();
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_voip_messages WHERE id='m1'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(st, "read");
    }

    #[tokio::test]
    async fn faili_huanza_offered_kisha_done() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_file_transfers (id,from_id,to_id,name,size,mime,lan_url,status,created_at) VALUES ('f1','u1','u2','notes.pdf',2048,'application/pdf','http://192.168.1.5/f/1','offered',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_file_transfers WHERE id='f1'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(st, "offered");
    }
}
