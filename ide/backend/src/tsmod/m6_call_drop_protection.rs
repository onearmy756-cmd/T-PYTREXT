//! m6_call_drop_protection.rs — MODULE 6: Call Drop Protection
//!
//! Spec: "Inagundua simu zinazokatika (call drops) na sababu zake."
//! Twist: Inagundua kukatika kunakosababishwa na mashambulizi.
//!
//! Injini HALISI: uchambuzi wa CDR: uwiano wa simu zilizokatika, na sababu zinazowezekana.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::parse_cdrs;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(6, "Call Drop Protection");
    let cdrs = parse_cdrs(params);
    let total = cdrs.len();
    let dropped = cdrs.iter().filter(|c| c.duration_s == 0 && c.direction == "out").count();
    let ratio = if total > 0 { dropped as f64 / total as f64 } else { 0.0 };
    let score = if ratio > 0.4 && total >= 5 { 65 } else if ratio > 0.2 && total >= 5 { 35 } else { 0 };
    r.finding("drop_rate", format!("{dropped}/{total} simu zilikatika ({:.0}%)", ratio * 100.0), score);
    if score >= 60 { r.finding("cause", "Uwezekano wa jamming/downgrade ya mtandao", score); }
    r.summary = format!("Call Drop Protection: {dropped}/{total} zilikatika.");
    Ok(r)
}

pub struct M6CallDropProtection;

#[async_trait::async_trait]
impl TsModule for M6CallDropProtection {
    fn id(&self) -> u16 { 6 }
    fn name(&self) -> &'static str { "Call Drop Protection" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Inagundua call drops zisizo za kawaida (jamming/downgrade) kwa CDR na inashauri hatua; namba za dharura haziathiriwi." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
