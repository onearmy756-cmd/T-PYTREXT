//! m5_forwarding_fraud.rs — MODULE 5: Call Forwarding Fraud Detection
//!
//! Spec: "Inagundua ulaghai wa forwarding (IRSF)."
//! Twist: Inagundua ulaghai wa kimataifa wa gharama (IRSF) kwa CDR.
//!
//! Injini HALISI: uchambuzi wa CDR: muda, gharama, na marudio kwa namba za nje (premium).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::parse_cdrs;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(5, "Call Forwarding Fraud Detection");
    let home_cc = params.get("home_cc").and_then(|v| v.as_str()).unwrap_or("255");
    let cdrs = parse_cdrs(params);
    let mut total_units = 0.0f64;
    let mut intl_long = 0usize;
    for c in &cdrs {
        let d: String = c.cld.chars().filter(|ch| ch.is_ascii_digit()).collect();
        let is_intl = !d.starts_with(home_cc) && d.len() > 6;
        if is_intl && c.duration_s >= 300 { intl_long += 1; }
        total_units += c.cost_units;
    }
    let score = if intl_long >= 5 || total_units >= 500.0 { 80 } else if intl_long >= 2 { 45 } else { 0 };
    r.finding("irsf", format!("Simu {intl_long} ndefu za kimataifa; gharama jumla {total_units:.0} units"), score);
    r.finding("decision", format!("{}", if score >= 80 { "block_forwarding" } else if score >= 40 { "warn" } else { "allow" }), score);
    r.summary = format!("Call Forwarding Fraud: {intl_long} simu za IRSF, gharama {total_units:.0} units.");
    Ok(r)
}

pub struct M5ForwardingFraud;

#[async_trait::async_trait]
impl TsModule for M5ForwardingFraud {
    fn id(&self) -> u16 { 5 }
    fn name(&self) -> &'static str { "Call Forwarding Fraud Detection" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "IRSF (International Revenue Share Fraud): simu nyingi za muda mrefu kwa namba za premium za nje — inahesabu kodi na inazuia." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
