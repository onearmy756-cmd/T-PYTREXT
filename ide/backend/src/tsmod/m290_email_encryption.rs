//! m290_email_encryption.rs — MODULE 290: EMAIL ENCRYPTION
//!
//! Spec: "Inaficha barua pepe kwa PGP/S/MIME. Twist: Inaficha attachment pia."
//!
//! Injini halisi: husimba mwili wa barua pepe NA viambatisho (attachment) kwa
//! ChaCha20+HMAC (mfano wa S/MIME envelope); hurudisha ujumbe uliosimbwa.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Kiambatisho kilichosimbwa.
#[derive(Debug, Clone)]
pub struct EncAttachment {
    pub name: String,
    pub nonce: Vec<u8>,
    pub ct: Vec<u8>,
    pub tag: Vec<u8>,
}

pub fn encrypt_body(key: &[u8; 32], body: &str) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let nonce = [0u8; 12];
    let ct = super::m273_voice_encryption::chacha20_xor(key, &nonce, body.as_bytes());
    let tag = super::m273_voice_encryption::aead_tag(key, &nonce, &ct);
    (nonce.to_vec(), ct, tag)
}

pub fn encrypt_attachment(key: &[u8; 32], name: &str, data: &[u8]) -> EncAttachment {
    let mut nonce = [0u8; 12];
    let nh = Sha256::digest(name.as_bytes());
    nonce[..8].copy_from_slice(&nh[..8]);
    let ct = super::m273_voice_encryption::chacha20_xor(key, &nonce, data);
    let tag = super::m273_voice_encryption::aead_tag(key, &nonce, &ct);
    EncAttachment { name: name.to_string(), nonce: nonce.to_vec(), ct, tag }
}

pub struct M290EmailEncryption;

impl TsModule for M290EmailEncryption {
    fn id(&self) -> u16 {
        290
    }
    fn name(&self) -> &'static str {
        "Email Encryption"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huficha barua pepe na viambatisho (S/MIME-style: ChaCha20+HMAC) — \
         kiambatisho pia husimbwa."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(290, self.name());
        let body = params.get("body").and_then(|v| v.as_str()).unwrap_or("Maudhui ya barua pepe");
        let key: [u8; 32] = Sha256::digest(b"email-smime-key").into();
        let (_n, ct, tag) = encrypt_body(&key, body);
        r.finding("body_encrypted", format!("Mwili umesimbwa (bytes {})", ct.len()), 1);
        let mut atts = 0;
        if let Some(arr) = params.get("attachments").and_then(|v| v.as_array()) {
            for a in arr {
                if let Some(name) = a.get("name").and_then(|v| v.as_str()) {
                    let data = a.get("data").and_then(|v| v.as_str()).unwrap_or("").as_bytes();
                    let e = encrypt_attachment(&key, name, data);
                    atts += 1;
                    r.findings.push(json!({ "kind": "attachment_encrypted", "severity": 1,
                        "detail": { "name": e.name, "bytes": e.ct.len(), "tag": hex(&e.tag) } }));
                }
            }
        }
        r.findings.push(json!({ "kind": "email_crypto", "severity": 1,
            "detail": { "alg": "ChaCha20+HMAC-SHA256", "attachments": atts, "body_tag": hex(&tag) } }));
        r.summary = format!("Email Encryption: mwili + viambatisho {atts} vimesimbwa.");
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
    fn body_na_attachment_vinasimbwa() {
        let key = [2u8; 32];
        let (n, ct, tag) = encrypt_body(&key, "siri");
        let mut nn = [0u8; 12];
        nn.copy_from_slice(&n);
        assert_eq!(super::super::m273_voice_encryption::chacha20_xor(&key, &nn, &ct), b"siri");
        assert!(!tag.is_empty());
        let a = encrypt_attachment(&key, "file.pdf", b"PDFDATA");
        assert_eq!(a.ct.len(), 7);
    }
}
