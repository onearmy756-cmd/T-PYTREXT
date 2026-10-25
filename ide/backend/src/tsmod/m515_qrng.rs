//! m515_qrng.rs — MODULE 515: QRNG Integration
//!
//! Spec: "Quantum Random Number Generator."
//! Twist: Random numbers za kweli kutoka quantum phenomena.
//!
//! Injini HALISI: simulator ya quantum (H + measure) na uthibitisho wa entropy ya bits.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use crate::quantum;
use super::common_ai::entropy_bits;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(515, "QRNG Integration");
    let bits = params.get("bits").and_then(|v| v.as_u64()).unwrap_or(256).min(4096);
    let out = quantum::api_qrng(&json!({ "bits": bits }));
    let hex = out.get("hex").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // Entropy ya bits (bytes za hex → nibbles).
    let mut hist = [0u64; 16];
    for c in hex.chars() { if let Some(d) = c.to_digit(16) { hist[d as usize] += 1; } }
    let h = entropy_bits(&hist);
    r.finding("qrng", format!("Bits {bits} kutoka quantum simulator (H+measure); entropy {h:.2}/4 bits"), if h < 3.0 { 40 } else { 0 });
    r.finding("sample", format!("Sampuli: {}...", &hex[..hex.len().min(32)]), 0);
    r.summary = format!("QRNG: {bits} bits, entropy {h:.2}.");
    Ok(r)
}

pub struct M515Qrng;

#[async_trait::async_trait]
impl TsModule for M515Qrng {
    fn id(&self) -> u16 { 515 }
    fn name(&self) -> &'static str { "QRNG Integration" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "QRNG: hutoa namba za kweli za quantum kwa kutumia simulator ya quantum (Hadamard + measurement) na kukagua ubora (entropy)." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
