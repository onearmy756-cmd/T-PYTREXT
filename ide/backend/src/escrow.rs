//! escrow.rs — MALIPO YA MTANDAONI: Stripe (USD) + simu (TZS) + escrow.
//!
//! Mahitaji ya mmiliki: "MALIPO yote yatafanyika online na kwenda kwenye account
//! ya kampuni baada ya kutoka escrow".
//!
//! MTIRIRIKO (hakuna kubuni):
//!   1. `checkout` — mteja huchagua sarafu. **USD → Stripe Checkout** (session
//!      halisi ya Stripe API). **TZS → malipo ya simu** (malipo.rs) au benki.
//!   2. Pesa inapothibitishwa → `escrow.hold` — pesa INAHIFADHIWA (si kutolewa).
//!   3. Kazi inakamilika + admin athibitisha → `escrow.release` — pesa huenda
//!      kwenye akaunti ya kampuni. `escrow.refund` hurudisha kwa mteja.
//!
//! Kanuni halisi:
//!   - Salio la escrow = JUMLA ya historia yake (ledger); hakuna column ya salio
//!     inayoweza kupingana na rekodi.
//!   - Kutolewa HAKUWEZI kuzidi kile kilichohifadhiwa.
//!   - Bila `STRIPE_SECRET_KEY`, Stripe hukataliwa kwa uwazi (`configured: false`)
//!     — HATUUNDI session ya uongo.
//!   - Malipo ya mteja wa nje (si TZ) HAYAWEZI kuwa TZS.
//!
//! Tables: bz_payments bz_escrow_ledger

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
        r#"CREATE TABLE IF NOT EXISTS bz_payments (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL DEFAULT '',
            client_name TEXT NOT NULL DEFAULT '',
            gateway TEXT NOT NULL DEFAULT 'stripe', -- stripe|mobile|bank
            currency TEXT NOT NULL DEFAULT 'TZS',
            amount_cents INTEGER NOT NULL DEFAULT 0,
            provider_ref TEXT NOT NULL DEFAULT '',
            checkout_url TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|paid|failed|refunded
            created_at TEXT NOT NULL,
            paid_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_escrow_ledger (
            id TEXT PRIMARY KEY,
            payment_id TEXT NOT NULL DEFAULT '',
            request_id TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'hold', -- hold|release|refund|fee
            amount_cents INTEGER NOT NULL DEFAULT 0,
            currency TEXT NOT NULL DEFAULT 'TZS',
            status TEXT NOT NULL DEFAULT 'Held',
            note TEXT NOT NULL DEFAULT '',
            actor TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/escrow/status", get(status))
        .route("/api/escrow/gateways", get(gateways))
        .route("/api/escrow/checkout", post(checkout))
        .route("/api/escrow/confirm", post(confirm))
        .route("/api/escrow/hold", post(hold_h))
        .route("/api/escrow/release", post(release_h))
        .route("/api/escrow/refund", post(refund_h))
        .route("/api/escrow/payments", get(payments_list))
        .route("/api/escrow/ledger", get(ledger_list))
        .route("/api/escrow/balance/:request", get(balance_h))
        .route("/api/escrow/stripe/webhook", post(stripe_webhook))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Aina za mienendo ya escrow.
pub const KINDS: [&str; 4] = ["hold", "release", "refund", "fee"];

/// Stripe imewekwa? (`STRIPE_SECRET_KEY`).
pub fn stripe_key() -> Option<String> {
    std::env::var("STRIPE_SECRET_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Akaunti ya kampuni inayopokea malipo (mazingira).
pub fn company_account() -> Value {
    json!({
        "name": std::env::var("COMPANY_ACCOUNT_NAME").unwrap_or_else(|_| "MTECHOS GROUP".into()),
        "bank": std::env::var("COMPANY_BANK").unwrap_or_default(),
        "account_no": std::env::var("COMPANY_ACCOUNT_NO").unwrap_or_default(),
        "mobile": std::env::var("COMPANY_MOBILE").unwrap_or_default(),
        "configured": std::env::var("COMPANY_ACCOUNT_NO").map(|v| !v.trim().is_empty()).unwrap_or(false)
    })
}

/// Stripe inatumika kwa sarafu hii? (TZS haitumiki Stripe kwa default.)
pub fn gateway_for(currency: &str) -> &'static str {
    if currency.eq_ignore_ascii_case("TZS") {
        "mobile"
    } else {
        "stripe"
    }
}

/// Mteja wa nje (si Tanzania) HAWEZI kulipa TZS.
pub fn currency_allowed_for(country: &str, currency: &str) -> bool {
    if country.eq_ignore_ascii_case("TZ") {
        true
    } else {
        !currency.eq_ignore_ascii_case("TZS")
    }
}

/// Muamala unaweza kuthibitishwa? (mara moja pekee)
pub fn can_confirm(status: &str) -> bool {
    status == "pending"
}

/// Salio linaweza kutolewa? HAKUNA kutolewa zaidi ya kilichohifadhiwa.
pub fn can_release(held_cents: i64, amount_cents: i64) -> bool {
    amount_cents > 0 && amount_cents <= held_cents
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let (payments,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bz_payments")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (paid,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bz_payments WHERE status='paid'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (held,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(CASE WHEN kind='hold' THEN amount_cents WHEN kind IN ('release','refund') THEN -amount_cents ELSE 0 END),0)
         FROM bz_escrow_ledger",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let (released,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(amount_cents),0) FROM bz_escrow_ledger WHERE kind='release'",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "stripe_configured": stripe_key().is_some(),
        "company_account": company_account(),
        "payments": payments, "paid": paid,
        "in_escrow_cents": held, "in_escrow": crate::fees::fmt_money(held),
        "released_cents": released, "released": crate::fees::fmt_money(released),
    }))
}

async fn gateways() -> Json<Value> {
    Json(json!({
        "ok": true,
        "stripe_configured": stripe_key().is_some(),
        "gateways": [
            {"code": "stripe", "name": "Stripe (Kadi, USD/EUR/GBP)", "currencies": ["USD","EUR","GBP","KES","ZAR","INR"], "online": true},
            {"code": "mobile", "name": "Malipo ya Simu (M-Pesa/Mixx/Airtel) — TZS", "currencies": ["TZS"], "online": true},
            {"code": "bank", "name": "Benki (TZS)", "currencies": ["TZS"], "online": false}
        ]
    }))
}

#[derive(Deserialize)]
pub struct CheckoutIn {
    pub request: String,
    pub client_name: String,
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub country: String,
}

/// Anzisha malipo ya mtandaoni. USD → Stripe Checkout halisi; TZS → simu.
async fn checkout(State(s): State<AppState>, Json(b): Json<CheckoutIn>) -> Json<Value> {
    let Some((rid, reference, req_currency, req_amount)) =
        sqlx::query_as::<_, (String, String, String, i64)>(
            "SELECT id,ref,currency,amount_cents FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
        )
        .bind(b.request.trim())
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let amount_cents = match crate::fees::to_cents(&b.amount) {
        Some(c) if c > 0 => c,
        _ => req_amount,
    };
    let currency = if b.currency.trim().is_empty() {
        req_currency
    } else {
        crate::biashara::norm_currency(&b.currency)
    };
    let country = if b.country.trim().is_empty() {
        "TZ".into()
    } else {
        b.country.trim().to_ascii_uppercase()
    };
    if !currency_allowed_for(&country, &currency) {
        return Json(json!({
            "ok": false,
            "error": "mteja wa nje hawezi kulipa TZS — tumia USD (Stripe)"
        }));
    }
    let gateway = gateway_for(&currency);
    let id = uid();

    if gateway == "stripe" {
        let Some(key) = stripe_key() else {
            // Hatuundi session ya uongo — tunasema ukweli.
            let _ = sqlx::query(
                "INSERT INTO bz_payments (id,request_id,client_name,gateway,currency,amount_cents,provider_ref,checkout_url,status,created_at)
                 VALUES (?1,?2,?3,'stripe',?4,?5,'','','pending',?6)",
            )
            .bind(&id)
            .bind(&rid)
            .bind(&b.client_name)
            .bind(&currency)
            .bind(amount_cents)
            .bind(now())
            .execute(&s.db)
            .await;
            return Json(json!({
                "ok": false, "id": id, "gateway": "stripe", "configured": false,
                "error": "STRIPE_SECRET_KEY haijawekwa — Stripe haiko tayari. Weka key kwenye mazingira."
            }));
        };
        match stripe_checkout(&key, &reference, amount_cents, &currency).await {
            Ok((sid, url)) => {
                let _ = sqlx::query(
                    "INSERT INTO bz_payments (id,request_id,client_name,gateway,currency,amount_cents,provider_ref,checkout_url,status,created_at)
                     VALUES (?1,?2,?3,'stripe',?4,?5,?6,?7,'pending',?8)",
                )
                .bind(&id)
                .bind(&rid)
                .bind(&b.client_name)
                .bind(&currency)
                .bind(amount_cents)
                .bind(&sid)
                .bind(&url)
                .bind(now())
                .execute(&s.db)
                .await;
                crate::biashara::audit(&s.db, "mteja", "payment.checkout", &id, "stripe").await;
                Json(json!({
                    "ok": true, "id": id, "gateway": "stripe", "configured": true,
                    "session_id": sid, "checkout_url": url,
                    "amount_cents": amount_cents,
                    "amount": crate::fees::fmt_money(amount_cents),
                    "currency": currency,
                    "note": "mteja akamilishe malipo kwenye ukurasa wa Stripe"
                }))
            }
            Err(e) => Json(json!({"ok": false, "gateway": "stripe", "error": e})),
        }
    } else {
        // TZS: foleni ya malipo ya simu; uthibitisho huja kwa `confirm`.
        let _ = sqlx::query(
            "INSERT INTO bz_payments (id,request_id,client_name,gateway,currency,amount_cents,provider_ref,checkout_url,status,created_at)
             VALUES (?1,?2,?3,'mobile',?4,?5,'','','pending',?6)",
        )
        .bind(&id)
        .bind(&rid)
        .bind(&b.client_name)
        .bind(&currency)
        .bind(amount_cents)
        .bind(now())
        .execute(&s.db)
        .await;
        crate::biashara::audit(&s.db, "mteja", "payment.checkout", &id, "mobile").await;
        Json(json!({
            "ok": true, "id": id, "gateway": "mobile", "configured": true,
            "amount_cents": amount_cents,
            "amount": crate::fees::fmt_money(amount_cents),
            "currency": currency,
            "note": "mteja alipe kwa M-Pesa/Mixx/Airtel, kisha thibitisha kwa provider_ref"
        }))
    }
}

/// Stripe Checkout Session halisi (form-encoded API).
async fn stripe_checkout(
    key: &str,
    reference: &str,
    amount_cents: i64,
    currency: &str,
) -> Result<(String, String), String> {
    let cur = currency.to_ascii_lowercase();
    // Sarafu za decimal-0 (mf. JPY) hazitumiki; zetu zote zina senti 2.
    let client = reqwest::Client::new();
    let form = [
        ("mode", "payment".to_string()),
        ("success_url", format!("https://mtechos.example/paid/{reference}")),
        ("cancel_url", format!("https://mtechos.example/cancelled/{reference}")),
        ("line_items[0][quantity]", "1".to_string()),
        ("line_items[0][price_data][currency]", cur.clone()),
        (
            "line_items[0][price_data][unit_amount]",
            amount_cents.to_string(),
        ),
        (
            "line_items[0][price_data][product_data][name]",
            format!("MTECHOS huduma {reference}"),
        ),
        ("client_reference_id", reference.to_string()),
    ];
    let resp = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .bearer_auth(key)
        .form(&form)
        .timeout(std::time::Duration::from_secs(25))
        .send()
        .await
        .map_err(|e| format!("Stripe haikufikiwa: {e}"))?;
    let status = resp.status();
    let v: Value = resp.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        let msg = v["error"]["message"].as_str().unwrap_or("kosa la Stripe");
        return Err(format!("Stripe: {msg}"));
    }
    let sid = v["id"].as_str().unwrap_or("").to_string();
    let url = v["url"].as_str().unwrap_or("").to_string();
    if sid.is_empty() || url.is_empty() {
        return Err("Stripe haikurudisha session".into());
    }
    Ok((sid, url))
}

#[derive(Deserialize)]
pub struct ConfirmIn {
    pub payment: String,
    #[serde(default)]
    pub provider_ref: String,
}

/// Thibitisha malipo yaliyolipwa → HIFADHI kwenye escrow.
async fn confirm(State(s): State<AppState>, Json(b): Json<ConfirmIn>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, i64, String, String)>(
        "SELECT id,request_id,gateway,amount_cents,currency,status FROM bz_payments WHERE id=?1 OR provider_ref=?1 LIMIT 1",
    )
    .bind(b.payment.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((id, request_id, gateway, amount_cents, currency, st)) = row else {
        return Json(json!({"ok": false, "error": "malipo hayapo"}));
    };
    if !can_confirm(&st) {
        return Json(json!({"ok": false, "error": format!("malipo ({st}) yameshathibitishwa")}));
    }
    let _ = sqlx::query(
        "UPDATE bz_payments SET status='paid', paid_at=?2, provider_ref=CASE WHEN ?3='' THEN provider_ref ELSE ?3 END WHERE id=?1",
    )
    .bind(&id)
    .bind(now())
    .bind(&b.provider_ref)
    .execute(&s.db)
    .await;
    // Pesa inaingia ESCROW (haijatolewa).
    let esc_id = escrow_entry(
        &s.db,
        &id,
        &request_id,
        "hold",
        amount_cents,
        &currency,
        "Held",
        &format!("malipo ya mtandaoni ({gateway})"),
        "mteja",
    )
    .await;
    crate::biashara::audit(&s.db, "mteja", "payment.confirm", &id, "escrow hold").await;
    Json(json!({
        "ok": true, "payment_id": id, "status": "paid",
        "escrow_id": esc_id, "held_cents": amount_cents,
        "held": crate::fees::fmt_money(amount_cents), "currency": currency,
        "note": "pesa iko kwenye escrow — hutolewa kwa kampuni baada ya kazi kukamilika"
    }))
}

#[derive(Deserialize)]
pub struct HoldIn {
    pub request: String,
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub payment_id: String,
    #[serde(default)]
    pub note: String,
}

async fn hold_h(State(s): State<AppState>, Json(b): Json<HoldIn>) -> Json<Value> {
    let Some((rid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,currency FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(b.request.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let Some(amount) = crate::fees::to_cents(&b.amount) else {
        return Json(json!({"ok": false, "error": "kiasi si sahihi"}));
    };
    if amount <= 0 {
        return Json(json!({"ok": false, "error": "kiasi lazima kiwe zaidi ya sifuri"}));
    }
    let currency = if b.currency.trim().is_empty() {
        cur
    } else {
        crate::biashara::norm_currency(&b.currency)
    };
    let id = escrow_entry(
        &s.db,
        &b.payment_id,
        &rid,
        "hold",
        amount,
        &currency,
        "Held",
        &b.note,
        "admin",
    )
    .await;
    crate::biashara::audit(&s.db, "admin", "escrow.hold", &rid, &b.amount).await;
    Json(json!({"ok": true, "id": id, "request_id": rid, "held_cents": amount,
        "held": crate::fees::fmt_money(amount), "currency": currency}))
}

#[derive(Deserialize)]
pub struct ReleaseIn {
    pub request: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub note: String,
}

/// Toa pesa kutoka escrow → akaunti ya KAMPUNI (baada ya kazi kukamilika).
async fn release_h(State(s): State<AppState>, Json(b): Json<ReleaseIn>) -> Json<Value> {
    settle(&s, &b.request, &b.amount, "release", &b.note).await
}

#[derive(Deserialize)]
pub struct RefundIn {
    pub request: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub note: String,
}

/// Rudisha pesa kwa mteja kutoka escrow.
async fn refund_h(State(s): State<AppState>, Json(b): Json<RefundIn>) -> Json<Value> {
    settle(&s, &b.request, &b.amount, "refund", &b.note).await
}

async fn settle(s: &AppState, request: &str, amount: &str, kind: &str, note: &str) -> Json<Value> {
    let Some((rid, cur, status)) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,currency,status FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(request.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let held = balance(&s.db, &rid).await;
    let amount_cents = match crate::fees::to_cents(amount) {
        Some(c) if c > 0 => c,
        _ => held,
    };
    if !can_release(held, amount_cents) {
        return Json(json!({
            "ok": false,
            "error": format!("escrow haina kiasi hicho (kilichopo: {})", crate::fees::fmt_money(held)),
            "held_cents": held, "held": crate::fees::fmt_money(held)
        }));
    }
    // Kutolewa kunahitaji kazi iwe imekamilika (imewasilishwa/imefungwa).
    if kind == "release" && !matches!(status.as_str(), "delivered" | "closed") {
        return Json(json!({
            "ok": false,
            "error": format!("ombi ({status}) halijakamilika — pesa haitolewi kabla ya kuwasilisha")
        }));
    }
    let status_label = if kind == "release" { "Released" } else { "Refunded" };
    let id = escrow_entry(
        &s.db, "", &rid, kind, amount_cents, &cur, status_label, note, "admin",
    )
    .await;
    crate::biashara::audit(
        &s.db,
        "admin",
        &format!("escrow.{kind}"),
        &rid,
        &crate::fees::fmt_money(amount_cents),
    )
    .await;
    let remaining = balance(&s.db, &rid).await;
    let mut out = json!({
        "ok": true, "id": id, "request_id": rid, "kind": kind,
        "amount_cents": amount_cents,
        "amount": crate::fees::fmt_money(amount_cents),
        "currency": cur, "remaining_cents": remaining,
        "remaining": crate::fees::fmt_money(remaining)
    });
    if kind == "release" {
        out["to_account"] = company_account();
        out["note"] = json!("pesa imetolewa kwenye escrow kwenda akaunti ya kampuni");
    }
    Json(out)
}

/// Ingiza mstari kwenye ledger ya escrow.
#[allow(clippy::too_many_arguments)]
async fn escrow_entry(
    db: &SqlitePool,
    payment_id: &str,
    request_id: &str,
    kind: &str,
    amount_cents: i64,
    currency: &str,
    status: &str,
    note: &str,
    actor: &str,
) -> String {
    let id = format!("ESC-{}", &uid()[..8].to_ascii_uppercase());
    let _ = sqlx::query(
        "INSERT INTO bz_escrow_ledger (id,payment_id,request_id,kind,amount_cents,currency,status,note,actor,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
    )
    .bind(&id)
    .bind(payment_id)
    .bind(request_id)
    .bind(kind)
    .bind(amount_cents)
    .bind(currency)
    .bind(status)
    .bind(note)
    .bind(actor)
    .bind(now())
    .execute(db)
    .await;
    id
}

/// Salio la escrow la ombi = historia yake.
pub async fn balance(db: &SqlitePool, request_id: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(SUM(CASE WHEN kind='hold' THEN amount_cents WHEN kind IN ('release','refund') THEN -amount_cents ELSE 0 END),0)
         FROM bz_escrow_ledger WHERE request_id=?1",
    )
    .bind(request_id)
    .fetch_one(db)
    .await
    .map(|r| r.0)
    .unwrap_or(0)
}

async fn balance_h(State(s): State<AppState>, Path(request): Path<String>) -> Json<Value> {
    let Some((rid, reference)) =
        sqlx::query_as::<_, (String, String)>("SELECT id,ref FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1")
            .bind(&request)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let held = balance(&s.db, &rid).await;
    Json(json!({
        "ok": true, "request_id": rid, "ref": reference,
        "held_cents": held, "held": crate::fees::fmt_money(held)
    }))
}

async fn payments_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, String, String, String)>(
        "SELECT id,request_id,client_name,gateway,currency,amount_cents,provider_ref,status,created_at
         FROM bz_payments ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "payments": rows.into_iter().map(|r| json!({
        "id": r.0, "request_id": r.1, "client_name": r.2, "gateway": r.3,
        "currency": r.4, "amount_cents": r.5, "amount": crate::fees::fmt_money(r.5),
        "provider_ref": r.6, "status": r.7, "created_at": r.8
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    limit: Option<i64>,
}

async fn ledger_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(300).clamp(1, 2000);
    let rows = sqlx::query_as::<_, (String, String, String, i64, String, String, String, String)>(
        "SELECT id,request_id,kind,amount_cents,currency,status,note,created_at
         FROM bz_escrow_ledger ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "ledger": rows.into_iter().map(|r| json!({
        "id": r.0, "request_id": r.1, "kind": r.2,
        "amount_cents": r.3, "amount": crate::fees::fmt_money(r.3),
        "currency": r.4, "status": r.5, "note": r.6, "at": r.7
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
pub struct WebhookIn {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub data: Value,
}

/// Muda wa sasa kwa sekunde za Unix.
fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Tovuti ya uvumilivu wa replay (sekunde) — Stripe hutumia 300.
pub const WEBHOOK_TOLERANCE_SECS: i64 = 300;

/// Thibitisha saini ya Stripe kwa **HMAC-SHA256** halisi.
///
/// Kichwa `Stripe-Signature` huwa na sehemu `t=<unix>,v1=<hex>[,v1=<hex>…]`.
/// Payload iliyotiwa saini ni `"<t>.<raw_body>"`. Hurudisha `Err` yenye sababu
/// kwa Kiswahili kama uthibitisho unashindwa.
pub fn stripe_verify(
    secret: &str,
    sig_header: &str,
    raw_body: &str,
    now: i64,
) -> Result<(), String> {
    let mut t: Option<i64> = None;
    let mut v1: Vec<String> = vec![];
    for part in sig_header.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("t=") {
            t = v.trim().parse::<i64>().ok();
        } else if let Some(v) = part.strip_prefix("v1=") {
            v1.push(v.trim().to_ascii_lowercase());
        }
    }
    let Some(t) = t else {
        return Err("saini haipo (t=…,v1=…)".into());
    };
    if v1.is_empty() {
        return Err("saini haipo (v1=…)".into());
    }
    if (now - t).abs() > WEBHOOK_TOLERANCE_SECS {
        return Err("saini imeisha muda (replay?)".into());
    }
    let signed = format!("{t}.{raw_body}");
    let expect = hmac_sha256_hex(secret.as_bytes(), signed.as_bytes());
    let ok = v1.iter().any(|s| crate::sms::constant_eq(s, &expect));
    if ok {
        Ok(())
    } else {
        Err("saini si sahihi".into())
    }
}

/// HMAC-SHA256 → hex (herufi ndogo). Hulinganisha kwa muda usiobadilika.
pub fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(key).expect("hmac key");
    mac.update(msg);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Webhook ya Stripe: malipo yamekamilika → thibitisha + escrow hold.
///
/// USALAMA (Stripe-grade):
///  * Kama `STRIPE_WEBHOOK_SECRET` imeanza na `whsec_`, thibitisho ni **HMAC-SHA256
///    halisi** ya Stripe (`Stripe-Signature: t=…,v1=…`) — hakuna token ya kubuni.
///  * Kama siri ni ya kubuni (bila `whsec_`), hutumia token ya header `x-stripe-token`
///    kwa kipindi cha mpito.
///  * Replay protection: `t` haipaswi kuwa zaidi ya sekunde 300 nyuma/baadaye.
async fn stripe_webhook(
    State(s): State<AppState>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Json<Value> {
    let secret = std::env::var("STRIPE_WEBHOOK_SECRET").unwrap_or_default();
    if secret.trim().is_empty() {
        return Json(json!({"ok": false, "error": "webhook imezimwa: weka STRIPE_WEBHOOK_SECRET"}));
    }
    let secret = secret.trim();
    let raw = String::from_utf8_lossy(&body).to_string();

    if secret.starts_with("whsec_") {
        let sig_header = headers
            .get("stripe-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if let Err(e) = stripe_verify(secret, sig_header, &raw, now_unix()) {
            return Json(json!({"ok": false, "error": e}));
        }
    } else {
        let got = headers
            .get("x-stripe-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !crate::sms::constant_eq(got, secret) {
            return Json(json!({"ok": false, "error": "token si sahihi"}));
        }
    }

    let b: WebhookIn = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return Json(json!({"ok": false, "error": "mwili si JSON halali"})),
    };
    let b = b;
    if b.r#type != "checkout.session.completed" {
        return Json(json!({"ok": true, "ignored": b.r#type}));
    }
    let sid = b.data["object"]["id"].as_str().unwrap_or("").to_string();
    if sid.is_empty() {
        return Json(json!({"ok": false, "error": "session id haipo"}));
    }
    let row = sqlx::query_as::<_, (String, String, i64, String, String)>(
        "SELECT id,request_id,amount_cents,currency,status FROM bz_payments WHERE provider_ref=?1 LIMIT 1",
    )
    .bind(&sid)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((id, request_id, amount_cents, currency, st)) = row else {
        return Json(json!({"ok": false, "error": "malipo hayapatikani kwa session hii"}));
    };
    if !can_confirm(&st) {
        return Json(json!({"ok": true, "already": st}));
    }
    let _ = sqlx::query("UPDATE bz_payments SET status='paid', paid_at=?2 WHERE id=?1")
        .bind(&id)
        .bind(now())
        .execute(&s.db)
        .await;
    escrow_entry(
        &s.db,
        &id,
        &request_id,
        "hold",
        amount_cents,
        &currency,
        "Held",
        "malipo ya Stripe (webhook)",
        "stripe",
    )
    .await;
    crate::biashara::audit(&s.db, "stripe", "payment.webhook", &id, "paid").await;
    Json(json!({"ok": true, "payment_id": id, "status": "paid", "escrow": "held"}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saini_ya_stripe_huthibitishwa() {
        let secret = "whsec_test123";
        let raw = r#"{"type":"checkout.session.completed"}"#;
        let t = 1_700_000_000i64;
        let sig = hmac_sha256_hex(secret.as_bytes(), format!("{t}.{raw}").as_bytes());
        let header = format!("t={t},v1={sig}");
        assert!(stripe_verify(secret, &header, raw, t).is_ok());
        // siri isiyo sahihi
        assert!(stripe_verify("whsec_other", &header, raw, t).is_err());
        // mwili uliobadilishwa
        assert!(stripe_verify(secret, &header, "{\"x\":1}", t).is_err());
        // saini imeisha muda (replay)
        assert!(stripe_verify(secret, &header, raw, t + 301).is_err());
        // v1 nyingi — moja sahihi inakubaliwa
        let multi = format!("t={t},v1=deadbeef,v1={sig}");
        assert!(stripe_verify(secret, &multi, raw, t).is_ok());
        // kichwa kibovu
        assert!(stripe_verify(secret, "v1=abc", raw, t).is_err());
        assert!(stripe_verify(secret, "", raw, t).is_err());
    }

    #[test]
    fn hmac_hex_ni_ya_kawaida() {
        // RFC 4231 test case 2.
        let out = hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            out,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn lango_huchaguliwa_kwa_sarafu() {
        assert_eq!(gateway_for("TZS"), "mobile");
        assert_eq!(gateway_for("USD"), "stripe");
        assert_eq!(gateway_for("EUR"), "stripe");
    }

    #[test]
    fn mteja_wa_nje_haruhusiwi_tzs() {
        assert!(currency_allowed_for("TZ", "TZS"));
        assert!(currency_allowed_for("TZ", "USD"));
        assert!(currency_allowed_for("US", "USD"));
        assert!(!currency_allowed_for("US", "TZS"));
    }

    #[test]
    fn malipo_huthibitishwa_mara_moja() {
        assert!(can_confirm("pending"));
        assert!(!can_confirm("paid"));
        assert!(!can_confirm("failed"));
    }

    #[test]
    fn escrow_hairuhusu_kutolewa_zaidi_ya_iliyopo() {
        assert!(can_release(100_000, 100_000));
        assert!(can_release(100_000, 1));
        assert!(!can_release(100_000, 100_001));
        assert!(!can_release(0, 1));
        assert!(!can_release(100_000, 0));
    }

    #[test]
    fn stripe_bila_key_haiko_tayari() {
        std::env::remove_var("STRIPE_SECRET_KEY");
        assert!(stripe_key().is_none());
    }
}
