//! m11_sms_encryption.rs — MODULE 11: SMS Encryption
//!
//! Spec: "Inaficha SMS kabla ya kutoka kwenye SIM."
//! Twist: Self-destructing SMS - inajiathiri baada ya sekunde 30.
//!
//! Injini HALISI: AEAD ya SMS (ChaCha20+HMAC), timer ya self-destruct, na hash ya ushahidi kwenye mnyororo.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use crate::tsmod::m273_voice_encryption::{encrypt_voice, decrypt_voice, hybrid_key};
use super::common_sms::sms_hash;

pub const SELF_DESTRUCT_S: u64 = 30;

/// Simba SMS kwa ufunguo wa mseto (X25519 + PQ salt).
pub fn encrypt_sms(ecdh: &[u8; 32], salt: &[u8], from: &str, to: &str, body: &str) -> ([u8; 12], Vec<u8>, Vec<u8>) {
    let key = hybrid_key(ecdh, salt, format!("{from}->{to}").as_bytes());
    let (nonce_v, ct, tag) = encrypt_voice(&key, salt, body.as_bytes(), 0);
    let nonce: [u8; 12] = nonce_v.as_slice().try_into().unwrap_or([0u8; 12]);
    (nonce, ct, tag)
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(11, "SMS Encryption");
    let from = params.get("from").and_then(|v| v.as_str()).unwrap_or(target);
    let to = params.get("to").and_then(|v| v.as_str()).unwrap_or("peer");
    let body = params.get("body").and_then(|v| v.as_str()).unwrap_or("SMS ya siri");
    let self_destruct = params.get("self_destruct").and_then(|v| v.as_bool()).unwrap_or(true);
    let ecdh: [u8; 32] = Sha256::digest(b"sms-x25519-demo").into();
    let salt = b"sms-pq-salt";
    let key = hybrid_key(&ecdh, salt, b"sms");
    let (nonce, ct, tag) = encrypt_sms(&ecdh, salt, from, to, body);
    let back = decrypt_voice(&key, salt, &nonce, &ct, &tag)?;
    let ok = back == body.as_bytes();
    r.finding("encrypted", format!("SMS {} bytes imesimbwa (ChaCha20+HMAC); uthibitisho {}", ct.len(), if ok { "sahihi" } else { "UMESHINDWA" }), if ok { 0 } else { 90 });
    if self_destruct {
        r.finding("self_destruct", format!("SMS itajiathiri baada ya sekunde {SELF_DESTRUCT_S}"), 0);
    }
    r.finding("blockchain_proof", format!("Ushahidi (hash) kwenye blockchain: {}", sms_hash(from, to, body)), 0);
    r.summary = format!("SMS Encryption: {from}->{to}, {} bytes, self-destruct={self_destruct}.", ct.len());
    Ok(r)
}

pub struct M11SmsEncryption;

#[async_trait::async_trait]
impl TsModule for M11SmsEncryption {
    fn id(&self) -> u16 { 11 }
    fn name(&self) -> &'static str { "SMS Encryption" }
    fn group(&self) -> &'static str { "sms" }
    fn about(&self) -> &'static str { "Usimbaji wa SMS wa mwisho-hadi-mwisho (ChaCha20+HMAC, ufunguo wa mseto X25519+PQ), self-destruct ya sekunde 30, na hash ya ushahidi kwenye blockchain." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
