//! m283_voip_security.rs — MODULE 283: VOIP SECURITY
//!
//! Spec: "Inalinda VoIP calls (SIP). Twist: Inathibitisha SIP server."
//!
//! Injini halisi ya uchanganuzi wa SIP (RFC 3261):
//!   - Hugundua REGISTER/INVITE bila uthibitisho (401/407 digest).
//!   - Hukagua TLS (sips:) vs UDP/TCP isiyo salama.
//!   - Hugundua SIP scanning/fuzzing (methods zisizo za kawaida, Via spoofing).
//!   - Huthibitisha SIP server dhidi ya orodha ya waendeshaji halali (allowlist).

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Uchanganuzi wa ujumbe mmoja wa SIP.
#[derive(Debug, Default, Clone)]
pub struct SipAudit {
    pub method: String,
    pub is_tls: bool,
    pub has_auth: bool,
    pub via_count: usize,
    pub contact_mismatch: bool,
    pub unknown_method: bool,
    pub user_agent: String,
}

const VALID_METHODS: &[&str] = &["REGISTER", "INVITE", "ACK", "BYE", "CANCEL", "OPTIONS", "PRACK", "SUBSCRIBE", "NOTIFY", "PUBLISH", "INFO", "REFER", "MESSAGE", "UPDATE"];

pub fn audit_sip(msg: &str) -> SipAudit {
    let mut a = SipAudit::default();
    let mut lines = msg.lines();
    if let Some(first) = lines.next() {
        a.method = first.split_whitespace().next().unwrap_or("").to_uppercase();
        a.is_tls = first.to_uppercase().contains("SIPS:") || first.to_uppercase().contains("SIPS ");
        a.unknown_method = !a.method.is_empty() && !VALID_METHODS.contains(&a.method.as_str());
    }
    let mut from_user = String::new();
    let mut contact = String::new();
    for l in msg.lines() {
        let low = l.to_lowercase();
        if low.starts_with("authorization:") || low.starts_with("proxy-authorization:") {
            a.has_auth = true;
        }
        if low.starts_with("via:") {
            a.via_count += 1;
        }
        if low.starts_with("from:") {
            if let Some(u) = extract_sip_user(l) {
                from_user = u;
            }
        }
        if low.starts_with("contact:") {
            if let Some(u) = extract_sip_user(l) {
                contact = u;
            }
        }
        if low.starts_with("user-agent:") {
            a.user_agent = l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
        }
    }
    a.contact_mismatch = !from_user.is_empty() && !contact.is_empty() && from_user != contact;
    a
}

fn extract_sip_user(line: &str) -> Option<String> {
    let at = line.find('@')?;
    let before = &line[..at];
    let start = before.rfind([':', '<', ' ']).map(|i| i + 1).unwrap_or(0);
    Some(before[start..].to_string())
}

/// Thibitisha SIP server dhidi ya allowlist.
pub fn verify_server(host: &str, allowlist: &[String]) -> bool {
    allowlist.iter().any(|a| a.eq_ignore_ascii_case(host))
}

pub struct M283VoipSecurity;

impl TsModule for M283VoipSecurity {
    fn id(&self) -> u16 {
        283
    }
    fn name(&self) -> &'static str {
        "VoIP Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganuzi SIP (RFC 3261): uthibitisho, TLS, Via spoofing, methods \
         zisizo za kawaida — na huthibitisha SIP server dhidi ya allowlist."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(283, self.name());
        let mut score = 0u8;
        if let Some(msg) = params.get("sip").and_then(|v| v.as_str()) {
            let a = audit_sip(msg);
            if a.unknown_method {
                score = score.max(70);
                r.finding("sip_unknown_method", format!("Method isiyo ya kawaida: {}", a.method), 70);
            }
            if !a.has_auth && matches!(a.method.as_str(), "REGISTER" | "INVITE") {
                score = score.max(65);
                r.finding("sip_no_auth", format!("{} bila uthibitisho (digest)", a.method), 65);
            }
            if !a.is_tls {
                score = score.max(50);
                r.finding("sip_no_tls", "SIP haitumii TLS (sips:)", 50);
            }
            if a.via_count > 3 {
                score = score.max(60);
                r.finding("sip_via_spoof", format!("Via nyingi ({}) — uwezekano wa spoofing", a.via_count), 60);
            }
            if a.contact_mismatch {
                score = score.max(55);
                r.finding("sip_contact_mismatch", "From na Contact hazifanani — uwezekano wa kuiga", 55);
            }
            r.findings.push(json!({ "kind": "sip_audit", "severity": 1, "detail": {
                "method": a.method, "tls": a.is_tls, "auth": a.has_auth, "via": a.via_count, "ua": a.user_agent } }));
        }
        // Uthibitisho wa server.
        if let Some(host) = params.get("server").and_then(|v| v.as_str()) {
            let allow: Vec<String> = params
                .get("allowlist")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let ok = allow.is_empty() || verify_server(host, &allow);
            if !ok {
                score = score.max(80);
                r.finding("sip_server_untrusted", format!("SIP server '{host}' haipo kwenye allowlist"), 80);
            } else {
                r.finding("sip_server_ok", format!("SIP server '{host}' imethibitishwa"), 1);
            }
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("VoIP Security: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_sip_bila_auth_na_tls() {
        let msg = "INVITE sip:bob@biloxi.com SIP/2.0\r\nVia: SIP/2.0/UDP pc33.atlanta.com\r\nFrom: <sip:alice@atlanta.com>\r\nContact: <sip:alice@atlanta.com>\r\n";
        let a = audit_sip(msg);
        assert_eq!(a.method, "INVITE");
        assert!(!a.has_auth && !a.is_tls);
        assert!(!a.contact_mismatch);
    }

    #[test]
    fn inagundua_method_ya_kigeni() {
        let a = audit_sip("FUZZ sip:x@y SIP/2.0\r\n");
        assert!(a.unknown_method);
    }

    #[test]
    fn server_verification() {
        let allow = vec!["sip.vodacom.co.tz".to_string()];
        assert!(verify_server("SIP.VODACOM.CO.TZ", &allow));
        assert!(!verify_server("evil.example", &allow));
    }
}
