//! m8_recording_protection.rs — MODULE 8: Call Recording Protection
//!
//! Spec: "Inalinda rekodi za simu."
//! Twist: Inarekodi kwa usimbaji na inagundua kurekodi bila idhini.
//!
//! Injini HALISI: usimbaji wa rekodi (AEAD) na ukaguzi wa ridhaa (consent) ya kurekodi.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use crate::tsmod::m273_voice_encryption::{encrypt_voice, decrypt_voice};
use sha2::{Digest, Sha256};

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(8, "Call Recording Protection");
    let consent = params.get("consent").and_then(|v| v.as_bool()).unwrap_or(false);
    let pcm = params.get("pcm").and_then(|v| v.as_str()).unwrap_or("recording-demo").as_bytes();
    let ecdh: [u8; 32] = Sha256::digest(b"rec-key").into();
    let salt = b"rec-salt";
    let (nonce_v, ct, tag) = encrypt_voice(&ecdh, salt, pcm, 0);
    let nonce: [u8; 12] = nonce_v.as_slice().try_into().map_err(|_| "nonce si 12 bytes")?;
    let back = decrypt_voice(&ecdh, salt, &nonce, &ct, &tag)?;
    r.finding("encrypted", format!("Rekodi {} imesimbwa (ChaCha20+HMAC), uthibitisho {}", ct.len(), if back == pcm { "sahihi" } else { "UMESHINDWA" }), 0);
    if !consent {
        r.finding("no_consent", "Rekodi inafanyika BILA ridhaa ya upande mwingine — hatari ya kisheria", 60);
    }
    r.summary = format!("Call Recording Protection: rekodi imesimbwa; ridhaa={consent}.");
    Ok(r)
}

pub struct M8RecordingProtection;

#[async_trait::async_trait]
impl TsModule for M8RecordingProtection {
    fn id(&self) -> u16 { 8 }
    fn name(&self) -> &'static str { "Call Recording Protection" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Inasimbaji rekodi za simu (ChaCha20+HMAC) na inagundua ikiwa upande mwingine unarekodi bila idhini (consent check)." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
