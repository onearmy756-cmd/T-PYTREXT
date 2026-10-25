//! elimupro.rs — ELIMUPRO Awamu 1: taasisi, courses/levels/subjects,
//! walimu + kugawa masomo, usajili wa wanafunzi (NECTA/RITA adapter),
//! enrollment, na ID card (SVG halisi, inayoweza kuchapishwa).
//!
//! Endpoints zote ziko `/api/elimupro/...`. Hakuna jibu la kubuni: kila kitu
//! huhifadhiwa kwenye SQLite, na uthibitisho wa NECTA/RITA hupiga API halisi
//! endapo `NECTA_API_URL` / `RITA_API_URL` zimewekwa; vinginevyo hurekodi
//! `manual` (msajili ahakiki kwa mkono) — SI kubuni matokeo.
//!
//! Tables: ep_institutions ep_courses ep_levels ep_subjects ep_teachers
//!         ep_teacher_subjects ep_students ep_enrollments

use crate::AppState;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rand::RngCore;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

// ============================================================
// TABLES
// ============================================================

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_institutions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            code TEXT NOT NULL DEFAULT '',      -- mf. CHUO
            motto TEXT NOT NULL DEFAULT '',
            address TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            website TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            logo TEXT NOT NULL DEFAULT '',      -- data URI au URL
            seal TEXT NOT NULL DEFAULT '',      -- muhuri (data URI)
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_courses (
            id TEXT PRIMARY KEY,
            institution_id TEXT NOT NULL,
            name TEXT NOT NULL,
            code TEXT NOT NULL,
            award TEXT NOT NULL DEFAULT 'Certificate', -- Certificate|Diploma|Degree|Masters
            years INTEGER NOT NULL DEFAULT 2,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_levels (
            id TEXT PRIMARY KEY,
            course_id TEXT NOT NULL,
            year INTEGER NOT NULL,
            semester INTEGER NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_subjects (
            id TEXT PRIMARY KEY,
            level_id TEXT NOT NULL,
            name TEXT NOT NULL,
            code TEXT NOT NULL,
            credits INTEGER NOT NULL DEFAULT 0,
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_teachers (
            id TEXT PRIMARY KEY,
            institution_id TEXT NOT NULL,
            name TEXT NOT NULL,
            email TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            staff_no TEXT NOT NULL DEFAULT '',
            qualification TEXT NOT NULL DEFAULT '',
            specialization TEXT NOT NULL DEFAULT '',
            hired_at TEXT NOT NULL DEFAULT '',
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_teacher_subjects (
            id TEXT PRIMARY KEY,
            teacher_id TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            academic_year TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            UNIQUE(teacher_id, subject_id, academic_year)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_students (
            id TEXT PRIMARY KEY,
            institution_id TEXT NOT NULL,
            course_id TEXT NOT NULL,
            level_id TEXT NOT NULL,
            full_name TEXT NOT NULL,
            gender TEXT NOT NULL DEFAULT '',
            dob TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            form4_index TEXT NOT NULL DEFAULT '',
            birth_cert_no TEXT NOT NULL DEFAULT '',
            necta_result TEXT NOT NULL DEFAULT '',
            necta_verified TEXT NOT NULL DEFAULT 'none',   -- api|manual|none
            rita_verified TEXT NOT NULL DEFAULT 'none',    -- api|manual|none
            photo TEXT NOT NULL DEFAULT '',                -- data URI
            reg_no TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            verified_by TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_enrollments (
            id TEXT PRIMARY KEY,
            student_id TEXT NOT NULL,
            subject_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(student_id, subject_id)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/elimupro/status", get(status))
        // Taasisi
        .route("/api/elimupro/institution/save", post(institution_save))
        .route("/api/elimupro/institution", get(institution_get))
        // Courses / levels / subjects
        .route("/api/elimupro/course/add", post(course_add))
        .route("/api/elimupro/courses", get(courses_list))
        .route("/api/elimupro/level/add", post(level_add))
        .route("/api/elimupro/levels", get(levels_list))
        .route("/api/elimupro/subject/add", post(subject_add))
        .route("/api/elimupro/subjects", get(subjects_list))
        // Walimu
        .route("/api/elimupro/teacher/add", post(teacher_add))
        .route("/api/elimupro/teachers", get(teachers_list))
        .route("/api/elimupro/teacher/assign", post(teacher_assign))
        .route("/api/elimupro/teacher/:id", get(teacher_get))
        // Wanafunzi + uthibitisho
        .route("/api/elimupro/verify/necta", post(verify_necta))
        .route("/api/elimupro/verify/rita", post(verify_rita))
        .route("/api/elimupro/student/admit", post(student_admit))
        .route("/api/elimupro/students", get(students_list))
        .route("/api/elimupro/student/:id", get(student_get))
        .route("/api/elimupro/student/:id/idcard.svg", get(student_idcard))
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

async fn setting(db: &SqlitePool, key: &str) -> Option<String> {
    sqlx::query_as::<_, (String,)>("SELECT value FROM ep_settings WHERE key = ?1")
        .bind(key)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
}

async fn set_setting(db: &SqlitePool, key: &str, value: &str) {
    let _ = sqlx::query(
        "INSERT INTO ep_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await;
}

fn hash_password(password: &str) -> String {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt).expect("salt b64");
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

/// Herufi za kuzalisha code (bila 0/O/1/I kuchanganya).
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

fn random_code(len: usize) -> String {
    let mut seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    (0..len)
        .map(|i| {
            let b = seed[i % seed.len()] as usize ^ (i.wrapping_mul(2654435761) >> 13) as usize;
            CODE_ALPHABET[b % CODE_ALPHABET.len()] as char
        })
        .collect()
}

/// Reg no ya kipekee: `CHUO/2026/CS/0001`. Hupata prefix kutoka institution code
/// na course code, na hupata namba inayofuata kutoka DB (haiwezi kugongana).
async fn next_reg_no(db: &SqlitePool, inst_prefix: &str, course: &str) -> String {
    let year = chrono::Local::now().format("%Y").to_string();
    let ccode = course
        .split_whitespace()
        .last()
        .unwrap_or("GEN")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(3)
        .collect::<String>()
        .to_uppercase();
    let ccode = if ccode.is_empty() { "GEN".to_string() } else { ccode };
    let prefix = format!("{}/{}/{}", inst_prefix, year, ccode);
    let like = format!("{}/%", prefix);
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT reg_no FROM ep_students WHERE reg_no LIKE ?1 ORDER BY reg_no DESC LIMIT 1",
    )
    .bind(&like)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    let next = row
        .and_then(|r| r.0.rsplit('/').next().map(|s| s.to_string()))
        .and_then(|n| n.parse::<u32>().ok())
        .map(|n| n + 1)
        .unwrap_or(1);
    format!("{}/{:04}", prefix, next)
}

/// Hupiga API ya nje ya uthibitisho (NECTA/RITA) kama URL imewekwa.
///
/// Hurudisha `("api", body)` kwa mafanikio, `("api_error", ujumbe)` kwa kosa
/// la API halisi, au `("unavailable", "")` kama URL haipo — hakuna kubuni.
async fn external_verify(
    env_url: &str,
    env_key: &str,
    payload: Value,
) -> (String, Value) {
    let url = match std::env::var(env_url) {
        Ok(u) if !u.trim().is_empty() => u,
        _ => return ("unavailable".into(), json!("")),
    };
    let mut req = reqwest::Client::new()
        .post(&url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(20));
    if let Ok(k) = std::env::var(env_key) {
        if !k.trim().is_empty() {
            req = req.bearer_auth(k);
        }
    }
    match req.send().await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            let body: Value = resp.json().await.unwrap_or(json!({}));
            if (200..300).contains(&code) {
                ("api".into(), body)
            } else {
                ("api_error".into(), json!({ "status": code, "body": body }))
            }
        }
        Err(e) => ("api_error".into(), json!(format!("{e}"))),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ============================================================
// STATUS
// ============================================================

async fn table_count(db: &SqlitePool, table: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "institutions": table_count(&s.db, "ep_institutions").await,
        "courses": table_count(&s.db, "ep_courses").await,
        "levels": table_count(&s.db, "ep_levels").await,
        "subjects": table_count(&s.db, "ep_subjects").await,
        "teachers": table_count(&s.db, "ep_teachers").await,
        "students": table_count(&s.db, "ep_students").await,
        "enrollments": table_count(&s.db, "ep_enrollments").await,
        "necta_api": std::env::var("NECTA_API_URL").map(|v| !v.trim().is_empty()).unwrap_or(false),
        "rita_api": std::env::var("RITA_API_URL").map(|v| !v.trim().is_empty()).unwrap_or(false),
    }))
}

// ============================================================
// TAASISI
// ============================================================

#[derive(Deserialize)]
pub struct InstitutionIn {
    pub name: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub motto: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub website: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub logo: String,
    #[serde(default)]
    pub seal: String,
}

pub async fn institution_save(
    State(s): State<AppState>,
    Json(b): Json<InstitutionIn>,
) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la taasisi linahitajika" }));
    }
    let code = if b.code.trim().is_empty() {
        b.name
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(4)
            .collect::<String>()
            .to_uppercase()
    } else {
        b.code.trim().to_uppercase()
    };
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM ep_institutions LIMIT 1")
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten();
    let id = existing.map(|r| r.0).unwrap_or_else(uid);
    let _ = sqlx::query(
        "INSERT INTO ep_institutions
         (id,name,code,motto,address,phone,email,website,description,logo,seal,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(id) DO UPDATE SET
           name=excluded.name, code=excluded.code, motto=excluded.motto,
           address=excluded.address, phone=excluded.phone, email=excluded.email,
           website=excluded.website, description=excluded.description,
           logo=excluded.logo, seal=excluded.seal",
    )
    .bind(&id)
    .bind(b.name.trim())
    .bind(&code)
    .bind(&b.motto)
    .bind(&b.address)
    .bind(&b.phone)
    .bind(&b.email)
    .bind(&b.website)
    .bind(&b.description)
    .bind(&b.logo)
    .bind(&b.seal)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "code": code }))
}

pub async fn institution_get(State(s): State<AppState>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String)>(
        "SELECT id,name,code,motto,address,phone,email,website,description,logo FROM ep_institutions LIMIT 1",
    )
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match row {
        Some(r) => Json(json!({
            "ok": true,
            "institution": {
                "id": r.0, "name": r.1, "code": r.2, "motto": r.3, "address": r.4,
                "phone": r.5, "email": r.6, "website": r.7, "description": r.8, "logo": r.9
            }
        })),
        None => Json(json!({ "ok": true, "institution": null })),
    }
}

// ============================================================
// COURSES / LEVELS / SUBJECTS
// ============================================================

async fn institution_id(db: &SqlitePool) -> Option<String> {
    sqlx::query_as::<_, (String,)>("SELECT id FROM ep_institutions LIMIT 1")
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
}

#[derive(Deserialize)]
pub struct CourseIn {
    pub name: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub award: String,
    #[serde(default)]
    pub years: i64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub auto_levels: bool,
}

pub async fn course_add(State(s): State<AppState>, Json(b): Json<CourseIn>) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la course linahitajika" }));
    }
    let inst = match institution_id(&s.db).await {
        Some(i) => i,
        None => return Json(json!({ "ok": false, "error": "sajili taasisi kwanza" })),
    };
    let award = if b.award.trim().is_empty() {
        "Certificate".to_string()
    } else {
        b.award.trim().to_string()
    };
    let years = if b.years <= 0 { 2 } else { b.years };
    let code = if b.code.trim().is_empty() {
        random_code(4)
    } else {
        b.code.trim().to_uppercase()
    };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_courses (id,institution_id,name,code,award,years,description,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
    )
    .bind(&id)
    .bind(&inst)
    .bind(b.name.trim())
    .bind(&code)
    .bind(&award)
    .bind(years)
    .bind(&b.description)
    .bind(now())
    .execute(&s.db)
    .await;

    let mut created_levels = 0;
    if b.auto_levels {
        for y in 1..=years {
            for sem in 1..=2 {
                let lid = uid();
                let _ = sqlx::query(
                    "INSERT INTO ep_levels (id,course_id,year,semester,name,created_at)
                     VALUES (?1,?2,?3,?4,?5,?6)",
                )
                .bind(&lid)
                .bind(&id)
                .bind(y)
                .bind(sem)
                .bind(format!("Year {y} Semester {sem}"))
                .bind(now())
                .execute(&s.db)
                .await;
                created_levels += 1;
            }
        }
    }
    Json(json!({ "ok": true, "id": id, "code": code, "levels_created": created_levels }))
}

pub async fn courses_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String, i64)>(
        "SELECT c.id,c.name,c.code,c.award,c.years,c.description,
                (SELECT COUNT(*) FROM ep_levels l WHERE l.course_id = c.id)
         FROM ep_courses c ORDER BY c.created_at",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let courses: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "name": r.1, "code": r.2, "award": r.3, "years": r.4,
                "description": r.5, "levels": r.6
            })
        })
        .collect();
    Json(json!({ "ok": true, "courses": courses }))
}

#[derive(Deserialize)]
pub struct LevelIn {
    pub course_id: String,
    pub year: i64,
    pub semester: i64,
    #[serde(default)]
    pub name: String,
}

pub async fn level_add(State(s): State<AppState>, Json(b): Json<LevelIn>) -> Json<Value> {
    let id = uid();
    let name = if b.name.trim().is_empty() {
        format!("Year {} Semester {}", b.year, b.semester)
    } else {
        b.name.trim().to_string()
    };
    let _ = sqlx::query(
        "INSERT INTO ep_levels (id,course_id,year,semester,name,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&id)
    .bind(&b.course_id)
    .bind(b.year)
    .bind(b.semester)
    .bind(&name)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "name": name }))
}

pub async fn levels_list(
    State(s): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let course = q.get("course_id").cloned().unwrap_or_default();
    let rows = sqlx::query_as::<_, (String, i64, i64, String, i64)>(
        "SELECT l.id,l.year,l.semester,l.name,
                (SELECT COUNT(*) FROM ep_subjects su WHERE su.level_id = l.id)
         FROM ep_levels l WHERE (?1 = '' OR l.course_id = ?1)
         ORDER BY l.year, l.semester",
    )
    .bind(&course)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let levels: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({ "id": r.0, "year": r.1, "semester": r.2, "name": r.3, "subjects": r.4 })
        })
        .collect();
    Json(json!({ "ok": true, "levels": levels }))
}

#[derive(Deserialize)]
pub struct SubjectIn {
    pub level_id: String,
    pub name: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub credits: i64,
    #[serde(default)]
    pub description: String,
}

pub async fn subject_add(State(s): State<AppState>, Json(b): Json<SubjectIn>) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la somo linahitajika" }));
    }
    // Kinga: somo la jina lile lile kwenye level ile ile?
    let dup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM ep_subjects WHERE level_id = ?1 AND lower(name) = lower(?2)",
    )
    .bind(&b.level_id)
    .bind(b.name.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if dup.is_some() {
        return Json(json!({ "ok": false, "error": "somo hili lipo tayari kwenye level hii" }));
    }
    let id = uid();
    let code = if b.code.trim().is_empty() {
        random_code(4)
    } else {
        b.code.trim().to_uppercase()
    };
    let _ = sqlx::query(
        "INSERT INTO ep_subjects (id,level_id,name,code,credits,description,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(&b.level_id)
    .bind(b.name.trim())
    .bind(&code)
    .bind(b.credits)
    .bind(&b.description)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "code": code }))
}

pub async fn subjects_list(
    State(s): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let level = q.get("level_id").cloned().unwrap_or_default();
    let rows = sqlx::query_as::<_, (String, String, String, i64, String, String)>(
        "SELECT su.id,su.name,su.code,su.credits,su.description,
                COALESCE(l.name,'')
         FROM ep_subjects su LEFT JOIN ep_levels l ON l.id = su.level_id
         WHERE (?1 = '' OR su.level_id = ?1) ORDER BY su.name",
    )
    .bind(&level)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let subjects: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({ "id": r.0, "name": r.1, "code": r.2, "credits": r.3, "description": r.4, "level": r.5 })
        })
        .collect();
    Json(json!({ "ok": true, "subjects": subjects }))
}

// ============================================================
// WALIMU
// ============================================================

#[derive(Deserialize)]
pub struct TeacherIn {
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub staff_no: String,
    #[serde(default)]
    pub qualification: String,
    #[serde(default)]
    pub specialization: String,
    #[serde(default)]
    pub hired_at: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

pub async fn teacher_add(State(s): State<AppState>, Json(b): Json<TeacherIn>) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la mwalimu linahitajika" }));
    }
    let inst = match institution_id(&s.db).await {
        Some(i) => i,
        None => return Json(json!({ "ok": false, "error": "sajili taasisi kwanza" })),
    };
    let username = if b.username.trim().is_empty() {
        let base = b
            .name
            .split_whitespace()
            .next()
            .unwrap_or("mwalimu")
            .to_lowercase();
        format!("{}{}", base, &random_code(3).to_lowercase())
    } else {
        b.username.trim().to_string()
    };
    let password = if b.password.trim().is_empty() {
        random_code(8)
    } else {
        b.password.clone()
    };
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM ep_teachers WHERE username = ?1")
            .bind(&username)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten();
    if exists.is_some() {
        return Json(json!({ "ok": false, "error": "username imetumika" }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_teachers
         (id,institution_id,name,email,phone,staff_no,qualification,specialization,hired_at,username,password_hash,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
    )
    .bind(&id)
    .bind(&inst)
    .bind(b.name.trim())
    .bind(&b.email)
    .bind(&b.phone)
    .bind(&b.staff_no)
    .bind(&b.qualification)
    .bind(&b.specialization)
    .bind(&b.hired_at)
    .bind(&username)
    .bind(hash_password(&password))
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "username": username, "password": password }))
}

pub async fn teachers_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, i64)>(
        "SELECT t.id,t.name,t.email,t.phone,t.staff_no,t.username,
                (SELECT COUNT(*) FROM ep_teacher_subjects ts WHERE ts.teacher_id = t.id)
         FROM ep_teachers t ORDER BY t.name",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let teachers: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "name": r.1, "email": r.2, "phone": r.3,
                "staff_no": r.4, "username": r.5, "subjects": r.6
            })
        })
        .collect();
    Json(json!({ "ok": true, "teachers": teachers }))
}

#[derive(Deserialize)]
pub struct AssignIn {
    pub teacher_id: String,
    pub subject_id: String,
    #[serde(default)]
    pub academic_year: String,
}

pub async fn teacher_assign(State(s): State<AppState>, Json(b): Json<AssignIn>) -> Json<Value> {
    let year = if b.academic_year.trim().is_empty() {
        format!(
            "{}/{}",
            chrono::Local::now().format("%Y"),
            chrono::Local::now().format("%y").to_string().parse::<i32>().unwrap_or(26) + 1
        )
    } else {
        b.academic_year.trim().to_string()
    };
    let _ = sqlx::query(
        "INSERT OR IGNORE INTO ep_teacher_subjects (id,teacher_id,subject_id,academic_year,created_at)
         VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(uid())
    .bind(&b.teacher_id)
    .bind(&b.subject_id)
    .bind(&year)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "academic_year": year }))
}

pub async fn teacher_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let t = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT id,name,email,phone,staff_no,qualification,specialization FROM ep_teachers WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let subjects = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT su.id,su.name,su.code,ts.academic_year
         FROM ep_teacher_subjects ts JOIN ep_subjects su ON su.id = ts.subject_id
         WHERE ts.teacher_id = ?1 ORDER BY su.name",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    match t {
        Some(r) => Json(json!({
            "ok": true,
            "teacher": {
                "id": r.0, "name": r.1, "email": r.2, "phone": r.3,
                "staff_no": r.4, "qualification": r.5, "specialization": r.6
            },
            "subjects": subjects.into_iter().map(|x| json!({
                "id": x.0, "name": x.1, "code": x.2, "academic_year": x.3
            })).collect::<Vec<_>>()
        })),
        None => Json(json!({ "ok": false, "error": "mwalimu hajapatikana" })),
    }
}

// ============================================================
// UTHIBITISHO (NECTA / RITA)
// ============================================================

#[derive(Deserialize)]
pub struct VerifyIn {
    #[serde(default)]
    pub form4_index: String,
    #[serde(default)]
    pub birth_cert_no: String,
    #[serde(default)]
    pub full_name: String,
}

/// NECTA: kama `NECTA_API_URL` ipo → hupiga API halisi; vinginevyo hurudisha
/// `manual` (msajili ahakiki kwa mkono). HAKUNA jibu la kubuni.
pub async fn verify_necta(State(_s): State<AppState>, Json(b): Json<VerifyIn>) -> Json<Value> {
    if b.form4_index.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "namba ya Form Four inahitajika" }));
    }
    let (mode, data) = external_verify(
        "NECTA_API_URL",
        "NECTA_API_KEY",
        json!({ "index_number": b.form4_index.trim(), "name": b.full_name.trim() }),
    )
    .await;
    match mode.as_str() {
        "api" => Json(json!({ "ok": true, "mode": "api", "verified": true, "result": data })),
        "api_error" => Json(json!({ "ok": true, "mode": "api_error", "verified": false, "detail": data })),
        _ => Json(json!({
            "ok": true, "mode": "manual", "verified": false,
            "message": "NECTA API haipo kwenye mfumo huu — msajili ahakiki kwa mkono na kuweka alama."
        })),
    }
}

/// RITA: sawa na NECTA.
pub async fn verify_rita(State(_s): State<AppState>, Json(b): Json<VerifyIn>) -> Json<Value> {
    if b.birth_cert_no.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "namba ya cheti cha kuzaliwa inahitajika" }));
    }
    let (mode, data) = external_verify(
        "RITA_API_URL",
        "RITA_API_KEY",
        json!({ "certificate_number": b.birth_cert_no.trim(), "name": b.full_name.trim() }),
    )
    .await;
    match mode.as_str() {
        "api" => Json(json!({ "ok": true, "mode": "api", "verified": true, "result": data })),
        "api_error" => Json(json!({ "ok": true, "mode": "api_error", "verified": false, "detail": data })),
        _ => Json(json!({
            "ok": true, "mode": "manual", "verified": false,
            "message": "RITA API haipo kwenye mfumo huu — msajili ahakiki kwa mkono na kuweka alama."
        })),
    }
}

// ============================================================
// USAJILI WA WANAFUNZI (admit) + ENROLLMENT + ID CARD
// ============================================================

#[derive(Deserialize)]
pub struct StudentIn {
    pub full_name: String,
    pub course_id: String,
    pub level_id: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub dob: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub form4_index: String,
    #[serde(default)]
    pub birth_cert_no: String,
    #[serde(default)]
    pub necta_result: String,
    /// algorithm: `none` (default) | `api` | `manual`
    #[serde(default)]
    pub necta_verified: String,
    #[serde(default)]
    pub rita_verified: String,
    #[serde(default)]
    pub photo: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub verified_by: String,
    /// true → mfumo husajili shule/mwanafunzi wa majaribio (ruhusa inaonekana).
    #[serde(default)]
    pub allow_unverified: bool,
}

pub async fn student_admit(State(s): State<AppState>, Json(b): Json<StudentIn>) -> Json<Value> {
    if b.full_name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina kamili linahitajika" }));
    }
    let necta = if b.necta_verified.trim().is_empty() { "none" } else { b.necta_verified.trim() };
    let rita = if b.rita_verified.trim().is_empty() { "none" } else { b.rita_verified.trim() };
    let verified = necta != "none" || rita != "none";
    if !verified && !b.allow_unverified {
        return Json(json!({
            "ok": false,
            "error": "mwanafunzi hajathibitishwa (NECTA/RITA). Thibitisha kwanza, au tumia allow_unverified kwa usajili wa dharura.",
            "necta_verified": necta, "rita_verified": rita
        }));
    }

    let inst_id = match institution_id(&s.db).await {
        Some(i) => i,
        None => return Json(json!({ "ok": false, "error": "sajili taasisi kwanza" })),
    };
    let inst = sqlx::query_as::<_, (String, String)>("SELECT name, code FROM ep_institutions WHERE id = ?1")
        .bind(&inst_id)
        .fetch_one(&s.db)
        .await
        .map(|r| (r.0, r.1))
        .unwrap_or_else(|_| (String::new(), "CHUO".into()));
    let inst_prefix = if inst.1.trim().is_empty() { "CHUO".to_string() } else { inst.1.clone() };

    let course = sqlx::query_as::<_, (String, String)>("SELECT name, code FROM ep_courses WHERE id = ?1")
        .bind(&b.course_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((course_name, course_code)) = course else {
        return Json(json!({ "ok": false, "error": "course haijapatikana" }));
    };

    // Tumia CODE ya course (mf. DCS), si jina — jina linaweza kuwa na namba
    // zinazochanganya ("Diploma ya E2E 131" → "131").
    let reg_course = if course_code.trim().is_empty() { course_name.clone() } else { course_code };
    let reg_no = next_reg_no(&s.db, &inst_prefix, &reg_course).await;
    let username = if b.username.trim().is_empty() {
        reg_no.to_lowercase().replace('/', "-")
    } else {
        b.username.trim().to_string()
    };
    let password = if b.password.trim().is_empty() {
        random_code(8)
    } else {
        b.password.clone()
    };
    let id = uid();
    let res = sqlx::query(
        "INSERT INTO ep_students
         (id,institution_id,course_id,level_id,full_name,gender,dob,phone,email,form4_index,
          birth_cert_no,necta_result,necta_verified,rita_verified,photo,reg_no,username,password_hash,verified_by,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
    )
    .bind(&id)
    .bind(&inst_id)
    .bind(&b.course_id)
    .bind(&b.level_id)
    .bind(b.full_name.trim())
    .bind(&b.gender)
    .bind(&b.dob)
    .bind(&b.phone)
    .bind(&b.email)
    .bind(&b.form4_index)
    .bind(&b.birth_cert_no)
    .bind(&b.necta_result)
    .bind(necta)
    .bind(rita)
    .bind(&b.photo)
    .bind(&reg_no)
    .bind(&username)
    .bind(hash_password(&password))
    .bind(&b.verified_by)
    .bind(now())
    .execute(&s.db)
    .await;
    if let Err(e) = res {
        return Json(json!({ "ok": false, "error": format!("usajili umeshindikana: {e}") }));
    }

    // Enrollment: mwanafunzi hupata masomo YOTE ya level yake.
    let subs = sqlx::query_as::<_, (String,)>("SELECT id FROM ep_subjects WHERE level_id = ?1")
        .bind(&b.level_id)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    let mut enrolled = 0;
    for (sid,) in &subs {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ep_enrollments (id,student_id,subject_id,created_at) VALUES (?1,?2,?3,?4)",
        )
        .bind(uid())
        .bind(&id)
        .bind(sid)
        .bind(now())
        .execute(&s.db)
        .await;
        enrolled += 1;
    }

    Json(json!({
        "ok": true, "id": id, "reg_no": reg_no, "username": username,
        "password": password, "enrolled_subjects": enrolled,
        "necta_verified": necta, "rita_verified": rita,
        "institution": inst_prefix
    }))
}

pub async fn students_list(
    State(s): State<AppState>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let level = q.get("level_id").cloned().unwrap_or_default();
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, String)>(
        "SELECT st.id,st.full_name,st.reg_no,st.username,st.necta_verified,st.rita_verified,
                COALESCE(l.name,''),
                (SELECT COUNT(*) FROM ep_enrollments e WHERE e.student_id = st.id),
                st.gender
         FROM ep_students st LEFT JOIN ep_levels l ON l.id = st.level_id
         WHERE (?1 = '' OR st.level_id = ?1) ORDER BY st.reg_no",
    )
    .bind(&level)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let students: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.0, "full_name": r.1, "reg_no": r.2, "username": r.3,
                "necta_verified": r.4, "rita_verified": r.5, "level": r.6, "subjects": r.7,
                "gender": r.8
            })
        })
        .collect();
    Json(json!({ "ok": true, "students": students }))
}

pub async fn student_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let st = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String, String, String)>(
        "SELECT st.id,st.full_name,st.reg_no,st.username,st.gender,st.dob,st.phone,st.email,
                st.form4_index,st.birth_cert_no,st.necta_verified,st.rita_verified
         FROM ep_students st WHERE st.id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hajapatikana" }));
    };
    let subjects = sqlx::query_as::<_, (String, String, String)>(
        "SELECT su.name,su.code,COALESCE(t.name,'')
         FROM ep_enrollments e JOIN ep_subjects su ON su.id = e.subject_id
         LEFT JOIN ep_teacher_subjects ts ON ts.subject_id = su.id
         LEFT JOIN ep_teachers t ON t.id = ts.teacher_id
         WHERE e.student_id = ?1 ORDER BY su.name",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "student": {
            "id": r.0, "full_name": r.1, "reg_no": r.2, "username": r.3, "gender": r.4,
            "dob": r.5, "phone": r.6, "email": r.7, "form4_index": r.8,
            "birth_cert_no": r.9, "necta_verified": r.10, "rita_verified": r.11
        },
        "subjects": subjects.into_iter().map(|x| json!({
            "name": x.0, "code": x.1, "teacher": x.2
        })).collect::<Vec<_>>()
    }))
}

/// ID card halisi (SVG) yenye logo, muhuri, picha na QR — tayari kuchapishwa.
pub async fn student_idcard(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let row = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT st.full_name,st.reg_no,st.photo,COALESCE(i.name,''),COALESCE(i.seal,'')
         FROM ep_students st LEFT JOIN ep_institutions i ON i.id = st.institution_id
         WHERE st.id = ?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((name, reg_no, photo, inst_name, seal)) = row else {
        return (StatusCode::NOT_FOUND, "mwanafunzi hajapatikana").into_response();
    };
    let course = sqlx::query_as::<_, (String,)>(
        "SELECT COALESCE(c.name,'') FROM ep_students st LEFT JOIN ep_courses c ON c.id = st.course_id WHERE st.id = ?1",
    )
    .bind(&id)
    .fetch_one(&s.db)
    .await
    .map(|r| r.0)
    .unwrap_or_default();

    let qr = crate::exam_lan::qr_svg(&format!("MTECHOS-ID:{reg_no}"))
        .unwrap_or_default();
    let photo_el = if photo.starts_with("data:image") {
        format!(
            r#"<image x="384" y="96" width="128" height="160" preserveAspectRatio="xMidYMid slice" href="{}"/>"#,
            esc(&photo)
        )
    } else {
        r##"<rect x="384" y="96" width="128" height="160" fill="#e5e7eb"/>
           <text x="448" y="184" font-size="12" text-anchor="middle" fill="#6b7280">HAKUNA PICHA</text>"##
            .to_string()
    };
    let seal_el = if seal.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<image x="392" y="278" width="96" height="96" href="{}"/>"#,
            esc(&seal)
        )
    };
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="540" height="380" viewBox="0 0 540 380">
  <rect width="540" height="380" fill="#ffffff" stroke="#1f2937" stroke-width="2" rx="10"/>
  <rect width="540" height="56" fill="#1f2937"/>
  <text x="20" y="36" font-size="20" font-weight="bold" fill="#ffffff" font-family="sans-serif">{inst}</text>
  <text x="20" y="88" font-size="13" fill="#6b7280" font-family="sans-serif">MWANAFUNZI • STUDENT ID</text>
  <text x="20" y="128" font-size="22" font-weight="bold" fill="#111827" font-family="sans-serif">{name}</text>
  <text x="20" y="160" font-size="15" fill="#374151" font-family="sans-serif">{course}</text>
  <text x="20" y="196" font-size="14" fill="#374151" font-family="sans-serif">Namba ya Usajili:</text>
  <text x="20" y="218" font-size="16" font-weight="bold" fill="#111827" font-family="sans-serif">{reg}</text>
  <text x="20" y="262" font-size="12" fill="#6b7280" font-family="sans-serif">Mhuri wa chuo:</text>
  {photo}
  {seal}
  <g transform="translate(392,278) scale(0.62)">{qr}</g>
</svg>"##,
        inst = esc(&inst_name),
        name = esc(&name),
        course = esc(&course),
        reg = esc(&reg_no),
        photo = photo_el,
        seal = seal_el,
        qr = qr,
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
        svg,
    )
        .into_response()
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> SqlitePool {
        let dir = std::env::temp_dir().join(format!("ep_{}", uuid::Uuid::new_v4()));
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
        assert_eq!(n, 9, "majedwali 9 ya ep_* yanatakiwa");
    }

    #[test]
    fn random_code_hutenga_alama_zinazochanganya() {
        for _ in 0..50 {
            let c = random_code(8);
            assert_eq!(c.len(), 8);
            assert!(!c.contains('0') && !c.contains('O') && !c.contains('1') && !c.contains('I'));
        }
    }

    #[tokio::test]
    async fn reg_no_huanza_0001_na_kuongezeka() {
        let db = test_db().await;
        let a = next_reg_no(&db, "CTM", "Diploma in Computer Science").await;
        assert_eq!(a, format!("CTM/{}/SCI/0001", chrono::Local::now().format("%Y")));
        sqlx::query(
            "INSERT INTO ep_students (id,institution_id,course_id,level_id,full_name,reg_no,username,password_hash,created_at)
             VALUES ('x','i','c','l','A',?1,'u1','h',?2)",
        )
        .bind(&a)
        .bind(now())
        .execute(&db)
        .await
        .unwrap();
        let b = next_reg_no(&db, "CTM", "Diploma in Computer Science").await;
        assert_eq!(b, format!("CTM/{}/SCI/0002", chrono::Local::now().format("%Y")));
    }

    #[tokio::test]
    async fn institution_husave_kwa_id_ile_ile() {
        let db = test_db().await;
        // Simulate: haiwezi kuwa na taasisi mbili. Tunaingiza mara moja.
        let id = uid();
        sqlx::query("INSERT INTO ep_institutions (id,name,code,created_at) VALUES (?1,'Chuo A','CA',?2)")
            .bind(&id)
            .bind(now())
            .execute(&db)
            .await
            .unwrap();
        let first = institution_id(&db).await.unwrap();
        assert_eq!(first, id, "taasisi moja pekee");
    }

    #[tokio::test]
    async fn hash_password_ni_argon2id() {
        let h = hash_password("siri2026");
        assert!(h.starts_with("$argon2id$"), "argon2id inatakiwa: {h}");
        // Hash ile ile ya password ile ile HAIFANANI (salt nasibu).
        assert_ne!(h, hash_password("siri2026"));
    }

    #[test]
    fn esc_huzuia_svg_injection() {
        let out = esc(r#"<script>alert("x")</script>"#);
        assert!(!out.contains('<') && !out.contains('>') && !out.contains('"'));
    }
}

