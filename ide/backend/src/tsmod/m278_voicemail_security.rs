//! m278_voicemail_security.rs — MODULE 278: VOICEMAIL SECURITY
//!
//! Spec: "Inalinda ujumbe wa sauti. Twist: Inaficha voicemail."
//!
//! Injini halisi: hukagua usalama wa voicemail:
//!   - Nguvu ya PIN ya voicemail (dhidi ya PINs dhaifu zinazojulikana).
//!   - Kuingia kwa mbali (remote access) bila PIN.
//!   - Usimbaji wa ujumbe uliohifadhiwa (encryption at rest).
//!   - Ratiba ya kubadilisha PIN (rotation).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// PINs dhaifu zinazojulikana kwa voicemail.
const WEAK_PINS: &[&str] = &["0000", "1111", "1234", "4321", "2222", "5555", "1212", "1122", "12345", "123456", "000000"];

/// Alama ya nguvu ya PIN (0-100).
pub fn pin_strength(pin: &str) -> (u8, &'static str) {
    if pin.is_empty() {
        return (0, "haipo");
    }
    if WEAK_PINS.contains(&pin) {
        return (5, "dhaifu sana");
    }
    let uniq = {
        let mut u: Vec<char> = pin.chars().collect();
        u.sort();
        u.dedup();
        u.len()
    };
    let seq = pin.chars().all(|c| c.is_ascii_digit());
    let score = match (pin.len(), uniq) {
        (l, u) if l >= 8 && u >= 5 => 90,
        (l, u) if l >= 6 && u >= 4 => 70,
        (l, u) if l >= 6 && u >= 3 => 50,
        _ if seq => 30,
        _ => 40,
    };
    (score, if score >= 70 { "nzuri" } else { "dhaifu" })
}

pub struct M278VoicemailSecurity;

impl TsModule for M278VoicemailSecurity {
    fn id(&self) -> u16 {
        278
    }
    fn name(&self) -> &'static str {
        "Voicemail Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Hukagua PIN ya voicemail, kuingia kwa mbali, na usimbaji wa ujumbe \
         uliohifadhiwa — hulinda ujumbe wa sauti."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(278, self.name());
        let pin = params.get("pin").and_then(|v| v.as_str()).unwrap_or("");
        let (ps, verdict) = pin_strength(pin);
        let remote = params.get("remote_access").and_then(|v| v.as_bool()).unwrap_or(true);
        let encrypted = params.get("encrypted").and_then(|v| v.as_bool()).unwrap_or(false);
        let rotated_days = params.get("pin_age_days").and_then(|v| v.as_i64()).unwrap_or(9999);

        let mut score = 0u8;
        if ps < 50 {
            score = score.max(70);
            r.finding("weak_pin", format!("PIN ya voicemail ni {verdict} (score {ps})"), 70);
        }
        if remote && pin.is_empty() {
            score = score.max(85);
            r.finding("remote_no_pin", "Kuingia kwa mbali kunawashwa bila PIN", 85);
        }
        if !encrypted {
            score = score.max(45);
            r.finding("unencrypted", "Ujumbe haujasimbwa (encryption at rest haipo)", 45);
        }
        if rotated_days > 180 {
            score = score.max(40);
            r.finding("stale_pin", format!("PIN haijabadilishwa kwa siku {rotated_days}"), 40);
        }
        // Usimbaji wa ujumbe (onyesho la twist: inaficha voicemail).
        if let Some(msg) = params.get("message").and_then(|v| v.as_str()) {
            let key: [u8; 32] = Sha256::digest(b"voicemail-at-rest").into();
            let nonce = [0u8; 12];
            let ct = super::m273_voice_encryption::chacha20_xor(&key, &nonce, msg.as_bytes());
            r.findings.push(json!({
                "kind": "voicemail_encrypted", "severity": 1,
                "detail": { "alg": "ChaCha20", "ciphertext_len": ct.len() }
            }));
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Voicemail Security: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_dhaifu_inagundulika() {
        assert_eq!(pin_strength("1234").0, 5);
        assert!(pin_strength("7391").0 < 50);
        assert!(pin_strength("k9x2m7q4").0 >= 70);
    }
}
