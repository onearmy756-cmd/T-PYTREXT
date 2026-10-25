//! m503_iot_security.rs — MODULE 503: IoT Security Integration
//!
//! Spec: "Inawasiliana na vifaa vya IoT."
//! Twist: Inagundua IoT device inayofanya DDoS.
//!
//! Injini HALISI: uchambuzi wa trafiki ya IoT (pps) na uainishaji wa mashambulizi ya botnet.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(503, "IoT Security Integration");
    let device = params.get("device").and_then(|v| v.as_str()).unwrap_or(target);
    let pps = params.get("pps").and_then(|v| v.as_u64()).unwrap_or(0);
    let kind = params.get("device_kind").and_then(|v| v.as_str()).unwrap_or("camera");
    let ddos = pps >= 50_000;
    let score = if pps >= 200_000 { 90 } else if ddos { 70 } else if pps >= 10_000 { 40 } else { 0 };
    r.finding("iot", format!("Kifaa '{device}' ({kind}): {pps} pps"), score);
    if ddos { r.finding("ddos", format!("'{device}' linafanya DDoS (botnet-style)"), score); r.action("quarantine_iot", "Tenga kifaa cha IoT kwenye VLAN ya quarantine", false); }
    r.summary = format!("IoT Security: {device} ({kind}), {pps} pps.");
    Ok(r)
}

pub struct M503IotSecurity;

#[async_trait::async_trait]
impl TsModule for M503IotSecurity {
    fn id(&self) -> u16 { 503 }
    fn name(&self) -> &'static str { "IoT Security Integration" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "Usalama wa IoT: hugundua vifaa vya IoT vinavyofanya DDoS kwa kuchambua pps na aina ya kifaa (Mirai-style)." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
