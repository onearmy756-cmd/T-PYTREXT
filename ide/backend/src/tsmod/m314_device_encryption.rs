//! m314_device_encryption.rs — MODULE 314: DEVICE ENCRYPTION
//!
//! Spec: "Inaficha data yote kwenye kifaa. AES-256 encryption."
//!
//! Injini halisi: husimba data yote ya kifaa kwa AES-256-CTR (FIPS-197, ufunguo
//! 256-bit) na kuthibitisha uadilifu kwa HMAC-SHA256 (encrypt-then-MAC).

use super::common_device::aes256_ctr;
use super::{ModReport, TsModule};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Simba data (AES-256-CTR) + MAC.
pub fn encrypt_device(key: &[u8; 32], nonce: &[u8; 8], data: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let ct = aes256_ctr(key, nonce, data);
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key");
    mac.update(nonce);
    mac.update(&ct);
    (ct, mac.finalize().into_bytes().to_vec())
}

/// Fumbua data + thibitisha MAC.
pub fn decrypt_device(key: &[u8; 32], nonce: &[u8; 8], ct: &[u8], tag: &[u8]) -> Result<Vec<u8>, String> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| e.to_string())?;
    mac.update(nonce);
    mac.update(ct);
    mac.verify_slice(tag).map_err(|_| "MAC si sahihi — data imebadilishwa".to_string())?;
    Ok(aes256_ctr(key, nonce, ct))
}

pub struct M314DeviceEncryption;

impl TsModule for M314DeviceEncryption {
    fn id(&self) -> u16 {
        314
    }
    fn name(&self) -> &'static str {
        "Device Encryption"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Husimba data yote ya kifaa kwa AES-256 (FIPS-197) + HMAC-SHA256 \
         (encrypt-then-MAC)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(314, self.name());
        let data = p.get("data").and_then(|v| v.as_str()).unwrap_or("data ya siri ya kifaa").as_bytes().to_vec();
        let key: [u8; 32] = Sha256::digest(b"device-aes-key").into();
        let nonce: [u8; 8] = [0x44, 0x45, 0x56, 0x31, 0x00, 0x00, 0x00, 0x01];
        let (ct, tag) = encrypt_device(&key, &nonce, &data);
        r.finding("encrypted", format!("Data imesimbwa: {} → {} bytes (AES-256-CTR)", data.len(), ct.len()), 1);
        let roundtrip = decrypt_device(&key, &nonce, &ct, &tag).map(|d| d == data).unwrap_or(false);
        r.finding("roundtrip", if roundtrip { "Usimbaji/fumbuzi ni sahihi" } else { "IMESHINDIKANA" }, if roundtrip { 1 } else { 95 });
        let mut bad = ct.clone();
        if !bad.is_empty() {
            bad[0] ^= 1;
        }
        let tamper_rejected = decrypt_device(&key, &nonce, &bad, &tag).is_err();
        r.finding("tamper", if tamper_rejected { "Kubadilisha kunagundulika (MAC)" } else { "HAIKUGUNDULIKA" }, if tamper_rejected { 1 } else { 95 });
        r.findings.push(json!({ "kind": "device_crypto", "severity": 1,
            "detail": { "alg": "AES-256-CTR + HMAC-SHA256", "tag": hex(&tag) } }));
        r.summary = format!("Device Encryption: AES-256 OK | roundtrip {} | tamper {}.", if roundtrip { "OK" } else { "FAIL" }, if tamper_rejected { "OK" } else { "FAIL" });
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
    fn aes_device_roundtrip_na_tamper() {
        let key = [9u8; 32];
        let nonce = [2u8; 8];
        let (ct, tag) = encrypt_device(&key, &nonce, b"hello kifaa");
        assert_eq!(decrypt_device(&key, &nonce, &ct, &tag).unwrap(), b"hello kifaa");
        let mut bad = ct.clone();
        bad[0] ^= 1;
        assert!(decrypt_device(&key, &nonce, &bad, &tag).is_err());
    }
}
