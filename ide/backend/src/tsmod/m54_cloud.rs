//! m54_cloud.rs — MODULE 54: CLOUD SECURITY
//!
//! Spec: "Inalinda data kwenye cloud (AWS, Azure, GCP). Twist: inaficha data
//! kabla ya kuweka kwenye cloud."
//!
//! Hii si catalog: ina authenticated encryption halisi kabla ya ku-upload —
//! stream cipher inayotokana na HMAC-SHA256 (PRF) pamoja na tag ya HMAC-SHA256.
//! Hii hulinda data-at-rest kabla ya kuweka kwenye cloud. Pia hukagua creds.

use super::{ModReport, TsModule};
use base64::Engine;
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct M54Cloud;

/// Stream cipher kutoka HMAC-SHA256 (keystream = HMAC(key, nonce || counter)).
fn keystream(key: &[u8], nonce: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut counter: u32 = 0;
    while out.len() < len {
        let mut mac = HmacSha256::new_from_slice(key).expect("hmac key");
        mac.update(nonce);
        mac.update(&counter.to_be_bytes());
        let block = mac.finalize().into_bytes();
        out.extend_from_slice(&block);
        counter += 1;
    }
    out.truncate(len);
    out
}

/// Tag ya uthibitisho.
fn tag(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Ficha data: rudisha (nonce||ciphertext||tag) base64.
pub fn encrypt(key: &[u8], plaintext: &[u8]) -> String {
    use rand::RngCore;
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce);
    let ks = keystream(key, &nonce, plaintext.len());
    let ct: Vec<u8> = plaintext
        .iter()
        .zip(ks.iter())
        .map(|(p, k)| p ^ k)
        .collect();
    let mut blob = Vec::with_capacity(16 + ct.len() + 32);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ct);
    let t = tag(key, &blob);
    blob.extend_from_slice(&t);
    base64::engine::general_purpose::STANDARD.encode(blob)
}

/// Fungua data (huthibitisha tag kwanza).
pub fn decrypt(key: &[u8], b64: &str) -> Result<Vec<u8>, String> {
    let blob = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| e.to_string())?;
    if blob.len() < 16 + 32 {
        return Err("data fupi mno".into());
    }
    let (body, t) = blob.split_at(blob.len() - 32);
    let expect = tag(key, body);
    if expect != t {
        return Err("tag ya HMAC hailingani — data imebadilishwa".into());
    }
    let nonce = &body[..16];
    let ct = &body[16..];
    let ks = keystream(key, nonce, ct.len());
    Ok(ct.iter().zip(ks.iter()).map(|(c, k)| c ^ k).collect())
}

impl TsModule for M54Cloud {
    fn id(&self) -> u16 {
        54
    }
    fn name(&self) -> &'static str {
        "Cloud Security"
    }
    fn group(&self) -> &'static str {
        "cloud"
    }
    fn about(&self) -> &'static str {
        "Huficha data (authenticated encryption, HMAC-SHA256 PRF) kabla ya kuiweka \
         kwenye cloud; hukagua creds zilizo wazi."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "encrypt" => self.encrypt_op(params),
            "decrypt" => self.decrypt_op(params),
            other => Err(format!("op '{other}' haipo: audit|encrypt|decrypt")),
        }
    }
}

impl M54Cloud {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(54, self.name());
        let mut found = false;
        for k in ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AZURE_CLIENT_SECRET", "GOOGLE_APPLICATION_CREDENTIALS"] {
            if std::env::var(k).map(|v| !v.trim().is_empty()).unwrap_or(false) {
                found = true;
                r.finding(
                    "cloud_cred",
                    format!("{k} ipo kwenye env — hakikisha haifichui kwenye logs"),
                    20,
                );
            }
        }
        if !found {
            r.finding("no_cloud_creds", "Hakuna cloud creds kwenye env", 2);
        }
        r.finding(
            "encryption_policy",
            "Data hufichwa (HMAC-SHA256 PRF stream + HMAC tag) kabla ya cloud",
            2,
        );
        r.summary = "Cloud audit imefanyika.".into();
        Ok(r)
    }

    fn encrypt_op(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(54, self.name());
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("params.key inahitajika")?;
        let data = params
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or("params.data inahitajika")?;
        let blob = encrypt(key.as_bytes(), data.as_bytes());
        r.findings.push(json!({
            "kind": "encrypted", "severity": 2,
            "detail": { "blob_b64": blob, "bytes": data.len() }
        }));
        r.summary = format!("Data ya bytes {} imefichwa.", data.len());
        Ok(r)
    }

    fn decrypt_op(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(54, self.name());
        let key = params
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("params.key inahitajika")?;
        let blob = params
            .get("blob_b64")
            .and_then(|v| v.as_str())
            .ok_or("params.blob_b64 inahitajika")?;
        match decrypt(key.as_bytes(), blob) {
            Ok(pt) => {
                let s = String::from_utf8_lossy(&pt).to_string();
                r.findings.push(json!({
                    "kind": "decrypted", "severity": 2, "detail": { "data": s }
                }));
                r.summary = "Data imefunguliwa (tag imethibitishwa).".into();
            }
            Err(e) => {
                r.finding("decrypt_fail", e, 60);
                r.ok = false;
                r.summary = "Imeshindwa kufungua data.".into();
            }
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = b"supersecretkey123";
        let pt = b"habari za siri za mteja";
        let blob = encrypt(key, pt);
        let out = decrypt(key, &blob).unwrap();
        assert_eq!(out, pt);
    }

    #[test]
    fn key_mbaya_inakataliwa() {
        let blob = encrypt(b"key1", b"data");
        assert!(decrypt(b"key2", &blob).is_err());
    }

    #[test]
    fn tampering_inagundulika() {
        let key = b"key1";
        let mut blob = base64::engine::general_purpose::STANDARD
            .decode(encrypt(key, b"data"))
            .unwrap();
        blob[20] ^= 0xff;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&blob);
        assert!(decrypt(key, &b64).is_err());
    }

    #[test]
    fn ciphertext_ni_tofauti_kila_mara() {
        let a = encrypt(b"k", b"same");
        let b = encrypt(b"k", b"same");
        assert_ne!(a, b); // nonce random
    }
}
