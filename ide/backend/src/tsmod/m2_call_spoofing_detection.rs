//! m2_call_spoofing_detection.rs — MODULE 2: Call Spoofing Detection
//!
//! Spec: "Inagundua namba bandia (caller ID spoofing)."
//! Twist: Inaonya kabla ya kuitisha.
//!
//! Injini HALISI: scoring ya attestation A/B/C, heuristics za kuiga taasisi, na E.164 validation.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::parse_e164;

const INSTITUTIONS: &[&str] = &["bank", "benki", "crdb", "nmb", "vodacom", "airtel", "tigo", "halotel", "nhif", "nida", "tra", "tcra", "police", "polisi", "mpesa", "m-pesa", "luku"];

pub fn spoof_score(number: &str, name: &str, attestation: &str, region_cc: &str) -> (u8, Vec<(&'static str, String)>) {
    let mut score = 0u8;
    let mut why = Vec::new();
    let low = number.to_lowercase();
    if low.contains("private") || low.contains("withheld") || low.contains("anonymous") || low.contains("unknown") {
        score = score.max(55);
        why.push(("cli_withheld", "Namba imefichwa".into()));
    }
    match attestation.to_uppercase().as_str() {
        "A" => {}
        "B" => { score = score.max(30); why.push(("attest_b", "Attestation B — sehemu tu".into())); }
        "C" => { score = score.max(70); why.push(("attest_c", "Attestation C — gateway ya nje".into())); }
        "" => { score = score.max(25); why.push(("attest_none", "Hakuna attestation".into())); }
        _ => {}
    }
    let (cc, nat, valid) = parse_e164(number);
    if !valid { score = score.max(45); why.push(("invalid_e164", format!("Namba si E.164: {number}"))); }
    let nl = name.to_lowercase();
    if INSTITUTIONS.iter().any(|i| nl.contains(i)) {
        let mobile = nat.len() >= 9 && matches!(nat.as_bytes().first(), Some(b'6' | b'7'));
        if mobile { score = score.max(80); why.push(("brand_cli_spoof", format!("Jina '{name}' linaiga taasisi lakini namba ni ya mkononi"))); }
        else { score = score.max(35); why.push(("brand_name", format!("Jina la taasisi '{name}' — thibitisha"))); }
    }
    if let Some(cc) = cc { if !region_cc.is_empty() && cc != region_cc { score = score.max(40); why.push(("foreign_cc", format!("CC {cc} tofauti na eneo {region_cc}"))); } }
    (score, why)
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(2, "Call Spoofing Detection");
    let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
    let name = params.get("presented_name").and_then(|v| v.as_str()).unwrap_or("");
    let att = params.get("attestation").and_then(|v| v.as_str()).unwrap_or("");
    let region = params.get("region_cc").and_then(|v| v.as_str()).unwrap_or("255");
    let (score, why) = spoof_score(number, name, att, region);
    for (k, d) in &why { r.finding(k, d.clone(), score); }
    let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
    r.finding("decision", format!("{number} -> {decision}"), score);
    r.summary = format!("Call Spoofing Detection: {number} -> {decision} (score {score}).");
    Ok(r)
}

pub struct M2CallSpoofingDetection;

#[async_trait::async_trait]
impl TsModule for M2CallSpoofingDetection {
    fn id(&self) -> u16 { 2 }
    fn name(&self) -> &'static str { "Call Spoofing Detection" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "STIR/SHAKEN attestation, kuiga taasisi, namba iliyofichwa, country code tofauti — hutoa tahadhari kabla ya kuitisha." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
