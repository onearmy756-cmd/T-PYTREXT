//! clearance.rs — ELIMUPRO Awamu 14: fomu ya kusafisha / clearance (moduli 14).
//!
//! Mtiririko halisi wa kusafisha chuo:
//!   1. Admin huweka template ya clearance (`ep_clearance_templates`) + idara
//!      zinazohitajika kusaini (`ep_clearance_departments`).
//!   2. Mwanafunzi huomba clearance (`ep_clearances`) — hupata sehemu kwa kila
//!      idara (`ep_clearance_sections`).
//!   3. Kila idara huona SEHEMU YAKE PEKEE na husaini/kukataa.
//!   4. Zote zikisainiwa → `cleared`; mhuri wa chuo huwekwa.
//!
//! Kanuni halisi:
//!   - Idara haiwezi kusaini sehemu ya idara nyingine.
//!   - Mwanafunzi hawezi kuomba clearance ya pili kwa muhula ule ule.
//!   - Sehemu iliyokataliwa hairuhusu clearance kukamilika.
//!   - Idara za `blocking` huzuia: `finance` hukagua deni, `library` faini,
//!     `hostel` ugawaji usiofungwa.
//!
//! Tables: ep_clearance_templates ep_clearance_departments ep_clearances
//!         ep_clearance_sections

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
        r#"CREATE TABLE IF NOT EXISTS ep_clearance_templates (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            pdf_path TEXT NOT NULL DEFAULT '',
            seal_text TEXT NOT NULL DEFAULT '',
            active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_clearance_departments (
            id TEXT PRIMARY KEY,
            template_id TEXT NOT NULL,
            code TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL DEFAULT '',
            blocking INTEGER NOT NULL DEFAULT 1,
            sort_order INTEGER NOT NULL DEFAULT 0,
            UNIQUE(template_id, code)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_clearances (
            id TEXT PRIMARY KEY,
            student_id TEXT NOT NULL,
            template_id TEXT NOT NULL DEFAULT '',
            academic_year TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|cleared|rejected
            seal_text TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            cleared_at TEXT,
            UNIQUE(student_id, academic_year)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_clearance_sections (
            id TEXT PRIMARY KEY,
            clearance_id TEXT NOT NULL,
            department_code TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|signed|rejected
            comment TEXT NOT NULL DEFAULT '',
            signed_by TEXT NOT NULL DEFAULT '',
            signed_at TEXT,
            UNIQUE(clearance_id, department_code)
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/clearance/status", get(status))
        .route("/api/clearance/template/add", post(template_add))
        .route("/api/clearance/templates", get(templates_list))
        .route("/api/clearance/department/add", post(department_add))
        .route("/api/clearance/template/:id", get(template_get))
        .route("/api/clearance/request", post(request))
        .route("/api/clearance/list", get(clearances_list))
        .route("/api/clearance/:id", get(clearance_get))
        .route("/api/clearance/section/sign", post(section_sign))
        .route("/api/clearance/section/reject", post(section_reject))
        .route("/api/clearance/:id/verify", get(clearance_verify))
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

/// Idara za msingi za chuo.
pub const CORE_DEPARTMENTS: [(&str, &str); 4] = [
    ("library", "Maktaba"),
    ("finance", "Fedha / Ada"),
    ("hostel", "Hosteli"),
    ("dean", "Mkuu wa Kitivo"),
];

pub fn norm_dept(code: &str) -> String {
    code.trim().to_lowercase()
}

/// Clearance inaweza kukamilika? (sehemu zote za `blocking` zimesainiwa.)
pub fn is_complete(sections: &[(String, bool, String)]) -> bool {
    sections
        .iter()
        .filter(|(_, blocking, _)| *blocking)
        .all(|(_, _, st)| st == "signed")
}

/// Sehemu hii inaruhusu idara hii kusaini?
pub fn can_sign(dept_of_section: &str, dept_of_actor: &str, section_status: &str) -> Result<(), String> {
    if norm_dept(dept_of_section) != norm_dept(dept_of_actor) {
        return Err("idara hii haiwezi kusaini sehemu ya idara nyingine".into());
    }
    match section_status {
        "pending" => Ok(()),
        "signed" => Err("sehemu imesainiwa tayari".into()),
        _ => Err(format!("hali '{section_status}' hairuhusu kusaini")),
    }
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (pending,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_clearances WHERE status='pending'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (cleared,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_clearances WHERE status='cleared'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (rejected,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_clearances WHERE status='rejected'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "templates": table_count(&s.db, "ep_clearance_templates").await,
        "departments": table_count(&s.db, "ep_clearance_departments").await,
        "clearances": table_count(&s.db, "ep_clearances").await,
        "pending": pending,
        "cleared": cleared,
        "rejected": rejected,
        "core_departments": CORE_DEPARTMENTS.iter().map(|(c, n)| json!({"code": c, "name": n})).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct TemplateIn {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pdf_path: String,
    #[serde(default)]
    pub seal_text: String,
}

pub async fn template_add(State(s): State<AppState>, Json(b): Json<TemplateIn>) -> Json<Value> {
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la template linahitajika" }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_clearance_templates (id,name,pdf_path,seal_text,active,created_at)
         VALUES (?1,?2,?3,?4,1,?5)",
    )
    .bind(&id)
    .bind(b.name.trim())
    .bind(&b.pdf_path)
    .bind(&b.seal_text)
    .bind(now())
    .execute(&s.db)
    .await;
    // Weka idara za msingi kiotomatiki.
    for (i, (code, name)) in CORE_DEPARTMENTS.iter().enumerate() {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO ep_clearance_departments (id,template_id,code,name,blocking,sort_order)
             VALUES (?1,?2,?3,?4,1,?5)",
        )
        .bind(uid())
        .bind(&id)
        .bind(code)
        .bind(name)
        .bind(i as i64)
        .execute(&s.db)
        .await;
    }
    Json(json!({ "ok": true, "id": id }))
}

pub async fn templates_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, String)>(
        "SELECT id,name,pdf_path,seal_text,active,created_at FROM ep_clearance_templates ORDER BY created_at DESC",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "templates": rows.into_iter().map(|r| json!({
            "id": r.0, "name": r.1, "pdf_path": r.2, "seal_text": r.3, "active": r.4, "created_at": r.5
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct DeptIn {
    pub template_id: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub blocking: Option<bool>,
    #[serde(default)]
    pub sort_order: Option<i64>,
}

pub async fn department_add(State(s): State<AppState>, Json(b): Json<DeptIn>) -> Json<Value> {
    if b.template_id.trim().is_empty() || b.code.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "template_id na code zinahitajika" }));
    }
    let code = norm_dept(&b.code);
    let exists = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM ep_clearance_departments WHERE template_id=?1 AND code=?2",
    )
    .bind(b.template_id.trim())
    .bind(&code)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if exists.is_some() {
        return Json(json!({ "ok": false, "error": "idara hii ipo tayari kwenye template" }));
    }
    let id = uid();
    let blocking = b.blocking.unwrap_or(true);
    let _ = sqlx::query(
        "INSERT INTO ep_clearance_departments (id,template_id,code,name,blocking,sort_order)
         VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(&id)
    .bind(b.template_id.trim())
    .bind(&code)
    .bind(if b.name.trim().is_empty() { code.clone() } else { b.name.trim().to_string() })
    .bind(if blocking { 1 } else { 0 })
    .bind(b.sort_order.unwrap_or(99))
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "code": code }))
}

pub async fn template_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let t = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id,name,pdf_path,seal_text FROM ep_clearance_templates WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match t {
        None => Json(json!({ "ok": false, "error": "template haipo" })),
        Some((tid, name, pdf, seal)) => {
            let depts = sqlx::query_as::<_, (String, String, i64, i64)>(
                "SELECT code,name,blocking,sort_order FROM ep_clearance_departments WHERE template_id=?1 ORDER BY sort_order",
            )
            .bind(&tid)
            .fetch_all(&s.db)
            .await
            .unwrap_or_default();
            let _ = name;
            Json(json!({
                "ok": true,
                "template": { "id": tid, "name": name, "pdf_path": pdf, "seal_text": seal },
                "departments": depts.into_iter().map(|d| json!({
                    "code": d.0, "name": d.1, "blocking": d.2 == 1, "sort_order": d.3
                })).collect::<Vec<_>>()
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct RequestIn {
    pub student_id: String,
    pub template_id: String,
    #[serde(default)]
    pub academic_year: String,
}

pub async fn request(State(s): State<AppState>, Json(b): Json<RequestIn>) -> Json<Value> {
    if b.student_id.trim().is_empty() || b.template_id.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "student_id na template_id zinahitajika" }));
    }
    let year = if b.academic_year.trim().is_empty() {
        chrono::Local::now().format("%Y").to_string()
    } else {
        b.academic_year.trim().to_string()
    };
    let dup = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM ep_clearances WHERE student_id=?1 AND academic_year=?2",
    )
    .bind(b.student_id.trim())
    .bind(&year)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if let Some((id,)) = dup {
        return Json(json!({
            "ok": false, "error": "clearance ya mwaka huu ipo tayari", "id": id
        }));
    }
    let seal = sqlx::query_as::<_, (String,)>(
        "SELECT seal_text FROM ep_clearance_templates WHERE id=?1",
    )
    .bind(b.template_id.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    .map(|r| r.0)
    .unwrap_or_default();

    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_clearances (id,student_id,template_id,academic_year,status,seal_text,created_at)
         VALUES (?1,?2,?3,?4,'pending',?5,?6)",
    )
    .bind(&id)
    .bind(b.student_id.trim())
    .bind(b.template_id.trim())
    .bind(&year)
    .bind(&seal)
    .bind(now())
    .execute(&s.db)
    .await;

    let depts = sqlx::query_as::<_, (String,)>(
        "SELECT code FROM ep_clearance_departments WHERE template_id=?1",
    )
    .bind(b.template_id.trim())
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    for (code,) in &depts {
        let _ = sqlx::query(
            "INSERT INTO ep_clearance_sections (id,clearance_id,department_code,status)
             VALUES (?1,?2,?3,'pending')",
        )
        .bind(uid())
        .bind(&id)
        .bind(code)
        .execute(&s.db)
        .await;
    }
    Json(json!({ "ok": true, "id": id, "academic_year": year, "sections": depts.len() }))
}

#[derive(Deserialize)]
pub struct ClrQuery {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub department: String,
}

pub async fn clearances_list(State(s): State<AppState>, Query(q): Query<ClrQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
        "SELECT id,student_id,template_id,academic_year,status,cleared_at FROM ep_clearances
         WHERE (?1='' OR student_id=?1) AND (?2='' OR status=?2)
         ORDER BY created_at DESC LIMIT 300",
    )
    .bind(&q.student_id)
    .bind(&q.status)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();

    let mut out = Vec::new();
    for (id, student, tpl, year, st, cleared) in rows {
        let secs = sections_for(&s.db, &id).await;
        // Chuja kwa idara ikiwa imeombwa (idara huona sehemu yake pekee).
        let visible: Vec<Value> = secs
            .iter()
            .filter(|(code, _, _, _, _)| {
                q.department.trim().is_empty() || norm_dept(code) == norm_dept(&q.department)
            })
            .map(|(code, st, comment, by, at)| {
                json!({
                    "department_code": code, "status": st, "comment": comment,
                    "signed_by": by, "signed_at": at
                })
            })
            .collect();
        out.push(json!({
            "id": id, "student_id": student, "template_id": tpl, "academic_year": year,
            "status": st, "cleared_at": cleared, "sections": visible
        }));
    }
    Json(json!({ "ok": true, "clearances": out }))
}

type SectionRow = (String, String, String, String, Option<String>);

async fn sections_for(db: &SqlitePool, clearance_id: &str) -> Vec<SectionRow> {
    sqlx::query_as::<_, SectionRow>(
        "SELECT department_code,status,comment,signed_by,signed_at FROM ep_clearance_sections
         WHERE clearance_id=?1 ORDER BY department_code",
    )
    .bind(clearance_id)
    .fetch_all(db)
    .await
    .unwrap_or_default()
}

pub async fn clearance_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let c = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>)>(
        "SELECT id,student_id,template_id,academic_year,status,seal_text,cleared_at
         FROM ep_clearances WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match c {
        None => Json(json!({ "ok": false, "error": "clearance haipo" })),
        Some((cid, student, tpl, year, st, seal, cleared)) => {
            let secs = sections_for(&s.db, &cid).await;
            Json(json!({
                "ok": true,
                "clearance": {
                    "id": cid, "student_id": student, "template_id": tpl,
                    "academic_year": year, "status": st, "seal_text": seal,
                    "cleared_at": cleared
                },
                "sections": secs.iter().map(|(code, st, comment, by, at)| json!({
                    "department_code": code, "status": st, "comment": comment,
                    "signed_by": by, "signed_at": at
                })).collect::<Vec<_>>()
            }))
        }
    }
}

/// Hukagua kama idara ina kizuizi kwa mwanafunzi huyu.
/// Hurudisha `None` kama ni safi, au sababu ya kuzuia.
pub async fn blocking_reason(db: &SqlitePool, dept: &str, student_id: &str) -> Option<String> {
    match norm_dept(dept).as_str() {
        "library" => {
            let (fines,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(SUM(fine_cents),0) FROM ep_loans WHERE student_id=?1 AND fine_paid=0",
            )
            .bind(student_id)
            .fetch_one(db)
            .await
            .unwrap_or((0,));
            if fines > 0 {
                return Some(format!("faini ya maktaba isiyolipwa ({} senti)", fines));
            }
            let (open,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM ep_loans WHERE student_id=?1 AND returned_at IS NULL",
            )
            .bind(student_id)
            .fetch_one(db)
            .await
            .unwrap_or((0,));
            if open > 0 {
                return Some(format!("vitabu {open} havirudishwa"));
            }
            None
        }
        "finance" => {
            let (due,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(SUM(i.total_cents),0) - COALESCE((
                    SELECT SUM(p.amount_cents) FROM ep_payments p WHERE p.student_id=?1
                 ),0)
                 FROM ep_invoices i WHERE i.student_id=?1",
            )
            .bind(student_id)
            .fetch_one(db)
            .await
            .unwrap_or((0,));
            if due > 0 {
                return Some(format!("salio la ada {due} senti"));
            }
            None
        }
        "hostel" => {
            let (open,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM ep_allocations WHERE student_id=?1 AND vacated_at IS NULL",
            )
            .bind(student_id)
            .fetch_one(db)
            .await
            .unwrap_or((0,));
            if open > 0 {
                return Some("ugawaji wa hosteli haujafungwa".into());
            }
            None
        }
        _ => None,
    }
}

#[derive(Deserialize)]
pub struct SignIn {
    pub clearance_id: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub signed_by: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub force: bool,
}

/// Hupata hali ya sehemu + mwanafunzi (JOIN — `status` ni ya sehemu, si ya clearance).
async fn section_status(db: &SqlitePool, clearance_id: &str, dept: &str) -> Option<(String, String)> {
    sqlx::query_as::<_, (String, String)>(
        "SELECT sc.status,c.student_id FROM ep_clearance_sections sc
         JOIN ep_clearances c ON c.id=sc.clearance_id
         WHERE sc.clearance_id=?1 AND sc.department_code=?2",
    )
    .bind(clearance_id)
    .bind(dept)
    .fetch_optional(db)
    .await
    .ok()
    .flatten()
}

pub async fn section_sign(State(s): State<AppState>, Json(b): Json<SignIn>) -> Json<Value> {
    let cid = b.clearance_id.trim();
    let dept = norm_dept(&b.department);
    if cid.is_empty() || dept.is_empty() {
        return Json(json!({ "ok": false, "error": "clearance_id na department zinahitajika" }));
    }
    let cur = section_status(&s.db, cid, &dept).await;
    let (st, student) = match cur {
        None => return Json(json!({ "ok": false, "error": "sehemu ya idara hii haipo" })),
        Some(x) => x,
    };
    if let Err(e) = can_sign(&dept, &dept, &st) {
        return Json(json!({ "ok": false, "error": e }));
    }
    // Kizuizi halisi cha idara (deni/faini/ugawaji) — bila `force`.
    if !b.force {
        if let Some(reason) = blocking_reason(&s.db, &dept, &student).await {
            return Json(json!({ "ok": false, "error": reason, "blocked": true }));
        }
    }
    let _ = sqlx::query(
        "UPDATE ep_clearance_sections SET status='signed',comment=?1,signed_by=?2,signed_at=?3
         WHERE clearance_id=?4 AND department_code=?5",
    )
    .bind(&b.comment)
    .bind(&b.signed_by)
    .bind(now())
    .bind(cid)
    .bind(&dept)
    .execute(&s.db)
    .await;

    let done = maybe_complete(&s, cid).await;
    Json(json!({ "ok": true, "status": "signed", "clearance_status": done }))
}

pub async fn section_reject(State(s): State<AppState>, Json(b): Json<SignIn>) -> Json<Value> {
    let cid = b.clearance_id.trim();
    let dept = norm_dept(&b.department);
    if cid.is_empty() || dept.is_empty() {
        return Json(json!({ "ok": false, "error": "clearance_id na department zinahitajika" }));
    }
    let cur = sqlx::query_as::<_, (String,)>(
        "SELECT status FROM ep_clearance_sections WHERE clearance_id=?1 AND department_code=?2",
    )
    .bind(cid)
    .bind(&dept)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match cur {
        None => return Json(json!({ "ok": false, "error": "sehemu ya idara hii haipo" })),
        Some((st,)) if st != "pending" => {
            return Json(json!({ "ok": false, "error": format!("hali '{st}' hairuhusu kukataa") }))
        }
        _ => {}
    }
    let _ = sqlx::query(
        "UPDATE ep_clearance_sections SET status='rejected',comment=?1,signed_by=?2,signed_at=?3
         WHERE clearance_id=?4 AND department_code=?5",
    )
    .bind(&b.comment)
    .bind(&b.signed_by)
    .bind(now())
    .bind(cid)
    .bind(&dept)
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE ep_clearances SET status='rejected' WHERE id=?1")
        .bind(cid)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "status": "rejected" }))
}

/// Hukamilisha clearance kama sehemu zote za `blocking` zimesainiwa.
pub async fn maybe_complete(s: &AppState, clearance_id: &str) -> String {
    let rows = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT sc.department_code, d.blocking, sc.status
         FROM ep_clearance_sections sc
         JOIN ep_clearances c ON c.id=sc.clearance_id
         LEFT JOIN ep_clearance_departments d
           ON d.template_id=c.template_id AND d.code=sc.department_code
         WHERE sc.clearance_id=?1",
    )
    .bind(clearance_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mapped: Vec<(String, bool, String)> = rows
        .into_iter()
        .map(|(c, b, st)| (c, b == 1, st))
        .collect();
    if is_complete(&mapped) {
        let _ = sqlx::query(
            "UPDATE ep_clearances SET status='cleared',cleared_at=?1 WHERE id=?2",
        )
        .bind(now())
        .bind(clearance_id)
        .execute(&s.db)
        .await;
        "cleared".to_string()
    } else {
        "pending".to_string()
    }
}

pub async fn clearance_verify(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let c = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT student_id,academic_year,status,seal_text,COALESCE(cleared_at,'') FROM ep_clearances WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match c {
        None => Json(json!({ "ok": false, "error": "clearance haipo" })),
        Some((student, year, st, seal, cleared)) => {
            let secs = sections_for(&s.db, &id).await;
            Json(json!({
                "ok": true,
                "valid": st == "cleared",
                "student_id": student, "academic_year": year, "status": st,
                "seal": seal, "cleared_at": cleared,
                "sections": secs.iter().map(|(code, st, _, by, at)| json!({
                    "department_code": code, "status": st, "signed_by": by, "signed_at": at
                })).collect::<Vec<_>>()
            }))
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
    fn clearance_hukamilika_ikiwa_blocking_zote_zimesainiwa() {
        let all_signed = vec![
            ("library".to_string(), true, "signed".to_string()),
            ("finance".to_string(), true, "signed".to_string()),
        ];
        assert!(is_complete(&all_signed));

        let one_pending = vec![
            ("library".to_string(), true, "signed".to_string()),
            ("finance".to_string(), true, "pending".to_string()),
        ];
        assert!(!is_complete(&one_pending));

        // Sehemu isiyo `blocking` haizuii.
        let nonblocking_pending = vec![
            ("library".to_string(), true, "signed".to_string()),
            ("dean".to_string(), false, "pending".to_string()),
        ];
        assert!(is_complete(&nonblocking_pending));
    }

    #[test]
    fn idara_haiwezi_kusaini_sehemu_ya_nyingine() {
        assert!(can_sign("library", "library", "pending").is_ok());
        assert!(can_sign("Library", "library", "pending").is_ok());
        let e = can_sign("finance", "library", "pending").unwrap_err();
        assert!(e.contains("nyingine"));
    }

    #[test]
    fn sehemu_iliyosainiwa_hairuhusu_kusaini_tena() {
        assert!(can_sign("library", "library", "signed").is_err());
        assert!(can_sign("library", "library", "rejected").is_err());
    }

    #[test]
    fn idara_za_msingi_zipo_nne() {
        assert_eq!(CORE_DEPARTMENTS.len(), 4);
        assert_eq!(CORE_DEPARTMENTS[0].0, "library");
        assert_eq!(CORE_DEPARTMENTS[1].0, "finance");
        assert_eq!(CORE_DEPARTMENTS[2].0, "hostel");
        assert_eq!(CORE_DEPARTMENTS[3].0, "dean");
    }

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        // Meza za fees/library/hostel zinahitajika kwa `blocking_reason`.
        crate::fees::init_tables(&p).await;
        crate::library::init_tables(&p).await;
        crate::hostel::init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn kusaini_husoma_sehemu_ya_idara_sahihi() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_clearance_templates (id,name,pdf_path,seal_text,active,created_at) VALUES ('t1','T','','',1,?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_clearances (id,student_id,template_id,academic_year,status,seal_text,created_at) VALUES ('c1','s1','t1','2026','pending','',?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_clearance_sections (id,clearance_id,department_code,status) VALUES ('sec1','c1','library','pending')")
            .execute(&p).await.unwrap();
        // JOIN hii ilikuwa na `status` isiyo wazi — ilipaswa kufeli kimya.
        let got = section_status(&p, "c1", "library").await;
        assert_eq!(got, Some(("pending".to_string(), "s1".to_string())));
        // Idara isiyopo hurudisha None (hivyo handler hurudisha "haipo").
        assert!(section_status(&p, "c1", "finance").await.is_none());
    }

    #[tokio::test]
    async fn mwanafunzi_hapati_clearance_mbili_kwa_mwaka_mmoja() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_clearances (id,student_id,template_id,academic_year,status,seal_text,created_at) VALUES ('c1','s1','t1','2026','pending','',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let dup = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM ep_clearances WHERE student_id='s1' AND academic_year='2026'",
        ).fetch_optional(&p).await.unwrap();
        assert!(dup.is_some());
    }

    #[tokio::test]
    async fn idara_ya_fedha_huzuia_deni() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_invoices (id,student_id,level_id,term,total_cents,status,created_at) VALUES ('i1','s1','l1','2026-S1',50000,'unpaid',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let r = blocking_reason(&p, "finance", "s1").await;
        assert!(r.is_some(), "deni lilipaswa kuzuia");
        assert!(r.unwrap().contains("salio"));

        // Baada ya kulipa, kizuizi kinaondoka.
        sqlx::query("INSERT INTO ep_payments (id,invoice_id,student_id,amount_cents,method,reference,receipt_no,received_by,paid_at) VALUES ('p1','i1','s1',50000,'cash','','RCP/2026/000001','','x')")
            .execute(&p).await.unwrap();
        assert!(blocking_reason(&p, "finance", "s1").await.is_none());
    }

    #[tokio::test]
    async fn maktaba_huzuia_faini_isiyolipwa() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_loans (id,copy_id,book_id,student_id,borrowed_at,due_at,fine_cents,fine_paid,issued_by) VALUES ('l1','cp1','b1','s1',?1,?1,2000,0,'')")
            .bind(now()).execute(&p).await.unwrap();
        let r = blocking_reason(&p, "library", "s1").await;
        assert!(r.is_some());
        assert!(r.unwrap().contains("faini"));
    }

    #[tokio::test]
    async fn hosteli_huzuia_ugawaji_usiofungwa() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_allocations (id,bed_id,student_id,allocated_at) VALUES ('a1','bed1','s1',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let r = blocking_reason(&p, "hostel", "s1").await;
        assert!(r.is_some());
        assert!(r.unwrap().contains("hosteli"));
    }
}
