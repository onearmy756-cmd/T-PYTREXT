//! ajira.rs — AJIRA: kutangaza nafasi, maombi, na tathmini mbili.
//!
//! Mahitaji ya mmiliki: "uweke sehemu ya kutangaza ajira mbalimbali kwenye
//! kampuni na sehemu ambapo wanaoomba ajira wajaze taarifa zao ili kutuma
//! maombi na kufanyiwa assessment practical na theoretically".
//!
//! Kanuni halisi (hakuna kubuni):
//!   - **Nadharia (theory)**: maswali ya chaguo-nyingi kutoka `aj_question_bank`;
//!     hupimwa KWA CODE (kulinganisha jibu) — si mtu anayepanga alama.
//!   - **Vitendo (practical)**: kazi yenye vigezo (rubric); mwombaji huwasilisha
//!     jibu/kazi, msimamizi hupima kwa vigezo halisi (`aj_practical_criteria`).
//!   - **Kuajiri**: admin huamua; akikubali, mfanyakazi HUUNDWA kwenye
//!     `bz_employees` (biashara) akiwa na idara na zana alizopewa.
//!   - Maombi yenye alama chini ya kiwango cha kufaulu (`pass_mark`) hayapendekezwi.
//!
//! Tables: aj_roles aj_applications aj_question_bank aj_assessments
//!         aj_practical_criteria

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
        r#"CREATE TABLE IF NOT EXISTS aj_roles (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL DEFAULT '',
            department_id TEXT NOT NULL DEFAULT '',
            service_code TEXT NOT NULL DEFAULT '',
            level TEXT NOT NULL DEFAULT 'mid',
            location TEXT NOT NULL DEFAULT 'Dar es Salaam',
            kind TEXT NOT NULL DEFAULT 'fulltime', -- fulltime|contract|intern
            description TEXT NOT NULL DEFAULT '',
            requirements TEXT NOT NULL DEFAULT '',
            salary_min_cents INTEGER NOT NULL DEFAULT 0,
            salary_max_cents INTEGER NOT NULL DEFAULT 0,
            currency TEXT NOT NULL DEFAULT 'TZS',
            status TEXT NOT NULL DEFAULT 'open', -- open|closed
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS aj_applications (
            id TEXT PRIMARY KEY,
            ref TEXT NOT NULL UNIQUE,
            role_id TEXT NOT NULL,
            full_name TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            photo TEXT NOT NULL DEFAULT '',
            cv_url TEXT NOT NULL DEFAULT '',
            education TEXT NOT NULL DEFAULT '',
            experience_years INTEGER NOT NULL DEFAULT 0,
            skills TEXT NOT NULL DEFAULT '[]',
            country TEXT NOT NULL DEFAULT 'TZ',
            region TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'submitted',
            score_theory REAL NOT NULL DEFAULT 0,
            score_practical REAL NOT NULL DEFAULT 0,
            total_score REAL NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS aj_question_bank (
            id TEXT PRIMARY KEY,
            family TEXT NOT NULL DEFAULT 'software',
            question TEXT NOT NULL,
            options TEXT NOT NULL DEFAULT '[]',
            answer TEXT NOT NULL DEFAULT '',
            marks INTEGER NOT NULL DEFAULT 1,
            active INTEGER NOT NULL DEFAULT 1
        )"#,
        r#"CREATE TABLE IF NOT EXISTS aj_assessments (
            id TEXT PRIMARY KEY,
            application_id TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT 'theory', -- theory|practical
            task TEXT NOT NULL DEFAULT '',
            questions TEXT NOT NULL DEFAULT '[]',
            answers TEXT NOT NULL DEFAULT '[]',
            max_score REAL NOT NULL DEFAULT 0,
            score REAL NOT NULL DEFAULT 0,
            graded_by TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|submitted|graded
            created_at TEXT NOT NULL,
            submitted_at TEXT,
            graded_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS aj_practical_criteria (
            id TEXT PRIMARY KEY,
            assessment_id TEXT NOT NULL,
            label TEXT NOT NULL DEFAULT '',
            weight REAL NOT NULL DEFAULT 1.0,
            awarded REAL NOT NULL DEFAULT 0,
            note TEXT NOT NULL DEFAULT ''
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
    seed(db).await;
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Kiwango cha kufaulu (asilimia).
pub const PASS_MARK: f64 = 60.0;

/// Hatua za ombi la ajira.
pub const STAGES: [&str; 8] = [
    "submitted",
    "screening",
    "theory",
    "practical",
    "interview",
    "offered",
    "hired",
    "rejected",
];

/// Mabadiliko halali ya hatua.
pub fn stage_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("submitted", "screening")
            | ("submitted", "rejected")
            | ("screening", "theory")
            | ("screening", "rejected")
            | ("theory", "practical")
            | ("theory", "rejected")
            | ("practical", "interview")
            | ("practical", "rejected")
            | ("interview", "offered")
            | ("interview", "rejected")
            | ("offered", "hired")
            | ("offered", "rejected")
    )
}

/// Kumbukumbu ya ombi la ajira (`APP-2026-000123`).
pub fn application_ref(year: i32, seq: i64) -> String {
    format!("APP-{year}-{seq:06}")
}

/// Mwombaji amefaulu? (wote nadharia na vitendo).
pub fn passed(theory: f64, practical: f64, pass: f64) -> bool {
    theory >= pass && practical >= pass
}

/// Alama ya jumla = wastani wa nadharia na vitendo (50/50).
pub fn total_score(theory: f64, practical: f64) -> f64 {
    ((theory + practical) / 2.0 * 100.0).round() / 100.0
}

/// Panga jibu la chaguo-nyingi (hulinganisha kwa uangalifu wa herufi/nafasi).
pub fn grade_mcq(expected: &str, given: &str) -> bool {
    let norm = |s: &str| s.trim().to_lowercase().replace([' ', '.', ')'], "");
    !expected.trim().is_empty() && norm(expected) == norm(given)
}

/// Alama za nadharia kutoka kwa majibu (asilimia).
pub fn theory_percent(questions: &Value, answers: &Value) -> f64 {
    let qs = questions.as_array().cloned().unwrap_or_default();
    let ans = answers.as_array().cloned().unwrap_or_default();
    if qs.is_empty() {
        return 0.0;
    }
    let mut total_marks = 0.0f64;
    let mut got = 0.0f64;
    for (i, q) in qs.iter().enumerate() {
        let marks = q["marks"].as_f64().unwrap_or(1.0);
        total_marks += marks;
        let expected = q["answer"].as_str().unwrap_or("");
        let given = ans
            .get(i)
            .and_then(|v| v.as_str())
            .or_else(|| ans.get(i).and_then(|v| v["answer"].as_str()))
            .unwrap_or("");
        if grade_mcq(expected, given) {
            got += marks;
        }
    }
    if total_marks <= 0.0 {
        return 0.0;
    }
    ((got / total_marks) * 100.0 * 100.0).round() / 100.0
}

/// Alama za vitendo kutoka vigezo (rubric) — asilimia.
pub fn practical_percent(criteria: &[(f64, f64)]) -> f64 {
    let total: f64 = criteria.iter().map(|(w, _)| *w).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let got: f64 = criteria.iter().map(|(_, a)| *a).sum();
    ((got / total) * 100.0 * 100.0).round() / 100.0
}

/// Familia za maswali (kwa benki ya maswali).
pub const FAMILIES: [&str; 6] = ["software", "security", "ai", "creative", "data", "general"];

pub fn norm_family(f: &str) -> &'static str {
    match f.trim().to_lowercase().as_str() {
        "software" | "programu" | "dev" => "software",
        "security" | "usalama" | "cyber" => "security",
        "ai" | "ml" => "ai",
        "creative" | "ubunifu" | "design" => "creative",
        "data" | "database" => "data",
        _ => "general",
    }
}

async fn seed(db: &SqlitePool) {
    let bank: [(&str, &str, [&str; 4], &str, i64); 12] = [
        ("software", "HTTP status gani humaanisha \"Not Found\"?", ["200", "404", "500", "302"], "404", 1),
        ("software", "Katika Git, amri gani huunda tawi jipya na kuliingia?", ["git branch x", "git checkout -b x", "git merge x", "git clone x"], "git checkout -b x", 1),
        ("software", "Big-O ya utafutaji wa binary kwenye safu iliyopangwa?", ["O(n)", "O(log n)", "O(n log n)", "O(1)"], "O(log n)", 1),
        ("software", "Ni lugha gani hutumia \"cargo\" kama msimamizi wa pakiti?", ["Python", "Rust", "Go", "PHP"], "Rust", 1),
        ("security", "CVE humaanisha nini?", ["Common Vulnerabilities and Exposures", "Central Virus Engine", "Cyber Verification Entry", "Critical Value Estimate"], "Common Vulnerabilities and Exposures", 1),
        ("security", "Ni ipi njia BORA ya kuhifadhi nenosiri?", ["MD5", "SHA-1", "Argon2id/bcrypt", "Base64"], "Argon2id/bcrypt", 1),
        ("security", "TLS hulinda nini hasa?", ["Ukubwa wa faili", "Usiri na uadilifu wa mawasiliano", "Kasi ya mtandao", "Rangi ya UI"], "Usiri na uadilifu wa mawasiliano", 1),
        ("ai", "Fine-tuning humaanisha nini?", ["Kufuta model", "Kurekebisha model kwa data maalum", "Kubadilisha GPU", "Kuongeza RAM"], "Kurekebisha model kwa data maalum", 1),
        ("ai", "RAG ni kifupi cha nini?", ["Random Access Graph", "Retrieval-Augmented Generation", "Rapid AI Gateway", "Recursive Agent Graph"], "Retrieval-Augmented Generation", 1),
        ("data", "SQL: amri gani HAIREKEBISHI data?", ["UPDATE", "INSERT", "SELECT", "DELETE"], "SELECT", 1),
        ("creative", "Muundo wa rangi unaohitajika kwa ufikivu (accessibility) ni kiwango gani?", ["1:1", "4.5:1 kwa maandishi", "10:1 kila mahali", "Haijalishi"], "4.5:1 kwa maandishi", 1),
        ("general", "Escrow humaanisha nini?", ["Pesa inayohifadhiwa hadi kazi ikamilike", "Pesa iliyolipwa moja kwa moja", "Deni la kampuni", "Kodi ya serikali"], "Pesa inayohifadhiwa hadi kazi ikamilike", 1),
    ];
    let (existing,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM aj_question_bank")
        .fetch_one(db)
        .await
        .unwrap_or((0,));
    if existing > 0 {
        return;
    }
    for (family, q, opts, ans, marks) in bank {
        let _ = sqlx::query(
            "INSERT INTO aj_question_bank (id,family,question,options,answer,marks,active)
             VALUES (?1,?2,?3,?4,?5,?6,1)",
        )
        .bind(uid())
        .bind(family)
        .bind(q)
        .bind(serde_json::to_string(&opts).unwrap_or_else(|_| "[]".into()))
        .bind(ans)
        .bind(marks)
        .execute(db)
        .await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/ajira/status", get(status))
        .route("/api/ajira/roles", get(roles_list))
        .route("/api/ajira/role/add", post(role_add))
        .route("/api/ajira/role/:id/close", post(role_close))
        .route("/api/ajira/applications", get(applications_list))
        .route("/api/ajira/apply", post(apply))
        .route("/api/ajira/application/:id", get(application_get))
        .route("/api/ajira/application/:id/stage", post(application_stage))
        .route("/api/ajira/application/:id/theory/start", post(theory_start))
        .route("/api/ajira/application/:id/theory/submit", post(theory_submit))
        .route("/api/ajira/application/:id/practical/assign", post(practical_assign))
        .route("/api/ajira/application/:id/practical/submit", post(practical_submit))
        .route("/api/ajira/application/:id/practical/grade", post(practical_grade))
        .route("/api/ajira/application/:id/hire", post(hire))
        .route("/api/ajira/application/:id/reject", post(reject_h))
        .route("/api/ajira/questions", get(questions_list))
        .route("/api/ajira/question/add", post(question_add))
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let (open_roles,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM aj_roles WHERE status='open'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "open_roles": open_roles,
        "applications": table_count(&s.db, "aj_applications").await,
        "assessments": table_count(&s.db, "aj_assessments").await,
        "questions": table_count(&s.db, "aj_question_bank").await,
        "pass_mark": PASS_MARK,
        "stages": STAGES,
        "families": FAMILIES,
    }))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

async fn roles_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = if let Some(st) = q.status.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String, i64, i64, String, String)>(
            "SELECT id,code,title,department_id,service_code,level,location,kind,salary_min_cents,salary_max_cents,currency,status
             FROM aj_roles WHERE status=?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(st).bind(lim).fetch_all(&s.db).await
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String, i64, i64, String, String)>(
            "SELECT id,code,title,department_id,service_code,level,location,kind,salary_min_cents,salary_max_cents,currency,status
             FROM aj_roles ORDER BY created_at DESC LIMIT ?1",
        )
        .bind(lim).fetch_all(&s.db).await
    }
    .unwrap_or_default();
    Json(json!({"ok": true, "roles": rows.into_iter().map(|r| json!({
        "id": r.0, "code": r.1, "title": r.2, "department_id": r.3,
        "service_code": r.4, "level": r.5, "location": r.6, "kind": r.7,
        "salary_min": crate::fees::fmt_money(r.8), "salary_max": crate::fees::fmt_money(r.9),
        "currency": r.10, "status": r.11
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
pub struct RoleIn {
    pub title: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub service: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub requirements: String,
    #[serde(default)]
    pub salary_min: String,
    #[serde(default)]
    pub salary_max: String,
    #[serde(default)]
    pub currency: String,
}

async fn role_add(State(s): State<AppState>, Json(b): Json<RoleIn>) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({"ok": false, "error": "kichwa cha nafasi kinahitajika"}));
    }
    let code = if b.code.trim().is_empty() {
        b.title
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
    } else {
        b.code.trim().to_string()
    };
    // Idara na huduma zinatakiwa kuwepo (hakuna marejeo ya uongo).
    let dep_id = if b.department.trim().is_empty() {
        String::new()
    } else {
        match sqlx::query_as::<_, (String,)>(
            "SELECT id FROM bz_departments WHERE code=?1 OR id=?1 LIMIT 1",
        )
        .bind(b.department.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        {
            Some((id,)) => id,
            None => return Json(json!({"ok": false, "error": "idara haipo"})),
        }
    };
    if !b.service.trim().is_empty() {
        let ok = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM bz_services WHERE code=?1 OR id=?1 LIMIT 1",
        )
        .bind(b.service.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .is_some();
        if !ok {
            return Json(json!({"ok": false, "error": "huduma haipo kwenye katalogi"}));
        }
    }
    let kind = match b.kind.trim().to_lowercase().as_str() {
        "contract" | "mkataba" => "contract",
        "intern" | "mwanafunzi" => "intern",
        _ => "fulltime",
    };
    let cur = if b.currency.trim().is_empty() {
        "TZS".into()
    } else {
        crate::biashara::norm_currency(&b.currency)
    };
    let id = uid();
    let min = crate::fees::to_cents(&b.salary_min).unwrap_or(0);
    let max = crate::fees::to_cents(&b.salary_max).unwrap_or(0);
    let res = sqlx::query(
        "INSERT INTO aj_roles (id,code,title,department_id,service_code,level,location,kind,description,requirements,salary_min_cents,salary_max_cents,currency,status,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'open',?14)",
    )
    .bind(&id)
    .bind(&code)
    .bind(b.title.trim())
    .bind(&dep_id)
    .bind(b.service.trim())
    .bind(crate::biashara::norm_level(&b.level))
    .bind(if b.location.trim().is_empty() { "Dar es Salaam" } else { b.location.trim() })
    .bind(kind)
    .bind(&b.description)
    .bind(&b.requirements)
    .bind(min)
    .bind(max)
    .bind(&cur)
    .bind(now())
    .execute(&s.db)
    .await;
    if res.is_err() {
        return Json(json!({"ok": false, "error": "nafasi yenye code hii ipo tayari"}));
    }
    crate::biashara::audit(&s.db, "admin", "role.add", &id, &code).await;
    Json(json!({"ok": true, "id": id, "code": code, "status": "open"}))
}

async fn role_close(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let res = sqlx::query("UPDATE aj_roles SET status='closed' WHERE id=?1 OR code=?1")
        .bind(&id)
        .execute(&s.db)
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            crate::biashara::audit(&s.db, "admin", "role.close", &id, "").await;
            Json(json!({"ok": true, "id": id, "status": "closed"}))
        }
        _ => Json(json!({"ok": false, "error": "nafasi haipo"})),
    }
}

async fn applications_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let role = q.role.as_deref().filter(|v| !v.is_empty());
    let rows = if let Some(r) = role {
        sqlx::query_as::<_, AppRow>(
            "SELECT id,ref,role_id,full_name,email,phone,education,experience_years,status,score_theory,score_practical,total_score,created_at
             FROM aj_applications WHERE role_id=?1 OR role_id=(SELECT id FROM aj_roles WHERE code=?1) ORDER BY total_score DESC LIMIT ?2",
        )
        .bind(r).bind(lim).fetch_all(&s.db).await
    } else if let Some(st) = q.status.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, AppRow>(
            "SELECT id,ref,role_id,full_name,email,phone,education,experience_years,status,score_theory,score_practical,total_score,created_at
             FROM aj_applications WHERE status=?1 ORDER BY total_score DESC LIMIT ?2",
        )
        .bind(st).bind(lim).fetch_all(&s.db).await
    } else {
        sqlx::query_as::<_, AppRow>(
            "SELECT id,ref,role_id,full_name,email,phone,education,experience_years,status,score_theory,score_practical,total_score,created_at
             FROM aj_applications ORDER BY total_score DESC LIMIT ?1",
        )
        .bind(lim).fetch_all(&s.db).await
    }
    .unwrap_or_default();
    Json(json!({"ok": true, "applications": rows.into_iter().map(app_json).collect::<Vec<_>>()}))
}

#[derive(sqlx::FromRow)]
struct AppRow {
    id: String,
    r#ref: String,
    role_id: String,
    full_name: String,
    email: String,
    phone: String,
    education: String,
    experience_years: i64,
    status: String,
    score_theory: f64,
    score_practical: f64,
    total_score: f64,
    created_at: String,
}

fn app_json(r: AppRow) -> Value {
    json!({
        "id": r.id, "ref": r.r#ref, "role_id": r.role_id, "full_name": r.full_name,
        "email": r.email, "phone": r.phone, "education": r.education,
        "experience_years": r.experience_years, "status": r.status,
        "score_theory": r.score_theory, "score_practical": r.score_practical,
        "total_score": r.total_score, "created_at": r.created_at,
        "passed": passed(r.score_theory, r.score_practical, PASS_MARK)
    })
}

#[derive(Deserialize)]
pub struct ApplyIn {
    pub role: String,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    #[serde(default)]
    pub photo: String,
    #[serde(default)]
    pub cv_url: String,
    #[serde(default)]
    pub education: String,
    #[serde(default)]
    pub experience_years: i64,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub region: String,
}

async fn apply(State(s): State<AppState>, Json(b): Json<ApplyIn>) -> Json<Value> {
    if b.full_name.trim().is_empty() || b.email.trim().is_empty() {
        return Json(json!({"ok": false, "error": "jina na barua pepe vinahitajika"}));
    }
    if !b.email.contains('@') {
        return Json(json!({"ok": false, "error": "barua pepe si sahihi"}));
    }
    if !crate::sms::phone_ok(&b.phone) {
        return Json(json!({"ok": false, "error": "namba ya simu si sahihi"}));
    }
    let Some((role_id, role_status, family)) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,status,service_code FROM aj_roles WHERE code=?1 OR id=?1 LIMIT 1",
    )
    .bind(b.role.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "nafasi haipo"}));
    };
    if role_status != "open" {
        return Json(json!({"ok": false, "error": "nafasi imefungwa"}));
    }
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM aj_applications")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let year = chrono::Local::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let reference = application_ref(year, seq);
    let id = uid();
    let country = if b.country.trim().is_empty() { "TZ".into() } else { b.country.trim().to_ascii_uppercase() };
    let _ = sqlx::query(
        "INSERT INTO aj_applications (id,ref,role_id,full_name,email,phone,photo,cv_url,education,experience_years,skills,country,region,status,score_theory,score_practical,total_score,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'submitted',0,0,0,?14,?14)",
    )
    .bind(&id)
    .bind(&reference)
    .bind(&role_id)
    .bind(b.full_name.trim())
    .bind(b.email.trim())
    .bind(b.phone.trim())
    .bind(&b.photo)
    .bind(&b.cv_url)
    .bind(&b.education)
    .bind(b.experience_years.max(0))
    .bind(serde_json::to_string(&b.skills).unwrap_or_else(|_| "[]".into()))
    .bind(&country)
    .bind(b.region.trim().to_ascii_uppercase())
    .bind(now())
    .execute(&s.db)
    .await;
    crate::biashara::audit(&s.db, "mwombaji", "apply", &id, &reference).await;
    let _ = family;
    Json(json!({
        "ok": true, "id": id, "ref": reference, "status": "submitted",
        "note": "maombi yamepokelewa — yatapitia nadharia na vitendo"
    }))
}

async fn application_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, AppRow>(
        "SELECT id,ref,role_id,full_name,email,phone,education,experience_years,status,score_theory,score_practical,total_score,created_at
         FROM aj_applications WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let assessments = sqlx::query_as::<_, (String, String, String, f64, f64, String, String)>(
        "SELECT id,kind,task,max_score,score,status,created_at FROM aj_assessments WHERE application_id=?1 ORDER BY created_at",
    )
    .bind(&r.id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let criteria = sqlx::query_as::<_, (String, String, f64, f64)>(
        "SELECT assessment_id,label,weight,awarded FROM aj_practical_criteria WHERE assessment_id IN (SELECT id FROM aj_assessments WHERE application_id=?1)",
    )
    .bind(&r.id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true, "application": app_json(r),
        "assessments": assessments.into_iter().map(|a| json!({
            "id": a.0, "kind": a.1, "task": a.2, "max_score": a.3,
            "score": a.4, "status": a.5, "created_at": a.6
        })).collect::<Vec<_>>(),
        "criteria": criteria.into_iter().map(|c| json!({
            "assessment_id": c.0, "label": c.1, "weight": c.2, "awarded": c.3
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct StageIn {
    pub stage: String,
}

async fn application_stage(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<StageIn>,
) -> Json<Value> {
    let Some((aid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM aj_applications WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let to = b.stage.trim().to_lowercase();
    if !STAGES.contains(&to.as_str()) {
        return Json(json!({"ok": false, "error": "hatua haijulikani"}));
    }
    if !stage_transition(&cur, &to) {
        return Json(json!({"ok": false, "error": format!("mabadiliko `{cur}` → `{to}` hayaruhusiwi")}));
    }
    let _ = sqlx::query("UPDATE aj_applications SET status=?2, updated_at=?3 WHERE id=?1")
        .bind(&aid)
        .bind(&to)
        .bind(now())
        .execute(&s.db)
        .await;
    crate::biashara::audit(&s.db, "admin", "apply.stage", &aid, &to).await;
    Json(json!({"ok": true, "id": aid, "status": to}))
}

/// Anzisha tathmini ya nadharia: chagua maswali kutoka benki kwa familia.
async fn theory_start(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let Some((aid, status, role_id)) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,status,role_id FROM aj_applications WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if !matches!(status.as_str(), "screening" | "theory") {
        return Json(json!({"ok": false, "error": format!("ombi ({status}) haliko hatua ya nadharia")}));
    }
    // Familia kutoka huduma ya nafasi (kama ipo), la sivyo `general`.
    let service = sqlx::query_as::<_, (String,)>("SELECT service_code FROM aj_roles WHERE id=?1")
        .bind(&role_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|r| r.0)
        .unwrap_or_default();
    let family = norm_family(&service);
    let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id,question,options,answer,marks FROM aj_question_bank
         WHERE active=1 AND (family=?1 OR family='general') ORDER BY RANDOM() LIMIT 8",
    )
    .bind(family)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    if rows.is_empty() {
        return Json(json!({"ok": false, "error": "benki ya maswali haina maswali"}));
    }
    // Maswali yanayotumwa kwa mwombaji HAYANA majibu.
    let questions: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.0, "question": r.1,
                "options": serde_json::from_str::<Value>(&r.2).unwrap_or(json!([])),
                "marks": r.4
            })
        })
        .collect();
    // Kadi ya ndani yenye majibu (kwa kupima).
    let key: Vec<Value> = rows
        .iter()
        .map(|r| json!({"id": r.0, "answer": r.3, "marks": r.4}))
        .collect();
    let max: f64 = rows.iter().map(|r| r.4 as f64).sum();
    let assessment_id = uid();
    let _ = sqlx::query(
        "INSERT INTO aj_assessments (id,application_id,kind,task,questions,answers,max_score,score,graded_by,status,created_at)
         VALUES (?1,?2,'theory','Nadharia',?3,?4,?5,0,'','pending',?6)",
    )
    .bind(&assessment_id)
    .bind(&aid)
    .bind(serde_json::to_string(&key).unwrap_or_else(|_| "[]".into()))
    .bind("[]")
    .bind(max)
    .bind(now())
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE aj_applications SET status='theory', updated_at=?2 WHERE id=?1")
        .bind(&aid)
        .bind(now())
        .execute(&s.db)
        .await;
    Json(json!({
        "ok": true, "assessment_id": assessment_id, "kind": "theory",
        "max_score": max, "questions": questions
    }))
}

#[derive(Deserialize)]
pub struct AnswersIn {
    pub assessment: String,
    pub answers: Vec<String>,
}

/// Wasilisha majibu ya nadharia → hupimwa KWA CODE mara moja.
async fn theory_submit(State(s): State<AppState>, Json(b): Json<AnswersIn>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, f64)>(
        "SELECT id,application_id,questions,max_score FROM aj_assessments WHERE id=?1 AND kind='theory' LIMIT 1",
    )
    .bind(b.assessment.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((asid, aid, key_json, max)) = row else {
        return Json(json!({"ok": false, "error": "tathmini ya nadharia haipo"}));
    };
    let key: Value = serde_json::from_str(&key_json).unwrap_or(json!([]));
    let answers: Value = json!(b.answers);
    let pct = theory_percent(&key, &answers);
    let _ = sqlx::query(
        "UPDATE aj_assessments SET answers=?2, score=?3, status='graded', graded_by='auto', submitted_at=?4, graded_at=?4 WHERE id=?1",
    )
    .bind(&asid)
    .bind(serde_json::to_string(&b.answers).unwrap_or_else(|_| "[]".into()))
    .bind(pct)
    .bind(now())
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE aj_applications SET score_theory=?2, updated_at=?3 WHERE id=?1")
        .bind(&aid)
        .bind(pct)
        .bind(now())
        .execute(&s.db)
        .await;
    recompute_total(&s.db, &aid).await;
    crate::biashara::audit(&s.db, "mwombaji", "theory.submit", &aid, &format!("{pct:.1}%")).await;
    Json(json!({
        "ok": true, "assessment_id": asid, "score": pct, "max_score": max,
        "passed": pct >= PASS_MARK, "graded_by": "auto",
        "note": "nadharia ilipimwa kwa code — hakuna mtu aliyepanga alama"
    }))
}

#[derive(Deserialize)]
pub struct PracticalIn {
    pub task: String,
    #[serde(default)]
    pub criteria: Vec<String>,
}

/// Toa kazi ya vitendo yenye vigezo (rubric).
async fn practical_assign(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<PracticalIn>) -> Json<Value> {
    let Some((aid, status)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM aj_applications WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if b.task.trim().is_empty() {
        return Json(json!({"ok": false, "error": "kazi ya vitendo inahitajika"}));
    }
    if !matches!(status.as_str(), "theory" | "practical") {
        return Json(json!({"ok": false, "error": format!("ombi ({status}) haliko hatua ya vitendo")}));
    }
    let criteria: Vec<String> = if b.criteria.is_empty() {
        vec![
            "Usahihi wa matokeo".into(),
            "Ubora wa code/muundo".into(),
            "Usalama na uangalifu".into(),
            "Ufafanuzi wa suluhisho".into(),
        ]
    } else {
        b.criteria.clone()
    };
    let assessment_id = uid();
    let max = criteria.len() as f64;
    let _ = sqlx::query(
        "INSERT INTO aj_assessments (id,application_id,kind,task,questions,answers,max_score,score,graded_by,status,created_at)
         VALUES (?1,?2,'practical',?3,'[]','[]',?4,0,'','pending',?5)",
    )
    .bind(&assessment_id)
    .bind(&aid)
    .bind(b.task.trim())
    .bind(max)
    .bind(now())
    .execute(&s.db)
    .await;
    for c in &criteria {
        let _ = sqlx::query(
            "INSERT INTO aj_practical_criteria (id,assessment_id,label,weight,awarded,note)
             VALUES (?1,?2,?3,1.0,0,'')",
        )
        .bind(uid())
        .bind(&assessment_id)
        .bind(c)
        .execute(&s.db)
        .await;
    }
    let _ = sqlx::query("UPDATE aj_applications SET status='practical', updated_at=?2 WHERE id=?1")
        .bind(&aid)
        .bind(now())
        .execute(&s.db)
        .await;
    crate::biashara::audit(&s.db, "admin", "practical.assign", &aid, "").await;
    Json(json!({
        "ok": true, "assessment_id": assessment_id, "kind": "practical",
        "task": b.task, "criteria": criteria, "max_score": max
    }))
}

#[derive(Deserialize)]
pub struct PracticalSubmitIn {
    pub assessment: String,
    pub submission: String,
}

/// Mwombaji huwasilisha kazi ya vitendo (inabaki `submitted` hadi kupimwa).
async fn practical_submit(State(s): State<AppState>, Json(b): Json<PracticalSubmitIn>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id,application_id FROM aj_assessments WHERE id=?1 AND kind='practical' LIMIT 1",
    )
    .bind(b.assessment.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((asid, aid)) = row else {
        return Json(json!({"ok": false, "error": "tathmini ya vitendo haipo"}));
    };
    if b.submission.trim().is_empty() {
        return Json(json!({"ok": false, "error": "kazi iliyowasilishwa hauwezi kuwa tupu"}));
    }
    let _ = sqlx::query(
        "UPDATE aj_assessments SET answers=?2, status='submitted', submitted_at=?3 WHERE id=?1",
    )
    .bind(&asid)
    .bind(serde_json::to_string(&json!([{"submission": b.submission}])).unwrap_or_else(|_| "[]".into()))
    .bind(now())
    .execute(&s.db)
    .await;
    crate::biashara::audit(&s.db, "mwombaji", "practical.submit", &aid, "").await;
    Json(json!({"ok": true, "assessment_id": asid, "status": "submitted",
        "note": "kazi imewasilishwa — msimamizi atapima kwa vigezo"}))
}

#[derive(Deserialize)]
pub struct PracticalGradeIn {
    pub assessment: String,
    pub scores: Vec<f64>,
    #[serde(default)]
    pub grader: String,
}

/// Msimamizi hupima kazi ya vitendo kwa vigezo.
async fn practical_grade(State(s): State<AppState>, Json(b): Json<PracticalGradeIn>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT id,application_id FROM aj_assessments WHERE id=?1 AND kind='practical' LIMIT 1",
    )
    .bind(b.assessment.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((asid, aid)) = row else {
        return Json(json!({"ok": false, "error": "tathmini ya vitendo haipo"}));
    };
    let crit = sqlx::query_as::<_, (String, f64)>(
        "SELECT id,weight FROM aj_practical_criteria WHERE assessment_id=?1 ORDER BY rowid",
    )
    .bind(&asid)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    if crit.len() != b.scores.len() {
        return Json(json!({
            "ok": false,
            "error": format!("vigezo {} lakini alama {} zimetolewa", crit.len(), b.scores.len())
        }));
    }
    let mut pairs: Vec<(f64, f64)> = Vec::new();
    for (i, (cid, w)) in crit.iter().enumerate() {
        // Alama haziwezi kuzidi uzito wa kigezo.
        let awarded = b.scores[i].clamp(0.0, *w);
        pairs.push((*w, awarded));
        let _ = sqlx::query("UPDATE aj_practical_criteria SET awarded=?2 WHERE id=?1")
            .bind(cid)
            .bind(awarded)
            .execute(&s.db)
            .await;
    }
    let pct = practical_percent(&pairs);
    let grader = if b.grader.trim().is_empty() { "admin" } else { b.grader.trim() };
    let _ = sqlx::query(
        "UPDATE aj_assessments SET score=?2, status='graded', graded_by=?3, graded_at=?4 WHERE id=?1",
    )
    .bind(&asid)
    .bind(pct)
    .bind(grader)
    .bind(now())
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE aj_applications SET score_practical=?2, updated_at=?3 WHERE id=?1")
        .bind(&aid)
        .bind(pct)
        .bind(now())
        .execute(&s.db)
        .await;
    recompute_total(&s.db, &aid).await;
    crate::biashara::audit(&s.db, grader, "practical.grade", &aid, &format!("{pct:.1}%")).await;
    Json(json!({"ok": true, "assessment_id": asid, "score": pct, "graded_by": grader,
        "passed": pct >= PASS_MARK}))
}

async fn recompute_total(db: &SqlitePool, aid: &str) {
    let row = sqlx::query_as::<_, (f64, f64)>(
        "SELECT score_theory, score_practical FROM aj_applications WHERE id=?1",
    )
    .bind(aid)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    if let Some((t, p)) = row {
        let _ = sqlx::query("UPDATE aj_applications SET total_score=?2, updated_at=?3 WHERE id=?1")
            .bind(aid)
            .bind(total_score(t, p))
            .bind(now())
            .execute(db)
            .await;
    }
}

#[derive(Deserialize)]
pub struct HireIn {
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub level: String,
}

/// Kuajiri: huunda mfanyakazi kwenye biashara (bz_employees) akiwa na zana.
async fn hire(State(s): State<AppState>, Path(id): Path<String>, Json(b): Json<HireIn>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, f64, f64)>(
        "SELECT id,ref,full_name,phone,email,score_theory,score_practical FROM aj_applications WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((aid, reference, name, phone, email, t, p)) = row else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if !passed(t, p, PASS_MARK) {
        return Json(json!({
            "ok": false,
            "error": format!("mwombaji hajafaulu (nadharia {t:.1}%, vitendo {p:.1}% — kiwango {PASS_MARK}%)")
        }));
    }
    let bad: Vec<&String> = b.tools.iter().filter(|x| !crate::biashara::tool_exists(x)).collect();
    if !bad.is_empty() {
        return Json(json!({"ok": false, "error": "zana zisizojulikana kwenye ombi"}));
    }
    let dep_id = if b.department.trim().is_empty() {
        String::new()
    } else {
        match sqlx::query_as::<_, (String,)>(
            "SELECT id FROM bz_departments WHERE code=?1 OR id=?1 LIMIT 1",
        )
        .bind(b.department.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        {
            Some((d,)) => d,
            None => return Json(json!({"ok": false, "error": "idara haipo"})),
        }
    };
    let level = if b.level.trim().is_empty() {
        "junior".to_string()
    } else {
        crate::biashara::norm_level(&b.level).to_string()
    };
    let emp_id = uid();
    let tools_json = serde_json::to_string(&b.tools).unwrap_or_else(|_| "[]".into());
    let _ = sqlx::query(
        "INSERT INTO bz_employees (id,user_id,full_name,photo,profession,level,department_id,phone,email,tools,status,hired_at)
         VALUES (?1,?2,?3,'',?4,?5,?6,?7,?8,?9,'active',?10)",
    )
    .bind(&emp_id)
    .bind(&aid)
    .bind(&name)
    .bind(&reference)
    .bind(&level)
    .bind(&dep_id)
    .bind(&phone)
    .bind(&email)
    .bind(&tools_json)
    .bind(now())
    .execute(&s.db)
    .await;
    for tl in &b.tools {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO bz_tool_grants (id,employee_id,tool_code,granted_by,granted_at,revoked_at)
             VALUES (?1,?2,?3,'admin',?4,NULL)",
        )
        .bind(uid())
        .bind(&emp_id)
        .bind(tl)
        .bind(now())
        .execute(&s.db)
        .await;
    }
    let _ = sqlx::query("UPDATE aj_applications SET status='hired', updated_at=?2 WHERE id=?1")
        .bind(&aid)
        .bind(now())
        .execute(&s.db)
        .await;
    crate::biashara::audit(&s.db, "admin", "apply.hire", &aid, &name).await;
    Json(json!({
        "ok": true, "application_id": aid, "status": "hired",
        "employee_id": emp_id, "full_name": name, "level": level, "tools": b.tools,
        "note": "mwombaji ameajiriwa na ameingizwa kwenye wafanyakazi wa kampuni"
    }))
}

async fn reject_h(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let res = sqlx::query(
        "UPDATE aj_applications SET status='rejected', updated_at=?2 WHERE (id=?1 OR ref=?1) AND status NOT IN ('hired','rejected')",
    )
    .bind(&id)
    .bind(now())
    .execute(&s.db)
    .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            crate::biashara::audit(&s.db, "admin", "apply.reject", &id, "").await;
            Json(json!({"ok": true, "id": id, "status": "rejected"}))
        }
        _ => Json(json!({"ok": false, "error": "ombi haipo au limeshafikia hatua ya mwisho"})),
    }
}

async fn questions_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(300).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT id,family,question,answer,marks FROM aj_question_bank WHERE active=1 ORDER BY family LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "questions": rows.into_iter().map(|r| json!({
        "id": r.0, "family": r.1, "question": r.2, "answer": r.3, "marks": r.4
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
pub struct QuestionIn {
    pub question: String,
    pub options: Vec<String>,
    pub answer: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub marks: i64,
}

async fn question_add(State(s): State<AppState>, Json(b): Json<QuestionIn>) -> Json<Value> {
    if b.question.trim().is_empty() || b.options.len() < 2 {
        return Json(json!({"ok": false, "error": "swali na chaguo (2+) vinahitajika"}));
    }
    if !b.options.iter().any(|o| o == &b.answer) {
        return Json(json!({"ok": false, "error": "jibu lazima liwe mojawapo ya chaguo"}));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO aj_question_bank (id,family,question,options,answer,marks,active) VALUES (?1,?2,?3,?4,?5,?6,1)",
    )
    .bind(&id)
    .bind(norm_family(&b.family))
    .bind(b.question.trim())
    .bind(serde_json::to_string(&b.options).unwrap_or_else(|_| "[]".into()))
    .bind(&b.answer)
    .bind(b.marks.max(1))
    .execute(&s.db)
    .await;
    Json(json!({"ok": true, "id": id}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kupima_chaguo_nyingi_hulinganisha_kwa_usafi() {
        assert!(grade_mcq("404", " 404 "));
        assert!(grade_mcq("O(log n)", "o(log n)"));
        assert!(grade_mcq("Argon2id/bcrypt", "Argon2id/bcrypt."));
        assert!(!grade_mcq("404", "500"));
        assert!(!grade_mcq("", ""));
    }

    #[test]
    fn alama_za_nadharia_hupimwa() {
        let key = json!([
            {"id": "q1", "answer": "404", "marks": 1},
            {"id": "q2", "answer": "Rust", "marks": 1},
            {"id": "q3", "answer": "SELECT", "marks": 2},
        ]);
        // Majibu mawili sahihi kati ya manne ya alama → 3/4 = 75%
        let answers = json!(["404", "Go", "SELECT"]);
        assert_eq!(theory_percent(&key, &answers), 75.0);
        // Yote sahihi → 100%
        assert_eq!(theory_percent(&key, &json!(["404", "Rust", "select"])), 100.0);
        // Hakuna → 0%
        assert_eq!(theory_percent(&key, &json!(["a", "b", "c"])), 0.0);
        assert_eq!(theory_percent(&json!([]), &json!([])), 0.0);
    }

    #[test]
    fn alama_za_vitendo_hupimwa_kwa_vigezo() {
        assert_eq!(practical_percent(&[(1.0, 1.0), (1.0, 0.0)]), 50.0);
        assert_eq!(practical_percent(&[(1.0, 1.0), (1.0, 1.0)]), 100.0);
        assert_eq!(practical_percent(&[(2.0, 1.0)]), 50.0);
        assert_eq!(practical_percent(&[]), 0.0);
    }

    #[test]
    fn kufaulu_kunahitaji_zote_mbili() {
        assert!(passed(70.0, 80.0, PASS_MARK));
        assert!(!passed(50.0, 90.0, PASS_MARK));
        assert!(!passed(90.0, 50.0, PASS_MARK));
        assert_eq!(total_score(80.0, 60.0), 70.0);
    }

    #[test]
    fn hatua_za_maombi_hulindwa() {
        assert!(stage_transition("submitted", "screening"));
        assert!(stage_transition("screening", "theory"));
        assert!(stage_transition("theory", "practical"));
        assert!(stage_transition("practical", "interview"));
        assert!(stage_transition("interview", "offered"));
        assert!(stage_transition("offered", "hired"));
        assert!(!stage_transition("submitted", "hired"));
        assert!(!stage_transition("hired", "rejected"));
    }

    #[test]
    fn kumbukumbu_ya_maombi_ina_umbo() {
        assert_eq!(application_ref(2026, 7), "APP-2026-000007");
    }
}
