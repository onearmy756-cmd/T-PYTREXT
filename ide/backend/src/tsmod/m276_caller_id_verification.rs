//! m276_caller_id_verification.rs — MODULE 276: CALLER ID VERIFICATION
//!
//! Spec: "Inathibitisha kama namba ni halisi. Twist: Inaonyesha '✓ Verified' badge."
//!
//! Injini halisi: huthibitisha namba dhidi ya rejista ya waendeshaji halali:
//!   - Prefix halali za waendeshaji wa Tanzania (MCC/MNC → Vodacom/Airtel/Tigo/Halotel).
//!   - Namba za huduma rasmi (shortcodes za benki/telco zilizothibitishwa).
//!   - Namba zilizothibitishwa kwenye DB (verified_numbers).
//!   - Inatoa beji "✓ Verified" tu ikiwa angalau vyanzo viwili vinakubali.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Prefix → mwendeshaji (Tanzania, E.164 national 9-digit).
pub fn operator_for(national: &str) -> Option<&'static str> {
    let p = &national[..national.len().min(3)];
    match p {
        "710" | "711" | "712" | "713" | "714" | "715" | "716" | "717" | "718" | "719" => Some("Vodacom"),
        "680" | "681" | "682" | "683" | "684" | "685" | "686" | "687" | "688" | "689" => Some("Airtel"),
        "650" | "651" | "652" | "653" | "654" | "655" | "656" | "657" | "658" | "659" => Some("Tigo"),
        "620" | "621" | "622" | "623" | "624" | "625" | "626" | "627" | "628" | "629" => Some("Halotel"),
        "730" | "731" | "732" | "733" | "734" | "735" | "736" | "737" | "738" | "739" => Some("TTCL"),
        "740" | "741" | "742" | "743" | "744" | "745" | "746" | "747" | "748" | "749" => Some("Vodacom"),
        "750" | "751" | "752" | "753" | "754" | "755" | "756" | "757" | "758" | "759" => Some("Airtel"),
        "760" | "761" | "762" | "763" | "764" | "765" | "766" | "767" | "768" | "769" => Some("Tigo"),
        "770" | "771" | "772" | "773" | "774" | "775" | "776" | "777" | "778" | "779" => Some("Halotel"),
        "780" | "781" | "782" | "783" | "784" | "785" | "786" | "787" | "788" | "789" => Some("Vodacom"),
        _ => None,
    }
}

pub struct M276CallerIdVerification;

#[async_trait::async_trait]
impl TsModule for M276CallerIdVerification {
    fn id(&self) -> u16 {
        276
    }
    fn name(&self) -> &'static str {
        "Caller ID Verification"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huthibitisha namba dhidi ya rejista ya waendeshaji (prefix halali) na \
         namba zilizothibitishwa; hutoa beji '✓ Verified' tu ikiwa vyanzo vinakubali."
    }

    fn run(&self, _target: &str, _params: &Value) -> Result<ModReport, String> {
        Err("tumia run_db (module hii inahitaji DB)".into())
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(276, self.name());
        let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
        let (cc, national, valid) = super::m275_spoofing_detection::parse_e164(number);
        let mut signals = 0;

        // 1) E.164 halali + country code ya TZ.
        let cc_ok = cc.as_deref() == Some("255");
        if valid && cc_ok {
            signals += 1;
        } else {
            r.finding("e164_fail", format!("Namba si E.164 halali ya TZ: {number}"), 45);
        }
        // 2) Prefix inajulikana (mwendeshaji halali).
        let op = operator_for(&national);
        if let Some(o) = op {
            signals += 1;
            r.finding("operator_known", format!("Mwendeshaji: {o}"), 1);
        } else {
            r.finding("operator_unknown", format!("Prefix haijulikani: {national}"), 40);
        }
        // 3) Ipo kwenye rejista ya namba zilizothibitishwa (DB).
        let verified: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM ts_iocs WHERE kind='verified_number' AND value=? LIMIT 1",
        )
        .bind(number)
        .fetch_optional(db)
        .await
        .ok()
        .flatten();
        if verified.is_some() {
            signals += 1;
            r.finding("registry_verified", "Namba ipo kwenye rejista iliyothibitishwa", 1);
        }
        // 4) Je, ni namba ya huduma rasmi (shortcode)? (3-5 tarakimu).
        let digits: String = number.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() <= 6 && !digits.is_empty() {
            signals += 1;
            r.finding("shortcode", format!("Namba fupi ya huduma: {number}"), 1);
        }

        let badge = signals >= 2;
        r.findings.push(json!({
            "kind": "verified_badge", "severity": if badge { 1 } else { 35 },
            "detail": { "badge": if badge { "✓ Verified" } else { "—" }, "signals": signals,
                        "operator": op, "confidence": signals * 30 }
        }));
        r.summary = format!(
            "Caller ID Verification: {number} → {} (vyanzo {signals}).",
            if badge { "✓ Verified" } else { "Haijathibitishwa" }
        );
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_lookup() {
        assert_eq!(operator_for("712345678"), Some("Vodacom"));
        assert_eq!(operator_for("682123456"), Some("Airtel"));
        assert_eq!(operator_for("650123456"), Some("Tigo"));
        assert_eq!(operator_for("999123456"), None);
    }
}
