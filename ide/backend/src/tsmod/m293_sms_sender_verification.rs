//! m293_sms_sender_verification.rs — MODULE 293: SMS SENDER VERIFICATION
//!
//! Spec: "Inathibitisha kama namba ya mtumaji ni halisi. Twist: Inaonyesha '✓ Verified' badge."
//!
//! Injini halisi: huthibitisha sender ID (alphanumeric au namba) dhidi ya rejista
//! ya waendeshaji (prefix) na sender IDs zilizosajiliwa (shortcode/brand).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Sender ID ni alphanumeric (brand) au namba?
pub fn sender_kind(sender: &str) -> &'static str {
    let digits = sender.chars().filter(|c| c.is_ascii_digit()).count();
    let alphas = sender.chars().filter(|c| c.is_ascii_alphabetic()).count();
    if alphas > 0 && digits > 0 {
        "mixed"
    } else if alphas > 0 {
        "alphanumeric"
    } else if digits > 0 {
        "numeric"
    } else {
        "unknown"
    }
}

pub struct M293SmsSenderVerification;

#[async_trait::async_trait]
impl TsModule for M293SmsSenderVerification {
    fn id(&self) -> u16 {
        293
    }
    fn name(&self) -> &'static str {
        "SMS Sender Verification"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huthibitisha sender ID (brand/shortcode/namba) dhidi ya rejista; hutoa \
         beji '✓ Verified' kwa waendeshaji na sender IDs halali."
    }

    fn run(&self, _target: &str, _params: &Value) -> Result<ModReport, String> {
        Err("tumia run_db (module hii inahitaji DB)".into())
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(293, self.name());
        let sender = params.get("sender").and_then(|v| v.as_str()).unwrap_or(target);
        let kind = sender_kind(sender);
        let mut signals = 0;
        r.finding("sender_kind", format!("Aina ya sender: {kind}"), 1);
        // Namba ya TZ: prefix inajulikana?
        let digits: String = sender.chars().filter(|c| c.is_ascii_digit()).collect();
        if kind == "numeric" {
            let (cc, nat, valid) = super::m275_spoofing_detection::parse_e164(sender);
            if valid && cc.as_deref() == Some("255") && super::m276_caller_id_verification::operator_for(&nat).is_some() {
                signals += 2;
                r.finding("operator_known", "Namba ya mwendeshaji halali", 1);
            }
        }
        // Sender ID iliyosajiliwa (DB).
        let reg: Option<(i64,)> = sqlx::query_as("SELECT id FROM ts_iocs WHERE kind='verified_sender' AND value=? LIMIT 1")
            .bind(sender)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
        if reg.is_some() {
            signals += 2;
            r.finding("registered_sender", "Sender ID imesajiliwa", 1);
        }
        let badge = signals >= 2;
        r.findings.push(json!({ "kind": "verified_badge", "severity": if badge { 1 } else { 35 },
            "detail": { "badge": if badge { "✓ Verified" } else { "—" }, "signals": signals, "kind": kind } }));
        r.summary = format!("SMS Sender Verification: {sender} → {} (vyanzo {signals}).", if badge { "✓ Verified" } else { "Haijathibitishwa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aina_ya_sender() {
        assert_eq!(sender_kind("CRDBBank"), "alphanumeric");
        assert_eq!(sender_kind("+255712345678"), "numeric");
        assert_eq!(sender_kind("Voda123"), "mixed");
    }
}
