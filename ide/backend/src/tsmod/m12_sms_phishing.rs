//! m12_sms_phishing.rs — MODULE 12: SMS Phishing Detection
//!
//! Spec: "Inagundua SMS za ulaghai (smishing)."
//! Twist: AI inachambua link (URL) kwenye sandbox kabla ya kuipeleka kwa mteja.
//!
//! Injini HALISI: scoring ya smishing (maneno + ombi la pesa + namba) na uchambuzi wa URL (IP/TLD/@).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_sms::{smish_score, extract_urls, url_risk};

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(12, "SMS Phishing Detection");
    let body = params.get("body").and_then(|v| v.as_str()).unwrap_or(target);
    let from = params.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let (score, why) = smish_score(body);
    for (k, d) in &why { r.finding(k, d.clone(), score); }
    for u in extract_urls(body) {
        let (us, uw) = url_risk(&u);
        r.finding("url_sandbox", format!("{u} -> {uw}"), us);
    }
    let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
    r.finding("decision", format!("SMS kutoka {from} -> {decision} (score {score})"), score);
    r.summary = format!("SMS Phishing: score {score} -> {decision}.");
    Ok(r)
}

pub struct M12SmsPhishing;

#[async_trait::async_trait]
impl TsModule for M12SmsPhishing {
    fn id(&self) -> u16 { 12 }
    fn name(&self) -> &'static str { "SMS Phishing Detection" }
    fn group(&self) -> &'static str { "sms" }
    fn about(&self) -> &'static str { "Inachambua maneno ya hatari na viungo (sandbox) kabla ya kupeleka SMS kwa mteja; inazuia phishing na kuonya kwa suspicious." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
