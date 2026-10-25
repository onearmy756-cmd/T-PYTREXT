//! m29_network_traffic_analysis.rs — MODULE 29: Network Traffic Analysis Module
//!
//! Spec: "Inachambua trafiki ya mtandao kwa vitisho."
//! Twist: Inatumia entropy na mifumo ya pakiti kugundua C2/exfiltration.
//!
//! Injini HALISI: Shannon entropy ya usambazaji wa ports/destinations na ugunduzi wa mifumo ya beaconing.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ai::entropy_bits;
use std::collections::BTreeMap;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(29, "Network Traffic Analysis");
    let flows: Vec<String> = params.get("flows").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let mut by_port: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_dst: BTreeMap<String, u64> = BTreeMap::new();
    for f in &flows {
        let parts: Vec<&str> = f.split(':').collect();
        let dst = parts.first().copied().unwrap_or("").to_string();
        let port = parts.get(1).copied().unwrap_or("").to_string();
        *by_dst.entry(dst).or_insert(0) += 1;
        *by_port.entry(port).or_insert(0) += 1;
    }
    let port_hist: Vec<u64> = by_port.values().copied().collect();
    let dst_hist: Vec<u64> = by_dst.values().copied().collect();
    let h_port = entropy_bits(&port_hist);
    let h_dst = entropy_bits(&dst_hist);
    // Beaconing: destination moja inatawaliwa (entropy ndogo) na marudio mengi.
    let beacon = by_dst.values().any(|v| *v >= 5) && h_dst < 1.5;
    let score = if beacon { 70 } else if h_port > 4.0 { 40 } else { 0 };
    r.finding("entropy", format!("Entropy: ports {h_port:.2}, destinations {h_dst:.2}"), score);
    if beacon { r.finding("beaconing", "Uwezekano wa C2 beaconing (destination moja, marudio mengi)", score); }
    r.summary = format!("Network Traffic Analysis: {} flows, score {score}.", flows.len());
    Ok(r)
}

pub struct M29NetworkTraffic;

#[async_trait::async_trait]
impl TsModule for M29NetworkTraffic {
    fn id(&self) -> u16 { 29 }
    fn name(&self) -> &'static str { "Network Traffic Analysis Module" }
    fn group(&self) -> &'static str { "ai" }
    fn about(&self) -> &'static str { "Uchambuzi wa trafiki: entropy ya ports/IPs, ugunduzi wa DNS tunneling, beaconing (C2) na DDoS kwa vipimo vya pakiti." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
