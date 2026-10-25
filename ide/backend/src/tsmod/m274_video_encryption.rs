//! m274_video_encryption.rs — MODULE 274: VIDEO ENCRYPTION
//!
//! Spec: "Inaficha video calls. Twist: Inaficha video frame kwa frame."
//!
//! Injini halisi: kila fremu ya video (YUV/RGB) inasimbwa PEKEE kwa nonce ya
//! fremu hiyo (frame counter) — hivyo fremu moja iliyoibiwa haiwezi kutumiwa
//! kuondoa nyingine. Inatumia ChaCha20 + HMAC-SHA256 (kutoka m273).

use super::m273_voice_encryption::{aead_tag, chacha20_xor, hybrid_key};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub struct M274VideoEncryption;

/// Simba fremu moja ya video. Nonce = counter ya fremu (8B) + salt (4B).
pub fn encrypt_frame(key: &[u8; 32], frame_no: u64, salt: &[u8], frame: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&frame_no.to_le_bytes());
    for i in 0..4 {
        nonce[8 + i] = salt.get(i).copied().unwrap_or(0);
    }
    let ct = chacha20_xor(key, &nonce, frame);
    let tag = aead_tag(key, &nonce, &ct);
    (nonce.to_vec(), ct, tag)
}

impl TsModule for M274VideoEncryption {
    fn id(&self) -> u16 {
        274
    }
    fn name(&self) -> &'static str {
        "Video Encryption"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Inaficha fremu za video kila moja kwa nonce yake (ChaCha20+HMAC). \
         Fremu iliyoibiwa haiwezi kuondoa nyingine."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("selftest");
        let frames = params.get("frames").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
        match op {
            "selftest" | "encrypt" => self.run_enc(frames),
            other => Err(format!("op '{other}' haipo: selftest|encrypt")),
        }
    }
}

impl M274VideoEncryption {
    fn run_enc(&self, frames: usize) -> Result<ModReport, String> {
        let mut r = ModReport::new(274, self.name());
        let ecdh = Sha256::digest(b"video-ecdh").into();
        let salt = b"vid1";
        let key = hybrid_key(&ecdh, salt, b"video");
        // Fremu za majaribio (kila fremu 640 bytes = 20x32).
        let mut all_unique = true;
        let mut seen: Vec<Vec<u8>> = Vec::new();
        let mut roundtrip_ok = true;
        for f in 0..frames.min(60) {
            let frame: Vec<u8> = (0..640).map(|i| ((i as u64 * 13 + f as u64 * 7) % 256) as u8).collect();
            let (nonce, ct, tag) = encrypt_frame(&key, f as u64, salt, &frame);
            let mut n = [0u8; 12];
            n.copy_from_slice(&nonce);
            if chacha20_xor(&key, &n, &ct) != frame {
                roundtrip_ok = false;
            }
            if seen.contains(&ct) {
                all_unique = false;
            }
            seen.push(ct);
            let _ = tag;
        }
        r.finding("per_frame_nonce", if all_unique { "Kila fremu ina ciphertext ya kipekee" } else { "Fremu zinarudia" }, if all_unique { 1 } else { 90 });
        r.finding("roundtrip", if roundtrip_ok { "Fremu zinaondolewa sawasawa" } else { "Mzunguko umeshindwa" }, if roundtrip_ok { 1 } else { 90 });
        r.findings.push(json!({
            "kind": "video_crypto", "severity": 1,
            "detail": { "alg": "ChaCha20+HMAC-SHA256", "mode": "per-frame nonce",
                        "frames": frames.min(60), "frame_bytes": 640 }
        }));
        r.applied = true;
        r.dry_run = false;
        r.summary = format!("Video Encryption: fremu {} zimesimbwa kila moja kwa nonce yake.", frames.min(60));
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fremu_zinatofautiana_na_zinarejea() {
        let ecdh = [5u8; 32];
        let key = hybrid_key(&ecdh, b"s", b"video");
        let frame = vec![9u8; 128];
        let (n0, c0, _) = encrypt_frame(&key, 0, b"s", &frame);
        let (n1, c1, _) = encrypt_frame(&key, 1, b"s", &frame);
        assert_ne!(c0, c1, "fremu tofauti zina ciphertext tofauti");
        let mut nn = [0u8; 12];
        nn.copy_from_slice(&n0);
        assert_eq!(chacha20_xor(&key, &nn, &c0), frame);
        assert_ne!(n0, n1);
    }
}
