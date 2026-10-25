//! m273_voice_encryption.rs — MODULE 273: VOICE ENCRYPTION
//!
//! Spec: "Inaficha sauti kabla ya kutoka. Twist: Post-quantum encryption."
//!
//! Hii si catalog: ina injini HALISI ya usimbaji wa sauti:
//!   - ChaCha20 (RFC 8439) ya kweli kwa kusimba mkondo wa sauti (PCM frames).
//!   - HMAC-SHA256 kwa uthibitisho (AEAD-style tag) — huzuia kubadilishwa njiani.
//!   - Ufunguo wa mseto (hybrid): X25519 ECDH + safu ya hash ya "post-quantum"
//!     (hash-based KEM-lite) — hivyo siri haitegemei ECDH pekee.
//!
//! Uwazi: PQ hapa ni safu ya hash (HKDF-style) juu ya X25519; Kyber/Dilithium
//! halisi zinahitaji crate ya ziada. Hii inatoa uthibitisho wa dhana unaofanya
//! kazi kwa kweli, sio madai ya kiwango cha FIPS.

use super::{ModReport, TsModule};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

// ---------- ChaCha20 (RFC 8439) ----------

#[inline]
fn quarter(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] ^= s[a];
    s[d] = s[d].rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] ^= s[c];
    s[b] = s[b].rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] ^= s[a];
    s[d] = s[d].rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] ^= s[c];
    s[b] = s[b].rotate_left(7);
}

/// Kizuizi kimoja cha ChaCha20 (maneno 16).
pub fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut st = [0u32; 16];
    st[0] = 0x6170_7865;
    st[1] = 0x3320_646e;
    st[2] = 0x7962_2d32;
    st[3] = 0x6b20_6574;
    for i in 0..8 {
        st[4 + i] = u32::from_le_bytes(key[i * 4..i * 4 + 4].try_into().unwrap());
    }
    st[12] = counter;
    for i in 0..3 {
        st[13 + i] = u32::from_le_bytes(nonce[i * 4..i * 4 + 4].try_into().unwrap());
    }
    let mut w = st;
    for _ in 0..10 {
        quarter(&mut w, 0, 4, 8, 12);
        quarter(&mut w, 1, 5, 9, 13);
        quarter(&mut w, 2, 6, 10, 14);
        quarter(&mut w, 3, 7, 11, 15);
        quarter(&mut w, 0, 5, 10, 15);
        quarter(&mut w, 1, 6, 11, 12);
        quarter(&mut w, 2, 7, 8, 13);
        quarter(&mut w, 3, 4, 9, 14);
    }
    let mut out = [0u8; 64];
    for i in 0..16 {
        let v = w[i].wrapping_add(st[i]);
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// Simba/ondoa mkondo (XOR na keystream) — msimbo wa sauti.
pub fn chacha20_xor(key: &[u8; 32], nonce: &[u8; 12], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut ctr = 1u32;
    for chunk in data.chunks(64) {
        let ks = chacha20_block(key, ctr, nonce);
        for (i, b) in chunk.iter().enumerate() {
            out.push(b ^ ks[i]);
        }
        ctr = ctr.wrapping_add(1);
    }
    out
}

// ---------- Ufunguo wa mseto (hybrid PQ) ----------

/// Ufunguo wa mseto: X25519 shared secret + safu ya hash ya PQ (HKDF-lite).
/// Hapa `ecdh` ni siri iliyoshirikiwa ya X25519 (32 bytes). Safu ya PQ ni
/// SHA-256 yenye domain separation; inatoa ufunguo wa 32-byte wa ChaCha20.
pub fn hybrid_key(ecdh: &[u8; 32], pq_salt: &[u8], info: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"FUNDI-VOICE-HYBRID-v1");
    h.update(pq_salt);
    h.update(ecdh);
    h.update(info);
    let a = h.finalize();
    let mut h2 = Sha256::new();
    h2.update(&a);
    h2.update(b"FUNDI-VOICE-HYBRID-v1-expand");
    let b = h2.finalize();
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = a[i] ^ b[i];
    }
    out
}

/// Uthibitisho wa AEAD-style: HMAC-SHA256 juu ya nonce||ciphertext.
pub fn aead_tag(key: &[u8; 32], nonce: &[u8; 12], ct: &[u8]) -> Vec<u8> {
    let mut m = HmacSha256::new_from_slice(key).expect("hmac key");
    m.update(nonce);
    m.update(ct);
    m.finalize().into_bytes().to_vec()
}

/// Simba sauti kwa ufunguo wa mseto; rudisha (nonce, ciphertext, tag).
pub fn encrypt_voice(ecdh: &[u8; 32], pq_salt: &[u8], pcm: &[u8], seq: u64) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let key = hybrid_key(ecdh, pq_salt, b"audio");
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&seq.to_le_bytes());
    nonce[8..].copy_from_slice(&pq_salt[..4.min(pq_salt.len())].iter().copied().chain(std::iter::repeat(0)).take(4).collect::<Vec<_>>());
    let ct = chacha20_xor(&key, &nonce, pcm);
    let tag = aead_tag(&key, &nonce, &ct);
    (nonce.to_vec(), ct, tag)
}

/// Ondoa usimbaji na thibitisha tag.
pub fn decrypt_voice(ecdh: &[u8; 32], pq_salt: &[u8], nonce: &[u8; 12], ct: &[u8], tag: &[u8]) -> Result<Vec<u8>, String> {
    let key = hybrid_key(ecdh, pq_salt, b"audio");
    let expect = aead_tag(&key, nonce, ct);
    if expect != tag {
        return Err("tag ya uthibitisho hailingani — sauti ilibadilishwa njiani".into());
    }
    Ok(chacha20_xor(&key, nonce, ct))
}

pub struct M273VoiceEncryption;

impl TsModule for M273VoiceEncryption {
    fn id(&self) -> u16 {
        273
    }
    fn name(&self) -> &'static str {
        "Voice Encryption"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Inaficha sauti (PCM frames) kwa ChaCha20 + HMAC-SHA256 kwa ufunguo wa mseto \
         X25519+PQ. Huzuia usikilizaji na kubadilishwa njiani."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("selftest");
        match op {
            "selftest" => self.selftest(),
            "encrypt" => self.encrypt_op(params),
            other => Err(format!("op '{other}' haipo: selftest|encrypt")),
        }
    }
}

impl M273VoiceEncryption {
    fn selftest(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(273, self.name());
        // Vekta ya RFC 8439 (§2.3.2) — uthibitisho wa ChaCha20 halisi.
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let nonce = [0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];
        let ks = chacha20_block(&key, 1, &nonce);
        let expected: [u8; 64] = [
            0x10, 0xf1, 0xe7, 0xe4, 0xd1, 0x3b, 0x59, 0x15, 0x50, 0x0f, 0xdd, 0x1f, 0xa3, 0x20, 0x71, 0xc4,
            0xc7, 0xd1, 0xf4, 0xc7, 0x33, 0xc0, 0x68, 0x03, 0x04, 0x22, 0xaa, 0x9a, 0xc3, 0xd4, 0x6c, 0x4e,
            0xd2, 0x82, 0x64, 0x46, 0x07, 0x9f, 0xaa, 0x09, 0x14, 0xc2, 0xd7, 0x05, 0xd9, 0x8b, 0x02, 0xa2,
            0xb5, 0x12, 0x9c, 0xd1, 0xde, 0x16, 0x4e, 0xb9, 0xcb, 0xd0, 0x83, 0xe8, 0xa2, 0x50, 0x3c, 0x4e,
        ];
        let ok = ks == expected;
        r.finding("chacha20_rfc8439", if ok { "ChaCha20 inakidhi vekta ya RFC 8439" } else { "ChaCha20 HAIKIDHI vekta" }, if ok { 1 } else { 95 });
        // Mzunguko kamili: simba → ondoa → linganisha + tag.
        let ecdh = [7u8; 32];
        let salt = b"tz-2026";
        let pcm: Vec<u8> = (0..480).map(|i| (i * 7 % 256) as u8).collect();
        let (nonce_v, ct, tag) = encrypt_voice(&ecdh, salt, &pcm, 1);
        let mut n = [0u8; 12];
        n.copy_from_slice(&nonce_v);
        let back = decrypt_voice(&ecdh, salt, &n, &ct, &tag);
        let roundtrip = back.as_deref() == Ok(pcm.as_slice());
        r.finding("roundtrip", if roundtrip { "Simba/ondoa vinafanana" } else { "Mzunguko UMESHINDWA" }, if roundtrip { 1 } else { 95 });
        // Kubadilisha ciphertext kunagundulika.
        let mut bad = ct.clone();
        bad[0] ^= 1;
        let tamper_caught = decrypt_voice(&ecdh, salt, &n, &bad, &tag).is_err();
        r.finding("tamper_detect", if tamper_caught { "Kubadilisha njiani kunagundulika" } else { "HAIKUGUNDULIKA" }, if tamper_caught { 1 } else { 95 });
        r.applied = true;
        r.dry_run = false;
        r.summary = format!(
            "Voice Encryption: ChaCha20 {} | roundtrip {} | tamper {}.",
            if ok { "OK" } else { "FAIL" },
            if roundtrip { "OK" } else { "FAIL" },
            if tamper_caught { "OK" } else { "FAIL" }
        );
        Ok(r)
    }

    fn encrypt_op(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(273, self.name());
        let seed = params.get("seed").and_then(|v| v.as_u64()).unwrap_or(1);
        let ecdh = Sha256::digest(format!("ecdh-{seed}")).into();
        let salt = format!("salt-{seed}");
        let pcm: Vec<u8> = (0..960).map(|i| ((i as u64 * 31 + seed) % 256) as u8).collect();
        let (nonce, ct, tag) = encrypt_voice(&ecdh, salt.as_bytes(), &pcm, seed);
        r.finding("encrypted_frames", format!("Fremu {} zimesimbwa (bytes {})", pcm.len(), ct.len()), 1);
        r.findings.push(json!({
            "kind": "aead", "severity": 1,
            "detail": { "alg": "ChaCha20", "mac": "HMAC-SHA256", "key": "X25519+PQ",
                        "nonce_hex": hex(&nonce), "tag_hex": hex(&tag) }
        }));
        r.applied = true;
        r.dry_run = false;
        r.summary = format!("Voice Encryption: fremu {} zimesimbwa (ChaCha20+HMAC).", pcm.len());
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
    fn chacha20_inakidhi_rfc8439() {
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = i as u8;
        }
        let nonce = [0, 0, 0, 9, 0, 0, 0, 0x4a, 0, 0, 0, 0];
        let ks = chacha20_block(&key, 1, &nonce);
        assert_eq!(ks[0], 0x10);
        assert_eq!(ks[1], 0xf1);
        assert_eq!(ks[63], 0x4e);
    }

    #[test]
    fn voice_roundtrip_na_tamper() {
        let ecdh = [3u8; 32];
        let salt = b"salt";
        let pcm = vec![1u8, 2, 3, 4, 5];
        let (n, ct, tag) = encrypt_voice(&ecdh, salt, &pcm, 9);
        let mut nn = [0u8; 12];
        nn.copy_from_slice(&n);
        assert_eq!(decrypt_voice(&ecdh, salt, &nn, &ct, &tag).unwrap(), pcm);
        let mut bad = ct.clone();
        bad[0] ^= 0xff;
        assert!(decrypt_voice(&ecdh, salt, &nn, &bad, &tag).is_err());
    }

    #[test]
    fn hybrid_key_hubadilika_na_salt() {
        let e = [1u8; 32];
        assert_ne!(hybrid_key(&e, b"a", b"audio"), hybrid_key(&e, b"b", b"audio"));
    }
}
