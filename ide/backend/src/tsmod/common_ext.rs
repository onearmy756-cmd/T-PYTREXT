//! common_ext.rs — visaidizi vinavyoshirikiwa kwa SEHEMU 20 (Modules 501-516).
//!
//! Algorithms HALISI: file carving (magic bytes), Merkle root (SBOM/blockchain),
//! onion address validation (dark web), na ufuatiliaji wa mabadiliko ya binary.

use sha2::{Digest, Sha256};

/// Saini za faili (magic bytes) za kawaida kwa data carving.
pub const SIGNATURES: &[(&[u8], &str)] = &[
    (&[0xFF, 0xD8, 0xFF], "jpeg"),
    (&[0x89, 0x50, 0x4E, 0x47], "png"),
    (&[0x25, 0x50, 0x44, 0x46], "pdf"),
    (&[0x50, 0x4B, 0x03, 0x04], "zip"),
    (&[0x47, 0x49, 0x46, 0x38], "gif"),
    (&[0x52, 0x49, 0x46, 0x46], "riff"),
    (&[0x4D, 0x5A], "pe/exe"),
    (&[0x7F, 0x45, 0x4C, 0x46], "elf"),
];

/// Carve: tafuta saini zote ndani ya buffer; rudisha (offset, aina).
pub fn carve(data: &[u8]) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (sig, kind) in SIGNATURES {
        if data.len() < sig.len() {
            continue;
        }
        for i in 0..=(data.len() - sig.len()) {
            if &data[i..i + sig.len()] == *sig {
                out.push((i, *kind));
            }
        }
    }
    out.sort_by_key(|(o, _)| *o);
    out
}

/// Onion v3 address ni herufi 56 za base32 (a-z2-7) + ".onion".
pub fn is_onion_v3(addr: &str) -> bool {
    let a = addr.trim_end_matches(".onion").to_lowercase();
    a.len() == 56 && a.chars().all(|c| matches!(c, 'a'..='z' | '2'..='7'))
}

/// Onion v2 address ni herufi 16 za base32.
pub fn is_onion_v2(addr: &str) -> bool {
    let a = addr.trim_end_matches(".onion").to_lowercase();
    a.len() == 16 && a.chars().all(|c| matches!(c, 'a'..='z' | '2'..='7'))
}

/// Merkle root kutoka orodha ya hash (hex) — kwa SBOM/blockchain.
pub fn merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return String::new();
    }
    let mut level: Vec<String> = leaves.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0;
        while i < level.len() {
            let a = &level[i];
            let b = if i + 1 < level.len() { &level[i + 1] } else { &level[i] };
            let mut h = Sha256::new();
            h.update(a.as_bytes());
            h.update(b.as_bytes());
            next.push(format!("{:x}", h.finalize()));
            i += 2;
        }
        level = next;
    }
    level.into_iter().next().unwrap_or_default()
}

/// Hash ya kipengele (jina + toleo) — kwa SBOM.
pub fn component_hash(name: &str, version: &str) -> String {
    let mut h = Sha256::new();
    h.update(name.as_bytes());
    h.update(b"@");
    h.update(version.as_bytes());
    format!("{:x}", h.finalize())
}

/// Ufuatiliaji wa mabadiliko ya binary (moving target defense):
/// hulinganisha hash ya binary ya leo na ya jana.
pub fn binary_changed(yesterday: &str, today: &str) -> bool {
    yesterday != today
}

/// Kipimo cha TEE attestation: hash ya (code + config + hardware id).
pub fn attestation_measurement(code_hash: &str, config_hash: &str, hw_id: &str) -> String {
    let mut h = Sha256::new();
    h.update(code_hash.as_bytes());
    h.update(b"|");
    h.update(config_hash.as_bytes());
    h.update(b"|");
    h.update(hw_id.as_bytes());
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carving_inapata_saini() {
        let mut data = vec![0u8; 8];
        data.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]);
        data.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47]);
        let found = carve(&data);
        assert!(found.iter().any(|(_, k)| *k == "jpeg"));
        assert!(found.iter().any(|(_, k)| *k == "png"));
    }

    #[test]
    fn onion_v3_inatambulika() {
        let good = "a".repeat(56) + ".onion";
        assert!(is_onion_v3(&good));
        assert!(!is_onion_v3("too-short.onion"));
    }

    #[test]
    fn merkle_root_ni_taratibu() {
        let a = vec![component_hash("a", "1"), component_hash("b", "2")];
        let b = vec![component_hash("a", "1"), component_hash("b", "2")];
        assert_eq!(merkle_root(&a), merkle_root(&b));
        let c = vec![component_hash("a", "1"), component_hash("b", "3")];
        assert_ne!(merkle_root(&a), merkle_root(&c));
    }
}
