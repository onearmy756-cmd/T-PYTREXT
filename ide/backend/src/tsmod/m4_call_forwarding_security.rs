//! m4_call_forwarding_security.rs — MODULE 4: Call Forwarding Security
//!
//! Spec: "Inalinda usambazaji wa simu (call forwarding)."
//! Twist: Inagundua forwarding ya siri iliyowekwa na mhalifu.
//!
//! Injini HALISI: ulinganisho wa forwarding halisi vs iliyoidhinishwa, na ugunduzi wa forwarding mpya ya nje.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(4, "Call Forwarding Security");
    let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
    let active = params.get("forwarding_to").and_then(|v| v.as_str()).unwrap_or("");
    let approved = params.get("approved_to").and_then(|v| v.as_str()).unwrap_or("");
    let enabled = params.get("forwarding_enabled").and_then(|v| v.as_bool()).unwrap_or(!active.is_empty());
    if !enabled {
        r.finding("forwarding_off", format!("{number}: hakuna forwarding inayotumika"), 0);
    } else if active.is_empty() {
        r.finding("forwarding_unset", format!("{number}: forwarding imewashwa bila lengo"), 30);
    } else if active != approved {
        r.finding("unauthorized_forward", format!("{number} inaelekeza simu kwa {active} — HAIJAIDHINISHWA (inayoidhinishwa: {approved})"), 85);
        r.action("block_forward", format!("Ondoa forwarding kwa {active}"), false);
    } else {
        r.finding("forwarding_ok", format!("{number}: forwarding kwa {active} imeidhinishwa"), 0);
    }
    r.summary = format!("Call Forwarding Security: {number}, hali imekaguliwa.");
    Ok(r)
}

pub struct M4CallForwardingSecurity;

#[async_trait::async_trait]
impl TsModule for M4CallForwardingSecurity {
    fn id(&self) -> u16 { 4 }
    fn name(&self) -> &'static str { "Call Forwarding Security" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Inagundua call forwarding ya siri (unauthorized) inayoelekeza simu kwa mhalifu, na inaizuia ikiwa haijaidhinishwa." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
