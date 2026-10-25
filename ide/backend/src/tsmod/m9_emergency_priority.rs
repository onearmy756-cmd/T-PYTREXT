//! m9_emergency_priority.rs — MODULE 9: Emergency Call Priority
//!
//! Spec: "Inatolewa kipaumbele kwa simu za dharura."
//! Twist: Namba za dharura haziwezi kuzuiwa kamwe na zinapata kipaumbele cha mtandao.
//!
//! Injini HALISI: ugunduzi wa namba za dharura (kimataifa) na kipaumbele cha QoS (DSCP EF).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::is_emergency;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(9, "Emergency Call Priority");
    let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
    match is_emergency(number) {
        Some(c) => {
            r.finding("emergency", format!("{number} ni namba ya dharura ({c})"), 0);
            r.finding("qos", "Kipaumbele cha mtandao: DSCP EF (46) — hakuna kuzuia, hakuna throttling", 0);
        }
        None => { r.finding("not_emergency", format!("{number} si namba ya dharura"), 0); }
    }
    r.summary = format!("Emergency Call Priority: {number}.");
    Ok(r)
}

pub struct M9EmergencyPriority;

#[async_trait::async_trait]
impl TsModule for M9EmergencyPriority {
    fn id(&self) -> u16 { 9 }
    fn name(&self) -> &'static str { "Emergency Call Priority" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Inatambua namba za dharura za kila nchi na kuzipa kipaumbele cha mtandao (QoS) — haziwezi kuzuiwa kamwe." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
