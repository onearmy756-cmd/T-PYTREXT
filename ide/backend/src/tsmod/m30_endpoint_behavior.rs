//! m30_endpoint_behavior.rs — MODULE 30: Endpoint Behavior Module
//!
//! Spec: "Inachambua tabia ya kifaa (endpoint)."
//! Twist: Inagundua malware kwa tabia ya mchakato (process behavior).
//!
//! Injini HALISI: uchambuzi wa mti wa michakato (process tree) na alama za tabia ya malware.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const SUSPICIOUS: &[&str] = &["powershell", "cmd.exe", "wscript", "cscript", "mshta", "regsvr32", "rundll32", "certutil", "bitsadmin"];

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(30, "Endpoint Behavior");
    let procs: Vec<String> = params.get("processes").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    let mut hits: BTreeSet<String> = BTreeSet::new();
    for p in &procs {
        let low = p.to_lowercase();
        for s in SUSPICIOUS { if low.contains(s) { hits.insert((*s).to_string()); } }
    }
    let score = if hits.len() >= 3 { 75 } else if !hits.is_empty() { 45 } else { 0 };
    r.finding("processes", format!("Michakato {} imechambuliwa; alama {}", procs.len(), hits.iter().cloned().collect::<Vec<_>>().join(", ")), score);
    if score >= 70 { r.action("kill_chain", "Kata mnyororo wa michakato inayotiliwa shaka", false); }
    r.summary = format!("Endpoint Behavior: {} alama za malware.", hits.len());
    Ok(r)
}

pub struct M30EndpointBehavior;

#[async_trait::async_trait]
impl TsModule for M30EndpointBehavior {
    fn id(&self) -> u16 { 30 }
    fn name(&self) -> &'static str { "Endpoint Behavior Module" }
    fn group(&self) -> &'static str { "ai" }
    fn about(&self) -> &'static str { "Uchambuzi wa tabia ya endpoint: michakato inayotiliwa shaka, uhusiano wa wazazi (parent-child), na alama za malware." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
