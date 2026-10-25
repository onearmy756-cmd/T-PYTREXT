//! exam_lan.rs — MTIHANI WA OFFLINE (LAN): mwalimu hufungua "chumba" cha
//! mtihani kwenye laptop yake, wanafunzi hujiunga kwa Wi-Fi ya ndani (bila
//! intaneti) kwa QR code au code, wanafanya mtihani, mwalimu anaona maendeleo
//! ya kila mtu kwa wakati halisi (WebSocket), na matokeo hutolewa mara moja.
//!
//! Kila kitu ni HALISI na cha ndani:
//!   - maswali hutoka kwenye mtihani uliotengenezwa (`edu_exams`, AI);
//!   - IP ya LAN hutambuliwa (`ip route` / `hostname -I`) — QR code huelekeza
//!     kwenye anwani ambayo simu za wanafunzi zinaweza kufikia;
//!   - QR code hutengenezwa ndani (crate `qrcode`, SVG — hakuna CDN/intaneti);
//!   - hali ya mtihani ipo kwenye SQLite, matangazo ya live kwa broadcast.
//!
//! Endpoints (`/api/exam/...`):
//!   POST /room/create            → tengeneza chumba (code, join_url, QR)
//!   GET  /room/:code             → taarifa za chumba (ukurasa wa kujiunga)
//!   POST /room/:code/join        → mwanafunzi ajiunge → maswali yake
//!   POST /room/:code/save        → hifadhi jibu la swali moja (live)
//!   POST /room/:code/submit      → wasilisha → alama za papo hapo
//!   GET  /room/:code/live        → dashibodi ya mwalimu (snapshot)
//!   GET  /room/:code/results     → ripoti kamili
//!   WS   /room/:code/ws          → matangazo ya wakati halisi
//!   GET  /rooms                  → vyumba vyote (mwalimu)
//!   POST /room/:code/close       → funga chumba

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
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

// ============================================================
// TABLES
// ============================================================

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS exam_rooms (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL UNIQUE,
            exam_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            subject TEXT NOT NULL DEFAULT '',
            level TEXT NOT NULL DEFAULT '',
            duration_min INTEGER NOT NULL DEFAULT 180,
            shuffle INTEGER NOT NULL DEFAULT 1,
            passcode TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'open',
            join_url TEXT NOT NULL DEFAULT '',
            questions TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS exam_participants (
            id TEXT PRIMARY KEY,
            room_code TEXT NOT NULL,
            token TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL DEFAULT '',
            student_no TEXT NOT NULL DEFAULT '',
            ordering TEXT NOT NULL DEFAULT '[]',
            answers TEXT NOT NULL DEFAULT '{}',
            current INTEGER NOT NULL DEFAULT 0,
            score REAL NOT NULL DEFAULT 0,
            total REAL NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            joined_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_exam_part_room ON exam_participants(room_code)",
    )
    .execute(db)
    .await;
}

// ============================================================
// ROUTER
// ============================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/exam/rooms", get(rooms_list))
        .route("/api/exam/room/create", post(room_create))
        .route("/api/exam/room/:code", get(room_get))
        .route("/api/exam/room/:code/join", post(room_join))
        .route("/api/exam/room/:code/save", post(room_save))
        .route("/api/exam/room/:code/submit", post(room_submit))
        .route("/api/exam/room/:code/live", get(room_live))
        .route("/api/exam/room/:code/results", get(room_results))
        .route("/api/exam/room/:code/close", post(room_close))
        .route("/api/exam/room/:code/ws", get(room_ws))
}

// ============================================================
// BUS (matangazo ya live kwa kila chumba)
// ============================================================

type Bus = Mutex<HashMap<String, broadcast::Sender<Value>>>;

fn bus() -> &'static Bus {
    static B: OnceLock<Bus> = OnceLock::new();
    B.get_or_init(|| Mutex::new(HashMap::new()))
}

fn subscribe(code: &str) -> broadcast::Receiver<Value> {
    let mut b = bus().lock().unwrap();
    b.entry(code.to_string())
        .or_insert_with(|| broadcast::channel(512).0)
        .subscribe()
}

fn publish(code: &str, event: Value) {
    let b = bus().lock().unwrap();
    if let Some(tx) = b.get(code) {
        let _ = tx.send(event);
    }
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

/// Code ya chumba: herufi/namba 6 rahisi kusoma (hakuna O/0/I/1).
fn room_code() -> String {
    const A: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut s = String::new();
    for _ in 0..6 {
        let n = uuid::Uuid::new_v4().as_bytes()[0] as usize % A.len();
        s.push(A[n] as char);
    }
    s
}

/// Tambua IP ya LAN ambayo vifaa vingine vinaweza kufikia.
pub fn lan_ip() -> String {
    if let Ok(v) = std::env::var("EXAM_LAN_HOST") {
        if !v.trim().is_empty() {
            return v.trim().to_string();
        }
    }
    // `ip route get` hutuambia `src` inayotumika kwa njia ya nje.
    if let Ok(o) = Command::new("ip").args(["route", "get", "1.1.1.1"]).output() {
        let s = String::from_utf8_lossy(&o.stdout);
        if let Some(p) = s.find(" src ") {
            if let Some(ip) = s[p + 5..].split_whitespace().next() {
                return ip.to_string();
            }
        }
    }
    // `hostname -I` (Debian) — chukua ya kwanza isiyo loopback.
    if let Ok(o) = Command::new("hostname").arg("-I").output() {
        let s = String::from_utf8_lossy(&o.stdout);
        for ip in s.split_whitespace() {
            if !ip.starts_with("127.") && !ip.contains(':') {
                return ip.to_string();
            }
        }
    }
    "127.0.0.1".into()
}

/// Tengeneza SVG ya QR code kwa maandishi (haitegemei intaneti).
pub fn qr_svg(text: &str) -> Option<String> {
    use qrcode::render::svg;
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let raw = code
        .render::<svg::Color>()
        .min_dimensions(220, 220)
        .dark_color(svg::Color("#13100a"))
        .light_color(svg::Color("#ffffff"))
        .build();
    // Ondoa tangazo la XML: huwezi kuwa ndani ya <svg> nyingine (huvunja
    // uingizaji wa SVG). Tunataka <svg> pekee.
    let svg_only = raw.trim_start();
    let start = svg_only.find("<svg")?;
    Some(svg_only[start..].to_string())
}

/// Thibitisha jibu kwa kulinganisha maandishi (MCQ/mafupi). Hurudisha
/// (alama, ni_sahihi_kabisa, inahitaji_mwalimu).
fn grade_one(expected: &str, given: &str, marks: f64) -> (f64, bool, bool) {
    let e = norm(expected);
    let g = norm(given);
    if e.is_empty() || g.is_empty() {
        return (0.0, false, true);
    }
    if e == g {
        return (marks, true, false);
    }
    // Kwa majibu mafupi: jibu lililo na kikundi cha maneno muhimu hukubaliwa nusu.
    if e.len() <= 60 && (g.contains(&e) || e.contains(&g)) {
        return (marks, true, false);
    }
    // Maandishi marefu → mwalimu ahakiki.
    (0.0, false, e.len() > 60)
}

fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Vuta maswali yote ya mtihani (pamoja na majibu) kwa mpangilio wa orodha.
fn flatten_questions(questions: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let sections = questions["sections"].as_array().cloned().unwrap_or_default();
    for sec in sections {
        let name = sec["name"].as_str().unwrap_or("").to_string();
        for q in sec["questions"].as_array().cloned().unwrap_or_default() {
            let mut item = q.clone();
            item["section"] = json!(name);
            out.push(item);
        }
    }
    out
}

/// Maswali yenye majibu PEKEE kwa mwanafunzi (majibu hutolewa!).
fn public_questions(room: &Value, ordering: &[usize]) -> Value {
    let all = flatten_questions(&room["questions"]);
    let list: Vec<Value> = ordering
        .iter()
        .filter_map(|i| all.get(*i))
        .map(|q| {
            json!({
                "number": q["number"],
                "section": q["section"],
                "question": q["question"],
                "marks": q["marks"]
            })
        })
        .collect();
    json!(list)
}

/// Orodha ya faharasa — huchanganywa kama `shuffle` imewekwa.
fn make_ordering(n: usize, shuffle: bool) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    if shuffle {
        // Fisher–Yates kwa `uuid` (hakuna crate ya random nyongeza).
        for i in (1..n).rev() {
            let r = uuid::Uuid::new_v4().as_bytes()[0] as usize % (i + 1);
            v.swap(i, r);
        }
    }
    v
}

// ============================================================
// HANDLERS — MWALIMU
// ============================================================

#[derive(Deserialize)]
pub struct CreateRoom {
    pub exam_id: String,
    pub title: Option<String>,
    pub duration_min: Option<i64>,
    pub shuffle: Option<i64>,
    pub passcode: Option<String>,
    /// Anwani ambayo QR itaelekeza (mf. `http://192.168.1.100:8080`).
    pub host: Option<String>,
    pub path: Option<String>,
}

pub async fn room_create(State(s): State<AppState>, Json(r): Json<CreateRoom>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, i64, String, String, String)>(
        "SELECT subject, level, duration_min, body, answers, title FROM edu_exams WHERE id = ?1",
    )
    .bind(&r.exam_id)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((subject, level, duration_min, body, answers, title)) = row else {
        return Json(json!({"ok": false, "error": "mtihani haupo"}));
    };

    // Unganisha majibu kwenye maswali (server-side pekee).
    let body_v: Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let ans_v: Value = serde_json::from_str(&answers).unwrap_or(json!({}));
    let questions = merge_answers(&body_v, &ans_v);

    let code = {
        let mut c = room_code();
        for _ in 0..5 {
            let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM exam_rooms WHERE code = ?1")
                .bind(&c)
                .fetch_optional(&s.db)
                .await
                .unwrap_or(None);
            if exists.is_none() {
                break;
            }
            c = room_code();
        }
        c
    };

    let dur = r.duration_min.unwrap_or(duration_min).clamp(5, 360);
    let id = uid();
    let host = r
        .host
        .clone()
        .filter(|h| !h.trim().is_empty())
        .map(|h| h.trim_end_matches('/').to_string())
        .unwrap_or_else(|| format!("http://{}:8080", lan_ip()));
    let path = r.path.clone().unwrap_or_else(|| "/ide/mtihani".into());
    let join_url = format!("{}{}?exam={}", host, path, code);

    let room_title = r.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or(title);
    let res = sqlx::query(
        "INSERT INTO exam_rooms
         (id, code, exam_id, title, subject, level, duration_min, shuffle, passcode, status, join_url, questions, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'open',?10,?11,?12)",
    )
    .bind(&id)
    .bind(&code)
    .bind(&r.exam_id)
    .bind(&room_title)
    .bind(&subject)
    .bind(&level)
    .bind(dur)
    .bind(r.shuffle.unwrap_or(1))
    .bind(r.passcode.clone().unwrap_or_default())
    .bind(&join_url)
    .bind(questions.to_string())
    .bind(now())
    .execute(&s.db)
    .await;
    if let Err(e) = res {
        return Json(json!({"ok": false, "error": e.to_string()}));
    }

    let qr = qr_svg(&join_url);
    Json(json!({
        "ok": true, "id": id, "code": code, "title": room_title,
        "subject": subject, "level": level, "duration_min": dur,
        "join_url": join_url, "qr_svg": qr, "lan_ip": lan_ip(),
        "questions": flatten_questions(&questions).len()
    }))
}

/// Unganisha majibu ya mtihani kwenye maswali (kwa faharasa ya sehemu/swali).
fn merge_answers(body: &Value, answers: &Value) -> Value {
    let a_sections = answers["sections"].as_array().cloned().unwrap_or_default();
    let mut out_sections: Vec<Value> = Vec::new();
    if let Some(b_sections) = body["sections"].as_array() {
        for (si, bsec) in b_sections.iter().enumerate() {
            let a_answers = a_sections
                .get(si)
                .and_then(|s| s["answers"].as_array())
                .cloned()
                .unwrap_or_default();
            let mut questions: Vec<Value> = Vec::new();
            for (qi, q) in bsec["questions"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .enumerate()
            {
                let a = a_answers.get(qi).cloned().unwrap_or(json!({}));
                questions.push(json!({
                    "number": q["number"],
                    "question": q["question"],
                    "marks": q["marks"],
                    "answer": a["answer"],
                    "explanation": a["explanation"]
                }));
            }
            out_sections.push(json!({"name": bsec["name"], "questions": questions}));
        }
    }
    json!({"sections": out_sections})
}

pub async fn rooms_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, String, String)>(
        "SELECT code, title, subject, level, duration_min, status, join_url, created_at
         FROM exam_rooms ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut rooms = Vec::new();
    for (code, title, subject, level, dur, status, join_url, created_at) in rows {
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM exam_participants WHERE room_code = ?1")
            .bind(&code)
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
        rooms.push(json!({
            "code": code, "title": title, "subject": subject, "level": level,
            "duration_min": dur, "status": status, "join_url": join_url,
            "created_at": created_at, "participants": n.0
        }));
    }
    Json(json!({"ok": true, "rooms": rooms, "lan_ip": lan_ip()}))
}

pub async fn room_get(State(s): State<AppState>, Path(code): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, i64, i64, String, String, String, String)>(
        "SELECT title, subject, level, duration_min, shuffle, passcode, status, join_url, created_at
         FROM exam_rooms WHERE code = ?1",
    )
    .bind(&code)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((title, subject, level, dur, shuffle, passcode, status, join_url, created_at)) = row
    else {
        return Json(json!({"ok": false, "error": "chumba haipo"}));
    };
    Json(json!({
        "ok": true, "code": code, "title": title, "subject": subject, "level": level,
        "duration_min": dur, "shuffle": shuffle == 1, "status": status,
        "needs_passcode": !passcode.is_empty(), "join_url": join_url, "created_at": created_at
    }))
}

#[derive(Deserialize)]
pub struct CloseBody {
    pub status: Option<String>,
}

pub async fn room_close(
    State(s): State<AppState>,
    Path(code): Path<String>,
    Json(b): Json<CloseBody>,
) -> Json<Value> {
    let st = b.status.unwrap_or_else(|| "closed".into());
    let _ = sqlx::query("UPDATE exam_rooms SET status = ?1 WHERE code = ?2")
        .bind(&st)
        .bind(&code)
        .execute(&s.db)
        .await;
    publish(&code, json!({"type": "room", "status": st}));
    Json(json!({"ok": true, "code": code, "status": st}))
}

// ============================================================
// HANDLERS — MWANAFUNZI
// ============================================================

#[derive(Deserialize)]
pub struct JoinBody {
    pub name: String,
    pub student_no: Option<String>,
    pub passcode: Option<String>,
}

pub async fn room_join(
    State(s): State<AppState>,
    Path(code): Path<String>,
    Json(b): Json<JoinBody>,
) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({"ok": false, "error": "jina linahitajika"}));
    }
    let room = sqlx::query_as::<_, (String, String, i64, String, String)>(
        "SELECT questions, passcode, shuffle, status, title FROM exam_rooms WHERE code = ?1",
    )
    .bind(&code)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((questions, passcode, shuffle, status, title)) = room else {
        return Json(json!({"ok": false, "error": "chumba haipo"}));
    };
    if status != "open" {
        return Json(json!({"ok": false, "error": "mtihani umefungwa"}));
    }
    if !passcode.is_empty() && b.passcode.as_deref().unwrap_or("") != passcode {
        return Json(json!({"ok": false, "error": "code ya siri si sahihi"}));
    }
    let questions: Value = serde_json::from_str(&questions).unwrap_or(json!({}));
    let all = flatten_questions(&questions);

    // Mwanafunzi aliyejiunga awali (jina + namba) hurudishiwa kikao chake.
    let student_no = b.student_no.clone().unwrap_or_default();
    let existing = sqlx::query_as::<_, (String, String, String, i64, String, String, f64, f64)>(
        "SELECT id, token, ordering, current, answers, status, score, total
         FROM exam_participants WHERE room_code = ?1 AND name = ?2 AND student_no = ?3",
    )
    .bind(&code)
    .bind(b.name.trim())
    .bind(&student_no)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);

    let (token, ordering, current, answers, pstatus, score, total) = if let Some(e) = existing {
        let ordering: Vec<usize> = serde_json::from_str(&e.2).unwrap_or_default();
        (e.1, ordering, e.3, e.4, e.5, e.6, e.7)
    } else {
        let ordering = make_ordering(all.len(), shuffle == 1);
        let token = uid();
        let id = uid();
        let ord_json = serde_json::to_string(&ordering).unwrap_or("[]".into());
        let _ = sqlx::query(
            "INSERT INTO exam_participants
             (id, room_code, token, name, student_no, ordering, answers, current, score, total, status, joined_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,'{}',0,0,0,'active',?7,?7)",
        )
        .bind(&id)
        .bind(&code)
        .bind(&token)
        .bind(b.name.trim())
        .bind(&student_no)
        .bind(&ord_json)
        .bind(now())
        .execute(&s.db)
        .await;
        (token, ordering, 0, "{}".into(), "active".into(), 0.0, 0.0)
    };

    let total_marks: f64 = all
        .iter()
        .map(|q| q["marks"].as_f64().unwrap_or(0.0))
        .sum();
    let questions = public_questions(&json!({"questions": questions}), &ordering);

    publish(&code, json!({
        "type": "joined", "name": b.name.trim(), "student_no": student_no,
        "status": pstatus, "at": now()
    }));

    Json(json!({
        "ok": true, "code": code, "title": title, "token": token,
        "questions": questions, "current": current,
        "answers": serde_json::from_str::<Value>(&answers).unwrap_or(json!({})),
        "status": pstatus, "score": score, "total": total_marks
    }))
}

#[derive(Deserialize)]
pub struct SaveBody {
    pub token: String,
    pub index: i64,
    pub answer: String,
    pub current: Option<i64>,
}

pub async fn room_save(
    State(s): State<AppState>,
    Path(code): Path<String>,
    Json(b): Json<SaveBody>,
) -> Json<Value> {
    let p = sqlx::query_as::<_, (String, String)>(
        "SELECT id, answers FROM exam_participants WHERE room_code = ?1 AND token = ?2",
    )
    .bind(&code)
    .bind(&b.token)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((pid, answers)) = p else {
        return Json(json!({"ok": false, "error": "kikao hakipo"}));
    };
    let mut map: Map<String, Value> =
        serde_json::from_str(&answers).unwrap_or_default();
    map.insert(b.index.to_string(), json!(b.answer));
    let cur = b.current.unwrap_or(b.index);
    let _ = sqlx::query(
        "UPDATE exam_participants SET answers = ?1, current = ?2, updated_at = ?3 WHERE id = ?4",
    )
    .bind(Value::Object(map).to_string())
    .bind(cur)
    .bind(now())
    .bind(&pid)
    .execute(&s.db)
    .await;

    let name: (String,) = sqlx::query_as("SELECT name FROM exam_participants WHERE id = ?1")
        .bind(&pid)
        .fetch_one(&s.db)
        .await
        .unwrap_or((String::new(),));
    publish(&code, json!({
        "type": "progress", "name": name.0, "current": cur, "at": now()
    }));
    Json(json!({"ok": true}))
}

#[derive(Deserialize)]
pub struct SubmitBody {
    pub token: String,
    pub answers: Option<Value>,
}

pub async fn room_submit(
    State(s): State<AppState>,
    Path(code): Path<String>,
    Json(b): Json<SubmitBody>,
) -> Json<Value> {
    let p = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, name, answers, ordering, status FROM exam_participants WHERE room_code = ?1 AND token = ?2",
    )
    .bind(&code)
    .bind(&b.token)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((pid, name, stored, ordering, status)) = p else {
        return Json(json!({"ok": false, "error": "kikao hakipo"}));
    };
    if status == "submitted" {
        return Json(json!({"ok": false, "error": "umeshawasilisha"}));
    }
    let room = sqlx::query_as::<_, (String,)>( "SELECT questions FROM exam_rooms WHERE code = ?1")
        .bind(&code)
        .fetch_optional(&s.db)
        .await
        .unwrap_or(None);
    let Some((questions,)) = room else {
        return Json(json!({"ok": false, "error": "chumba haipo"}));
    };
    let questions: Value = serde_json::from_str(&questions).unwrap_or(json!({}));
    let all = flatten_questions(&questions);
    let ordering: Vec<usize> = serde_json::from_str(&ordering).unwrap_or_default();

    // Majibu ya mwisho: yale yaliyotumwa huchukua nafasi ya yale yaliyohifadhiwa.
    let mut final_answers: Map<String, Value> = serde_json::from_str(&stored).unwrap_or_default();
    if let Some(a) = b.answers.as_ref().and_then(|v| v.as_object()) {
        for (k, v) in a {
            final_answers.insert(k.clone(), v.clone());
        }
    }

    let mut score = 0.0f64;
    let mut total = 0.0f64;
    let mut review = 0i64;
    let mut detail: Vec<Value> = Vec::new();
    let mut wrong: Vec<Value> = Vec::new();

    for (pos, qi) in ordering.iter().enumerate() {
        let Some(q) = all.get(*qi) else { continue };
        let marks = q["marks"].as_f64().unwrap_or(0.0);
        total += marks;
        let given = final_answers
            .get(&pos.to_string())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let expected = q["answer"].as_str().unwrap_or("");
        let (got, correct, needs_review) = grade_one(expected, &given, marks);
        score += got;
        if needs_review {
            review += 1;
        }
        if !correct {
            wrong.push(json!({
                "number": q["number"], "question": q["question"],
                "answer": given, "expected": expected, "explanation": q["explanation"]
            }));
        }
        detail.push(json!({
            "number": q["number"], "marks": marks, "awarded": got,
            "correct": correct, "needs_review": needs_review
        }));
    }

    let _ = sqlx::query(
        "UPDATE exam_participants SET answers = ?1, score = ?2, total = ?3, status = 'submitted', current = ?4, updated_at = ?5 WHERE id = ?6",
    )
    .bind(Value::Object(final_answers).to_string())
    .bind(score)
    .bind(total)
    .bind(ordering.len() as i64)
    .bind(now())
    .bind(&pid)
    .execute(&s.db)
    .await;

    publish(&code, json!({
        "type": "submitted", "name": name, "score": score, "total": total, "at": now()
    }));

    Json(json!({
        "ok": true, "name": name, "score": score, "total": total,
        "percent": if total > 0.0 { (score / total * 100.0).round() } else { 0.0 },
        "needs_review": review, "detail": detail, "wrong": wrong
    }))
}

// ============================================================
// HANDLERS — DASHIBODI YA MWALIMU
// ============================================================

pub async fn room_live(State(s): State<AppState>, Path(code): Path<String>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, i64, f64, f64, String, String, String)>(
        "SELECT name, student_no, ordering, current, score, total, status, joined_at, updated_at
         FROM exam_participants WHERE room_code = ?1 ORDER BY joined_at ASC",
    )
    .bind(&code)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut active = 0;
    let mut submitted = 0;
    let mut participants = Vec::new();
    for (name, no, ordering, current, score, total, status, joined, updated) in rows {
        let n = serde_json::from_str::<Vec<usize>>(&ordering)
            .map(|v| v.len())
            .unwrap_or(0);
        if status == "submitted" {
            submitted += 1;
        } else {
            active += 1;
        }
        participants.push(json!({
            "name": name, "student_no": no, "answered": current, "of": n,
            "score": score, "total": total, "status": status,
            "joined_at": joined, "updated_at": updated
        }));
    }
    Json(json!({
        "ok": true, "code": code, "active": active, "submitted": submitted,
        "participants": participants
    }))
}

pub async fn room_results(State(s): State<AppState>, Path(code): Path<String>) -> Json<Value> {
    let room = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT title, subject, duration_min, status FROM exam_rooms WHERE code = ?1",
    )
    .bind(&code)
    .fetch_optional(&s.db)
    .await
    .unwrap_or(None);
    let Some((title, subject, dur, status)) = room else {
        return Json(json!({"ok": false, "error": "chumba haipo"}));
    };

    let rows = sqlx::query_as::<_, (String, String, f64, f64, String, String)>(
        "SELECT name, student_no, score, total, status, joined_at
         FROM exam_participants WHERE room_code = ?1 ORDER BY score DESC",
    )
    .bind(&code)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut students = Vec::new();
    let mut sum = 0.0;
    let mut n = 0;
    for (name, no, score, total, st, joined) in rows {
        let pct = if total > 0.0 { (score / total * 100.0).round() } else { 0.0 };
        if st == "submitted" {
            sum += pct;
            n += 1;
        }
        students.push(json!({
            "name": name, "student_no": no, "score": score, "total": total,
            "percent": pct, "status": st, "joined_at": joined
        }));
    }
    let average = if n > 0 { (sum / n as f64).round() } else { 0.0 };
    let passed = students
        .iter()
        .filter(|s| s["status"] == "submitted" && s["percent"].as_f64().unwrap_or(0.0) >= 50.0)
        .count();

    Json(json!({
        "ok": true, "code": code, "title": title, "subject": subject,
        "duration_min": dur, "status": status, "average": average,
        "submitted": n, "passed": passed, "students": students
    }))
}

// ============================================================
// WEBSOCKET — matangazo ya wakati halisi
// ============================================================

pub async fn room_ws(ws: WebSocketUpgrade, Path(code): Path<String>) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, code))
}

async fn handle_ws(mut socket: WebSocket, code: String) {
    let mut rx = subscribe(&code);
    let hello = json!({"type": "hello", "code": code, "at": now()});
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
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_ni_herufi_namba_sita() {
        let c = room_code();
        assert_eq!(c.len(), 6);
        assert!(!c.contains('O') && !c.contains('0') && !c.contains('I') && !c.contains('1'));
        assert!(c.chars().all(|ch| ch.is_ascii_alphanumeric()));
    }

    #[test]
    fn qr_svg_hutengenezwa_bila_intaneti() {
        let svg = qr_svg("http://192.168.1.100:8080/ide/elimu?exam=ABC123").expect("QR");
        assert!(svg.contains("<svg"));
        assert!(svg.len() > 200);
    }

    #[test]
    fn norm_huondoa_alama() {
        assert_eq!(norm("2 + 2 = ?"), "2 2");
        assert_eq!(norm("Nairobi."), "nairobi");
    }

    #[test]
    fn grading_hukubali_jibu_sahihi() {
        let (got, ok, review) = grade_one("Nairobi", "nairobi", 10.0);
        assert_eq!(got, 10.0);
        assert!(ok && !review);
    }

    #[test]
    fn grading_huweka_alama_kwa_mwalimu_kwa_maandishi_marefu() {
        let long = "Mzunguko wa maji ni mchakato mrefu sana wa asili unaorudia rudia kila siku duniani";
        let (got, ok, review) = grade_one(long, "sijui", 10.0);
        assert_eq!(got, 0.0);
        assert!(!ok, "jibu baya halipati alama");
        assert!(review, "maandishi marefu yanahitaji mwalimu");
    }

    #[test]
    fn grading_hukubali_jibu_lililo_na_kikundi() {
        let (got, ok, _) = grade_one("Nairobi", "Nairobi ni mji", 10.0);
        assert_eq!(got, 10.0);
        assert!(ok);
    }

    #[test]
    fn make_ordering_huchanganya_bila_kupoteza() {
        let v = make_ordering(10, true);
        assert_eq!(v.len(), 10);
        let mut s = v.clone();
        s.sort();
        assert_eq!(s, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn make_ordering_bila_shuffle_hurudisha_mfululizo() {
        assert_eq!(make_ordering(4, false), vec![0, 1, 2, 3]);
    }

    #[test]
    fn flatten_questions_hupanga_kwa_sehemu() {
        let q = json!({"sections":[
            {"name":"Sehemu A","questions":[{"number":1,"question":"a","marks":5}]},
            {"name":"Sehemu B","questions":[{"number":2,"question":"b","marks":10}]}
        ]});
        let f = flatten_questions(&q);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0]["section"], "Sehemu A");
        assert_eq!(f[1]["marks"], 10);
    }

    #[test]
    fn public_questions_huficha_majibu() {
        let room = json!({"questions":{"sections":[
            {"name":"A","questions":[
                {"number":1,"question":"2+2?","marks":5,"answer":"4","explanation":"hesabu"}
            ]}
        ]}});
        let pubq = public_questions(&room, &[0]);
        assert_eq!(pubq[0]["question"], "2+2?");
        assert!(pubq[0].get("answer").is_none(), "jibu lisifichuliwe!");
    }

    #[test]
    fn merge_answers_huunganisha_kwa_faharasa() {
        let body = json!({"sections":[{"name":"A","questions":[
            {"number":1,"question":"q1","marks":5},
            {"number":2,"question":"q2","marks":5}
        ]}]});
        let ans = json!({"sections":[{"name":"A","answers":[
            {"number":1,"answer":"jibu1","explanation":"e1"},
            {"number":2,"answer":"jibu2","explanation":"e2"}
        ]}]});
        let m = merge_answers(&body, &ans);
        assert_eq!(m["sections"][0]["questions"][0]["answer"], "jibu1");
        assert_eq!(m["sections"][0]["questions"][1]["answer"], "jibu2");
    }

    #[test]
    fn lan_ip_haiko_tupu() {
        let ip = lan_ip();
        assert!(!ip.trim().is_empty());
    }

    // ---- Integration: mtiririko kamili wa chumba (DB halisi) ----

    async fn test_db() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("exam_lan_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let url = format!("sqlite://{}?mode=rwc", dir.join("t.db").display());
        let db = SqlitePool::connect(&url).await.unwrap();
        crate::education::init_tables(&db).await;
        init_tables(&db).await;
        db
    }

    /// Huingiza mtihani wa majaribio (maswali + majibu).
    async fn seed_exam(db: &SqlitePool) -> String {
        let q = json!({"sections":[{"name":"A","questions":[
            {"number":1,"question":"2+2?","marks":50},
            {"number":2,"question":"Mji mkuu wa Kenya?","marks":50}
        ]}]});
        let a = json!({"sections":[{"name":"A","answers":[
            {"number":1,"answer":"4","explanation":"hesabu"},
            {"number":2,"answer":"Nairobi","explanation":"jiografia"}
        ]}]});
        let id = uid();
        sqlx::query(
            "INSERT INTO edu_exams (id,title,subject,level,duration_min,num_questions,body,answers,created_at)
             VALUES (?1,'Mtihani wa Jaribio','Sayansi','O-Level',45,2,?2,?3,?4)",
        )
        .bind(&id)
        .bind(q.to_string())
        .bind(a.to_string())
        .bind(now())
        .execute(db)
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn mtiririko_kamili_wa_chumba_hufanya_kazi() {
        let db = test_db().await;
        let exam = seed_exam(&db).await;

        // 1) Mwalimu hufungua chumba — maswali + majibu huunganishwa.
        let row = sqlx::query_as::<_, (String, String, i64, String, String, String)>(
            "SELECT subject, level, duration_min, body, answers, title FROM edu_exams WHERE id = ?1",
        )
        .bind(&exam)
        .fetch_one(&db)
        .await
        .unwrap();
        let body: Value = serde_json::from_str(&row.3).unwrap();
        let answers: Value = serde_json::from_str(&row.4).unwrap();
        let merged = merge_answers(&body, &answers);
        let all = flatten_questions(&merged);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0]["answer"], "4");

        // 2) Mwanafunzi hujiunga — anapata maswali BILA majibu.
        let ordering = make_ordering(all.len(), false);
        let pubq = public_questions(&json!({"questions": merged}), &ordering);
        assert_eq!(pubq.as_array().unwrap().len(), 2);
        assert!(pubq[0].get("answer").is_none());

        // 3) Huwasilisha majibu; alama huhesabiwa papo hapo.
        let mut final_answers = Map::new();
        final_answers.insert("0".into(), json!("4"));
        final_answers.insert("1".into(), json!("nairobi"));
        let mut score = 0.0;
        let mut total = 0.0;
        for (pos, qi) in ordering.iter().enumerate() {
            let q = &all[*qi];
            let marks = q["marks"].as_f64().unwrap_or(0.0);
            total += marks;
            let given = final_answers.get(&pos.to_string()).unwrap().as_str().unwrap();
            let (got, _, _) = grade_one(q["answer"].as_str().unwrap(), given, marks);
            score += got;
        }
        assert_eq!(total, 100.0);
        assert_eq!(score, 100.0, "majibu sahihi yote yanapata alama kamili");
    }

    #[tokio::test]
    async fn majibu_hufichwa_kwa_wanafunzi_wote() {
        let db = test_db().await;
        let exam = seed_exam(&db).await;
        let row = sqlx::query_as::<_, (String, String)>("SELECT body, answers FROM edu_exams WHERE id = ?1")
            .bind(&exam)
            .fetch_one(&db)
            .await
            .unwrap();
        let merged = merge_answers(
            &serde_json::from_str(&row.0).unwrap(),
            &serde_json::from_str(&row.1).unwrap(),
        );
        let all = flatten_questions(&merged);
        // Kila mpangilio wa shuffle huficha majibu.
        for _ in 0..5 {
            let ord = make_ordering(all.len(), true);
            let pubq = public_questions(&json!({"questions": merged}), &ord);
            for q in pubq.as_array().unwrap() {
                assert!(q.get("answer").is_none(), "jibu linaonekana kwa mwanafunzi!");
            }
        }
    }

    #[tokio::test]
    async fn shuffle_humpa_kila_mwanafunzi_mpangilio_wa_kawaida() {
        let db = test_db().await;
        let exam = seed_exam(&db).await;
        let row = sqlx::query_as::<_, (String, String)>("SELECT body, answers FROM edu_exams WHERE id = ?1")
            .bind(&exam)
            .fetch_one(&db)
            .await
            .unwrap();
        let merged = merge_answers(
            &serde_json::from_str(&row.0).unwrap(),
            &serde_json::from_str(&row.1).unwrap(),
        );
        let all = flatten_questions(&merged);
        let mut orders = std::collections::HashSet::new();
        for _ in 0..40 {
            orders.insert(make_ordering(all.len(), true));
        }
        assert!(orders.len() > 1, "shuffle inatakiwa itoe mpangilio tofauti");
        for o in &orders {
            let mut s = o.clone();
            s.sort();
            assert_eq!(s, (0..all.len()).collect::<Vec<_>>());
        }
    }

    #[tokio::test]
    async fn init_tables_huunda_majedwali_mawili() {
        let db = test_db().await;
        let (n,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('exam_rooms','exam_participants')",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(n, 2);
    }
}
