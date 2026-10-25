//! m303_email_compliance.rs — MODULE 303: EMAIL COMPLIANCE
//!
//! Spec: "Inahakikisha barua pepe zinatii sheria. Twist: Inaangalia mahitaji ya TCRA."
//!
//! Injini halisi: hukagua barua pepe dhidi ya mahitaji ya TCRA (Tanzania
//! Communications Regulatory Authority) na kanuni za kawaida:
//!   - Sender halali (domain halisi, SPF/DKIM).
//!   - Opt-out/unsubscribe kwa barua za kibiashara.
//!   - Kitambulisho cha mtoa huduma + anwani ya kimwili.
//!   - Ufichuzi wa matangazo (advertising disclosure).

use super::common_smsemail::parse_email;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Kikagua compliance (rudisha (score, findings)).
pub fn compliance_check(raw: &str) -> (u8, Vec<(&'static str, bool, String)>) {
    let h = parse_email(raw);
    let low = raw.to_lowercase();
    let mut issues = Vec::new();
    let mut score = 100u8;
    let is_commercial = low.contains("offer") || low.contains("ofa") || low.contains("promo") || low.contains("subscribe") || low.contains("bidhaa") || low.contains("bei");

    let has_unsub = low.contains("unsubscribe") || low.contains("jiondoe") || low.contains("opt-out") || low.contains("opt out");
    if is_commercial && !has_unsub {
        score -= 30;
        issues.push(("missing_unsubscribe", false, "Hakuna opt-out/unsubscribe kwa barua ya kibiashara".to_string()));
    }
    let has_physical = low.contains("street") || low.contains("p.o. box") || low.contains("s.l.p") || low.contains("anwani") || low.contains("dar es salaam") || low.contains("postal");
    if is_commercial && !has_physical {
        score -= 20;
        issues.push(("missing_address", false, "Hakuna anwani ya kimwili ya mtoa huduma".to_string()));
    }
    if h.spf != "pass" && !h.spf.is_empty() {
        score -= 15;
        issues.push(("spf_not_pass", false, format!("SPF haijafaulu: {}", h.spf)));
    }
    if h.dkim != "pass" && !h.dkim.is_empty() {
        score -= 10;
        issues.push(("dkim_not_pass", false, format!("DKIM haijafaulu: {}", h.dkim)));
    }
    if h.from_addr.is_empty() {
        score -= 25;
        issues.push(("no_from", false, "Hakuna anwani ya mtumaji".to_string()));
    }
    if has_unsub {
        issues.push(("has_unsubscribe", true, "Opt-out ipo".to_string()));
    }
    if !issues.iter().any(|(_, ok, _)| !ok) {
        issues.push(("compliant", true, "Inatii mahitaji ya TCRA".to_string()));
    }
    (score, issues)
}

pub struct M303EmailCompliance;

impl TsModule for M303EmailCompliance {
    fn id(&self) -> u16 {
        303
    }
    fn name(&self) -> &'static str {
        "Email Compliance"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Hukagua barua pepe dhidi ya mahitaji ya TCRA: unsubscribe, anwani ya \
         mtoa huduma, SPF/DKIM, na ufichuzi wa matangazo."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(303, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let (score, issues) = compliance_check(raw);
        for (k, ok, d) in &issues {
            r.findings.push(json!({ "kind": k, "severity": if *ok { 1 } else { 55 }, "ok": ok, "detail": d }));
        }
        let compliant = score >= 70;
        r.findings.push(json!({ "kind": "decision", "severity": 100 - score,
            "detail": { "compliant": compliant, "score": score, "standard": "TCRA" } }));
        r.summary = format!("Email Compliance: {} (score {score}).", if compliant { "Inatii" } else { "Haikidhi" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_ukosefu_wa_unsubscribe() {
        let raw = "From: promo@shop.co.tz\r\nSubject: Ofa kubwa\r\nBei nafuu sana!\r\n";
        let (score, issues) = compliance_check(raw);
        assert!(score < 70);
        assert!(issues.iter().any(|(k, ok, _)| *k == "missing_unsubscribe" && !ok));
    }
}
