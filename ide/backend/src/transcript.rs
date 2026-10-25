//! transcript.rs — ELIMUPRO Awamu 11: matokeo, transkripti na vyeti (moduli 15–16).
//!
//! Huchukua alama zilizohifadhiwa (`ep_marks`, Awamu 6) na kuzigeuza:
//!   - Matokeo ya muhula: kila somo na alama yake, jumla na wastani.
//!   - GPA: pointi kwa herufi (A=5, B=4, C=3, D=2, F=0) × credits / credits.
//!   - Daraja (classification): First/Second/Third kwa GPA.
//!   - Transkripti: matokeo ya muhula wote wa mwanafunzi, GPA ya jumla.
//!   - Cheti: hupata `serial` ya kipekee na `verify_code` ya kuthibitisha.

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
        r#"CREATE TABLE IF NOT EXISTS ep_transcripts (
            id TEXT PRIMARY KEY,
            student_id TEXT NOT NULL,
            gpa REAL NOT NULL DEFAULT 0,
            classification TEXT NOT NULL DEFAULT '',
            issued_at TEXT NOT NULL,
            issued_by TEXT NOT NULL DEFAULT ''
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_certificates (
            id TEXT PRIMARY KEY,
            student_id TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'transcript', -- transcript|completion|award
            title TEXT NOT NULL DEFAULT '',
            serial TEXT NOT NULL UNIQUE,
            verify_code TEXT NOT NULL UNIQUE,
            issued_at TEXT NOT NULL,
            revoked INTEGER NOT NULL DEFAULT 0
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/results/status", get(status))
        .route("/api/results/student", get(student_results))
        .route("/api/results/level", get(level_results))
        .route("/api/results/transcript", get(transcript))
        .route("/api/results/transcript/issue", post(issue_transcript))
        .route("/api/results/certificate/issue", post(issue_certificate))
        .route("/api/results/certificates", get(certificates_list))
        .route("/api/results/certificate/:id", get(certificate_get))
        .route("/api/results/verify", get(verify))
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

/// Pointi za GPA kwa alama (0–100).
pub fn grade_point(score: f64) -> f64 {
    if score >= 80.0 {
        5.0
    } else if score >= 70.0 {
        4.0
    } else if score >= 60.0
    {
        3.0
    } else if score >= 50.0 {
        2.0
    } else if score >= 40.0 {
        1.0
    } else {
        0.0
    }
}

/// Herufi ya daraja.
pub fn grade_letter(score: f64) -> &'static str {
    if score >= 80.0 {
        "A"
    } else if score >= 70.0 {
        "B"
    } else if score >= 60.0 {
        "C"
    } else if score >= 50.0 {
        "D"
    } else if score >= 40.0 {
        "E"
    } else {
        "F"
    }
}

/// Daraja la mwisho kutokana na GPA (0–5).
pub fn classification(gpa: f64) -> &'static str {
    if gpa >= 4.4 {
        "First Class"
    } else if gpa >= 3.5 {
        "Second Class Upper"
    } else if gpa >= 2.7 {
        "Second Class Lower"
    } else if gpa >= 2.0 {
        "Third Class"
    } else if gpa > 0.0 {
        "Pass"
    } else {
        "Fail"
    }
}

/// GPA kutokana na jozi (pointi, credits).
pub fn compute_gpa(rows: &[(f64, f64)]) -> f64 {
    let credits: f64 = rows.iter().map(|r| r.1).sum();
    if credits <= 0.0 {
        return 0.0;
    }
    let pts: f64 = rows.iter().map(|r| r.0 * r.1).sum();
    ((pts / credits) * 100.0).round() / 100.0
}

/// Namba ya serial ya cheti: `CERT/YYYY/NNNNNN`.
pub fn serial_no(year: i32, seq: i64) -> String {
    format!("CERT/{year}/{:06}", seq.max(1))
}

/// Kodi ya kuthibitisha: herufi 10 bila herufi zinazochanganya.
pub fn verify_code(seed: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut out = String::with_capacity(10);
    for _ in 0..10 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        out.push(ALPHABET[(x >> 33) as usize % ALPHABET.len()] as char);
    }
    out
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (students,): (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT student_id) FROM ep_marks")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "marks": table_count(&s.db, "ep_marks").await,
        "students_with_marks": students,
        "transcripts": table_count(&s.db, "ep_transcripts").await,
        "certificates": table_count(&s.db, "ep_certificates").await,
        "grade_scale": [
            { "min": 80, "letter": "A", "points": 5.0 },
            { "min": 70, "letter": "B", "points": 4.0 },
            { "min": 60, "letter": "C", "points": 3.0 },
            { "min": 50, "letter": "D", "points": 2.0 },
            { "min": 40, "letter": "E", "points": 1.0 },
            { "min": 0,  "letter": "F", "points": 0.0 }
        ]
    }))
}

// ============================================================
// MATOKEO
// ============================================================

#[derive(Deserialize)]
pub struct ResultQuery {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub term: String,
}

/// Matokeo ya mwanafunzi: alama kwa kila somo + jumla + GPA.
///
/// **KANUNI**: `ep_marks` ina `column_id`, SI `subject_id` — somo hupatikana
/// kwa `ep_marks_columns.subject_id`. Asilimia hupimwa kwa uzito kwa kutumia
/// `marks::compute_total` (mstari mmoja wa ukweli na Awamu 6).
pub async fn student_results(State(s): State<AppState>, Query(q): Query<ResultQuery>) -> Json<Value> {
    compute_student_results(&s.db, &q.student_id).await
}

/// Hesabu halisi ya matokeo ya mwanafunzi (hupimwa moja kwa moja kwenye tests).
async fn compute_student_results(db: &SqlitePool, student_id: &str) -> Json<Value> {
    if student_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "student_id inahitajika" }));
    }
    let st = sqlx::query_as::<_, (String, String, String)>(
        "SELECT COALESCE(full_name,''),COALESCE(reg_no,''),COALESCE(level_id,'')
         FROM ep_students WHERE id=?1",
    )
    .bind(student_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    let Some((name, reg, level_id)) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hayupo" }));
    };

    let rows = sqlx::query_as::<_, (String, String, String, f64, String, f64, f64, f64)>(
        "SELECT su.id,COALESCE(su.name,''),COALESCE(su.code,''),
                CAST(COALESCE(su.credits,0) AS REAL),
                COALESCE(mc.id,''),CAST(COALESCE(mc.max_marks,100.0) AS REAL),
                CAST(COALESCE(mc.weight,1.0) AS REAL),
                CAST(COALESCE(m.score,-1.0) AS REAL)
         FROM ep_subjects su
         LEFT JOIN ep_marks_columns mc ON mc.subject_id=su.id AND mc.level_id=su.level_id
         LEFT JOIN ep_marks m ON m.column_id=mc.id AND m.student_id=?1
         WHERE (?2='' OR su.level_id=?2)
         ORDER BY su.name",
    )
    .bind(student_id)
    .bind(&level_id)
    .fetch_all(db)
    .await
    .unwrap_or_default();

    // Panga kwa somo (rows zinaweza kurudi kwa kila nguzo).
    let mut order: Vec<String> = Vec::new();
    let mut grouped: std::collections::HashMap<String, (String, String, f64, Vec<(String, f64, f64)>, std::collections::HashMap<String, f64>)> =
        std::collections::HashMap::new();
    for (sid, sname, scode, credits, col_id, max_marks, weight, score) in rows {
        let e = grouped.entry(sid.clone()).or_insert_with(|| {
            order.push(sid.clone());
            (sname.clone(), scode.clone(), credits, Vec::new(), std::collections::HashMap::new())
        });
        if !col_id.is_empty() {
            e.3.push((col_id.clone(), max_marks, weight));
            if score >= 0.0 {
                e.4.insert(col_id, score);
            }
        }
    }

    let mut pairs: Vec<(f64, f64)> = Vec::new();
    let mut subjects: Vec<Value> = Vec::new();
    let mut total = 0.0f64;
    let mut entered = 0usize;
    for sid in &order {
        let (sname, scode, credits, cols, scores) = grouped.get(sid).cloned().unwrap();
        let credits = if credits > 0.0 { credits } else { 1.0 };
        let (_t, _m, pct, _grade) = crate::marks::compute_total(&cols, &scores);
        let has_marks = pct >= 0.0;
        if has_marks {
            total += pct;
            entered += 1;
            pairs.push((grade_point(pct), credits));
        }
        subjects.push(json!({
            "subject_id": sid, "subject": sname, "code": scode,
            "score": if has_marks { json!((pct * 100.0).round() / 100.0) } else { json!(null) },
            "letter": if has_marks { grade_letter(pct) } else { "-" },
            "points": if has_marks { grade_point(pct) } else { 0.0 },
            "credits": credits,
            "entered": has_marks
        }));
    }
    let gpa = compute_gpa(&pairs);
    Json(json!({
        "ok": true,
        "student": { "id": student_id, "name": name, "reg_no": reg, "level_id": level_id },
        "subjects": subjects,
        "total": (total * 100.0).round() / 100.0,
        "average": if entered == 0 { 0.0 } else { ((total / entered as f64) * 100.0).round() / 100.0 },
        "subjects_entered": entered,
        "gpa": gpa,
        "classification": classification(gpa)
    }))
}

/// Matokeo ya level yote: kila mwanafunzi, wastani, nafasi (rank).
pub async fn level_results(State(s): State<AppState>, Query(q): Query<ResultQuery>) -> Json<Value> {
    if q.level_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "level_id inahitajika" }));
    }
    let ids = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM ep_students WHERE level_id=?1 ORDER BY full_name",
    )
    .bind(&q.level_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut list: Vec<(String, String, String, f64, f64, i64)> = Vec::new();
    for (sid,) in ids {
        let v = compute_student_results(&s.db, &sid).await.0;
        if v["ok"] != json!(true) {
            continue;
        }
        let sname = v["student"]["name"].as_str().unwrap_or("").to_string();
        let sreg = v["student"]["reg_no"].as_str().unwrap_or("").to_string();
        list.push((
            sid,
            sname,
            sreg,
            v["average"].as_f64().unwrap_or(0.0),
            v["gpa"].as_f64().unwrap_or(0.0),
            v["subjects_entered"].as_i64().unwrap_or(0),
        ));
    }
    // Nafasi kwa wastani (juu→chini).
    list.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    let total = list.len();
    let out: Vec<Value> = list
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            json!({
                "rank": i + 1, "student_id": r.0, "student": r.1, "reg_no": r.2,
                "average": r.3, "gpa": r.4, "subjects": r.5,
                "classification": classification(r.4)
            })
        })
        .collect();
    Json(json!({ "ok": true, "level_id": q.level_id, "count": total, "results": out }))
}

// ============================================================
// TRANSKRIPTI
// ============================================================

pub async fn transcript(State(s): State<AppState>, Query(q): Query<ResultQuery>) -> Json<Value> {
    let mut v = compute_student_results(&s.db, &q.student_id).await.0;
    if v["ok"] != json!(true) {
        return Json(v);
    }
    // Ongeza rekodi ya transkripti ikiwa imetolewa.
    let sid = v["student"]["id"].as_str().unwrap_or("").to_string();
    let issued = sqlx::query_as::<_, (String, f64, String, String)>(
        "SELECT id,gpa,classification,issued_at FROM ep_transcripts WHERE student_id=?1 ORDER BY issued_at DESC LIMIT 1",
    )
    .bind(&sid)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if let Some((id, gpa, cls, at)) = issued {
        v["issued_transcript"] = json!({ "id": id, "gpa": gpa, "classification": cls, "issued_at": at });
    }
    Json(v)
}

#[derive(Deserialize)]
pub struct IssueIn {
    pub student_id: String,
    #[serde(default)]
    pub issued_by: String,
}

pub async fn issue_transcript(State(s): State<AppState>, Json(b): Json<IssueIn>) -> Json<Value> {
    let v = compute_student_results(&s.db, &b.student_id).await.0;
    if v["ok"] != json!(true) {
        return Json(v);
    }
    let gpa = v["gpa"].as_f64().unwrap_or(0.0);
    let cls = v["classification"].as_str().unwrap_or("").to_string();
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_transcripts (id,student_id,gpa,classification,issued_at,issued_by)
         VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&id)
    .bind(&b.student_id)
    .bind(gpa)
    .bind(&cls)
    .bind(now())
    .bind(&b.issued_by)
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "gpa": gpa, "classification": cls }))
}

// ============================================================
// VYETI
// ============================================================

#[derive(Deserialize)]
pub struct CertIn {
    pub student_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub issued_by: String,
}

pub async fn issue_certificate(State(s): State<AppState>, Json(b): Json<CertIn>) -> Json<Value> {
    let st = sqlx::query_as::<_, (String, String)>(
        "SELECT COALESCE(full_name,''),COALESCE(reg_no,'') FROM ep_students WHERE id=?1",
    )
    .bind(&b.student_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((name, _reg)) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hayupo" }));
    };
    let kind = match b.kind.trim().to_lowercase().as_str() {
        "completion" | "kumaliza" => "completion",
        "award" | "tuzo" => "award",
        _ => "transcript",
    };
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM ep_certificates")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let year = chrono::Local::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let serial = serial_no(year, seq);
    let seed = (chrono::Local::now().timestamp_millis() as u64) ^ (seq as u64).wrapping_mul(2654435761);
    let code = verify_code(seed);
    let id = uid();
    let title = if b.title.trim().is_empty() {
        format!("Cheti cha {name}")
    } else {
        b.title.trim().to_string()
    };
    let _ = sqlx::query(
        "INSERT INTO ep_certificates (id,student_id,kind,title,serial,verify_code,issued_at,revoked)
         VALUES (?1,?2,?3,?4,?5,?6,?7,0)",
    )
    .bind(&id)
    .bind(&b.student_id)
    .bind(kind)
    .bind(&title)
    .bind(&serial)
    .bind(&code)
    .bind(now())
    .bind(&b.issued_by)
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "id": id, "serial": serial, "verify_code": code,
        "kind": kind, "title": title, "student": name
    }))
}

#[derive(Deserialize)]
pub struct CertQuery {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub kind: String,
}

pub async fn certificates_list(State(s): State<AppState>, Query(q): Query<CertQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64)>(
        "SELECT c.id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),c.kind,c.title,
                c.serial,c.verify_code,c.revoked
         FROM ep_certificates c LEFT JOIN ep_students st ON st.id=c.student_id
         WHERE (?1='' OR c.student_id=?1) AND (?2='' OR c.kind=?2)
         ORDER BY c.issued_at DESC LIMIT 500",
    )
    .bind(&q.student_id)
    .bind(&q.kind)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "certificates": rows.into_iter().map(|r| json!({
            "id": r.0, "student": r.1, "reg_no": r.2, "kind": r.3, "title": r.4,
            "serial": r.5, "verify_code": r.6, "revoked": r.7 == 1
        })).collect::<Vec<_>>()
    }))
}

pub async fn certificate_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, i64)>(
        "SELECT c.id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),c.kind,c.title,
                c.serial,c.verify_code,c.issued_at,c.revoked
         FROM ep_certificates c LEFT JOIN ep_students st ON st.id=c.student_id WHERE c.id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match r {
        None => Json(json!({ "ok": false, "error": "cheti hakipo" })),
        Some(x) => Json(json!({
            "ok": true,
            "certificate": {
                "id": x.0, "student": x.1, "reg_no": x.2, "kind": x.3, "title": x.4,
                "serial": x.5, "verify_code": x.6, "issued_at": x.7, "revoked": x.8 == 1
            }
        })),
    }
}

#[derive(Deserialize)]
pub struct VerifyQuery {
    #[serde(default)]
    pub serial: String,
    #[serde(default)]
    pub code: String,
}

/// Uthibitisho wa umma: serial + kodi → halali/imebatilishwa.
pub async fn verify(State(s): State<AppState>, Query(q): Query<VerifyQuery>) -> Json<Value> {
    if q.serial.trim().is_empty() || q.code.trim().is_empty() {
        return Json(json!({ "ok": false, "valid": false, "error": "serial na code zinahitajika" }));
    }
    let r = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT COALESCE(st.full_name,''),c.kind,c.title,c.issued_at,c.revoked
         FROM ep_certificates c LEFT JOIN ep_students st ON st.id=c.student_id
         WHERE c.serial=?1 AND c.verify_code=?2",
    )
    .bind(q.serial.trim())
    .bind(q.code.trim().to_uppercase())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match r {
        None => Json(json!({ "ok": true, "valid": false, "error": "cheti hakipatikani" })),
        Some((name, kind, title, at, revoked)) => Json(json!({
            "ok": true,
            "valid": revoked == 0,
            "revoked": revoked == 1,
            "certificate": { "student": name, "kind": kind, "title": title, "issued_at": at }
        })),
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointi_za_gpa_ni_sahihi() {
        assert_eq!(grade_point(85.0), 5.0);
        assert_eq!(grade_point(80.0), 5.0);
        assert_eq!(grade_point(75.0), 4.0);
        assert_eq!(grade_point(65.0), 3.0);
        assert_eq!(grade_point(55.0), 2.0);
        assert_eq!(grade_point(45.0), 1.0);
        assert_eq!(grade_point(30.0), 0.0);
    }

    #[test]
    fn herufi_za_daraja_ni_sahihi() {
        assert_eq!(grade_letter(90.0), "A");
        assert_eq!(grade_letter(80.0), "A");
        assert_eq!(grade_letter(70.0), "B");
        assert_eq!(grade_letter(60.0), "C");
        assert_eq!(grade_letter(50.0), "D");
        assert_eq!(grade_letter(40.0), "E");
        assert_eq!(grade_letter(39.9), "F");
    }

    #[test]
    fn gpa_hupimwa_kwa_credits() {
        // A(5) ya credits 3 + C(3) ya credits 1 = (15+3)/4 = 4.5
        assert_eq!(compute_gpa(&[(5.0, 3.0), (3.0, 1.0)]), 4.5);
        // Hakuna credits → 0.
        assert_eq!(compute_gpa(&[]), 0.0);
        assert_eq!(compute_gpa(&[(5.0, 0.0)]), 0.0);
    }

    #[test]
    fn daraja_la_mwisho_hupimwa() {
        assert_eq!(classification(4.8), "First Class");
        assert_eq!(classification(4.4), "First Class");
        assert_eq!(classification(3.9), "Second Class Upper");
        assert_eq!(classification(3.0), "Second Class Lower");
        assert_eq!(classification(2.2), "Third Class");
        assert_eq!(classification(1.0), "Pass");
        assert_eq!(classification(0.0), "Fail");
    }

    #[test]
    fn serial_na_kodi_ni_za_kipekee() {
        assert_eq!(serial_no(2026, 1), "CERT/2026/000001");
        assert_eq!(serial_no(2026, 0), "CERT/2026/000001");
        let a = verify_code(1);
        let b = verify_code(2);
        assert_eq!(a.len(), 10);
        assert_ne!(a, b);
        // Herufi zinazochanganya hazitumiwi.
        for ch in a.chars().chain(b.chars()) {
            assert!(!"O0I1".contains(ch), "herufi inayochanganya: {ch}");
        }
    }

    #[tokio::test]
    async fn cheti_huthibitishwa_kwa_serial_na_kodi() {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS ep_students (id TEXT PRIMARY KEY, full_name TEXT NOT NULL DEFAULT '', reg_no TEXT NOT NULL DEFAULT '')",
        )
        .execute(&p)
        .await;
        init_tables(&p).await;
        sqlx::query("INSERT INTO ep_students (id,full_name,reg_no) VALUES ('st1','Asha','R1')")
            .execute(&p)
            .await
            .unwrap();
        let code = verify_code(42);
        sqlx::query("INSERT INTO ep_certificates (id,student_id,kind,title,serial,verify_code,issued_at,revoked) VALUES ('c1','st1','completion','Cheti','CERT/2026/000001',?1,?2,0)")
            .bind(&code).bind(now()).execute(&p).await.unwrap();
        // Kodi sahihi → halali.
        let ok: Option<(i64,)> = sqlx::query_as(
            "SELECT revoked FROM ep_certificates WHERE serial='CERT/2026/000001' AND verify_code=?1",
        )
        .bind(&code)
        .fetch_optional(&p)
        .await
        .unwrap();
        assert_eq!(ok.map(|r| r.0), Some(0));
        // Kodi mbovu → haipatikani.
        let bad: Option<(i64,)> = sqlx::query_as(
            "SELECT revoked FROM ep_certificates WHERE serial='CERT/2026/000001' AND verify_code='XXXXXXXXXX'",
        )
        .fetch_optional(&p)
        .await
        .unwrap();
        assert!(bad.is_none());
        // Serial ni ya kipekee.
        let dup = sqlx::query("INSERT INTO ep_certificates (id,student_id,kind,title,serial,verify_code,issued_at,revoked) VALUES ('c2','st1','award','X','CERT/2026/000001','YYYYYYYYYY',?1,0)")
            .bind(now()).execute(&p).await;
        assert!(dup.is_err());
    }

    async fn db_with_marks() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        for d in [
            "CREATE TABLE IF NOT EXISTS ep_students (id TEXT PRIMARY KEY, full_name TEXT NOT NULL DEFAULT '', reg_no TEXT NOT NULL DEFAULT '', level_id TEXT NOT NULL DEFAULT '', gender TEXT NOT NULL DEFAULT '')",
            "CREATE TABLE IF NOT EXISTS ep_subjects (id TEXT PRIMARY KEY, level_id TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '', code TEXT NOT NULL DEFAULT '', credits INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS ep_marks_columns (id TEXT PRIMARY KEY, level_id TEXT NOT NULL DEFAULT '', subject_id TEXT NOT NULL DEFAULT '', kind TEXT NOT NULL DEFAULT 'other', label TEXT NOT NULL DEFAULT '', max_marks REAL NOT NULL DEFAULT 100, weight REAL NOT NULL DEFAULT 1, position INTEGER NOT NULL DEFAULT 0)",
            "CREATE TABLE IF NOT EXISTS ep_marks (id TEXT PRIMARY KEY, column_id TEXT NOT NULL, student_id TEXT NOT NULL, score REAL NOT NULL DEFAULT -1)",
        ] {
            let _ = sqlx::query(d).execute(&p).await;
        }
        sqlx::query("INSERT INTO ep_students (id,full_name,reg_no,level_id) VALUES ('st1','Asha','R1','lv1')")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_subjects (id,level_id,name,code,credits) VALUES ('su1','lv1','Fizikia','PHY',3)")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_subjects (id,level_id,name,code,credits) VALUES ('su2','lv1','Kemia','CHE',1)")
            .execute(&p).await.unwrap();
        p
    }

    #[tokio::test]
    async fn matokeo_hujumlisha_alama_kwa_uzito() {
        let p = db_with_marks().await;
        // Fizikia ina nguzo mbili: 80 (uzito 1) + 90 (uzito 3).
        sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight) VALUES ('c1','lv1','su1','t1','Zoezi',100,1)")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight) VALUES ('c2','lv1','su1','final','Mtihani',100,3)")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight) VALUES ('c3','lv1','su2','final','Mtihani',100,1)")
            .execute(&p).await.unwrap();
        for (id, col, sc) in [("m1", "c1", 80.0), ("m2", "c2", 90.0), ("m3", "c3", 65.0)] {
            sqlx::query("INSERT INTO ep_marks (id,column_id,student_id,score) VALUES (?1,?2,'st1',?3)")
                .bind(id).bind(col).bind(sc).execute(&p).await.unwrap();
        }
        let v = compute_student_results(&p, "st1").await.0;
        assert_eq!(v["ok"], json!(true));
        // Fizikia: (80*1+90*3)/4 = 87.5 → A; Kemia 65 → C.
        let phy = v["subjects"].as_array().unwrap().iter()
            .find(|s| s["code"] == json!("PHY")).cloned().unwrap();
        assert_eq!(phy["score"], json!(87.5));
        assert_eq!(phy["letter"], json!("A"));
        // Wastani = (87.5 + 65) / 2 = 76.25
        assert_eq!(v["average"], json!(76.25));
        assert_eq!(v["subjects_entered"], json!(2));
        // GPA = (A:5*3cr + C:3*1cr) / 4 = 4.5
        assert_eq!(v["gpa"], json!(4.5));
        assert_eq!(v["classification"], json!("First Class"));
    }

    #[tokio::test]
    async fn masomo_bila_alama_hayaingii_gpa() {
        let p = db_with_marks().await;
        sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight) VALUES ('c1','lv1','su1','final','Mtihani',100,1)")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_marks_columns (id,level_id,subject_id,kind,label,max_marks,weight) VALUES ('c2','lv1','su2','final','Mtihani',100,1)")
            .execute(&p).await.unwrap();
        // Alama ya Fizikia pekee; Kemia haina alama.
        sqlx::query("INSERT INTO ep_marks (id,column_id,student_id,score) VALUES ('m1','c1','st1',90)")
            .execute(&p).await.unwrap();
        let v = compute_student_results(&p, "st1").await.0;
        assert_eq!(v["subjects_entered"], json!(1), "somo bila alama liliingia");
        assert_eq!(v["gpa"], json!(5.0));
        let che = v["subjects"].as_array().unwrap().iter()
            .find(|s| s["code"] == json!("CHE")).cloned().unwrap();
        assert_eq!(che["score"], json!(null));
        assert_eq!(che["entered"], json!(false));
    }
}
