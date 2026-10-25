//! m291_email_phishing.rs — MODULE 291: EMAIL PHISHING
//!
//! Spec: "Inagundua barua pepe bandia. Twist: Inachambua header (SPF, DKIM, DMARC)."
//!
//! Injini halisi: huchanganua headers za uthibitisho (SPF/DKIM/DMARC), kutofautiana
//! kwa From vs Reply-To/Return-Path, na viungo hatarishi.

use super::common_smsemail::{extract_links, link_score, parse_email};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Alama ya phishing ya barua pepe (0-100).
pub fn email_phish_score(raw: &str) -> (u8, Vec<(&'static str, String)>, serde_json::Value) {
    let h = parse_email(raw);
    let mut s = 0u8;
    let mut why = Vec::new();

    let bad = |v: &str| matches!(v, "fail" | "softfail" | "none" | "permerror" | "temperror");
    if bad(&h.spf) {
        s = s.max(if h.spf == "fail" { 70 } else { 45 });
        why.push(("spf", format!("SPF: {} — mtumaji hajathibitishwa", if h.spf.is_empty() { "haipo" } else { &h.spf })));
    }
    if h.dkim == "fail" || (!h.has_dkim_sig && h.dkim.is_empty()) {
        s = s.max(50);
        why.push(("dkim", format!("DKIM: {}", if h.dkim.is_empty() { "haipo" } else { &h.dkim })));
    }
    if bad(&h.dmarc) {
        s = s.max(60);
        why.push(("dmarc", format!("DMARC: {}", if h.dmarc.is_empty() { "haipo" } else { &h.dmarc })));
    }
    // From vs Reply-To/Return-Path kutofautiana (uwezekano wa spoof).
    let fd = super::common_smsemail::domain_of(&format!("http://{}", h.from_addr));
    if !h.reply_to.is_empty() {
        let rd = super::common_smsemail::domain_of(&format!("http://{}", h.reply_to));
        if !fd.is_empty() && !rd.is_empty() && fd != rd {
            s = s.max(75);
            why.push(("reply_to_mismatch", format!("From domain {fd} ≠ Reply-To domain {rd}")));
        }
    }
    if !h.return_path.is_empty() {
        let rpd = super::common_smsemail::domain_of(&format!("http://{}", h.return_path));
        if !fd.is_empty() && !rpd.is_empty() && fd != rpd {
            s = s.max(55);
            why.push(("return_path_mismatch", format!("From {fd} ≠ Return-Path {rpd}")));
        }
    }
    // Viungo hatarishi kwenye mwili.
    for l in extract_links(raw) {
        let (ls, lw) = link_score(&l);
        if ls > s {
            s = ls;
            why.extend(lw);
        }
    }
    let detail = json!({ "from": h.from_addr, "from_name": h.from_name, "reply_to": h.reply_to,
        "return_path": h.return_path, "spf": h.spf, "dkim": h.dkim, "dmarc": h.dmarc,
        "received": h.received_count, "dkim_sig": h.has_dkim_sig });
    (s, why, detail)
}

pub struct M291EmailPhishing;

impl TsModule for M291EmailPhishing {
    fn id(&self) -> u16 {
        291
    }
    fn name(&self) -> &'static str {
        "Email Phishing"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huchanganua headers (SPF/DKIM/DMARC), From vs Reply-To/Return-Path, na \
         viungo hatarishi kugundua barua pepe bandia."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(291, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let (score, why, detail) = email_phish_score(raw);
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        r.findings.push(json!({ "kind": "header_audit", "severity": 1, "detail": detail }));
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Email Phishing: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_spoof_ya_from_replyto() {
        let raw = "From: \"CRDB\" <no-reply@crdb.co.tz>\r\nReply-To: attacker@evil.xyz\r\nSubject: Verify\r\nAuthentication-Results: spf=fail dkim=fail dmarc=fail\r\n";
        let (s, why, _) = email_phish_score(raw);
        assert!(s >= 70);
        assert!(why.iter().any(|(k, _)| *k == "reply_to_mismatch"));
    }

    #[test]
    fn barua_halali_haina_alama_nyingi() {
        let raw = "From: \"Juma\" <juma@benki.co.tz>\r\nReply-To: juma@benki.co.tz\r\nAuthentication-Results: spf=pass dkim=pass dmarc=pass\r\n";
        let (s, _, _) = email_phish_score(raw);
        assert!(s < 40, "score {s}");
    }
}
