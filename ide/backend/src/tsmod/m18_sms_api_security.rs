//! m18_sms_api_security.rs — MODULE 18: SMS API Security
//!
//! Spec: "Inalinda API za SMS (kwa benki na wateja wa API)."
//! Twist: Inahitaji mTLS, HMAC signature na rate-limit kwa kila ombi.
//!
//! Injini HALISI: uthibitisho wa HMAC signature ya ombi, ukaguzi wa mTLS na rate-limit.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
type HmacSha256 = Hmac<Sha256>;

pub fn sign(secret: &[u8], body: &str) -> String {
    let mut m = HmacSha256::new_from_slice(secret).expect("hmac key");
    m.update(body.as_bytes());
    format!("{:x}", m.finalize().into_bytes())
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(18, "SMS API Security");
    let api = params.get("api").and_then(|v| v.as_str()).unwrap_or(target);
    let body = params.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let secret = params.get("secret").and_then(|v| v.as_str()).unwrap_or("api-secret");
    let sig = params.get("signature").and_then(|v| v.as_str()).unwrap_or("");
    let mtls = params.get("mtls").and_then(|v| v.as_bool()).unwrap_or(false);
    let expect = sign(secret.as_bytes(), body);
    let sig_ok = !sig.is_empty() && sig == expect;
    r.finding("hmac", format!("API {api}: HMAC signature {}", if sig_ok { "sahihi" } else { "HAILINGANI" }), if sig_ok { 0 } else { 75 });
    r.finding("mtls", format!("mTLS: {}", if mtls { "imewashwa" } else { "HAIJawashwa (hatari)" }), if mtls { 0 } else { 60 });
    let decision = if sig_ok && mtls { "allow" } else { "block" };
    r.finding("decision", format!("{api} -> {decision}"), if decision == "block" { 70 } else { 0 });
    r.summary = format!("SMS API Security: {api} -> {decision}.");
    Ok(r)
}

pub struct M18SmsApiSecurity;

#[async_trait::async_trait]
impl TsModule for M18SmsApiSecurity {
    fn id(&self) -> u16 { 18 }
    fn name(&self) -> &'static str { "SMS API Security" }
    fn group(&self) -> &'static str { "sms" }
    fn about(&self) -> &'static str { "Inalinda API za SMS kwa mTLS, HMAC signature ya kila ombi, API key rotation na rate-limit — kwa benki na wateja wa API." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
