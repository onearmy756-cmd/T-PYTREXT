//! common_human.rs — visaidizi vinavyoshirikiwa kwa SEHEMU 17 (International &
//! Humanitarian, modules 323-327).
//!
//! Kina AEAD halisi (ChaCha20 RFC 8439 + HMAC-SHA256, encrypt-then-MAC) kwa
//! mawasiliano nyeti, padding ya ukubwa maalum (kuficha urefu), pseudonymization,
//! hash-chain, na uainishaji wa IP kwa nchi.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

/// AEAD: simba pt kwa ChaCha20 + HMAC-SHA256 (AAD inashirikishwa kwenye MAC).
pub fn aead_encrypt(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], pt: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let ct = super::m273_voice_encryption::chacha20_xor(key, nonce, pt);
    let mut m = HmacSha256::new_from_slice(key).expect("hmac");
    m.update(aad);
    m.update(nonce);
    m.update(&ct);
    (ct, m.finalize().into_bytes().to_vec())
}

/// AEAD: fumbua + thibitisha MAC (encrypt-then-MAC).
pub fn aead_decrypt(key: &[u8; 32], nonce: &[u8; 12], aad: &[u8], ct: &[u8], tag: &[u8]) -> Result<Vec<u8>, String> {
    let mut m = HmacSha256::new_from_slice(key).map_err(|e| e.to_string())?;
    m.update(aad);
    m.update(nonce);
    m.update(ct);
    m.verify_slice(tag).map_err(|_| "MAC si sahihi — mawasiliano yamebadilishwa".to_string())?;
    Ok(super::m273_voice_encryption::chacha20_xor(key, nonce, ct))
}

/// Weka data kwenye block ya ukubwa maalum (kuficha urefu halisi).
pub fn pad_to(data: &[u8], block: usize) -> Vec<u8> {
    let block = block.max(1);
    let n = data.len().div_ceil(block) * block;
    let mut out = vec![0u8; n];
    out[..data.len()].copy_from_slice(data);
    out
}

/// Pseudonym: kitambulisho kisichoweza kurudishwa (SHA-256, hex 32).
pub fn pseudonymize(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update(b"\x1f");
    }
    let d = h.finalize();
    d[..16].iter().map(|b| format!("{b:02x}")).collect()
}

/// Hash-chain (reuse ya common_smsemail).
pub fn chain(prev: &str, entry: &str) -> String {
    super::common_smsemail::chain_hash(prev, entry)
}

/// Ramani ndogo ya prefix za IP → nchi (geoip-lite).
pub fn ip_country(ip: &str) -> Option<&'static str> {
    let table: &[(&str, &str)] = &[
        ("41.", "ZA/africa"),
        ("196.2", "TZ"),
        ("197.2", "TZ"),
        ("41.59", "TZ"),
        ("41.86", "TZ"),
        ("80.2", "RU"),
        ("95.1", "RU"),
        ("5.1", "RU"),
        ("203.0", "CN"),
        ("1.2", "CN"),
        ("58.1", "CN"),
        ("175.4", "KR"),
        ("211.1", "KR"),
        ("24.0", "US"),
        ("8.8", "US"),
        ("3.0", "US"),
        ("185.2", "IR"),
        ("2.144", "IR"),
        ("77.2", "EU"),
        ("91.2", "EU"),
    ];
    table.iter().find(|(p, _)| ip.starts_with(p)).map(|(_, c)| *c)
}

/// Aina za taarifa za INTERPOL (notice types).
pub fn interpol_notice(code: &str) -> Option<&'static str> {
    Some(match code.to_uppercase().as_str() {
        "RED" => "Red — mtuhumiwa wa kukamatwa",
        "BLUE" => "Blue — taarifa kuhusu mtu",
        "YELLOW" => "Yellow — mtu aliyepotea",
        "GREEN" => "Green — tahadhari ya uhalifu",
        "BLACK" => "Black — mtu asiyefahamika aliyekufa",
        "ORANGE" => "Orange — tahadhari ya vitisho",
        "PURPLE" => "Purple — mbinu za uhalifu",
        _ => return None,
    })
}

/// Hesabu ya mara kwa kitu (kwa uchanganuzi wa mifumo).
pub fn counts(items: &[String]) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for i in items {
        *m.entry(i.clone()).or_insert(0) += 1;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aead_roundtrip_na_aad() {
        let key = [3u8; 32];
        let nonce = [4u8; 12];
        let (ct, tag) = aead_encrypt(&key, &nonce, b"mission-x", b"habari ya siri");
        assert_eq!(aead_decrypt(&key, &nonce, b"mission-x", &ct, &tag).unwrap(), b"habari ya siri");
        assert!(aead_decrypt(&key, &nonce, b"mission-y", &ct, &tag).is_err());
    }

    #[test]
    fn padding_na_pseudonym() {
        assert_eq!(pad_to(b"abc", 16).len(), 16);
        assert_eq!(pad_to(&[0u8; 17], 16).len(), 32);
        let a = pseudonymize(&["Juma", "1990-01-01"]);
        let b = pseudonymize(&["Juma", "1990-01-01"]);
        let c = pseudonymize(&["Juma", "1991-01-01"]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
