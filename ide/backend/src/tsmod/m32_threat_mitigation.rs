//! m32_threat_mitigation.rs — MODULE 32: Threat Mitigation Module
//!
//! Spec: "Inapunguza athari za vitisho."
//! Twist: Inatumia rate-limiting na kuchuja trafiki kuzuia DDoS.
//!
//! Injini HALISI: hesabu ya rate na uamuzi wa kuchuja (filter/ratelimit/drop) kwa IP.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::engine;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(32, "Threat Mitigation");
    let ip = params.get("ip").and_then(|v| v.as_str()).unwrap_or(target);
    let pps = params.get("pps").and_then(|v| v.as_u64()).unwrap_or(0);
    let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
    let score = if pps >= 100_000 { 90 } else if pps >= 10_000 { 60 } else if pps >= 1_000 { 30 } else { 0 };
    let action = if score >= 80 { "drop" } else if score >= 50 { "ratelimit" } else { "allow" };
    r.finding("mitigation", format!("{ip}: {pps} pps -> {action}"), score);
    if apply && action == "drop" && engine::valid_target(ip) {
        match engine::fw_block(ip) {
            Ok(_) => { r.action("nft_block", format!("{ip} imezuiwa kwenye nftables"), true); r.applied = true; }
            Err(e) => { r.action("nft_block", format!("nft: {e}"), false); }
        }
    }
    r.summary = format!("Threat Mitigation: {ip} -> {action}.");
    Ok(r)
}

pub struct M32ThreatMitigation;

#[async_trait::async_trait]
impl TsModule for M32ThreatMitigation {
    fn id(&self) -> u16 { 32 }
    fn name(&self) -> &'static str { "Threat Mitigation Module" }
    fn group(&self) -> &'static str { "ai" }
    fn about(&self) -> &'static str { "Upunguzaji wa vitisho: rate-limiting, kuchuja trafiki na uainishaji wa DDoS — kwa kutumia nftables halisi (m35) inapowezekana." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
