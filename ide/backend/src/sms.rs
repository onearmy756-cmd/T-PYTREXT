//! sms.rs — SMS YA KAMPUNI: kutuma taarifa + kupokea maamuzi ya admin.
//!
//! Mahitaji ya mmiliki: "kama kuna request yoyote admin ataona request zote kwa
//! normal sms na kuingiza kwenye mfumo ili kuidhinisha na kusign mikataba".
//!
//! Kanuni halisi (hakuna kubuni):
//!   - Kutuma ni HALISI tu ikiwa mtoa huduma amewekwa (`SMS_API_URL` +
//!     `SMS_API_KEY`). Bila usanidi, ujumbe huingia kwenye `sms_outbox` kama
//!     `queued` na API huripoti `configured: false` — HATUANDIKI "imetumwa"
//!     kwa ujumbe ambao haujatumwa.
//!   - Kupokea: webhook `/api/sms/inbound` hulindwa kwa token
//!     (`SMS_INBOUND_TOKEN`); bila token iliyowekwa, webhook hukataliwa.
//!   - Ujumbe unaoingia huchambuliwa kwa amri: `APPROVE <ref>`, `REJECT <ref>`,
//!     `SIGN <ref>` → huingizwa kwenye `sms_inbox` + `sms_commands` kwa admin
//!     kuidhinisha kwenye UI.
//!
//! Tables: sms_outbox sms_inbox sms_commands

use crate::AppState;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS sms_outbox (
            id TEXT PRIMARY KEY,
            to_msisdn TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'info',
            ref TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'queued', -- queued|sent|failed
            provider_ref TEXT NOT NULL DEFAULT '',
            error TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            sent_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS sms_inbox (
            id TEXT PRIMARY KEY,
            from_msisdn TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'received', -- received|parsed|applied|ignored
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS sms_commands (
            id TEXT PRIMARY KEY,
            inbox_id TEXT NOT NULL DEFAULT '',
            from_msisdn TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL DEFAULT '',   -- approve|reject|sign|unknown
            ref TEXT NOT NULL DEFAULT '',
            raw TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'pending', -- pending|applied|ignored
            created_at TEXT NOT NULL,
            applied_at TEXT
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sms/status", get(status))
        .route("/api/sms/send", post(send))
        .route("/api/sms/outbox", get(outbox_list))
        .route("/api/sms/inbox", get(inbox_list))
        .route("/api/sms/commands", get(commands_list))
        .route("/api/sms/inbound", post(inbound))
        .route("/api/sms/command/:id/apply", post(command_apply))
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Mtoa huduma wa SMS kutoka mazingira. `None` = haujawekwa.
pub fn provider() -> Option<(String, String, String)> {
    let url = std::env::var("SMS_API_URL").ok()?.trim().to_string();
    if url.is_empty() {
        return None;
    }
    let key = std::env::var("SMS_API_KEY").unwrap_or_default();
    let sender = std::env::var("SMS_SENDER").unwrap_or_else(|_| "MTECHOS".into());
    Some((url, key, sender))
}

/// Namba ya simu ya admin kwa taarifa (mazingira, si DB).
pub fn admin_phone() -> Option<String> {
    std::env::var("FUNDI_ADMIN_PHONE")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Namba ya simu ya Tanzania? (inakubali `+255…`, `255…`, `0…`).
pub fn tz_phone_ok(p: &str) -> bool {
    let d: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
    let local = if let Some(r) = d.strip_prefix("255") {
        r.to_string()
    } else if let Some(r) = d.strip_prefix('0') {
        r.to_string()
    } else {
        d.clone()
    };
    if local.len() != 9 {
        return false;
    }
    matches!(
        &local[..2],
        "71" | "72" | "73" | "74" | "75" | "76" | "77" | "78" | "79" | "61" | "62" | "63" | "64"
            | "65" | "66" | "67" | "68" | "69"
    )
}

/// Namba mbili ni sawa? Hulinganisha tarakimu pekee (hupuuza `+`, nafasi, `0`
/// ya mwanzo na kiambishi cha nchi `255`) — hivyo `+255712000000`, `0712000000`
/// na `255712000000` hutambulika kama namba moja.
pub fn same_phone(a: &str, b: &str) -> bool {
    fn canon(p: &str) -> String {
        let d: String = p.chars().filter(|c| c.is_ascii_digit()).collect();
        let d = d.strip_prefix("255").unwrap_or(&d).to_string();
        d.strip_prefix('0').unwrap_or(&d).to_string()
    }
    let (x, y) = (canon(a), canon(b));
    !x.is_empty() && x == y
}

/// Namba ya kimataifa (E.164) — inaanza `+` na tarakimu 8–15.
pub fn intl_phone_ok(p: &str) -> bool {
    let t = p.trim();
    let Some(rest) = t.strip_prefix('+') else {
        return false;
    };
    let n = rest.chars().filter(|c| c.is_ascii_digit()).count();
    n >= 8 && n <= 15 && rest.chars().all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
}

/// Namba yoyote inayokubalika (TZ au kimataifa).
pub fn phone_ok(p: &str) -> bool {
    tz_phone_ok(p) || intl_phone_ok(p)
}

/// Amri kutoka kwa SMS ya admin → (action, ref).
pub fn parse_command(body: &str) -> (&'static str, String) {
    let t = body.trim();
    let mut parts = t.split_whitespace();
    let Some(first) = parts.next() else {
        return ("unknown", String::new());
    };
    let action = match first.to_ascii_uppercase().as_str() {
        "APPROVE" | "IDHINISHA" | "KUBALI" => "approve",
        "REJECT" | "KATAA" | "KATA" => "reject",
        "SIGN" | "TIA" | "SAINI" => "sign",
        _ => "unknown",
    };
    let ref_id = if action == "unknown" {
        String::new()
    } else {
        parts.next().unwrap_or("").trim().to_ascii_uppercase()
    };
    (action, ref_id)
}

/// Aina za taarifa (kwa ufuatiliaji/UI).
pub const KINDS: [&str; 6] = ["info", "request", "assignment", "approval", "payment", "contract"];

/// Weka ujumbe kwenye foleni NA ujaribu kutuma ikiwa mtoa huduma amewekwa.
/// Hurudisha `(id, sent)` — `sent=false` maana yake bado kwenye foleni.
pub async fn notify(
    db: &SqlitePool,
    to: &str,
    body: &str,
    kind: &str,
    reference: &str,
) -> (String, bool) {
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO sms_outbox (id,to_msisdn,body,kind,ref,status,provider_ref,error,created_at)
         VALUES (?1,?2,?3,?4,?5,'queued','','',?6)",
    )
    .bind(&id)
    .bind(to.trim())
    .bind(body)
    .bind(kind)
    .bind(reference)
    .bind(now())
    .execute(db)
    .await;
    let sent = deliver(db, &id, to, body).await;
    (id, sent)
}

/// Tuma ujumbe mmoja kwa mtoa huduma (ikiwa amewekwa). Hurudisha `true` ikiwa
/// mtoa huduma amethibitisha kukubali.
async fn deliver(db: &SqlitePool, id: &str, to: &str, body: &str) -> bool {
    let Some((url, key, sender)) = provider() else {
        return false;
    };
    let client = reqwest::Client::new();
    let payload = json!({"to": to, "sender": sender, "message": body});
    let mut req = client.post(&url).json(&payload);
    if !key.is_empty() {
        req = req.bearer_auth(&key);
    }
    match req.timeout(std::time::Duration::from_secs(20)).send().await {
        Ok(resp) if resp.status().is_success() => {
            let pref = resp
                .json::<Value>()
                .await
                .ok()
                .and_then(|v| {
                    v["id"]
                        .as_str()
                        .or(v["message_id"].as_str())
                        .map(str::to_string)
                })
                .unwrap_or_default();
            let _ = sqlx::query(
                "UPDATE sms_outbox SET status='sent', provider_ref=?2, sent_at=?3 WHERE id=?1",
            )
            .bind(id)
            .bind(&pref)
            .bind(now())
            .execute(db)
            .await;
            true
        }
        Ok(resp) => {
            let code = resp.status().as_u16();
            let _ = sqlx::query("UPDATE sms_outbox SET status='failed', error=?2 WHERE id=?1")
                .bind(id)
                .bind(format!("HTTP {code}"))
                .execute(db)
                .await;
            false
        }
        Err(e) => {
            let _ = sqlx::query("UPDATE sms_outbox SET status='failed', error=?2 WHERE id=?1")
                .bind(id)
                .bind(e.to_string())
                .execute(db)
                .await;
            false
        }
    }
}

/// Taarifa kwa admin kuhusu ombi jipya (SMS ya kawaida).
pub async fn notify_admin_new_request(
    db: &SqlitePool,
    reference: &str,
    title: &str,
    amount: &str,
    currency: &str,
) -> Option<String> {
    let to = admin_phone()?;
    let body = format!(
        "MTECHOS: ombi jipya {reference} — {title} ({amount} {currency}). Idhinisha kwenye mfumo."
    );
    let (id, _) = notify(db, &to, &body, "request", reference).await;
    Some(id)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let (queued,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sms_outbox WHERE status='queued'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (sent,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sms_outbox WHERE status='sent'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (failed,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sms_outbox WHERE status='failed'")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (inbox,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sms_inbox")
        .fetch_one(&s.db)
        .await
        .unwrap_or((0,));
    let (pending_cmds,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sms_commands WHERE status='pending'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "configured": provider().is_some(),
        "sender": provider().map(|(_,_,s)| s).unwrap_or_default(),
        "admin_phone": admin_phone().unwrap_or_default(),
        "queued": queued, "sent": sent, "failed": failed,
        "inbox": inbox, "pending_commands": pending_cmds,
        "kinds": KINDS,
    }))
}

#[derive(Deserialize)]
pub struct SendIn {
    pub to: String,
    pub body: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub r#ref: String,
}

async fn send(State(s): State<AppState>, Json(b): Json<SendIn>) -> Json<Value> {
    if !phone_ok(&b.to) {
        return Json(json!({"ok": false, "error": "namba ya simu si sahihi"}));
    }
    if b.body.trim().is_empty() {
        return Json(json!({"ok": false, "error": "ujumbe hauwezi kuwa tupu"}));
    }
    let kind = if KINDS.contains(&b.kind.as_str()) {
        b.kind.as_str()
    } else {
        "info"
    };
    let (id, sent) = notify(&s.db, &b.to, &b.body, kind, &b.r#ref).await;
    Json(json!({
        "ok": true, "id": id, "sent": sent,
        "status": if sent { "sent" } else { "queued" },
        "note": if sent { "imefika kwa mtoa huduma" } else { "mtoa huduma hajawekwa — ujumbe uko kwenye foleni" }
    }))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

async fn outbox_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = if let Some(st) = q.status.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
            "SELECT id,to_msisdn,body,kind,ref,status,provider_ref,created_at FROM sms_outbox
             WHERE status=?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(st)
        .bind(lim)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as::<_, (String, String, String, String, String, String, String, String)>(
            "SELECT id,to_msisdn,body,kind,ref,status,provider_ref,created_at FROM sms_outbox
             ORDER BY created_at DESC LIMIT ?1",
        )
        .bind(lim)
        .fetch_all(&s.db)
        .await
        .unwrap_or_default()
    };
    Json(json!({"ok": true, "messages": rows.into_iter().map(|r| json!({
        "id": r.0, "to": r.1, "body": r.2, "kind": r.3, "ref": r.4,
        "status": r.5, "provider_ref": r.6, "created_at": r.7
    })).collect::<Vec<_>>()}))
}

async fn inbox_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id,from_msisdn,body,status,created_at FROM sms_inbox ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "inbox": rows.into_iter().map(|r| json!({
        "id": r.0, "from": r.1, "body": r.2, "status": r.3, "created_at": r.4
    })).collect::<Vec<_>>()}))
}

async fn commands_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
        "SELECT id,from_msisdn,action,ref,raw,status,created_at FROM sms_commands
         ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "commands": rows.into_iter().map(|r| json!({
        "id": r.0, "from": r.1, "action": r.2, "ref": r.3, "raw": r.4,
        "status": r.5, "created_at": r.6
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
pub struct InboundIn {
    pub from: String,
    pub body: String,
    #[serde(default)]
    pub received_at: Option<String>,
}

/// Webhook: SMS inayoingia. Hulindwa kwa `SMS_INBOUND_TOKEN`.
async fn inbound(
    State(s): State<AppState>,
    headers: HeaderMap,
    Json(b): Json<InboundIn>,
) -> Json<Value> {
    let expected = std::env::var("SMS_INBOUND_TOKEN").unwrap_or_default();
    if expected.trim().is_empty() {
        return Json(json!({
            "ok": false,
            "error": "webhook imezimwa: weka SMS_INBOUND_TOKEN kwenye mazingira kwanza"
        }));
    }
    let got = headers
        .get("x-sms-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !constant_eq(got, expected.trim()) {
        return Json(json!({"ok": false, "error": "token si sahihi"}));
    }
    // USALAMA: amri za idhini hutoka kwa Admin Mkuu pekee. Namba nyingine
    // hurekodiwa kwenye inbox lakini HAZIINGIZWI kama amri.
    let admin_only = admin_phone();
    let from_ok = match &admin_only {
        Some(a) => same_phone(a, &b.from),
        None => false,
    };
    let inbox_id = uid();
    let (action, reference) = parse_command(&b.body);
    let status = if action == "unknown" {
        "ignored"
    } else if !from_ok {
        "rejected"
    } else {
        "parsed"
    };
    let _ = sqlx::query(
        "INSERT INTO sms_inbox (id,from_msisdn,body,status,created_at) VALUES (?1,?2,?3,?4,?5)",
    )
    .bind(&inbox_id)
    .bind(b.from.trim())
    .bind(&b.body)
    .bind(status)
    .bind(b.received_at.clone().unwrap_or_else(now))
    .execute(&s.db)
    .await;
    if action == "unknown" {
        return Json(json!({"ok": true, "inbox_id": inbox_id, "action": "unknown", "queued": false}));
    }
    if !from_ok {
        return Json(json!({
            "ok": false, "inbox_id": inbox_id, "action": action, "queued": false,
            "error": "namba hii si ya Admin Mkuu — amri haikubaliwi"
        }));
    }
    let cmd_id = uid();
    let _ = sqlx::query(
        "INSERT INTO sms_commands (id,inbox_id,from_msisdn,action,ref,raw,status,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,'pending',?7)",
    )
    .bind(&cmd_id)
    .bind(&inbox_id)
    .bind(b.from.trim())
    .bind(action)
    .bind(&reference)
    .bind(&b.body)
    .bind(now())
    .execute(&s.db)
    .await;
    Json(json!({
        "ok": true, "inbox_id": inbox_id, "command_id": cmd_id,
        "action": action, "ref": reference, "queued": true,
        "note": "amri imeingizwa kwenye mfumo — admin aidhinisha kwenye UI"
    }))
}

/// Ulinganisho wa token kwa muda usiobadilika (constant-time) — kuzuia timing attack.
pub fn constant_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

async fn command_apply(State(s): State<AppState>, axum::extract::Path(id): axum::extract::Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT action,ref,status,raw FROM sms_commands WHERE id=?1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((action, reference, status, raw)) = row else {
        return Json(json!({"ok": false, "error": "amri haipo"}));
    };
    if status != "pending" {
        return Json(json!({"ok": false, "error": format!("amri imekwisha kushughulikiwa ({status})")}));
    }
    let outcome = crate::biashara::apply_sms_command(&s.db, &action, &reference).await;
    let new_status = if outcome["ok"].as_bool().unwrap_or(false) {
        "applied"
    } else {
        "ignored"
    };
    let _ = sqlx::query("UPDATE sms_commands SET status=?2, applied_at=?3 WHERE id=?1")
        .bind(&id)
        .bind(new_status)
        .bind(now())
        .execute(&s.db)
        .await;
    Json(json!({
        "ok": outcome["ok"].as_bool().unwrap_or(false),
        "action": action, "ref": reference, "status": new_status,
        "raw": raw, "result": outcome
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namba_za_simu_hukaguliwa() {
        assert!(tz_phone_ok("0712345678"));
        assert!(tz_phone_ok("+255712345678"));
        assert!(!tz_phone_ok("0212345678"));
        assert!(intl_phone_ok("+14155552671"));
        assert!(intl_phone_ok("+442071234567"));
        assert!(!intl_phone_ok("14155552671"));
        assert!(phone_ok("0712345678"));
        assert!(phone_ok("+14155552671"));
        assert!(!phone_ok("abc"));
    }

    #[test]
    fn namba_ya_admin_hulinganishwa_kwa_umbo_zote() {
        assert!(same_phone("+255712000000", "0712000000"));
        assert!(same_phone("255712000000", "+255 712 000 000"));
        assert!(same_phone("0712000000", "0712000000"));
        assert!(!same_phone("+255712000000", "+255700000000"));
        assert!(!same_phone("", "0712000000"));
    }

    #[test]
    fn amri_hutambuliwa() {
        assert_eq!(parse_command("APPROVE REQ-001"), ("approve", "REQ-001".into()));
        assert_eq!(parse_command("idhinisha req-001"), ("approve", "REQ-001".into()));
        assert_eq!(parse_command("REJECT REQ-002"), ("reject", "REQ-002".into()));
        assert_eq!(parse_command("kataa REQ-002"), ("reject", "REQ-002".into()));
        assert_eq!(parse_command("SIGN CON-7"), ("sign", "CON-7".into()));
        assert_eq!(parse_command("habari yako"), ("unknown", String::new()));
        assert_eq!(parse_command(""), ("unknown", String::new()));
    }

    #[test]
    fn ulinganisho_wa_token_ni_wa_muda_usobadilika() {
        assert!(constant_eq("abc123", "abc123"));
        assert!(!constant_eq("abc123", "abc124"));
        assert!(!constant_eq("abc", "abcd"));
        assert!(!constant_eq("", "x"));
    }

    #[test]
    fn mtoa_huduma_hutoka_mazingira() {
        // Bila SMS_API_URL, mtoa huduma hayupo — hakuna "sent" ya uongo.
        std::env::remove_var("SMS_API_URL");
        assert!(provider().is_none());
    }
}
