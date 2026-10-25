//! library.rs — ELIMUPRO Awamu 9: maktaba (moduli 13).
//!
//! Mtiririko: vitabu (`ep_books`) → nakala (`ep_book_copies`) → kukopa
//! (`ep_loans`) → kurudisha. Faini hupimwa kwa siku zilizochelewa.
//!
//! Kanuni halisi:
//!   - Nakala iliyokopwa haiwezi kukopwa tena hadi irudishwe.
//!   - Mwanafunzi hawezi kuwa na mikopo zaidi ya `MAX_LOANS_PER_STUDENT`.
//!   - Faini = siku zilizochelewa × `FINE_PER_DAY_CENTS` (senti, i64).
//!   - Kurudisha huhifadhi `returned_at` na hali ya faini.

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Kikomo cha mikopo kwa mwanafunzi mmoja.
pub const MAX_LOANS_PER_STUDENT: i64 = 3;
/// Faini kwa siku ya ucheleweshaji (senti).
pub const FINE_PER_DAY_CENTS: i64 = 20_000; // 200.00
/// Siku za kukopa kwa kawaida.
pub const LOAN_DAYS: i64 = 14;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS ep_books (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT '',
            author TEXT NOT NULL DEFAULT '',
            isbn TEXT NOT NULL DEFAULT '',
            category TEXT NOT NULL DEFAULT '',
            copies INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_book_copies (
            id TEXT PRIMARY KEY,
            book_id TEXT NOT NULL,
            barcode TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'available', -- available|loaned|lost
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_loans (
            id TEXT PRIMARY KEY,
            copy_id TEXT NOT NULL,
            book_id TEXT NOT NULL DEFAULT '',
            student_id TEXT NOT NULL,
            borrowed_at TEXT NOT NULL,
            due_at TEXT NOT NULL,
            returned_at TEXT,
            fine_cents INTEGER NOT NULL DEFAULT 0,
            fine_paid INTEGER NOT NULL DEFAULT 0,
            issued_by TEXT NOT NULL DEFAULT ''
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/library/status", get(status))
        .route("/api/library/books", get(books_list))
        .route("/api/library/book/add", post(book_add))
        .route("/api/library/book/delete", post(book_delete))
        .route("/api/library/book/:id", get(book_get))
        .route("/api/library/borrow", post(borrow))
        .route("/api/library/return", post(return_book))
        .route("/api/library/loans", get(loans_list))
        .route("/api/library/overdue", get(overdue_list))
        .route("/api/library/fine/pay", post(fine_pay))
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

/// Tarehe (YYYY-MM-DD) baada ya siku `days` kuanzia leo.
pub fn due_date_from(days: i64) -> String {
    (chrono::Local::now() + chrono::Duration::days(days))
        .format("%Y-%m-%d")
        .to_string()
}

/// Siku zilizochelewa kati ya `due` (YYYY-MM-DD) na `today` (YYYY-MM-DD).
pub fn days_late(due: &str, today: &str) -> i64 {
    let parse = |s: &str| chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok();
    match (parse(due), parse(today)) {
        (Some(d), Some(t)) => (t - d).num_days().max(0),
        _ => 0,
    }
}

/// Faini (senti) kwa siku zilizochelewa.
pub fn fine_for(late_days: i64) -> i64 {
    late_days.max(0) * FINE_PER_DAY_CENTS
}

pub fn fmt_money(cents: i64) -> String {
    let c = cents.max(0);
    let s = format!("{}.{:02}", c / 100, c % 100);
    let (int_part, dec) = s.split_once('.').unwrap_or((s.as_str(), "00"));
    let chars: Vec<char> = int_part.chars().collect();
    let mut out = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    format!("{out}.{dec}")
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (on_loan,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_loans WHERE returned_at IS NULL")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (fines,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(fine_cents),0) FROM ep_loans WHERE fine_paid=0 AND fine_cents>0",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "books": table_count(&s.db, "ep_books").await,
        "copies": table_count(&s.db, "ep_book_copies").await,
        "loans": table_count(&s.db, "ep_loans").await,
        "on_loan": on_loan,
        "outstanding_fines_cents": fines,
        "outstanding_fines": fmt_money(fines),
        "max_loans": MAX_LOANS_PER_STUDENT,
        "fine_per_day_cents": FINE_PER_DAY_CENTS,
        "loan_days": LOAN_DAYS
    }))
}

// ============================================================
// VITABU
// ============================================================

#[derive(Deserialize)]
pub struct BookIn {
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub isbn: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub copies: Option<i64>,
}

pub async fn book_add(State(s): State<AppState>, Json(b): Json<BookIn>) -> Json<Value> {
    if b.title.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la kitabu linahitajika" }));
    }
    let n = b.copies.unwrap_or(1).clamp(1, 500);
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_books (id,title,author,isbn,category,copies,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(b.title.trim())
    .bind(b.author.trim())
    .bind(b.isbn.trim())
    .bind(b.category.trim())
    .bind(n)
    .bind(now())
    .execute(&s.db)
    .await;

    // Tengeneza nakala, kila moja na barcode ya kipekee.
    let mut codes = Vec::new();
    for i in 1..=n {
        let code = format!("BK-{}-{:04}", id.chars().take(6).collect::<String>(), i);
        let _ = sqlx::query(
            "INSERT INTO ep_book_copies (id,book_id,barcode,status,created_at)
             VALUES (?1,?2,?3,'available',?4)",
        )
        .bind(uid())
        .bind(&id)
        .bind(&code)
        .bind(now())
        .execute(&s.db)
        .await;
        codes.push(code);
    }
    Json(json!({ "ok": true, "id": id, "copies": codes }))
}

pub async fn books_list(State(s): State<AppState>, Query(q): Query<LibQuery>) -> Json<Value> {
    let like = format!("%{}%", q.search.trim());
    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, i64)>(
        "SELECT b.id,b.title,b.author,b.isbn,b.category,b.copies,
                (SELECT COUNT(*) FROM ep_book_copies c WHERE c.book_id=b.id AND c.status='available')
         FROM ep_books b
         WHERE (?1='' OR b.title LIKE ?2 OR b.author LIKE ?2 OR b.isbn LIKE ?2)
         ORDER BY b.title",
    )
    .bind(&q.search)
    .bind(&like)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "books": rows.into_iter().map(|r| json!({
            "id": r.0, "title": r.1, "author": r.2, "isbn": r.3, "category": r.4,
            "copies": r.5, "available": r.6
        })).collect::<Vec<_>>()
    }))
}

pub async fn book_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let b = sqlx::query_as::<_, (String, String, String, String, String, i64)>(
        "SELECT id,title,author,isbn,category,copies FROM ep_books WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((bid, title, author, isbn, cat, copies)) = b else {
        return Json(json!({ "ok": false, "error": "kitabu hakipo" }));
    };
    let cs = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,barcode,status FROM ep_book_copies WHERE book_id=?1 ORDER BY barcode",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "book": { "id": bid, "title": title, "author": author, "isbn": isbn, "category": cat, "copies": copies },
        "items": cs.into_iter().map(|c| json!({ "id": c.0, "barcode": c.1, "status": c.2 })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct IdIn {
    pub id: String,
}

pub async fn book_delete(State(s): State<AppState>, Json(b): Json<IdIn>) -> Json<Value> {
    let (on_loan,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_loans WHERE book_id=?1 AND returned_at IS NULL")
            .bind(&b.id)
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    if on_loan > 0 {
        return Json(json!({ "ok": false, "error": "kitabu kina nakala zilizokopwa" }));
    }
    let _ = sqlx::query("DELETE FROM ep_book_copies WHERE book_id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    let _ = sqlx::query("DELETE FROM ep_books WHERE id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true }))
}

// ============================================================
// KUKOPA NA KURUDISHA
// ============================================================

#[derive(Deserialize)]
pub struct BorrowIn {
    pub student_id: String,
    #[serde(default)]
    pub book_id: String,
    #[serde(default)]
    pub barcode: String,
    #[serde(default)]
    pub issued_by: String,
}

pub async fn borrow(State(s): State<AppState>, Json(b): Json<BorrowIn>) -> Json<Value> {
    // Tafuta nakala inayopatikana (kwa barcode au kwa kitabu).
    let copy = if !b.barcode.trim().is_empty() {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT id,book_id,status FROM ep_book_copies WHERE barcode=?1",
        )
        .bind(b.barcode.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    } else {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT id,book_id,status FROM ep_book_copies WHERE book_id=?1 AND status='available' LIMIT 1",
        )
        .bind(&b.book_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    };
    let Some((copy_id, book_id, cstatus)) = copy else {
        return Json(json!({ "ok": false, "error": "nakala haipo" }));
    };
    if cstatus != "available" {
        return Json(json!({ "ok": false, "error": "nakala imekopwa tayari", "status": cstatus }));
    }

    // Mwanafunzi yupo?
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM ep_students WHERE id=?1")
        .bind(&b.student_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    if exists.is_none() {
        return Json(json!({ "ok": false, "error": "mwanafunzi hayupo" }));
    }

    // Kikomo cha mikopo.
    let (active,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_loans WHERE student_id=?1 AND returned_at IS NULL")
            .bind(&b.student_id)
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    if active >= MAX_LOANS_PER_STUDENT {
        return Json(json!({
            "ok": false, "error": "kikomo cha mikopo kimefikiwa",
            "max": MAX_LOANS_PER_STUDENT
        }));
    }
    // Faini isiyolipwa huzuia kukopa.
    let (unpaid,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(fine_cents),0) FROM ep_loans WHERE student_id=?1 AND fine_paid=0",
    )
    .bind(&b.student_id)
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    if unpaid > 0 {
        return Json(json!({
            "ok": false, "error": "faini isiyolipwa", "fine_cents": unpaid, "fine": fmt_money(unpaid)
        }));
    }

    let id = uid();
    let due = due_date_from(LOAN_DAYS);
    let _ = sqlx::query(
        "INSERT INTO ep_loans (id,copy_id,book_id,student_id,borrowed_at,due_at,fine_cents,fine_paid,issued_by)
         VALUES (?1,?2,?3,?4,?5,?6,0,0,?7)",
    )
    .bind(&id)
    .bind(&copy_id)
    .bind(&book_id)
    .bind(&b.student_id)
    .bind(now())
    .bind(&due)
    .bind(&b.issued_by)
    .execute(&s.db)
    .await;
    let _ = sqlx::query("UPDATE ep_book_copies SET status='loaned' WHERE id=?1")
        .bind(&copy_id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "loan_id": id, "due_at": due, "loan_days": LOAN_DAYS }))
}

#[derive(Deserialize)]
pub struct ReturnIn {
    pub loan_id: String,
    #[serde(default)]
    pub today: String,
}

pub async fn return_book(State(s): State<AppState>, Json(b): Json<ReturnIn>) -> Json<Value> {
    let loan = sqlx::query_as::<_, (String, String, String, Option<String>)>(
        "SELECT copy_id,due_at,student_id,returned_at FROM ep_loans WHERE id=?1",
    )
    .bind(&b.loan_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((copy_id, due_at, _student, returned)) = loan else {
        return Json(json!({ "ok": false, "error": "mkopo haupo" }));
    };
    if returned.is_some() {
        return Json(json!({ "ok": false, "error": "mkopo umerudishwa tayari" }));
    }
    let today = if b.today.trim().is_empty() {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    } else {
        b.today.trim().to_string()
    };
    let late = days_late(&due_at, &today);
    let fine = fine_for(late);
    let _ = sqlx::query("UPDATE ep_loans SET returned_at=?1, fine_cents=?2 WHERE id=?3")
        .bind(now())
        .bind(fine)
        .bind(&b.loan_id)
        .execute(&s.db)
        .await;
    let _ = sqlx::query("UPDATE ep_book_copies SET status='available' WHERE id=?1")
        .bind(&copy_id)
        .execute(&s.db)
        .await;
    Json(json!({
        "ok": true, "late_days": late, "fine_cents": fine, "fine": fmt_money(fine),
        "fine_due": fine > 0
    }))
}

#[derive(Deserialize)]
pub struct LibQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub active: Option<bool>,
}

pub async fn loans_list(State(s): State<AppState>, Query(q): Query<LibQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, i64, i64)>(
        "SELECT l.id,COALESCE(b.title,''),COALESCE(st.full_name,''),COALESCE(st.reg_no,''),
                l.borrowed_at,l.due_at,l.returned_at,l.fine_cents,l.fine_paid
         FROM ep_loans l
         LEFT JOIN ep_books b ON b.id=l.book_id
         LEFT JOIN ep_students st ON st.id=l.student_id
         WHERE (?1='' OR l.student_id=?1)
           AND (?2=0 OR (l.returned_at IS NULL) = ?2)
         ORDER BY l.borrowed_at DESC LIMIT 500",
    )
    .bind(&q.student_id)
    .bind(if q.active.unwrap_or(false) { 1 } else { 0 })
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    Json(json!({
        "ok": true,
        "loans": rows.into_iter().map(|r| {
            let late = if r.6.is_none() { days_late(&r.5, &today) } else { 0 };
            json!({
                "id": r.0, "book": r.1, "student": r.2, "reg_no": r.3,
                "borrowed_at": r.4, "due_at": r.5, "returned_at": r.6,
                "active": r.6.is_none(), "late_days": late,
                "fine_cents": r.7, "fine": fmt_money(r.7), "fine_paid": r.8 == 1,
                "projected_fine_cents": fine_for(late)
            })
        }).collect::<Vec<_>>()
    }))
}

pub async fn overdue_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
        "SELECT l.id,COALESCE(b.title,''),COALESCE(st.full_name,''),COALESCE(st.reg_no,''),
                l.due_at,l.student_id
         FROM ep_loans l
         LEFT JOIN ep_books b ON b.id=l.book_id
         LEFT JOIN ep_students st ON st.id=l.student_id
         WHERE l.returned_at IS NULL ORDER BY l.due_at",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let overdue: Vec<Value> = rows
        .into_iter()
        .filter_map(|r| {
            let late = days_late(&r.4, &today);
            if late <= 0 {
                return None;
            }
            Some(json!({
                "loan_id": r.0, "book": r.1, "student": r.2, "reg_no": r.3,
                "student_id": r.5, "due_at": r.4, "late_days": late,
                "fine_cents": fine_for(late), "fine": fmt_money(fine_for(late))
            }))
        })
        .collect();
    let total: i64 = overdue
        .iter()
        .filter_map(|v| v["fine_cents"].as_i64())
        .sum();
    Json(json!({
        "ok": true, "count": overdue.len(),
        "total_fine_cents": total, "total_fine": fmt_money(total),
        "overdue": overdue
    }))
}

#[derive(Deserialize)]
pub struct FinePay {
    pub loan_id: String,
}

pub async fn fine_pay(State(s): State<AppState>, Json(b): Json<FinePay>) -> Json<Value> {
    let r = sqlx::query_as::<_, (i64, i64)>("SELECT fine_cents,fine_paid FROM ep_loans WHERE id=?1")
        .bind(&b.loan_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    match r {
        None => Json(json!({ "ok": false, "error": "mkopo haupo" })),
        Some((0, _)) => Json(json!({ "ok": false, "error": "hakuna faini" })),
        Some((cents, 1)) => Json(json!({ "ok": false, "error": "faini imelipwa tayari", "fine_cents": cents })),
        Some((cents, _)) => {
            let _ = sqlx::query("UPDATE ep_loans SET fine_paid=1 WHERE id=?1")
                .bind(&b.loan_id)
                .execute(&s.db)
                .await;
            Json(json!({ "ok": true, "paid_cents": cents, "paid": fmt_money(cents) }))
        }
    }
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS ep_students (
                id TEXT PRIMARY KEY, full_name TEXT NOT NULL DEFAULT '', reg_no TEXT NOT NULL DEFAULT '')",
        )
        .execute(&p)
        .await;
        init_tables(&p).await;
        p
    }

    #[test]
    fn siku_zilizochelewa_hupimwa() {
        assert_eq!(days_late("2026-10-01", "2026-10-05"), 4);
        assert_eq!(days_late("2026-10-05", "2026-10-05"), 0);
        // Kurudisha mapema si "ucheleweshaji" hasi.
        assert_eq!(days_late("2026-10-10", "2026-10-05"), 0);
        assert_eq!(days_late("tarehe-mbovu", "2026-10-05"), 0);
    }

    #[test]
    fn faini_hupimwa_kwa_siku() {
        assert_eq!(fine_for(0), 0);
        assert_eq!(fine_for(1), 20_000);
        assert_eq!(fine_for(5), 100_000);
        assert_eq!(fmt_money(fine_for(5)), "1,000.00");
        // Siku hasi hazizalishi faini hasi.
        assert_eq!(fine_for(-3), 0);
    }

    #[test]
    fn tarehe_ya_kurejesha_ni_siku_14_mbele() {
        let d = due_date_from(LOAN_DAYS);
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(days_late(&d, &today), 0, "siku ya kurejesha haipaswi kuwa imepita");
        let expected = (chrono::Local::now() + chrono::Duration::days(14))
            .format("%Y-%m-%d")
            .to_string();
        assert_eq!(d, expected);
    }

    #[test]
    fn fedha_huonyeshwa_kwa_maelfu() {
        assert_eq!(fmt_money(0), "0.00");
        assert_eq!(fmt_money(20_000), "200.00");
        assert_eq!(fmt_money(123_456_78), "123,456.78");
    }

    #[tokio::test]
    async fn kitabu_hutengeneza_nakala_kwa_barcode_ya_kipekee() {
        let p = db().await;
        let bid = uid();
        sqlx::query("INSERT INTO ep_books (id,title,author,copies,created_at) VALUES (?1,'Fizikia','Mwl','3',?2)")
            .bind(&bid).bind(now()).execute(&p).await.unwrap();
        for i in 1..=3 {
            let code = format!("BK-{}-{:04}", bid.chars().take(6).collect::<String>(), i);
            sqlx::query("INSERT INTO ep_book_copies (id,book_id,barcode,status,created_at) VALUES (?1,?2,?3,'available',?4)")
                .bind(uid()).bind(&bid).bind(&code).bind(now()).execute(&p).await.unwrap();
        }
        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM ep_book_copies WHERE book_id=?1").bind(&bid)
                .fetch_one(&p).await.unwrap();
        let (distinct,): (i64,) =
            sqlx::query_as("SELECT COUNT(DISTINCT barcode) FROM ep_book_copies WHERE book_id=?1").bind(&bid)
                .fetch_one(&p).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(distinct, 3, "barcode zilirudiwa");
    }

    #[tokio::test]
    async fn nakala_iliyokopwa_haiwezi_kukopwa_tena() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_students (id,full_name,reg_no) VALUES ('st1','Asha','R1')")
            .execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_book_copies (id,book_id,barcode,status,created_at) VALUES ('c1','b1','BK-1','available',?1)")
            .bind(now()).execute(&p).await.unwrap();
        // Mkopo wa kwanza.
        sqlx::query("INSERT INTO ep_loans (id,copy_id,book_id,student_id,borrowed_at,due_at,fine_cents,fine_paid) VALUES ('l1','c1','b1','st1',?1,?2,0,0)")
            .bind(now()).bind(due_date_from(14)).execute(&p).await.unwrap();
        sqlx::query("UPDATE ep_book_copies SET status='loaned' WHERE id='c1'").execute(&p).await.unwrap();
        // Jaribio la kukopa tena linapaswa kushindwa (nakala si 'available').
        let avail: Option<(String,)> =
            sqlx::query_as("SELECT id FROM ep_book_copies WHERE id='c1' AND status='available'")
                .fetch_optional(&p).await.unwrap();
        assert!(avail.is_none(), "nakala iliyokopwa ilionekana kama inapatikana");
        // Baada ya kurudisha, inapatikana tena.
        sqlx::query("UPDATE ep_book_copies SET status='available' WHERE id='c1'").execute(&p).await.unwrap();
        let avail2: Option<(String,)> =
            sqlx::query_as("SELECT id FROM ep_book_copies WHERE id='c1' AND status='available'")
                .fetch_optional(&p).await.unwrap();
        assert!(avail2.is_some());
    }

    #[tokio::test]
    async fn kikomo_cha_mikopo_huzuiwa() {
        let p = db().await;
        for i in 1..=4 {
            sqlx::query("INSERT INTO ep_loans (id,copy_id,book_id,student_id,borrowed_at,due_at,fine_cents,fine_paid) VALUES (?1,?2,'b','st1',?3,?4,0,0)")
                .bind(format!("l{i}")).bind(format!("c{i}")).bind(now()).bind(due_date_from(14))
                .execute(&p).await.unwrap();
        }
        let (active,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM ep_loans WHERE student_id='st1' AND returned_at IS NULL")
                .fetch_one(&p).await.unwrap();
        assert!(active >= MAX_LOANS_PER_STUDENT, "kikomo hakikugunduliwa");
        assert_eq!(MAX_LOANS_PER_STUDENT, 3);
    }

    #[tokio::test]
    async fn faini_isiyolipwa_huzuia_kukopa() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_loans (id,copy_id,book_id,student_id,borrowed_at,due_at,returned_at,fine_cents,fine_paid) VALUES ('l1','c1','b1','st1',?1,?2,?3,50000,0)")
            .bind(now()).bind("2026-10-01").bind(now()).execute(&p).await.unwrap();
        let (unpaid,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(fine_cents),0) FROM ep_loans WHERE student_id='st1' AND fine_paid=0",
        ).fetch_one(&p).await.unwrap();
        assert_eq!(unpaid, 50_000);
        // Baada ya kulipa, hakuna kizuizi.
        sqlx::query("UPDATE ep_loans SET fine_paid=1 WHERE id='l1'").execute(&p).await.unwrap();
        let (unpaid2,): (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(fine_cents),0) FROM ep_loans WHERE student_id='st1' AND fine_paid=0",
        ).fetch_one(&p).await.unwrap();
        assert_eq!(unpaid2, 0);
    }
}
