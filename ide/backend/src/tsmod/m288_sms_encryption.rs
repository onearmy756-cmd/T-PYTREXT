//! m288_sms_encryption.rs — MODULE 288: SMS ENCRYPTION
//!
//! Spec: "Inaficha SMS kabla ya kutoka. Twist: Self-destructing SMS."
//!
//! Injini halisi: husimba SMS (ChaCha20 + HMAC) na kuweka TTL — SMS
//! inajiharibu baada ya muda/idadi ya usomaji (self-destruct).

use super::common_smsemail::chain_hash;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// SMS iliyosimbwa.
#[derive(Debug, Clone)]
pub struct EncryptedSms {
    pub nonce: Vec<u8>,
    pub ct: Vec<u8>,
    pub tag: Vec<u8>,
    pub ttl_secs: u64,
    pub max_reads: u8,
    pub created_at: i64,
}

pub fn encrypt_sms(key: &[u8; 32], text: &str, ttl_secs: u64, max_reads: u8, now: i64) -> EncryptedSms {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&(now as u64).to_le_bytes());
    nonce[8..].copy_from_slice(&[0x53, 0x4d, 0x53, 0x31]); // "SMS1"
    let ct = super::m273_voice_encryption::chacha20_xor(key, &nonce, text.as_bytes());
    let tag = super::m273_voice_encryption::aead_tag(key, &nonce, &ct);
    EncryptedSms { nonce: nonce.to_vec(), ct, tag, ttl_secs, max_reads, created_at: now }
}

/// Je, SMS inaweza kusomwa? (muda + idadi ya usomaji).
pub fn can_read(e: &EncryptedSms, now: i64, reads: u8) -> bool {
    let fresh = e.ttl_secs == 0 || (now - e.created_at) < e.ttl_secs as i64;
    fresh && reads < e.max_reads
}

pub struct M288SmsEncryption;

impl TsModule for M288SmsEncryption {
    fn id(&self) -> u16 {
        288
    }
    fn name(&self) -> &'static str {
        "SMS Encryption"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huficha SMS (ChaCha20+HMAC) na kuweka TTL/idadi ya usomaji — SMS \
         inajiharibu yenyewe (self-destructing)."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(288, self.name());
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("Habari");
        let ttl = params.get("ttl_secs").and_then(|v| v.as_u64()).unwrap_or(3600);
        let max_reads = params.get("max_reads").and_then(|v| v.as_u64()).unwrap_or(1).clamp(1, 10) as u8;
        let now = params.get("now").and_then(|v| v.as_i64()).unwrap_or(1_700_000_000);
        let key: [u8; 32] = Sha256::digest(b"sms-e2e-key").into();
        let e = encrypt_sms(&key, text, ttl, max_reads, now);
        r.finding("sms_encrypted", format!("SMS imesimbwa (bytes {})", e.ct.len()), 1);
        // Self-destruct: usomaji wa kwanza unaruhusiwa, wa pili hauruhusiwi (max_reads=1).
        let first = can_read(&e, now + 10, 0);
        let after = can_read(&e, now + 10, max_reads);
        let expired = can_read(&e, now + ttl as i64 + 1, 0);
        r.finding("self_destruct", format!("Usomaji wa kwanza {first}, baada ya kikomo {after}, baada ya muda {expired}"), 1);
        r.findings.push(json!({
            "kind": "sms_crypto", "severity": 1,
            "detail": { "alg": "ChaCha20+HMAC-SHA256", "ttl_secs": ttl, "max_reads": max_reads,
                        "tag_hex": hex(&e.tag), "chain": chain_hash("", text) }
        }));
        r.summary = format!("SMS Encryption: imesimbwa, TTL {ttl}s, usomaji {max_reads}.");
        Ok(r)
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sms_self_destruct() {
        let key = [1u8; 32];
        let e = encrypt_sms(&key, "siri", 100, 1, 1000);
        assert!(can_read(&e, 1010, 0));
        assert!(!can_read(&e, 1010, 1));
        assert!(!can_read(&e, 2000, 0));
    }
}
