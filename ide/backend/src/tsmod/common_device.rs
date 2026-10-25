//! common_device.rs — visaidizi vinavyoshirikiwa kwa SEHEMU 16 (SIM & Device).
//!
//! Kina AES-256 halisi (FIPS-197, encrypt + CTR mode) — hakuna crate ya AES,
//! kwa hiyo tumeitekeleza kwa mkono na kuithibitisha kwa vector ya NIST.
//! Pia: IMEI (Luhn), MAC OUI, fingerprints, entropy.

use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

// ======================= AES-256 (FIPS-197) =======================

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 {
            p ^= a;
        }
        let hi = a & 0x80 != 0;
        a <<= 1;
        if hi {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    p
}

fn gf_pow(mut a: u8, mut e: u32) -> u8 {
    let mut r = 1u8;
    while e > 0 {
        if e & 1 != 0 {
            r = gf_mul(r, a);
        }
        a = gf_mul(a, a);
        e >>= 1;
    }
    r
}

/// S-box ya AES (inatengenezwa kwa GF(2^8) inverse + affine transform).
fn sbox() -> &'static [u8; 256] {
    static SB: OnceLock<[u8; 256]> = OnceLock::new();
    SB.get_or_init(|| {
        let mut t = [0u8; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let inv = if i == 0 { 0 } else { gf_pow(i as u8, 254) };
            let s = inv
                ^ inv.rotate_left(1)
                ^ inv.rotate_left(2)
                ^ inv.rotate_left(3)
                ^ inv.rotate_left(4)
                ^ 0x63;
            *v = s;
        }
        t
    })
}

/// Panua ufunguo wa 256-bit → 15 round keys (Nr=14).
pub fn expand_key(key: &[u8; 32]) -> [[u8; 16]; 15] {
    const NK: usize = 8; // 256/32
    const NR: usize = 14;
    const NB: usize = 4;
    let mut w = [[0u8; 4]; 4 * (NR + 1)]; // 60 words
    for i in 0..NK {
        w[i] = [key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]];
    }
    let sb = sbox();
    let mut rcon = 1u8;
    for i in NK..(NB * (NR + 1)) {
        let mut temp = w[i - 1];
        if i % NK == 0 {
            temp = [temp[1], temp[2], temp[3], temp[0]]; // RotWord
            temp = [sb[temp[0] as usize], sb[temp[1] as usize], sb[temp[2] as usize], sb[temp[3] as usize]]; // SubWord
            temp[0] ^= rcon;
            rcon = gf_mul(rcon, 2);
        } else if i % NK == 4 {
            temp = [sb[temp[0] as usize], sb[temp[1] as usize], sb[temp[2] as usize], sb[temp[3] as usize]];
        }
        for j in 0..4 {
            w[i][j] = w[i - NK][j] ^ temp[j];
        }
    }
    let mut rk = [[0u8; 16]; 15];
    for r in 0..=NR {
        for c in 0..4 {
            for row in 0..4 {
                rk[r][4 * c + row] = w[4 * r + c][row];
            }
        }
    }
    rk
}

fn add_round_key(s: &mut [u8; 16], k: &[u8; 16]) {
    for i in 0..16 {
        s[i] ^= k[i];
    }
}

fn sub_bytes(s: &mut [u8; 16]) {
    let sb = sbox();
    for b in s.iter_mut() {
        *b = sb[*b as usize];
    }
}

/// ShiftRows: byte index = row + 4*col; row r huhamishwa kushoto kwa r.
fn shift_rows(s: &mut [u8; 16]) {
    let mut o = [0u8; 16];
    for r in 0..4 {
        for c in 0..4 {
            o[r + 4 * c] = s[r + 4 * ((c + r) % 4)];
        }
    }
    *s = o;
}

fn mix_columns(s: &mut [u8; 16]) {
    for c in 0..4 {
        let a = [s[4 * c], s[4 * c + 1], s[4 * c + 2], s[4 * c + 3]];
        s[4 * c] = gf_mul(a[0], 2) ^ gf_mul(a[1], 3) ^ a[2] ^ a[3];
        s[4 * c + 1] = a[0] ^ gf_mul(a[1], 2) ^ gf_mul(a[2], 3) ^ a[3];
        s[4 * c + 2] = a[0] ^ a[1] ^ gf_mul(a[2], 2) ^ gf_mul(a[3], 3);
        s[4 * c + 3] = gf_mul(a[0], 3) ^ a[1] ^ a[2] ^ gf_mul(a[3], 2);
    }
}

/// Simba block moja (16 bytes) kwa AES-256.
pub fn aes256_encrypt_block(block: &[u8; 16], rk: &[[u8; 16]; 15]) -> [u8; 16] {
    let mut s = *block;
    add_round_key(&mut s, &rk[0]);
    for r in 1..14 {
        sub_bytes(&mut s);
        shift_rows(&mut s);
        mix_columns(&mut s);
        add_round_key(&mut s, &rk[r]);
    }
    sub_bytes(&mut s);
    shift_rows(&mut s);
    add_round_key(&mut s, &rk[14]);
    s
}

/// AES-256-CTR: simba data kwa counter ya 16-byte (big-endian increment).
pub fn aes256_ctr(key: &[u8; 32], nonce: &[u8; 8], data: &[u8]) -> Vec<u8> {
    let rk = expand_key(key);
    let mut out = Vec::with_capacity(data.len());
    let mut counter: u64 = 0;
    for chunk in data.chunks(16) {
        let mut blk = [0u8; 16];
        blk[..8].copy_from_slice(nonce);
        blk[8..].copy_from_slice(&counter.to_be_bytes());
        let ks = aes256_encrypt_block(&blk, &rk);
        for (i, b) in chunk.iter().enumerate() {
            out.push(b ^ ks[i]);
        }
        counter = counter.wrapping_add(1);
    }
    out
}

// ======================= Device helpers =======================

/// IMEI ni sahihi? (15 tarakimu + Luhn).
pub fn imei_valid(imei: &str) -> bool {
    let d: Vec<u32> = imei.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() != 15 {
        return false;
    }
    super::common_smsemail::luhn_valid(&d.iter().map(|x| x.to_string()).collect::<String>())
}

/// MAC OUI → mtengenezaji (sehemu ndogo).
pub fn oui_vendor(mac: &str) -> Option<&'static str> {
    let clean: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_uppercase();
    if clean.len() < 6 {
        return None;
    }
    Some(match &clean[..6] {
        "001A11" | "3C5A37" => "Google",
        "001B63" | "A4C361" => "Apple",
        "001CC4" | "3C2AF4" => "Samsung",
        "0022FB" | "5C514F" => "Huawei",
        "001E10" | "2C54CF" => "Xiaomi",
        "001636" | "00E0FC" => "Techno",
        _ => return None,
    })
}

/// Entropy ya Shannon (bits/symbol) kwa orodha ya fonti.
pub fn shannon_entropy(items: &[String]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }
    let mut freq = std::collections::HashMap::new();
    for it in items {
        *freq.entry(it.as_str()).or_insert(0usize) += 1;
    }
    let n = items.len() as f64;
    -freq.values().map(|&c| {
        let p = c as f64 / n;
        p * p.log2()
    }).sum::<f64>()
}

/// Screen aspect ratio (kategoria).
pub fn screen_ratio_class(w: u32, h: u32) -> &'static str {
    if h == 0 {
        return "unknown";
    }
    let r = w as f64 / h as f64;
    if r > 1.9 {
        "ultrawide"
    } else if r > 1.6 {
        "widescreen"
    } else if r > 1.2 {
        "tablet"
    } else if r > 0.9 {
        "square"
    } else {
        "portrait"
    }
}

/// Fingerprint ya kifaa (hash ya sifa).
pub fn device_fingerprint(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update(b"|");
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Haversine (umbali km) — inatumika location.
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0_f64;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Aina za SIM (spec: Nano/Micro/eSIM).
pub fn sim_kind(id: &str) -> &'static str {
    match id.to_lowercase().as_str() {
        "nano" | "4ff" => "nano",
        "micro" | "3ff" => "micro",
        "mini" | "2ff" => "mini",
        "esim" | "euicc" => "esim",
        _ => "unknown",
    }
}

pub fn json_ok(kind: &str, detail: serde_json::Value) -> serde_json::Value {
    json!({ "kind": kind, "detail": detail })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes256_fips_vector() {
        // FIPS-197 Appendix C.3 (AES-256).
        let mut key = [0u8; 32];
        for (i, k) in key.iter_mut().enumerate() {
            *k = i as u8;
        }
        let pt: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let rk = expand_key(&key);
        let ct = aes256_encrypt_block(&pt, &rk);
        let expect: [u8; 16] = [
            0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49, 0x60, 0x89,
        ];
        assert_eq!(ct, expect);
    }

    #[test]
    fn ctr_roundtrip() {
        let key = [7u8; 32];
        let nonce = [1u8; 8];
        let msg = b"data ya kifaa cha mteja";
        let ct = aes256_ctr(&key, &nonce, msg);
        assert_ne!(&ct[..], &msg[..]);
        assert_eq!(aes256_ctr(&key, &nonce, &ct), msg.to_vec());
    }

    #[test]
    fn imei_na_oui() {
        assert!(imei_valid("490154203237518")); // IMEI halali inayojulikana
        assert!(!imei_valid("12345"));
        assert_eq!(oui_vendor("3C:5A:37:11:22:33"), Some("Google"));
    }
}
