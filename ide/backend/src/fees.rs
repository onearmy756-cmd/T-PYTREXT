//! fees.rs — ELIMUPRO Awamu 8: ada, ankara na malipo (moduli 12).
//!
//! Mtiririko halisi wa fedha za chuo:
//!   1. Aina za ada (`ep_fee_items`) — mfano "Ada ya Muhula", "Bima", "Hosteli".
//!   2. Ankara (`ep_invoices`) — hutengenezwa kwa mwanafunzi kwa muhula; huchukua
//!      aina zote za ada za level yake (au zilizoainishwa).
//!   3. Malipo (`ep_payments`) — kila malipo hupata namba ya risiti.
//!   4. Salio = jumla − malipo. Hali: `unpaid` | `partial` | `paid`.
//!
//! Fedha huhifadhiwa kwa SENTI (integers) — kuepuka makosa ya `f64`.
//! Kila malipo hupata `receipt_no` ya kipekee (`RCP/YYYY/NNNNNN`).

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
        r#"CREATE TABLE IF NOT EXISTS ep_fee_items (
            id TEXT PRIMARY KEY,
            level_id TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL DEFAULT '',
            code TEXT NOT NULL DEFAULT '',
            amount_cents INTEGER NOT NULL DEFAULT 0,
            mandatory INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_invoices (
            id TEXT PRIMARY KEY,
            student_id TEXT NOT NULL,
            level_id TEXT NOT NULL DEFAULT '',
            term TEXT NOT NULL DEFAULT '',
            total_cents INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'unpaid',
            created_at TEXT NOT NULL,
            UNIQUE(student_id, term)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_invoice_items (
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL,
            fee_item_id TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL DEFAULT '',
            amount_cents INTEGER NOT NULL DEFAULT 0
        )"#,
        r#"CREATE TABLE IF NOT EXISTS ep_payments (
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL,
            student_id TEXT NOT NULL DEFAULT '',
            amount_cents INTEGER NOT NULL DEFAULT 0,
            method TEXT NOT NULL DEFAULT 'cash',
            reference TEXT NOT NULL DEFAULT '',
            receipt_no TEXT NOT NULL UNIQUE,
            received_by TEXT NOT NULL DEFAULT '',
            paid_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/fees/status", get(status))
        .route("/api/fees/items", get(items_list))
        .route("/api/fees/items/add", post(item_add))
        .route("/api/fees/items/delete", post(item_delete))
        .route("/api/fees/invoice/create", post(invoice_create))
        .route("/api/fees/invoices", get(invoices_list))
        .route("/api/fees/invoice/:id", get(invoice_get))
        .route("/api/fees/pay", post(pay))
        .route("/api/fees/receipt/:id", get(receipt))
        .route("/api/fees/summary", get(summary))
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

/// Hali ya ankara kutokana na jumla na malipo. `total`/`paid` ni senti.
pub fn invoice_status(total_cents: i64, paid_cents: i64) -> &'static str {
    if total_cents <= 0 || paid_cents >= total_cents {
        "paid"
    } else if paid_cents > 0 {
        "partial"
    } else {
        "unpaid"
    }
}

/// Hugeuza kiasi cha maandishi kuwa senti. Hukataa namba hasi.
pub fn to_cents(s: &str) -> Option<i64> {
    let t = s.trim().replace(',', "");
    let v: f64 = t.parse().ok()?;
    if v.is_finite() && v >= 0.0 {
        Some((v * 100.0).round() as i64)
    } else {
        None
    }
}

pub fn fmt_money(cents: i64) -> String {
    let neg = cents < 0;
    let c = cents.abs();
    let s = format!("{}.{:02}", c / 100, c % 100);
    // Tenganisha maelfu kwa koma.
    let (int_part, dec) = s.split_once('.').unwrap_or((s.as_str(), "00"));
    let mut out = String::new();
    let chars: Vec<char> = int_part.chars().collect();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    format!("{}{}.{}", if neg { "-" } else { "" }, out, dec)
}

/// Namba ya risiti: `RCP/YYYY/NNNNNN`.
pub fn receipt_no(year: i32, seq: i64) -> String {
    format!("RCP/{year}/{:06}", seq.max(1))
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (billed,): (i64,) = sqlx::query_as("SELECT COALESCE(SUM(total_cents),0) FROM ep_invoices")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (collected,): (i64,) = sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "items": table_count(&s.db, "ep_fee_items").await,
        "invoices": table_count(&s.db, "ep_invoices").await,
        "payments": table_count(&s.db, "ep_payments").await,
        "billed": billed,
        "collected": collected,
        "outstanding": (billed - collected).max(0),
    }))
}

// ============================================================
// AINA ZA ADA
// ============================================================

#[derive(Deserialize)]
pub struct ItemIn {
    #[serde(default)]
    pub level_id: String,
    pub name: String,
    #[serde(default)]
    pub code: String,
    pub amount: String,
    #[serde(default)]
    pub mandatory: Option<bool>,
}

pub async fn item_add(State(s): State<AppState>, Json(b): Json<ItemIn>) -> Json<Value> {
    let Some(cents) = to_cents(&b.amount) else {
        return Json(json!({ "ok": false, "error": "kiasi si sahihi" }));
    };
    if b.name.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "jina la ada linahitajika" }));
    }
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_fee_items (id,level_id,name,code,amount_cents,mandatory,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
    )
    .bind(&id)
    .bind(&b.level_id)
    .bind(b.name.trim())
    .bind(b.code.trim())
    .bind(cents)
    .bind(if b.mandatory.unwrap_or(true) { 1 } else { 0 })
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({ "ok": true, "id": id, "amount_cents": cents }))
}

pub async fn items_list(State(s): State<AppState>, Query(q): Query<FeesQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, String)>(
        "SELECT f.id,f.level_id,COALESCE(l.name,''),f.name,f.amount_cents,f.mandatory,f.code
         FROM ep_fee_items f LEFT JOIN ep_levels l ON l.id=f.level_id
         WHERE (?1='' OR f.level_id=?1) ORDER BY f.name",
    )
    .bind(&q.level_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "items": rows.into_iter().map(|r| json!({
            "id": r.0, "level_id": r.1, "level": r.2, "name": r.3,
            "amount_cents": r.4, "amount": fmt_money(r.4), "mandatory": r.5 == 1, "code": r.6
        })).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct IdIn {
    pub id: String,
}

pub async fn item_delete(State(s): State<AppState>, Json(b): Json<IdIn>) -> Json<Value> {
    let _ = sqlx::query("DELETE FROM ep_fee_items WHERE id=?1")
        .bind(&b.id)
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true }))
}

// ============================================================
// ANKARA
// ============================================================

#[derive(Deserialize)]
pub struct InvoiceIn {
    pub student_id: String,
    pub term: String,
    /// Kama tupu, huchukua aina zote za ada za level ya mwanafunzi.
    #[serde(default)]
    pub item_ids: Vec<String>,
}

pub async fn invoice_create(State(s): State<AppState>, Json(b): Json<InvoiceIn>) -> Json<Value> {
    if b.term.trim().is_empty() {
        return Json(json!({ "ok": false, "error": "muhula unahitajika" }));
    }
    let st = sqlx::query_as::<_, (String, String)>(
        "SELECT level_id,COALESCE(full_name,'') FROM ep_students WHERE id=?1",
    )
    .bind(&b.student_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((level_id, _name)) = st else {
        return Json(json!({ "ok": false, "error": "mwanafunzi hayupo" }));
    };

    // Ankara ya mwanafunzi + muhula huo ipo tayari?
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM ep_invoices WHERE student_id=?1 AND term=?2")
            .bind(&b.student_id)
            .bind(b.term.trim())
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten();
    if let Some((id,)) = existing {
        return Json(json!({ "ok": true, "id": id, "existing": true }));
    }

    let items = if b.item_ids.is_empty() {
        sqlx::query_as::<_, (String, String, i64)>(
            "SELECT id,name,amount_cents FROM ep_fee_items WHERE level_id=?1 AND mandatory=1",
        )
        .bind(&level_id)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
    } else {
        let mut v = Vec::new();
        for iid in &b.item_ids {
            if let Ok(Some((name, cents))) = sqlx::query_as::<_, (String, i64)>(
                "SELECT name,amount_cents FROM ep_fee_items WHERE id=?1",
            )
            .bind(iid)
            .fetch_optional(&s.db)
            .await
            {
                v.push((iid.clone(), name, cents));
            }
        }
        v
    };

    let id = uid();
    let total: i64 = items.iter().map(|i| i.2).sum();
    let _ = sqlx::query(
        "INSERT INTO ep_invoices (id,student_id,level_id,term,total_cents,status,created_at)
         VALUES (?1,?2,?3,?4,?5,'unpaid',?6)",
    )
    .bind(&id)
    .bind(&b.student_id)
    .bind(&level_id)
    .bind(b.term.trim())
    .bind(total)
    .bind(now())
    .execute(&s.db)
    .await;
    for (fid, name, cents) in &items {
        let _ = sqlx::query(
            "INSERT INTO ep_invoice_items (id,invoice_id,fee_item_id,name,amount_cents)
             VALUES (?1,?2,?3,?4,?5)",
        )
        .bind(uid())
        .bind(&id)
        .bind(fid)
        .bind(name)
        .bind(cents)
        .execute(&s.db)
        .await;
    }
    Json(json!({ "ok": true, "id": id, "total_cents": total, "lines": items.len() }))
}

#[derive(Deserialize)]
pub struct FeesQuery {
    #[serde(default)]
    pub level_id: String,
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub term: String,
    #[serde(default)]
    pub status: String,
}

pub async fn invoices_list(State(s): State<AppState>, Query(q): Query<FeesQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, i64, String)>(
        "SELECT i.id,i.student_id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),i.term,
                i.total_cents,
                (SELECT COALESCE(SUM(p.amount_cents),0) FROM ep_payments p WHERE p.invoice_id=i.id),
                i.status
         FROM ep_invoices i LEFT JOIN ep_students st ON st.id=i.student_id
         WHERE (?1='' OR i.level_id=?1) AND (?2='' OR i.student_id=?2) AND (?3='' OR i.term=?3)
         ORDER BY i.created_at DESC LIMIT 500",
    )
    .bind(&q.level_id)
    .bind(&q.student_id)
    .bind(&q.term)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut out = Vec::new();
    for r in rows {
        let paid = r.6;
        let status = invoice_status(r.5, paid);
        if !q.status.is_empty() && q.status != status {
            continue;
        }
        out.push(json!({
            "id": r.0, "student_id": r.1, "student": r.2, "reg_no": r.3, "term": r.4,
            "total_cents": r.5, "total": fmt_money(r.5), "paid_cents": paid,
            "paid": fmt_money(paid), "balance_cents": (r.5 - paid).max(0),
            "balance": fmt_money((r.5 - paid).max(0)), "status": status
        }));
    }
    Json(json!({ "ok": true, "invoices": out }))
}

pub async fn invoice_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let head = sqlx::query_as::<_, (String, String, String, String, String, i64)>(
        "SELECT i.id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),i.term,i.level_id,i.total_cents
         FROM ep_invoices i LEFT JOIN ep_students st ON st.id=i.student_id WHERE i.id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((iid, name, reg, term, level_id, total)) = head else {
        return Json(json!({ "ok": false, "error": "ankara haipo" }));
    };
    let lines = sqlx::query_as::<_, (String, i64)>(
        "SELECT name,amount_cents FROM ep_invoice_items WHERE invoice_id=?1",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let pays = sqlx::query_as::<_, (String, i64, String, String, String)>(
        "SELECT receipt_no,amount_cents,method,paid_at,reference FROM ep_payments
         WHERE invoice_id=?1 ORDER BY paid_at",
    )
    .bind(&id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let paid: i64 = pays.iter().map(|p| p.1).sum();
    Json(json!({
        "ok": true,
        "invoice": {
            "id": iid, "student": name, "reg_no": reg, "term": term, "level_id": level_id,
            "total_cents": total, "total": fmt_money(total), "paid_cents": paid,
            "paid": fmt_money(paid), "balance_cents": (total - paid).max(0),
            "balance": fmt_money((total - paid).max(0)),
            "status": invoice_status(total, paid),
            "lines": lines.into_iter().map(|l| json!({
                "name": l.0, "amount_cents": l.1, "amount": fmt_money(l.1)
            })).collect::<Vec<_>>(),
            "payments": pays.into_iter().map(|p| json!({
                "receipt_no": p.0, "amount_cents": p.1, "amount": fmt_money(p.1),
                "method": p.2, "paid_at": p.3, "reference": p.4
            })).collect::<Vec<_>>()
        }
    }))
}

// ============================================================
// MALIPO
// ============================================================

#[derive(Deserialize)]
pub struct PayIn {
    pub invoice_id: String,
    pub amount: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub received_by: String,
}

pub async fn pay(State(s): State<AppState>, Json(b): Json<PayIn>) -> Json<Value> {
    let Some(amount) = to_cents(&b.amount) else {
        return Json(json!({ "ok": false, "error": "kiasi si sahihi" }));
    };
    if amount <= 0 {
        return Json(json!({ "ok": false, "error": "kiasi lazima kiwe zaidi ya sifuri" }));
    }
    let inv = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT student_id,total_cents,term FROM ep_invoices WHERE id=?1",
    )
    .bind(&b.invoice_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((student_id, total, _term)) = inv else {
        return Json(json!({ "ok": false, "error": "ankara haipo" }));
    };
    let (already,): (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments WHERE invoice_id=?1")
            .bind(&b.invoice_id)
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    if already + amount > total {
        return Json(json!({
            "ok": false, "error": "malipo yanazidi salio",
            "balance_cents": total - already
        }));
    }

    // Namba ya risiti inayofuata.
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM ep_payments")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let year = chrono::Local::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let rcpt = receipt_no(year, seq);
    let method = match b.method.trim().to_lowercase().as_str() {
        "bank" | "benki" => "bank",
        "mobile" | "mpesa" | "tigo" | "airtel" => "mobile",
        "cheque" | "hundi" => "cheque",
        _ => "cash",
    };
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_payments (id,invoice_id,student_id,amount_cents,method,reference,receipt_no,received_by,paid_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
    )
    .bind(&id)
    .bind(&b.invoice_id)
    .bind(&student_id)
    .bind(amount)
    .bind(method)
    .bind(&b.reference)
    .bind(&rcpt)
    .bind(&b.received_by)
    .bind(now())
    .execute(&s.db)
    .await;

    let new_paid = already + amount;
    let _ = sqlx::query("UPDATE ep_invoices SET status=?1 WHERE id=?2")
        .bind(invoice_status(total, new_paid))
        .bind(&b.invoice_id)
        .execute(&s.db)
        .await;

    Json(json!({
        "ok": true, "payment_id": id, "receipt_no": rcpt, "amount_cents": amount,
        "status": invoice_status(total, new_paid), "balance_cents": total - new_paid
    }))
}

pub async fn receipt(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query_as::<_, (String, String, i64, String, String, String, String)>(
        "SELECT p.receipt_no,COALESCE(st.full_name,''),p.amount_cents,p.method,p.paid_at,
                COALESCE(p.reference,''),COALESCE(st.reg_no,'')
         FROM ep_payments p LEFT JOIN ep_students st ON st.id=p.student_id WHERE p.id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match r {
        None => Json(json!({ "ok": false, "error": "risiti haipo" })),
        Some((rcpt, name, cents, method, paid_at, reference, reg)) => Json(json!({
            "ok": true,
            "receipt": {
                "receipt_no": rcpt, "student": name, "reg_no": reg,
                "amount_cents": cents, "amount": fmt_money(cents),
                "method": method, "paid_at": paid_at, "reference": reference
            }
        })),
    }
}

// ============================================================
// MUHTASARI
// ============================================================

pub async fn summary(State(s): State<AppState>, Query(q): Query<FeesQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64)>(
        "SELECT i.student_id,COALESCE(st.full_name,''),COALESCE(st.reg_no,''),i.term,i.total_cents,
                (SELECT COALESCE(SUM(p.amount_cents),0) FROM ep_payments p WHERE p.invoice_id=i.id)
         FROM ep_invoices i LEFT JOIN ep_students st ON st.id=i.student_id
         WHERE (?1='' OR i.level_id=?1) AND (?2='' OR i.student_id=?2)
         ORDER BY st.full_name",
    )
    .bind(&q.level_id)
    .bind(&q.student_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let mut billed = 0i64;
    let mut collected = 0i64;
    let students: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            billed += r.4;
            collected += r.5;
            json!({
                "student_id": r.0, "student": r.1, "reg_no": r.2, "term": r.3,
                "total_cents": r.4, "paid_cents": r.5, "balance_cents": (r.4 - r.5).max(0),
                "status": invoice_status(r.4, r.5)
            })
        })
        .collect();
    Json(json!({
        "ok": true,
        "billed_cents": billed, "billed": fmt_money(billed),
        "collected_cents": collected, "collected": fmt_money(collected),
        "outstanding_cents": (billed - collected).max(0),
        "outstanding": fmt_money((billed - collected).max(0)),
        "collection_rate": if billed > 0 { (collected as f64 / billed as f64 * 100.0 * 10.0).round() / 10.0 } else { 0.0 },
        "students": students
    }))
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
                id TEXT PRIMARY KEY, level_id TEXT NOT NULL DEFAULT '',
                full_name TEXT NOT NULL DEFAULT '', reg_no TEXT NOT NULL DEFAULT '')",
        )
        .execute(&p)
        .await;
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS ep_levels (id TEXT PRIMARY KEY, name TEXT NOT NULL DEFAULT '')")
            .execute(&p)
            .await;
        init_tables(&p).await;
        p
    }

    #[test]
    fn fedha_hutafsiriwa_kuwa_senti() {
        assert_eq!(to_cents("1000"), Some(100_000));
        assert_eq!(to_cents("1000.50"), Some(100_050));
        assert_eq!(to_cents("1,500.25"), Some(150_025));
        assert_eq!(to_cents("0"), Some(0));
        assert_eq!(to_cents("-5"), None);
        assert_eq!(to_cents("abc"), None);
        assert_eq!(to_cents(""), None);
    }

    #[test]
    fn fedha_huonyeshwa_kwa_maelfu() {
        assert_eq!(fmt_money(100_000), "1,000.00");
        assert_eq!(fmt_money(150_025), "1,500.25");
        assert_eq!(fmt_money(99), "0.99");
        assert_eq!(fmt_money(0), "0.00");
        assert_eq!(fmt_money(1_234_567_89), "1,234,567.89");
    }

    #[test]
    fn hali_ya_ankara_ni_sahihi() {
        assert_eq!(invoice_status(100_000, 0), "unpaid");
        assert_eq!(invoice_status(100_000, 40_000), "partial");
        assert_eq!(invoice_status(100_000, 100_000), "paid");
        // Kulipa zaidi bado ni 'paid'.
        assert_eq!(invoice_status(100_000, 120_000), "paid");
        // Ankara ya sifuri inachukuliwa kama iliyolipwa.
        assert_eq!(invoice_status(0, 0), "paid");
    }

    #[test]
    fn namba_ya_risiti_ina_umbizo_sahihi() {
        assert_eq!(receipt_no(2026, 1), "RCP/2026/000001");
        assert_eq!(receipt_no(2026, 123456), "RCP/2026/123456");
        assert_eq!(receipt_no(2026, 0), "RCP/2026/000001");
    }

    #[tokio::test]
    async fn ankara_huchukua_ada_zote_za_level() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_students (id,level_id,full_name,reg_no) VALUES ('st1','lv1','Asha','R1')")
            .execute(&p).await.unwrap();
        for (n, amt, mand) in [("Ada ya Muhula", "1500", 1), ("Bima", "50.50", 1), ("Klabu", "20", 0)] {
            sqlx::query("INSERT INTO ep_fee_items (id,level_id,name,amount_cents,mandatory,created_at)
                         VALUES (?1,'lv1',?2,?3,?4,?5)")
                .bind(uid()).bind(n).bind(to_cents(amt).unwrap()).bind(mand).bind(now())
                .execute(&p).await.unwrap();
        }
        // Ni ada za lazima pekee zinazochukuliwa kiotomatiki.
        let items = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT id,name,amount_cents FROM ep_fee_items WHERE level_id='lv1' AND mandatory=1",
        )
        .fetch_all(&p)
        .await
        .unwrap();
        let total: i64 = items.iter().map(|i| i.2).sum();
        assert_eq!(items.len(), 2, "ada za lazima pekee");
        assert_eq!(total, 150_000 + 5_050);
        assert_eq!(fmt_money(total), "1,550.50");
    }

    #[tokio::test]
    async fn malipo_huzuia_kuzidi_salio() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_invoices (id,student_id,level_id,term,total_cents,status,created_at)
                     VALUES ('inv1','st1','lv1','2026/T1',100000,'unpaid',?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_payments (id,invoice_id,student_id,amount_cents,method,receipt_no,paid_at)
                     VALUES ('p1','inv1','st1',40000,'cash','RCP/2026/000001',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let (paid,): (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments WHERE invoice_id='inv1'")
                .fetch_one(&p).await.unwrap();
        let (total,): (i64,) =
            sqlx::query_as("SELECT total_cents FROM ep_invoices WHERE id='inv1'").fetch_one(&p).await.unwrap();
        // Malipo ya 70,000 yanatosha (40,000+70,000=110,000 > 100,000 → yanazidi).
        assert!(paid + 70_000 > total);
        // 60,000 yanakubaliwa.
        assert!(paid + 60_000 <= total);
        assert_eq!(invoice_status(total, paid + 60_000), "paid");
    }

    #[tokio::test]
    async fn risiti_haiwezi_kurudiwa() {
        let p = db().await;
        for id in ["p1", "p2"] {
            let res = sqlx::query(
                "INSERT INTO ep_payments (id,invoice_id,student_id,amount_cents,method,receipt_no,paid_at)
                 VALUES (?1,'inv1','st1',1000,'cash','RCP/2026/000001',?2)",
            )
            .bind(id).bind(now()).execute(&p).await;
            if id == "p2" {
                assert!(res.is_err(), "receipt_no UNIQUE haikufanya kazi");
            } else {
                assert!(res.is_ok());
            }
        }
    }

    #[tokio::test]
    async fn ankara_moja_kwa_mwanafunzi_na_muhula() {
        let p = db().await;
        for (id, term) in [("i1", "2026/T1"), ("i2", "2026/T1")] {
            let res = sqlx::query(
                "INSERT INTO ep_invoices (id,student_id,level_id,term,total_cents,status,created_at)
                 VALUES (?1,'st1','lv1',?2,1000,'unpaid',?3)",
            )
            .bind(id).bind(term).bind(now()).execute(&p).await;
            if id == "i2" {
                assert!(res.is_err(), "UNIQUE(student_id,term) haikufanya kazi");
            } else {
                assert!(res.is_ok());
            }
        }
    }
}
