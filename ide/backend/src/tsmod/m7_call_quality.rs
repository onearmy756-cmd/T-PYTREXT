//! m7_call_quality.rs — MODULE 7: Call Quality Monitoring (MOS)
//!
//! Spec: "Inapima ubora wa simu (MOS)."
//! Twist: Inagundua kushuka kwa ubora kunakosababishwa na mashambulizi.
//!
//! Injini HALISI: E-model ya ITU-T (R-factor -> MOS) na uainishaji wa ubora (excellent/good/fair/poor).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::e_model_mos;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(7, "Call Quality Monitoring (MOS)");
    let lat = params.get("latency_ms").and_then(|v| v.as_f64()).unwrap_or(40.0);
    let jit = params.get("jitter_ms").and_then(|v| v.as_f64()).unwrap_or(5.0);
    let loss = params.get("loss_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let (rf, mos) = e_model_mos(lat, jit, loss);
    let grade = if mos >= 4.3 { "excellent" } else if mos >= 4.0 { "good" } else if mos >= 3.0 { "fair" } else { "poor" };
    r.finding("mos", format!("latency {lat}ms, jitter {jit}ms, loss {loss}% -> R={rf:.1}, MOS={mos} ({grade})"), if mos < 3.0 { 60 } else { 0 });
    r.summary = format!("Call Quality: MOS {mos} ({grade}).");
    Ok(r)
}

pub struct M7CallQuality;

#[async_trait::async_trait]
impl TsModule for M7CallQuality {
    fn id(&self) -> u16 { 7 }
    fn name(&self) -> &'static str { "Call Quality Monitoring (MOS)" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "ITU-T G.107 E-model: hukokotoa R-factor na MOS kutoka latency, jitter na upotevu — hutoa daraja la ubora." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
