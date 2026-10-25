//! m13_sms_forwarding_security.rs — MODULE 13: SMS Forwarding Security
//!
//! Spec: "Inadhibiti uelekezaji wa SMS kwenye SIM."
//! Twist: Inahitaji uthibitisho wa fingerprint na OTP kabla ya kuwezesha SMS forwarding.
//!
//! Injini HALISI: ukaguzi wa uthibitisho wa hatua mbili (fingerprint + OTP) na uandikishaji kwenye mnyororo.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_sms::chain;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(13, "SMS Forwarding Security");
    let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
    let target_fwd = params.get("forward_to").and_then(|v| v.as_str()).unwrap_or("");
    let fingerprint = params.get("fingerprint_ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let otp = params.get("otp_ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let enabled = params.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let approved = params.get("approved_to").and_then(|v| v.as_str()).unwrap_or("");
    if !enabled {
        r.finding("forwarding_off", format!("{number}: SMS forwarding haijawashwa"), 0);
        r.summary = format!("SMS Forwarding Security: {number} imezimwa.");
        return Ok(r);
    }
    if !(fingerprint && otp) {
        r.finding("auth_failed", format!("{number}: forwarding inahitaji fingerprint+OTP (fingerprint={fingerprint}, otp={otp})"), 80);
        r.action("block_forward", "Kataa kuwezesha forwarding bila uthibitisho", false);
    } else if !approved.is_empty() && target_fwd != approved {
        r.finding("unauthorized_target", format!("{number}: lengo {target_fwd} si {approved} — imekataliwa"), 85);
    } else {
        r.finding("authorized", format!("{number}: forwarding kwa {target_fwd} imeidhinishwa (2FA imepita)"), 0);
        r.finding("blockchain_log", format!("Mabadiliko yameandikwa: {}", chain("genesis", &format!("{number}->{target_fwd}"))), 0);
    }
    r.summary = format!("SMS Forwarding Security: {number} -> {target_fwd}.");
    Ok(r)
}

pub struct M13SmsForwardingSecurity;

#[async_trait::async_trait]
impl TsModule for M13SmsForwardingSecurity {
    fn id(&self) -> u16 { 13 }
    fn name(&self) -> &'static str { "SMS Forwarding Security" }
    fn group(&self) -> &'static str { "sms" }
    fn about(&self) -> &'static str { "Inahitaji uthibitisho wa fingerprint + OTP kabla ya kuwezesha SMS forwarding; kila mabadiliko yanaandikwa kwenye blockchain." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
