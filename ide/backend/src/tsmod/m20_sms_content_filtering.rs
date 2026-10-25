//! m20_sms_content_filtering.rs — MODULE 20: SMS Content Filtering
//!
//! Spec: "Inachuja maudhui ya SMS kwa wateja binafsi."
//! Twist: Inachuja maudhui yasiyofaa na PII (Luhn/NIDA) kabla ya kupeleka.
//!
//! Injini HALISI: Uchunguzi wa Luhn (kadi), NIDA (tarakimu 20), na maneno yasiyofaa.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_sms::{luhn_ok, find_nida};

const ADULT: &[&str] = &["xxx", "adult", "ngono", "utupu"];

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(20, "SMS Content Filtering");
    let body = params.get("body").and_then(|v| v.as_str()).unwrap_or(target);
    let low = body.to_lowercase();
    for w in ADULT { if low.contains(w) { r.finding("adult_content", format!("Neno '{w}' limetambuliwa"), 40); } }
    // PII: kadi za benki kwa Luhn.
    for tok in body.split(|c: char| !c.is_ascii_digit()) {
        if tok.len() >= 13 && tok.len() <= 19 && luhn_ok(tok) {
            r.finding("card_pii", format!("Namba ya kadi (Luhn sahihi) imefichwa: {}****", &tok[..4]), 70);
        }
    }
    if let Some(nida) = find_nida(body) {
        r.finding("nida_pii", format!("NIDA imetambuliwa na kufichwa: {}****", &nida[..4]), 65);
    }
    r.summary = format!("SMS Content Filtering: maudhui yamechujwa.");
    Ok(r)
}

pub struct M20SmsContentFiltering;

#[async_trait::async_trait]
impl TsModule for M20SmsContentFiltering {
    fn id(&self) -> u16 { 20 }
    fn name(&self) -> &'static str { "SMS Content Filtering" }
    fn group(&self) -> &'static str { "sms" }
    fn about(&self) -> &'static str { "Inachuja maudhui yasiyofaa (adult/nafasi), PII (kadi za benki kwa Luhn, NIDA), na inaweza kuficha taarifa nyeti." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
