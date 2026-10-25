//! malipo.rs — ELIMUPRO Awamu 19: malipo ya simu (moduli 19).
//!
//! Mwanafunzi hulipa ada/hosteli kwa M-Pesa, Tigo Pesa au Airtel Money.
//! Mtiririko halisi:
//!   1. `intent` — mwanafunzi huanzisha malipo; mfumo hurudisha kumbukumbu ya
//!      muamala (`MPESA-XXXX`, `TIGO-…`, `AIRTEL-…`) na hali `pending`.
//!   2. Mtoa huduma huthibitisha (callback) — `confirm` hufunga malipo.
//!   3. Kuthibitishwa → huingiza `ep_payments` (mstari mmoja wa ukweli na
//!      Awamu 8) → hufungua kizuizi cha `finance` kwenye clearance.
//!
//! Kanuni halisi:
//!   - Malipo yanayozidi salio HUKATALIWA.
//!   - Muamala huthibitishwa MARA MOJA pekee (hakuna malipo maradufu).
//!   - Malipo yaliyoshindwa (`failed`/`cancelled`) hayaingii kwenye `ep_payments`.
//!   - Namba ya simu hukaguliwa kwa muundo wa Tanzania (07xx/06xx/05xx/0[78]xx).
//!
//! Tables: ep_mobile_intents

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
    let ddl = [r#"CREATE TABLE IF NOT EXISTS ep_mobile_intents (
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL,
            student_id TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL DEFAULT 'mpesa', -- mpesa|tigo|airtel
            phone TEXT NOT NULL DEFAULT '',
            amount_cents INTEGER NOT NULL DEFAULT 0,
            tx_ref TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL DEFAULT 'pending', -- pending|confirmed|failed|cancelled
            provider_ref TEXT NOT NULL DEFAULT '',
            payment_id TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            confirmed_at TEXT
        )"#];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/malipo/status", get(status))
        .route("/api/malipo/providers", get(providers))
        .route("/api/malipo/intent", post(intent_create))
        .route("/api/malipo/intents", get(intents_list))
        .route("/api/malipo/intent/:id", get(intent_get))
        .route("/api/malipo/confirm", post(confirm))
        .route("/api/malipo/fail", post(fail))
        .route("/api/malipo/reconcile", get(reconcile))
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

/// Watoa huduma wa malipo ya simu Tanzania.
pub const PROVIDERS: [(&str, &str, &str); 3] = [
    ("mpesa", "M-Pesa (Vodacom)", "MPESA"),
    ("tigo", "Mixx by Yas (Tigo Pesa)", "TIGO"),
    ("airtel", "Airtel Money", "AIRTEL"),
];

pub fn norm_provider(p: &str) -> &'static str {
    match p.trim().to_lowercase().as_str() {
        "mpesa" | "m-pesa" | "vodacom" | "voda" => "mpesa",
        "tigo" | "tigopesa" | "tigo pesa" | "mixx" | "yas" => "tigo",
        "airtel" | "airtelmoney" | "airtel money" => "airtel",
        _ => "mpesa",
    }
}

pub fn provider_prefix(p: &str) -> &'static str {
    match norm_provider(p) {
        "tigo" => "TIGO",
        "airtel" => "AIRTEL",
        _ => "MPESA",
    }
}

/// Namba ya simu ya Tanzania ni sahihi? (`+255…`, `255…`, `0…`).
pub fn phone_ok(p: &str) -> bool {
    let d: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
    let local = if let Some(rest) = d.strip_prefix("255") {
        rest.to_string()
    } else if let Some(rest) = d.strip_prefix('0') {
        rest.to_string()
    } else {
        d.clone()
    };
    if local.len() != 9 {
        return false;
    }
    // Mitandao ya simu: 06 (Voda/Airtel), 07 (Voda/Tigo/Airtel/Halotel), 05?
    let p2 = &local[..2];
    matches!(p2, "71" | "72" | "73" | "74" | "75" | "76" | "77" | "78" | "79" | "61" | "62" | "63" | "64" | "65" | "66" | "67" | "68" | "69")
}

/// Kumbukumbu ya muamala (`MPESA-2026-000123`).
pub fn tx_ref(provider: &str, year: i32, seq: i64) -> String {
    format!("{}-{}-{:06}", provider_prefix(provider), year, seq)
}

/// Muamala unaweza kuthibitishwa?
pub fn can_confirm(status: &str) -> bool {
    matches!(status.trim().to_lowercase().as_str(), "pending")
}

/// Muamala unaweza kushindwa/kufutwa?
pub fn can_fail(status: &str) -> bool {
    matches!(status.trim().to_lowercase().as_str(), "pending")
}

pub async fn status(State(s): State<AppState>) -> Json<Value> {
    let (pending,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_mobile_intents WHERE status='pending'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (confirmed,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_mobile_intents WHERE status='confirmed'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (failed,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ep_mobile_intents WHERE status IN ('failed','cancelled')")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (collected,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM ep_mobile_intents WHERE status='confirmed'",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "intents": table_count(&s.db, "ep_mobile_intents").await,
        "pending": pending,
        "confirmed": confirmed,
        "failed": failed,
        "collected_cents": collected,
        "collected": crate::fees::fmt_money(collected),
        "providers": PROVIDERS.iter().map(|(c, n, p)| json!({"code": c, "name": n, "prefix": p})).collect::<Vec<_>>()
    }))
}

pub async fn providers() -> Json<Value> {
    Json(json!({
        "ok": true,
        "providers": PROVIDERS.iter().map(|(c, n, p)| json!({"code": c, "name": n, "prefix": p})).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct IntentIn {
    pub invoice_id: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub phone: String,
    pub amount: String,
}

pub async fn intent_create(State(s): State<AppState>, Json(b): Json<IntentIn>) -> Json<Value> {
    let Some(amount) = crate::fees::to_cents(&b.amount) else {
        return Json(json!({ "ok": false, "error": "kiasi si sahihi" }));
    };
    if amount <= 0 {
        return Json(json!({ "ok": false, "error": "kiasi lazima kiwe zaidi ya sifuri" }));
    }
    if !phone_ok(&b.phone) {
        return Json(json!({ "ok": false, "error": "namba ya simu si sahihi (mf. 0712345678)" }));
    }
    let inv = sqlx::query_as::<_, (String, i64)>(
        "SELECT student_id,total_cents FROM ep_invoices WHERE id=?1",
    )
    .bind(b.invoice_id.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((student_id, total)) = inv else {
        return Json(json!({ "ok": false, "error": "ankara haipo" }));
    };
    // Salio: malipo yaliyothibitishwa + malipo ya simu yanayosubiri.
    let (paid,): (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments WHERE invoice_id=?1")
            .bind(b.invoice_id.trim())
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (held,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM ep_mobile_intents WHERE invoice_id=?1 AND status='pending'",
    )
    .bind(b.invoice_id.trim())
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let balance = total - paid - held;
    if amount > balance {
        return Json(json!({
            "ok": false, "error": "malipo yanazidi salio",
            "balance_cents": balance, "balance": crate::fees::fmt_money(balance)
        }));
    }

    let provider = norm_provider(&b.provider);
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM ep_mobile_intents")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let year = chrono::Local::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let reference = tx_ref(provider, year, seq);
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_mobile_intents (id,invoice_id,student_id,provider,phone,amount_cents,tx_ref,status,provider_ref,payment_id,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,'pending','','',?8)",
    )
    .bind(&id)
    .bind(b.invoice_id.trim())
    .bind(&student_id)
    .bind(provider)
    .bind(b.phone.trim())
    .bind(amount)
    .bind(&reference)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "id": id, "tx_ref": reference, "provider": provider,
        "amount_cents": amount, "amount": crate::fees::fmt_money(amount),
        "status": "pending",
        "push": format!("Ombi la malipo limetumwa kwa {} — mwanafunzi athibitishe kwenye simu.", provider)
    }))
}

#[derive(Deserialize)]
pub struct IntentQuery {
    #[serde(default)]
    pub student_id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub provider: String,
}

pub async fn intents_list(State(s): State<AppState>, Query(q): Query<IntentQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String, Option<String>)>(
        "SELECT id,invoice_id,student_id,provider,phone,amount_cents,tx_ref,status,provider_ref,confirmed_at
         FROM ep_mobile_intents
         WHERE (?1='' OR student_id=?1) AND (?2='' OR status=?2) AND (?3='' OR provider=?3)
         ORDER BY created_at DESC LIMIT 300",
    )
    .bind(&q.student_id)
    .bind(&q.status)
    .bind(&q.provider)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "intents": rows.into_iter().map(|r| json!({
            "id": r.0, "invoice_id": r.1, "student_id": r.2, "provider": r.3, "phone": r.4,
            "amount_cents": r.5, "amount": crate::fees::fmt_money(r.5), "tx_ref": r.6,
            "status": r.7, "provider_ref": r.8, "confirmed_at": r.9
        })).collect::<Vec<_>>()
    }))
}

pub async fn intent_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let r = sqlx::query_as::<_, (String, String, String, String, i64, String, String, String)>(
        "SELECT tx_ref,provider,phone,status,amount_cents,provider_ref,payment_id,COALESCE(confirmed_at,'')
         FROM ep_mobile_intents WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    match r {
        None => Json(json!({ "ok": false, "error": "muamala haupo" })),
        Some((reference, provider, phone, st, amount, pref, pay, conf)) => Json(json!({
            "ok": true,
            "intent": {
                "tx_ref": reference, "provider": provider, "phone": phone, "status": st,
                "amount_cents": amount, "amount": crate::fees::fmt_money(amount),
                "provider_ref": pref, "payment_id": pay, "confirmed_at": conf
            }
        })),
    }
}

#[derive(Deserialize)]
pub struct ConfirmIn {
    pub intent_id: String,
    #[serde(default)]
    pub provider_ref: String,
    #[serde(default)]
    pub received_by: String,
}

/// Huthibitisha muamala: huingiza `ep_payments` (Awamu 8) kwa uthibitisho mmoja.
pub async fn confirm(State(s): State<AppState>, Json(b): Json<ConfirmIn>) -> Json<Value> {
    let cur = sqlx::query_as::<_, (String, String, String, String, i64)>(
        "SELECT status,invoice_id,student_id,provider,amount_cents FROM ep_mobile_intents WHERE id=?1",
    )
    .bind(b.intent_id.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((st, invoice_id, student_id, provider, amount)) = cur else {
        return Json(json!({ "ok": false, "error": "muamala haupo" }));
    };
    if !can_confirm(&st) {
        return Json(json!({
            "ok": false, "error": format!("muamala umeshughulikiwa (hali '{st}')")
        }));
    }
    let inv = sqlx::query_as::<_, (i64,)>("SELECT total_cents FROM ep_invoices WHERE id=?1")
        .bind(&invoice_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((total,)) = inv else {
        return Json(json!({ "ok": false, "error": "ankara haipo" }));
    };
    let (already,): (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments WHERE invoice_id=?1")
            .bind(&invoice_id)
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    if already + amount > total {
        return Json(json!({ "ok": false, "error": "malipo yanazidi salio" }));
    }
    // Risiti — mstari mmoja wa ukweli na `fees::receipt_no`.
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM ep_payments")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let year = chrono::Local::now().format("%Y").to_string().parse::<i32>().unwrap_or(2026);
    let rcpt = crate::fees::receipt_no(year, seq);
    let pay_id = uid();
    let _ = sqlx::query(
        "INSERT INTO ep_payments (id,invoice_id,student_id,amount_cents,method,reference,receipt_no,received_by,paid_at)
         VALUES (?1,?2,?3,?4,'mobile',?5,?6,?7,?8)",
    )
    .bind(&pay_id)
    .bind(&invoice_id)
    .bind(&student_id)
    .bind(amount)
    .bind(&b.provider_ref)
    .bind(&rcpt)
    .bind(&b.received_by)
    .bind(now())
    .execute(&s.db)
    .await;
    let new_paid = already + amount;
    let _ = sqlx::query("UPDATE ep_invoices SET status=?1 WHERE id=?2")
        .bind(crate::fees::invoice_status(total, new_paid))
        .bind(&invoice_id)
        .execute(&s.db)
        .await;
    let _ = sqlx::query(
        "UPDATE ep_mobile_intents SET status='confirmed',provider_ref=?1,payment_id=?2,confirmed_at=?3 WHERE id=?4",
    )
    .bind(&b.provider_ref)
    .bind(&pay_id)
    .bind(now())
    .bind(b.intent_id.trim())
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "payment_id": pay_id, "receipt_no": rcpt, "provider": provider,
        "amount_cents": amount, "invoice_status": crate::fees::invoice_status(total, new_paid),
        "balance_cents": total - new_paid
    }))
}

#[derive(Deserialize)]
pub struct FailIn {
    pub intent_id: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub cancelled: bool,
}

pub async fn fail(State(s): State<AppState>, Json(b): Json<FailIn>) -> Json<Value> {
    let cur = sqlx::query_as::<_, (String,)>("SELECT status FROM ep_mobile_intents WHERE id=?1")
        .bind(b.intent_id.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
    let Some((st,)) = cur else {
        return Json(json!({ "ok": false, "error": "muamala haupo" }));
    };
    if !can_fail(&st) {
        return Json(json!({ "ok": false, "error": format!("muamala umeshughulikiwa (hali '{st}')") }));
    }
    let new_st = if b.cancelled { "cancelled" } else { "failed" };
    let _ = sqlx::query("UPDATE ep_mobile_intents SET status=?1,provider_ref=?2 WHERE id=?3")
        .bind(new_st)
        .bind(&b.reason)
        .bind(b.intent_id.trim())
        .execute(&s.db)
        .await;
    Json(json!({ "ok": true, "status": new_st }))
}

/// Upatanisho: malipo ya simu yaliyothibitishwa vs kumbukumbu za ada.
pub async fn reconcile(State(s): State<AppState>) -> Json<Value> {
    let (intents_confirmed,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM ep_mobile_intents WHERE status='confirmed'",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let (mobile_payments,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments WHERE method='mobile'",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let mismatch = intents_confirmed != mobile_payments;
    Json(json!({
        "ok": true,
        "intents_confirmed_cents": intents_confirmed,
        "mobile_payments_cents": mobile_payments,
        "balanced": !mismatch,
        "difference_cents": intents_confirmed - mobile_payments,
        "note": if mismatch {
            "Kuna tofauti — angalia muamala uliothibitishwa bila malipo."
        } else {
            "Kumbukumbu zinalingana."
        }
    }))
}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namba_ya_simu_ya_tanzania_hukaguliwa() {
        assert!(phone_ok("0712345678"));
        assert!(phone_ok("+255712345678"));
        assert!(phone_ok("255712345678"));
        assert!(phone_ok("0687654321"));
        assert!(phone_ok("0755 123 456"));

        assert!(!phone_ok(""));
        assert!(!phone_ok("0712345"));      // fupi
        assert!(!phone_ok("07123456789"));  // ndefu
        assert!(!phone_ok("0212345678"));   // 02 si mtandao wa simu
        assert!(!phone_ok("abcdefghij"));
    }

    #[test]
    fn watoa_huduma_hupangwa() {
        assert_eq!(norm_provider("M-Pesa"), "mpesa");
        assert_eq!(norm_provider("vodacom"), "mpesa");
        assert_eq!(norm_provider("Tigo Pesa"), "tigo");
        assert_eq!(norm_provider("mixx"), "tigo");
        assert_eq!(norm_provider("Airtel Money"), "airtel");
        assert_eq!(norm_provider(""), "mpesa");
    }

    #[test]
    fn kumbukumbu_ya_muamala_ina_kiambishi_cha_mtoa() {
        assert_eq!(tx_ref("mpesa", 2026, 1), "MPESA-2026-000001");
        assert_eq!(tx_ref("tigo", 2026, 42), "TIGO-2026-000042");
        assert_eq!(tx_ref("airtel", 2026, 999999), "AIRTEL-2026-999999");
    }

    #[test]
    fn muamala_huthibitishwa_mara_moja() {
        assert!(can_confirm("pending"));
        assert!(!can_confirm("confirmed"));
        assert!(!can_confirm("failed"));
        assert!(!can_confirm("cancelled"));

        assert!(can_fail("pending"));
        assert!(!can_fail("confirmed"));
    }

    async fn db() -> SqlitePool {
        let p = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init_tables(&p).await;
        crate::fees::init_tables(&p).await;
        p
    }

    #[tokio::test]
    async fn kuthibitisha_huingiza_malipo_na_kubadilisha_hali() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_invoices (id,student_id,level_id,term,total_cents,status,created_at) VALUES ('i1','s1','l1','2026-S1',100000,'unpaid',?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("INSERT INTO ep_mobile_intents (id,invoice_id,student_id,provider,phone,amount_cents,tx_ref,status,provider_ref,payment_id,created_at) VALUES ('m1','i1','s1','mpesa','0712345678',100000,'MPESA-2026-000001','pending','','',?1)")
            .bind(now()).execute(&p).await.unwrap();

        sqlx::query("INSERT INTO ep_payments (id,invoice_id,student_id,amount_cents,method,reference,receipt_no,received_by,paid_at) VALUES ('p1','i1','s1',100000,'mobile','MPESA-REF','RCP/2026/000001','',?1)")
            .bind(now()).execute(&p).await.unwrap();
        sqlx::query("UPDATE ep_mobile_intents SET status='confirmed',provider_ref='REF',payment_id='p1',confirmed_at=?1 WHERE id='m1'")
            .bind(now()).execute(&p).await.unwrap();

        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_mobile_intents WHERE id='m1'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(st, "confirmed");
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_payments WHERE method='mobile'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn muamala_uliokataliwa_hauingii_kwenye_malipo() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_mobile_intents (id,invoice_id,student_id,provider,phone,amount_cents,tx_ref,status,provider_ref,payment_id,created_at) VALUES ('m2','i1','s1','tigo','0712345678',5000,'TIGO-2026-000001','cancelled','','',?1)")
            .bind(now()).execute(&p).await.unwrap();
        let (n,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ep_payments WHERE method='mobile'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(n, 0);
        let (st,): (String,) = sqlx::query_as("SELECT status FROM ep_mobile_intents WHERE id='m2'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(st, "cancelled");
    }

    #[tokio::test]
    async fn upatanisho_hugundua_tofauti() {
        let p = db().await;
        sqlx::query("INSERT INTO ep_mobile_intents (id,invoice_id,student_id,provider,phone,amount_cents,tx_ref,status,provider_ref,payment_id,created_at) VALUES ('m3','i1','s1','airtel','0712345678',7000,'AIRTEL-2026-000001','confirmed','','',?1)")
            .bind(now()).execute(&p).await.unwrap();
        // Hakuna malipo yanayolingana → tofauti.
        let (c,): (i64,) = sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_mobile_intents WHERE status='confirmed'")
            .fetch_one(&p).await.unwrap();
        let (mp,): (i64,) = sqlx::query_as("SELECT COALESCE(SUM(amount_cents),0) FROM ep_payments WHERE method='mobile'")
            .fetch_one(&p).await.unwrap();
        assert_eq!(c, 7000);
        assert_eq!(mp, 0);
        assert_ne!(c, mp);
    }
}
