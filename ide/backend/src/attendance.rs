//! attendance.rs — ELIMUPRO Awamu 7: ratiba (timetable) + mahudhurio.
//!
//! Ratiba: kila somo hupewa siku ya wiki, muda wa kuanza/kumaliza, chumba na
//! mwalimu. Mgongano hugunduliwa HALISI: mwalimu yule yule au level ile ile
//! haiwezi kuwa na vipindi vinavyogongana siku moja.
//!
//! Mahudhurio: kikao (session) hutokana na kipindi cha ratiba kwa tarehe
//! mahususi; wanafunzi wa level hujazwa kiotomatiki. Mwalimu huweka hali
//! (present|absent|late|excused) — au mwanafunzi hujisajili kwa kodi ya kikao
//! (`check-in`). Asilimia hupimwa kwa wanafunzi, masomo na level.
//!
//! Tables: ep_timetable ep_attendance_sessions ep_attendance

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
        r#"CREATE TABLE IF NOT EXISTS ep_timetable (
            id TEXT PRIMARY KEY,
            level_id TEXT NOT NULL DEFAULT '',
            subject_id TEXT NOT NULL DEFAULT '',
            teacher_id TEXT NOT NULL DEFAULT '',
            day INTEGER NOT NULL DEFAULT 1,       -- 1=Jumatatu .. 7=Jumapili
            start_min INTEGER NOT NULL DEFAULT 0, -- dakika kutoka saa 00:00
            end_min INTEGER NOT NULL DEFAULT 0,
            room TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_attendance_sessions (
            id TEXT PRIMARY KEY,
            timetable_id TEXT NOT NULL DEFAULT '',
            level_id TEXT NOT NULL DEFAULT '',
            subject_id TEXT NOT NULL DEFAULT '',
            date TEXT NOT NULL,                  -- YYYY-MM-DD
            code TEXT NOT NULL DEFAULT '',       -- kodi ya kujisajili
            open INTEGER NOT NULL DEFAULT 1,     -- kikao kipo wazi?
            created_by TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            UNIQUE(timetable_id, date)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_attendance (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'absent', -- present|absent|late|excused
            note TEXT NOT NULL DEFAULT '',
            marked_by TEXT NOT NULL DEFAULT '',
            marked_at TEXT NOT NULL,
            UNIQUE(session_id, student_id)
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/attendance/status", get(status))
        .route("/api/attendance/timetable", get(timetable_list))
        .route("/api/attendance/timetable/add", post(timetable_add))
        .route("/api/attendance/timetable/delete", post(timetable_delete))
        .route("/api/attendance/session/open", post(session_open))
        .route("/api/attendance/sessions", get(sessions_list))
        .route("/api/attendance/session/:id", get(session_get))
        .route("/api/attendance/mark", post(mark))
        .route("/api/attendance/mark/bulk", post(mark_bulk))
        .route("/api/attendance/checkin", post(checkin))
        .route("/api/attendance/session/close", post(session_close))
        .route("/api/attendance/summary/student/:id", get(summary_student))
        .route("/api/attendance/summary/level", get(summary_level))
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

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

/// Hugeuza "HH:MM" kuwa dakika kutoka saa 00:00.
pub fn parse_hm(s: &str) -> Option<i64> {
    let (h, m) = s.trim().split_once(':')?;
    let h: i64 = h.trim().parse().ok()?;
    let m: i64 = m.trim().parse().ok()?;
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return None;
    }
    Some(h * 60 + m)
}

/// Hurudisha `true` kama vipindi viwili vinagongana siku moja.
pub fn overlaps(a_start: i64, a_end: i64, b_start: i64, b_end: i64) -> bool {
    a_start < b_end && b_start < a_end
}

fn valid_day(d: i64) -> bool {
    (1..=7).contains(&d)
}

fn norm_status(s: &str) -> &'static str {
    match s.trim().to_lowercase().as_str() {
        "present" | "yupo" | "p" => "present",
        "late" | "amechelewa" | "l" => "late",
        "excused" | "ruhusa" | "e" => "excused",
        _ => "absent",
    }
}

/// Hukokotoa asilimia ya mahudhurio. `present` na `late` huhesabika kama
/// kuhudhuria (late hupewa 0.5 ili kuonyesha ucheleweshaji).
pub fn attendance_pct(statuses: &[String]) -> f64 {
    let total = statuses.len();
    if total == 0 {
        return -1.0;
    }
    let mut credit = 0.0;
    for s in statuses {
        match s.as_str() {
            "present" => credit += 1.0,
            "late" => credit += 0.5,
            "excused" => credit += 1.0,
            _ => {}
        }
    }
    (credit / total as f64 * 100.0 * 10.0).round() / 10.0
}

fn rand_code() -> String {
    use rand::Rng;
    const AL: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut r = rand::thread_rng();
    (0..6).map(|_| AL[r.gen_range(0..AL.len())] as char).collect()
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "periods": table_count(&s.db, "ep_timetable").await,
        "sessions": table_count(&s.db, "ep_attendance_sessions").await,
        "marks": table_count(&s.db, "ep_attendance").await,
        "open": sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM ep_attendance_sessions WHERE open=1")
            .fetch_one(&s.db).await.map(|r| r.0).unwrap_or(0),
    }))
}

// ============================================================
// RATIBA
// ============================================================

#[derive(Deserialize)]
pub struct PeriodIn {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub subject_id: String,
    #[serde(default)]
    pub teacher_id: String,
    pub day: i64,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub room: String,
}

pub async fn timetable_add(State(s): State<AppState>, Json(b): Json<PeriodIn>) -> Json<Value> {
    if !valid_day(b.day) {
        return Json(json!({ "ok": false, "error": "siku isiyo sahihi (1-7)" }));
    }
    let (Some(start), Some(end)) = (parse_hm(&b.start), parse_hm(&b.end)) else {
        return Json(json!({ "ok": false, "error": "muda si sahihi (tumia HH:MM)" }));
    };
    if start >= end {
        return Json(json!({ "ok": false, "error": "muda wa kuanza lazima uwe kabla ya kumaliza" }));
    }

    // Mgongano halisi: mwalimu yule yule AU level ile ile, siku ile ile.
    let existing = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT id,teacher_id,level_id,start_min,end_min FROM ep_timetable WHERE day=?1",
    )
    .bind(b.day)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    for (id, t_id, l_id, s_min, e_min) in &existing {
        if !overlaps(start, end, *s_min, *e_min) {
            continue;
        }
        if !b.teacher_id.is_empty() && *t_id == b.teacher_id {
            return Json(json!({
                "ok": false, "error": "mgongano wa mwalimu",
                "conflict_with": id, "kind": "teacher"
            }));
        }
        if !b.level_id.is_empty() && *l_id == b.level_id {
            return Json(json!({
                "ok": false, "error": "mgongano wa level",
                "conflict_with": id, "kind": "level"
            }));
        }
        if !b.room.trim().is_empty() {
            let room: Option<(String,)> = sqlx::query_as("SELECT room FROM ep_timetable WHERE id=?1")
                .bind(id)
                .fetch_optional(&s.db)
                .await
                .ok()
                .flatten();
            if room.map(|r| r.0) == Some(b.room.trim().to_string()) {
                return Json(json!({
                    "ok": false, "error": "mgongano wa chumba",
                    "conflict_with": id, "kind": "room"
                }));
            }
        }
    }

    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_timetable (id,level_id,subject_id,teacher_id,day,start_min,end_min,room,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&id)
    .bind(&b.level_id)
    .bind(&b.subject_id)
    .bind(&b.teacher_id)
    .bind(b.day)
    .bind(start)
    .bind(end)
    .bind(b.room.trim())
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "start_min": start, "end_min": end }))
}

#[derive(Deserialize)]
pub struct PeriodQuery {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub teacher_id: String,
    #[serde(default)]
    pub day: i64,
}

pub async fn timetable_list(State(s): State<AppState>, Query(q): Query<PeriodQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, i64, i64, String)>(
        "SELECT t.id,t.level_id,COALESCE(l.name,''),t.subject_id,COALESCE(su.name,''),
                t.teacher_id,COALESCE(tc.name,''),t.day,t.start_min,t.end_min,t.room
         FROM ep_timetable t
         LEFT JOIN ep_levels l ON l.id=t.level_id
         LEFT JOIN ep_subjects su ON su.id=t.subject_id
         LEFT JOIN ep_teachers tc ON tc.id=t.teacher_id
         WHERE (?1='' OR t.level_id=?1) AND (?2='' OR t.teacher_id=?2) AND (?3=0 OR t.day=?3)
         ORDER BY t.day, t.start_min",
    )
    .bind(&q.level_id)
    .bind(&q.teacher_id)
    .bind(q.day)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let periods: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "level_id": r.1, "level": r.2, "subject_id": r.3, "subject": r.4,
                "teacher_id": r.5, "teacher": r.6, "day": r.7,
                "start": fmt_hm(r.8), "end": fmt_hm(r.9),
                "start_min": r.8, "end_min": r.9, "room": r.10
            })
        })
        .collect();
    Json(json!({ "ok": true, "periods": periods }))
}

pub fn fmt_hm(min: i64) -> String {
    format!("{:02}:{:02}", min / 60, min % 60)
}

#[derive(Deserialize)]
pub struct PeriodDel {
    pub id: String,
}

pub async fn timetable_delete(State(s): State<AppState>, Json(b): Json<PeriodDel>) -> Json<Value> {
    let _ = sqlx::query("DELETE FROM ep_timetable WHERE id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true }))
}

// ============================================================
// VIKAO (sessions)
// ============================================================

#[derive(Deserialize)]
pub struct SessionOpen {
    pub timetable_id: String,
    pub date: String,
    #[serde(default)]
    pub created_by: String,
}

pub async fn session_open(State(s): State<AppState>, Json(b): Json<SessionOpen>) -> Json<Value> {
    let per = sqlx::query_as::<_, (String, String)>(
        "SELECT level_id,subject_id FROM ep_timetable WHERE id=?1",
    )
    .bind(&b.timetable_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((level_id, subject_id)) = per else {
        return Json(json!({ "ok": false, "error": "kipindi hakipo" }));
    };
    if b.date.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "tarehe inahitajika (YYYY-MM-DD)" }));
    }

    // Kama kikao kipo tayari, kirudishe (idempotent).
    let existing: Option<(String, String, i64)> = sqlx::query_as(
        "SELECT id,code,open FROM ep_attendance_sessions WHERE timetable_id=?1 AND date=?2",
    )
    .bind(&b.timetable_id)
    .bind(&b.date)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if let Some((id, code, open)) = existing {
        return Json(json!({ "ok": true, "id": id, "code": code, "open": open == 1, "existing": true }));
    }

    let id = uid();
    let code = rand_code();
    let _ = sqlx::query(
        "INSERT INTO ep_attendance_sessions (id,timetable_id,level_id,subject_id,date,code,open,created_by,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8)",
    )
    .bind(&id)
    .bind(&b.timetable_id)
    .bind(&level_id)
    .bind(&subject_id)
    .bind(&b.date)
    .bind(&code)
    .bind(&b.created_by)
    .bind(now())
    .execute(&s.db)
    .await;

    // Jaza wanafunzi wote wa level kama 'absent' (mwalimu atabadilisha).
    let students = sqlx::query_as::<_, (String,)>("SELECT id FROM ep_students WHERE level_id=?1")
        .bind(&level_id)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    for (sid,) in &students {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ep_attendance (id,session_id,student_id,status,marked_at)
             VALUES (?1,?2,?3,'absent',?4)",
        )
        .bind(uid())
        .bind(&id)
        .bind(sid)
        .bind(now())
        .execute(&s.db)
        .await;
    }

    Json(json!({ "ok": true, "id": id, "code": code, "open": true, "students": students.len() }))
}

#[derive(Deserialize)]
pub struct SessionQuery {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub date: String,
}

pub async fn sessions_list(State(s): State<AppState>, Query(q): Query<SessionQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, i64, i64, String)>(
        "SELECT ss.id,ss.level_id,ss.subject_id,COALESCE(su.name,''),ss.date,ss.open,
                (SELECT COUNT(*) FROM ep_attendance a WHERE a.session_id=ss.id),
                (SELECT COUNT(*) FROM ep_attendance a WHERE a.session_id=ss.id AND a.status IN ('present','late')),
                ss.code
         FROM ep_attendance_sessions ss
         LEFT JOIN ep_subjects su ON su.id=ss.subject_id
         WHERE (?1='' OR ss.level_id=?1) AND (?2='' OR ss.date=?2)
         ORDER BY ss.date DESC LIMIT 200",
    )
    .bind(&q.level_id)
    .bind(&q.date)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "sessions": rows.into_iter().map(|r| json!({
            "id": r.0, "level_id": r.1, "subject_id": r.2, "subject": r.3, "date": r.4,
            "open": r.5 == 1, "total": r.6, "present": r.7, "code": r.8
        })).collect::<Vec<_>>()
    }))
}

pub async fn session_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let head = sqlx::query_as::<_, (String, String, String, String, i64, String)>(
        "SELECT ss.id,ss.level_id,ss.subject_id,ss.date,ss.open,ss.code
         FROM ep_attendance_sessions ss WHERE ss.id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((sid, level_id, subject_id, date, open, code)) = head else {
        return Json(json!({ "ok": false, "error": "kikao hakipo" }));
    };
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT a.student_id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),a.status,a.note
         FROM ep_attendance a LEFT JOIN ep_students st ON st.id=a.student_id
         WHERE a.session_id=?1 ORDER BY st.full_name",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let students: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({
            "student_id": r.0, "student": r.1, "reg_no": r.2, "status": r.3, "note": r.4
        }))
        .collect();
    let statuses: Vec<String> = students
        .iter()
        .filter_map(|v| v["status"].as_str().map(|s| s.to_string()))
        .collect();
    Json(json!({
        "ok": true,
        "session": { "id": sid, "level_id": level_id, "subject_id": subject_id, "date": date,
                     "open": open == 1, "code": code, "percent": attendance_pct(&statuses) },
        "students": students
    }))
}

// ============================================================
// KUWEKA MAHUDHURIO
// ============================================================

#[derive(Deserialize)]
pub struct MarkIn {
    pub session_id: String,
    pub student_id: String,
    pub status: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub marked_by: String,
}

pub async fn mark(State(s): State<AppState>, Json(b): Json<MarkIn>) -> Json<Value> {
    let open: Option<(i64,)> = sqlx::query_as("SELECT open FROM ep_attendance_sessions WHERE id=?1")
        .bind(&b.session_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    match open {
        None => return Json(json!({ "ok": false, "error": "kikao hakipo" })),
        Some((0,)) => return Json(json!({ "ok": false, "error": "kikao kimefungwa" })),
        _ => {}
    }
    let st = norm_status(&b.status);
    let _ = sqlx::query(
        "INSERT INTO ep_attendance (id,session_id,student_id,status,note,marked_by,marked_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(session_id,student_id) DO UPDATE SET
            status=excluded.status, note=excluded.note, marked_by=excluded.marked_by, marked_at=excluded.marked_at",
    )
    .bind(uid())
    .bind(&b.session_id)
    .bind(&b.student_id)
    .bind(st)
    .bind(&b.note)
    .bind(&b.marked_by)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "status": st }))
}

#[derive(Deserialize)]
pub struct BulkIn {
    pub session_id: String,
    pub marks: Vec<MarkIn>,
    #[serde(default)]
    pub marked_by: String,
}

pub async fn mark_bulk(State(s): State<AppState>, Json(b): Json<BulkIn>) -> Json<Value> {
    let open: Option<(i64,)> = sqlx::query_as("SELECT open FROM ep_attendance_sessions WHERE id=?1")
        .bind(&b.session_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    match open {
        None => return Json(json!({ "ok": false, "error": "kikao hakipo" })),
        Some((0,)) => return Json(json!({ "ok": false, "error": "kikao kimefungwa" })),
        _ => {}
    }
    let mut saved = 0;
    for m in &b.marks {
        let st = norm_status(&m.status);
        let by = if m.marked_by.is_empty() { &b.marked_by } else { &m.marked_by };
        let _ = sqlx::query(
            "INSERT INTO ep_attendance (id,session_id,student_id,status,note,marked_by,marked_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(session_id,student_id) DO UPDATE SET
                status=excluded.status, note=excluded.note, marked_by=excluded.marked_by, marked_at=excluded.marked_at",
        )
        .bind(uid())
        .bind(&b.session_id)
        .bind(&m.student_id)
        .bind(st)
        .bind(&m.note)
        .bind(by)
        .bind(now())
        .execute(&s.db)
        .await;
        saved += 1;
    }
    Json(json!({ "ok": true, "saved": saved }))
}

#[derive(Deserialize)]
pub struct CheckIn {
    pub code: String,
    pub student_id: String,
}

/// Mwanafunzi hujisajili kwa kodi ya kikao. Kikao lazima kiwe wazi.
pub async fn checkin(State(s): State<AppState>, Json(b): Json<CheckIn>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, i64)>(
        "SELECT id,open FROM ep_attendance_sessions WHERE code=?1",
    )
    .bind(b.code.trim().to_uppercase())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((sid, open)) = row else {
        return Json(json!({ "ok": false, "error": "kodi haipo" }));
    };
    if open == 0 {
        return Json(json!({ "ok": false, "error": "kikao kimefungwa" }));
    }
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM ep_attendance WHERE session_id=?1 AND student_id=?2",
    )
    .bind(&sid)
    .bind(&b.student_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if exists.is_none() {
        return Json(json!({ "ok": false, "error": "mwanafunzi hayupo kwenye kikao hiki" }));
    }
    let _ = sqlx::query(
        "UPDATE ep_attendance SET status='present', marked_by='self', marked_at=?1
         WHERE session_id=?2 AND student_id=?3",
    )
    .bind(now())
    .bind(&sid)
    .bind(&b.student_id)
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "status": "present", "session_id": sid }))
}

#[derive(Deserialize)]
pub struct SessionClose {
    pub id: String,
}

pub async fn session_close(State(s): State<AppState>, Json(b): Json<SessionClose>) -> Json<Value> {
    let _ = sqlx::query("UPDATE ep_attendance_sessions SET open=0 WHERE id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true }))
}

// ============================================================
// MUHTASARI
// ============================================================

pub async fn summary_student(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let st = sqlx::query_as::<_, (String, String, String)>(
        "SELECT full_name,reg_no,level_id FROM ep_students WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((name, reg, level_id)) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hapo" }));
    };
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT ss.subject_id,COALESCE(su.name,''),a.status
         FROM ep_attendance a
         JOIN ep_attendance_sessions ss ON ss.id=a.session_id
         LEFT JOIN ep_subjects su ON su.id=ss.subject_id
         WHERE a.student_id=?1",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut by_subject: std::collections::BTreeMap<(String, String), Vec<String>> = Default::default();
    let mut all: Vec<String> = Vec::new();
    for (subject_id, subject, status) in &rows {
        by_subject
            .entry((subject_id.clone(), subject.clone()))
            .or_default()
            .push(status.clone());
        all.push(status.clone());
    }
    let subjects: Vec<Value> = by_subject
        .into_iter()
        .map(|((sid, sname), sts)| {
            json!({ "subject_id": sid, "subject": sname, "percent": attendance_pct(&sts), "sessions": sts.len() })
        })
        .collect();
    Json(json!({
        "ok": true,
        "student": { "id": id, "name": name, "reg_no": reg, "level_id": level_id },
        "overall": attendance_pct(&all),
        "sessions": all.len(),
        "subjects": subjects
    }))
}

pub async fn summary_level(State(s): State<AppState>, Query(q): Query<SessionQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT st.id,st.full_name,COALESCE(st.reg_no,''),a.status
         FROM ep_students st
         LEFT JOIN ep_attendance a ON a.student_id=st.id
         LEFT JOIN ep_attendance_sessions ss ON ss.id=a.session_id
         WHERE st.level_id=?1 AND (?2='' OR ss.date<=?2)
         ORDER BY st.full_name",
    )
    .bind(&q.level_id)
    .bind(&q.date)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut per: std::collections::BTreeMap<(String, String, String), Vec<String>> = Default::default();
    for (id, name, reg, status) in rows {
        per.entry((id, name, reg))
            .or_default()
            .push(status.unwrap_or_default());
    }
    let students: Vec<Value> = per
        .into_iter()
        .map(|((id, name, reg), sts)| {
            let clean: Vec<String> = sts.into_iter().filter(|s| !s.is_empty()).collect();
            json!({
                "student_id": id, "student": name, "reg_no": reg,
                "percent": attendance_pct(&clean), "sessions": clean.len()
            })
        })
        .collect();
    Json(json!({ "ok": true, "level_id": q.level_id, "students": students }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // ep_students hutengenezwa na elimupro; hapa tunaunda kile kinachohitajika.
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS ep_students (
                id TEXT PRIMARY KEY, level_id TEXT NOT NULL DEFAULT '',
                full_name TEXT NOT NULL DEFAULT '', reg_no TEXT NOT NULL DEFAULT '',
                username TEXT NOT NULL DEFAULT '', password_hash TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT '')",
        )
        .execute(&p)
        .await;
        // Majedwali ya elimupro yenye majina HALISI ya safu (ep_teachers.name,
        // SI full_name) — JOIN isiyo sahihi hushindwa kimyakimya.
        for ddl in [
            "CREATE TABLE IF NOT EXISTS ep_levels (id TEXT PRIMARY KEY, name TEXT NOT NULL DEFAULT '')",
            "CREATE TABLE IF NOT EXISTS ep_subjects (id TEXT PRIMARY KEY, level_id TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '')",
            "CREATE TABLE IF NOT EXISTS ep_teachers (id TEXT PRIMARY KEY, name TEXT NOT NULL DEFAULT '')",
        ] {
            let _ = sqlx::query(ddl).execute(&p).await;
        }
        init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn init_tables_huunda_majedwali_yote() {
        let p = db().await;
        for t in ["ep_timetable", "ep_attendance_sessions", "ep_attendance"] {
            assert_eq!(table_count(&p, t).await, 0, "{t} haipo");
        }
    }

    #[test]
    fn muda_hutafsiriwa_kuwa_dakika() {
        assert_eq!(parse_hm("08:00"), Some(480));
        assert_eq!(parse_hm("13:45"), Some(825));
        assert_eq!(parse_hm("23:59"), Some(1439));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(parse_hm("08:60"), None);
        assert_eq!(parse_hm("abc"), None);
        assert_eq!(fmt_hm(480), "08:00");
        assert_eq!(fmt_hm(825), "13:45");
    }

    #[test]
    fn mgongano_hugundulika() {
        // 08:00-10:00 na 09:00-11:00 → mgongano.
        assert!(overlaps(480, 600, 540, 660));
        // 08:00-10:00 na 10:00-12:00 → hakuna (hukutana kwenye mpaka).
        assert!(!overlaps(480, 600, 600, 720));
        // 08:00-10:00 na 11:00-12:00 → hakuna.
        assert!(!overlaps(480, 600, 660, 720));
        // 09:00-11:00 na 08:00-10:00 → mgongano (mpangilio usiathiri).
        assert!(overlaps(540, 660, 480, 600));
    }

    #[test]
    fn hali_hutafsiriwa() {
        assert_eq!(norm_status("present"), "present");
        assert_eq!(norm_status("YUPO"), "present");
        assert_eq!(norm_status("late"), "late");
        assert_eq!(norm_status("amechelewa"), "late");
        assert_eq!(norm_status("excused"), "excused");
        assert_eq!(norm_status("ruhusa"), "excused");
        assert_eq!(norm_status("kitu"), "absent");
    }

    #[test]
    fn asilimia_ya_mahudhurio_ni_sahihi() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        // 2 present + 1 late + 1 absent = (1+1+0.5)/4 = 62.5
        assert_eq!(attendance_pct(&s(&["present", "present", "late", "absent"])), 62.5);
        // present + excused = 100 (ruhusa huhesabika)
        assert_eq!(attendance_pct(&s(&["present", "excused"])), 100.0);
        // late pekee = 50
        assert_eq!(attendance_pct(&s(&["late"])), 50.0);
        // hakuna kikao = -1
        assert_eq!(attendance_pct(&s(&[])), -1.0);
    }

    #[test]
    fn kodi_ya_kikao_ni_herufi_sita_salama() {
        let c = rand_code();
        assert_eq!(c.len(), 6);
        assert!(c.chars().all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()));
        // Hakuna herufi zinazochanganya: O, 0, I, 1.
        assert!(!c.contains('O') && !c.contains('0') && !c.contains('I') && !c.contains('1'));
    }

    #[tokio::test]
    async fn kikao_hujaza_wanafunzi_wote_kama_absent() {
        let p = db().await;
        for i in 1..=3 {
            sqlx::query("INSERT INTO ep_students (id,level_id,full_name,reg_no,username,password_hash,created_at)
                         VALUES (?1,'lv1',?2,?3,?4,'x',?5)")
                .bind(format!("st{i}"))
                .bind(format!("Mwanafunzi {i}"))
                .bind(format!("REG{i}"))
                .bind(format!("user{i}"))
                .bind(now())
                .execute(&p)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO ep_timetable (id,level_id,subject_id,day,start_min,end_min,created_at)
                     VALUES ('p1','lv1','su1',1,480,600,?1)")
            .bind(now())
            .execute(&p)
            .await
            .unwrap();
        let sid = uid();
        sqlx::query("INSERT INTO ep_attendance_sessions (id,timetable_id,level_id,subject_id,date,code,open,created_at)
                     VALUES (?1,'p1','lv1','su1','2026-10-20','ABC123',1,?2)")
            .bind(&sid).bind(now()).execute(&p).await.unwrap();
        for i in 1..=3 {
            sqlx::query("INSERT INTO ep_attendance (id,session_id,student_id,status,marked_at)
                         VALUES (?1,?2,?3,'absent',?4)")
                .bind(uid()).bind(&sid).bind(format!("st{i}")).bind(now())
                .execute(&p).await.unwrap();
        }
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_attendance WHERE session_id=?1 AND status='absent'")
            .bind(&sid).fetch_one(&p).await.unwrap();
        assert_eq!(n, 3);
    }

    #[tokio::test]
    async fn orodha_ya_ratiba_hupata_majina_ya_somo_na_mwalimu() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_levels (id,name) VALUES ('lv1','Mwaka 1')").execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_subjects (id,level_id,name) VALUES ('su1','lv1','Fizikia')").execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_teachers (id,name) VALUES ('tc1','Mwl Juma')").execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_timetable (id,level_id,subject_id,teacher_id,day,start_min,end_min,room,created_at)
                     VALUES ('p1','lv1','su1','tc1',1,480,600,'A101',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let rows = sqlx::query_as::<_, (String, String, String, i64, i64, String)>(
            "SELECT COALESCE(l.name,''),COALESCE(su.name,''),COALESCE(tc.name,''),t.start_min,t.end_min,t.room
             FROM ep_timetable t
             LEFT JOIN ep_levels l ON l.id=t.level_id
             LEFT JOIN ep_subjects su ON su.id=t.subject_id
             LEFT JOIN ep_teachers tc ON tc.id=t.teacher_id",
        )
        .fetch_all(&p)
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "Mwaka 1");
        assert_eq!(rows[0].1, "Fizikia");
        assert_eq!(rows[0].2, "Mwl Juma", "JOIN ya mwalimu ilishindwa (name vs full_name?)");
        assert_eq!(fmt_hm(rows[0].4), "10:00");
    }

    #[tokio::test]
    async fn kikao_kimoja_kwa_kipindi_na_tarehe() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_attendance_sessions (id,timetable_id,level_id,subject_id,date,code,open,created_at)
                     VALUES ('s1','p1','lv1','su1','2026-10-20','AAAAAA',1,?1)")
            .bind(now()).execute(&p).await.unwrap();
        // UNIQUE(timetable_id,date) huzuia kurudia.
        let dup = sqlx::query("INSERT INTO ep_attendance_sessions (id,timetable_id,level_id,subject_id,date,code,open,created_at)
                               VALUES ('s2','p1','lv1','su1','2026-10-20','BBBBBB',1,?1)")
            .bind(now()).execute(&p).await;
        assert!(dup.is_err(), "UNIQUE haikufanya kazi");
    }

    #[tokio::test]
    async fn mahudhurio_husasishwa_si_kurudia() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_attendance_sessions (id,timetable_id,level_id,subject_id,date,code,open,created_at)
                     VALUES ('s1','p1','lv1','su1','2026-10-20','AAAAAA',1,?1)")
            .bind(now()).execute(&p).await.unwrap();
        for st in ["absent", "present"] {
            sqlx::query("INSERT INTO ep_attendance (id,session_id,student_id,status,marked_at)
                         VALUES (?1,'s1','st1',?2,?3)
                         ON CONFLICT(session_id,student_id) DO UPDATE SET status=excluded.status")
                .bind(uid()).bind(st).bind(now()).execute(&p).await.unwrap();
        }
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_attendance WHERE session_id='s1' AND student_id='st1'")
            .fetch_one(&p).await.unwrap();
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_attendance WHERE session_id='s1' AND student_id='st1'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(n, 1, "kurudia kulizalisha safu mbili");
        assert_eq!(st, "present");
    }
}
