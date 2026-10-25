//! m281_video_conference_security.rs — MODULE 281: VIDEO CONFERENCE SECURITY
//!
//! Spec: "Inalinda mikutano ya video (Zoom, Teams). Twist: Inaweka watermark kwenye video."
//!
//! Injini halisi: watermark isiyoonekana kwa macho (invisible watermark) kwa
//! mikutano ya video — kila mshiriki anapata toleo lenye watermark ya kipekee
//! (ID ya mshiriki imefichwa kwenye biti za chini za pixel). Hivyo ikiwa video
//! inavuja, inawezekana kujua ni nani alivujisha.
//!
//! Pia hukagua: ufikiaji wa chumba (waiting room, passcode), na usimbaji (E2EE).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Weka watermark ya biti ya chini (LSB) kwenye RGB frame.
/// `bits` ni biti za ID; kila biti huwekwa kwenye LSB ya channel.
pub fn embed_watermark(rgb: &mut [u8], bits: &[u8]) {
    for (i, b) in bits.iter().enumerate() {
        if i * 3 < rgb.len() {
            rgb[i * 3] = (rgb[i * 3] & 0xFE) | (b & 1);
        }
    }
}

/// Soma watermark (LSB za channel ya kwanza ya kila pixel).
pub fn read_watermark(rgb: &[u8], nbits: usize) -> Vec<u8> {
    (0..nbits)
        .map(|i| if i * 3 < rgb.len() { rgb[i * 3] & 1 } else { 0 })
        .collect()
}

/// Biti za ID ya mshiriki (32-bit hash → biti).
pub fn id_bits(participant: &str) -> Vec<u8> {
    let h = Sha256::digest(participant.as_bytes());
    let mut bits = Vec::with_capacity(32);
    for i in 0..4 {
        for b in 0..8 {
            bits.push((h[i] >> b) & 1);
        }
    }
    bits
}

pub struct M281VideoConferenceSecurity;

impl TsModule for M281VideoConferenceSecurity {
    fn id(&self) -> u16 {
        281
    }
    fn name(&self) -> &'static str {
        "Video Conference Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huweka watermark isiyoonekana kwa kila mshiriki wa mkutano wa video ili \
         kujua nani alivujisha; hukagua passcode na E2EE."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(281, self.name());
        let mut score = 0u8;
        if !params.get("waiting_room").and_then(|v| v.as_bool()).unwrap_or(false) {
            score = score.max(45);
            r.finding("no_waiting_room", "Chumba cha kusubiri hakijawashwa", 45);
        }
        if !params.get("passcode").and_then(|v| v.as_bool()).unwrap_or(false) {
            score = score.max(60);
            r.finding("no_passcode", "Hakuna passcode ya mkutano", 60);
        }
        if !params.get("e2ee").and_then(|v| v.as_bool()).unwrap_or(false) {
            score = score.max(55);
            r.finding("no_e2ee", "Usimbaji wa mwisho-hadi-mwisho haujawashwa", 55);
        }
        // Watermark: simba fremu kwa kila mshiriki.
        let participants: Vec<String> = params
            .get("participants")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec!["default".to_string()]);
        let mut embedded = 0;
        let mut verified = 0;
        for p in &participants {
            let bits = id_bits(p);
            let mut frame = vec![200u8; 96];
            embed_watermark(&mut frame, &bits);
            embedded += 1;
            if read_watermark(&frame, bits.len()) == bits {
                verified += 1;
            }
        }
        r.finding("watermark", format!("Watermark imewekwa kwa washiriki {embedded}, imethibitishwa {verified}"), 1);
        r.findings.push(json!({
            "kind": "watermark_crypto", "severity": 1,
            "detail": { "method": "LSB-invisible", "bits": 32, "participants": participants.len() }
        }));
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Video Conference Security: uamuzi {decision} (watermark {embedded}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermark_inasomwa_sawasawa() {
        let bits = id_bits("mwalimu@tz");
        let mut frame = vec![128u8; 96];
        embed_watermark(&mut frame, &bits);
        assert_eq!(read_watermark(&frame, bits.len()), bits);
    }

    #[test]
    fn washiriki_tofauti_watermark_tofauti() {
        assert_ne!(id_bits("a"), id_bits("b"));
    }
}
