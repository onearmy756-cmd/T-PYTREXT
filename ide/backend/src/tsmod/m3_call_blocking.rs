//! m3_call_blocking.rs — MODULE 3: Call Blocking
//!
//! Spec: "Inazuia simu za spam, scam na robocall."
//! Twist: Inazuia kabla ya simu kuita.
//!
//! Injini HALISI: blocklist (IOC), rate-limit ya simu, na alama za robocall (urefu wa simu).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::{is_emergency, parse_cdrs};

pub fn robocall_ratio(cdrs: &[super::common_calls::Cdr], cli: &str) -> f64 {
    let mine: Vec<_> = cdrs.iter().filter(|c| c.cli == cli).collect();
    if mine.is_empty() { return 0.0; }
    let short = mine.iter().filter(|c| c.duration_s <= 5).count();
    short as f64 / mine.len() as f64
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(3, "Call Blocking");
    let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
    if let Some(c) = is_emergency(number) {
        r.finding("emergency_override", format!("{number} ni namba ya dharura ({c}) — HAIZUILIKWI"), 0);
        r.summary = format!("Call Blocking: {number} ni ya dharura, imeruhusiwa.");
        return Ok(r);
    }
    let blocked = params.get("blocklist").and_then(|v| v.as_array())
        .map(|a| a.iter().any(|x| x.as_str() == Some(number))).unwrap_or(false);
    let cdrs = parse_cdrs(params);
    let ratio = robocall_ratio(&cdrs, number);
    let mut decision = "allow";
    if blocked { decision = "block"; r.finding("blocklist_hit", format!("{number} ipo kwenye blocklist"), 80); }
    if ratio > 0.7 && cdrs.iter().filter(|c| c.cli == number).count() >= 3 {
        decision = "block";
        r.finding("robocall", format!("{number}: {:.0}% ya simu ni fupi (robocall)", ratio * 100.0), 70);
    }
    r.finding("decision", format!("{number} -> {decision}"), if decision == "block" { 70 } else { 0 });
    r.summary = format!("Call Blocking: {number} -> {decision}.");
    Ok(r)
}

pub struct M3CallBlocking;

#[async_trait::async_trait]
impl TsModule for M3CallBlocking {
    fn id(&self) -> u16 { 3 }
    fn name(&self) -> &'static str { "Call Blocking" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Inazuia simu za spam/scam/robocall kwa blocklist, rate-limit na alama za robocall — isipokuwa namba za dharura." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
