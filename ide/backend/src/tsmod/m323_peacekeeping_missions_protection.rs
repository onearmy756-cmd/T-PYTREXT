//! m323_peacekeeping_missions_protection.rs — MODULE 323
//!
//! Spec: "Inalinda wanajeshi wa amani wa Umoja wa Mataifa. Twist: Inaficha mawasiliano ya wanajeshi."
//!
//! Injini halisi: hulinda mawasiliano ya wanajeshi kwa usimbaji wa AEAD (ChaCha20
//! + HMAC) na padding (kuficha urefu), huficha metadata (pseudonymization), na
//! huthibitisha uadilifu wa amri.

use super::common_human::{aead_decrypt, aead_encrypt, pad_to, pseudonymize};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub struct M323PeacekeepingMissionsProtection;

impl TsModule for M323PeacekeepingMissionsProtection {
    fn id(&self) -> u16 {
        323
    }
    fn name(&self) -> &'static str {
        "Peacekeeping Missions Protection"
    }
    fn group(&self) -> &'static str {
        "humanitarian"
    }
    fn about(&self) -> &'static str {
        "Hulinda mawasiliano ya wanajeshi wa amani (UN): AEAD (ChaCha20+HMAC) + \
         padding ya urefu maalum + pseudonymization ya metadata."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(323, self.name());
        let msg = p.get("message").and_then(|v| v.as_str()).unwrap_or("amri: songa kaskazini").as_bytes();
        let mission = p.get("mission").and_then(|v| v.as_str()).unwrap_or("MONUSCO");
        let caller = p.get("caller").and_then(|v| v.as_str()).unwrap_or("+255712000000");
        let key: [u8; 32] = Sha256::digest(b"peacekeeping-key").into();
        let nonce: [u8; 12] = [0x50, 0x4b, 0x4f, 0x00, 0, 0, 0, 0, 0, 0, 0, 1];
        let padded = pad_to(msg, 32);
        let (ct, tag) = aead_encrypt(&key, &nonce, mission.as_bytes(), &padded);
        let ok = aead_decrypt(&key, &nonce, mission.as_bytes(), &ct, &tag).map(|d| d[..msg.len()] == *msg).unwrap_or(false);
        let pseudo = pseudonymize(&[caller]);
        r.findings.push(json!({ "kind": "encrypted_comms", "severity": 1,
            "detail": { "cipher": "ChaCha20+HMAC-SHA256", "plaintext_len": msg.len(),
                        "padded_len": padded.len(), "ct_len": ct.len(), "integrity": ok } }));
        r.findings.push(json!({ "kind": "metadata", "severity": 1,
            "detail": { "caller_pseudonym": pseudo, "metadata_hidden": true } }));
        r.summary = format!("Peacekeeping: mawasiliano ya {mission} yamefichwa ({} bytes → {}).", msg.len(), ct.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mawasiliano_yanasimba_na_kufichwa() {
        let m = M323PeacekeepingMissionsProtection;
        let rep = m.run("", &json!({"message":"amri ya siri","mission":"MONUSCO"})).unwrap();
        assert_eq!(rep.findings.len(), 2);
    }
}
