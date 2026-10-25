//! m294_sms_link_scanner.rs — MODULE 294: SMS LINK SCANNER
//!
//! Spec: "Inachambua viungo kwenye SMS. Twist: Inabonyeza link kwenye sandbox."
//!
//! Injini halisi: huchanganua kila kiungo kwenye SMS kwa sandbox (muundo/hatari):
//! viungo vilivyofupishwa, IP, punycode, TLD hatarishi, ombi la nywila, upakuaji.

use super::common_smsemail::{extract_links, is_ip_url, is_punycode, is_shortener, risky_tld};
use super::m289_sms_phishing::sandbox_scan;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M294SmsLinkScanner;

impl TsModule for M294SmsLinkScanner {
    fn id(&self) -> u16 {
        294
    }
    fn name(&self) -> &'static str {
        "SMS Link Scanner"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huchanganua kila kiungo kwenye SMS kwa sandbox: viungo vifupi, IP, \
         punycode, TLD hatarishi, upakuaji — bila kutekeleza ombi la mtandao."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(294, self.name());
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or(target);
        let links = extract_links(text);
        if links.is_empty() {
            r.finding("no_links", "Hakuna viungo kwenye SMS", 1);
        }
        let mut worst = 0u8;
        for l in &links {
            let sb = sandbox_scan(l);
            worst = worst.max(sb.score);
            r.findings.push(json!({ "kind": "link_scan", "severity": sb.score.max(1),
                "detail": { "url": sb.url, "domain": sb.domain, "score": sb.score,
                            "shortener": is_shortener(l), "ip": is_ip_url(l), "punycode": is_punycode(l),
                            "risky_tld": risky_tld(l), "asks_credentials": sb.asks_credentials,
                            "downloads_file": sb.downloads_file, "reasons": sb.reasons } }));
        }
        let decision = if worst >= 70 { "block" } else if worst >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": worst, "detail": { "decision": decision, "links": links.len() } }));
        r.summary = format!("SMS Link Scanner: viungo {} → {decision} (score {worst}).", links.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_kiungo_hatarishi() {
        let r = M294SmsLinkScanner.run("Bofya http://192.168.0.1/verify", &json!({})).unwrap();
        let dec = r.findings.iter().find(|f| f["kind"] == "decision").unwrap();
        assert_eq!(dec["detail"]["decision"], "block");
    }
}
