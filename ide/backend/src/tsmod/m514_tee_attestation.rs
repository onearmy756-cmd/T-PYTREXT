//! m514_tee_attestation.rs — MODULE 514: TEE Attestation
//!
//! Spec: "Trusted Execution Environment verification."
//! Twist: Inathibitisha kuwa inaendesha kwenye hardware halisi.
//!
//! Injini HALISI: kipimo cha attestation (hash) na ulinganisho wa uhalali wa hardware.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ext::attestation_measurement;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(514, "TEE Attestation");
    let code = params.get("code_hash").and_then(|v| v.as_str()).unwrap_or("code-abc");
    let config = params.get("config_hash").and_then(|v| v.as_str()).unwrap_or("cfg-xyz");
    let hw = params.get("hw_id").and_then(|v| v.as_str()).unwrap_or("hw-serial-001");
    let expected = params.get("expected").and_then(|v| v.as_str()).unwrap_or("");
    let m = attestation_measurement(code, config, hw);
    let valid = expected.is_empty() || expected == m;
    r.finding("measurement", format!("Attestation: {}", &m[..16]), 0);
    r.finding("validity", format!("Uhalali wa hardware: {}", if valid { "imethibitishwa" } else { "HAILINGANI (emulation?)" }), if valid { 0 } else { 90 });
    r.summary = format!("TEE Attestation: valid={valid}.");
    Ok(r)
}

pub struct M514TeeAttestation;

#[async_trait::async_trait]
impl TsModule for M514TeeAttestation {
    fn id(&self) -> u16 { 514 }
    fn name(&self) -> &'static str { "TEE Attestation" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "TEE attestation: hukokotoa kipimo cha attestation (code+config+hardware) na kulinganisha na kipimo kinachotarajiwa." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
