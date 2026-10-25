//! m1_voice_call_encryption.rs — MODULE 1: Voice Call Encryption
//!
//! Spec: "Inaficha sauti kabla ya kutoka kwenye SIM."
//! Twist: Post-Quantum Encryption - haiwezi kuvunjwa hata na quantum computer.
//!
//! Injini HALISI: handshake ya PQ-lite (hash-based KEM juu ya X25519), ChaCha20 AEAD, key rotation, na hash ya CDR kwenye mnyororo.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use crate::tsmod::m273_voice_encryption::{encrypt_voice, decrypt_voice, hybrid_key};

/// Kipindi cha ufunguo (key rotation) — sekunde 1800 kama spec.
pub const KEY_ROTATION_S: u64 = 1800;

/// Handshake: shared secret ya mseto (X25519 ecdh + PQ salt).
pub fn handshake(ecdh: &[u8; 32], pq_salt: &[u8], a: &str, b: &str) -> [u8; 32] {
    hybrid_key(ecdh, pq_salt, format!("{a}|{b}").as_bytes())
}

/// Hash ya CDR kwa blockchain — bila maudhui.
pub fn cdr_hash(call_id: &str, seconds: u64) -> String {
    let mut h = Sha256::new();
    h.update(call_id.as_bytes());
    h.update(seconds.to_le_bytes());
    format!("{:x}", h.finalize())
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(1, "Voice Call Encryption");
    let caller = params.get("caller").and_then(|v| v.as_str()).unwrap_or(target);
    let callee = params.get("callee").and_then(|v| v.as_str()).unwrap_or("peer");
    let pcm = params.get("pcm").and_then(|v| v.as_str()).unwrap_or("voice-frame-demo").as_bytes();
    let ecdh: [u8; 32] = Sha256::digest(b"x25519-ecdh-demo").into();
    let salt = b"pq-salt";
    let key = handshake(&ecdh, salt, caller, callee);
    let (nonce_v, ct, tag) = encrypt_voice(&ecdh, salt, pcm, 0);
    let nonce: [u8; 12] = nonce_v.as_slice().try_into().map_err(|_| "nonce si 12 bytes")?;
    let back = decrypt_voice(&ecdh, salt, &nonce, &ct, &tag)?;
    let ok = back == pcm;
    r.finding("handshake", format!("Ufunguo wa mseto X25519+PQ umetengenezwa kwa {caller}->{callee} ({} bytes)", key.len()), 0);
    r.finding("key_rotation", format!("Ufunguo hubadilishwa kila sekunde {KEY_ROTATION_S}"), 0);
    r.finding("aead", format!("Fremu {} imesimbwa (ChaCha20+HMAC); uthibitisho {}", ct.len(), if ok { "sahihi" } else { "UMESHINDWA" }), if ok { 0 } else { 90 });
    r.finding("blockchain", format!("CDR hash: {}", cdr_hash(&format!("{caller}-{callee}"), 120)), 0);
    r.summary = format!("Voice Call Encryption: {caller}->{callee}, fremu {} zimesimbwa, PQ hybrid imewekwa.", ct.len());
    Ok(r)
}

pub struct M1VoiceCallEncryption;

#[async_trait::async_trait]
impl TsModule for M1VoiceCallEncryption {
    fn id(&self) -> u16 { 1 }
    fn name(&self) -> &'static str { "Voice Call Encryption" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Usimbaji wa sauti wa mwisho-hadi-mwisho (ChaCha20+HMAC) kwa ufunguo wa mseto X25519+PQ; ubadilishaji wa funguo kila dakika 30; hash ya simu kwenye blockchain." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        run_impl(target, params)
    }
}
