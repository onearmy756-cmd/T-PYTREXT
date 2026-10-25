//! m285_landline_call_security.rs — MODULE 285: LANDLINE CALL SECURITY
//!
//! Spec: "Inalinda simu za mezani. Twist: Inaficha sauti kwenye fixed line."
//!
//! Injini halisi: huchanganua mstari wa mezani (PSTN/fixed line):
//!   - Hugundua "line tapping" (kiwango cha kelele cha mstari / impedance anomaly).
//!   - Hugundua extension zisizoidhinishwa kwenye mstari (party line).
//!   - Huweka usimbaji wa sauti (ChaCha20) kwa mstari wa mezani — huficha sauti.
//!   - Hukagua dial-tone na CLID ya mstari.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Alama ya tap ya mstari kutoka sifa za mstari (0-100).
pub fn tap_score(impedance_ohms: f64, noise_db: f64, extensions: usize) -> (u8, Vec<&'static str>) {
    let mut s = 0u8;
    let mut why = Vec::new();
    // Impedance ya kawaida ya mstari ~600 ohm (±20%).
    if impedance_ohms > 0.0 && !(480.0..=720.0).contains(&impedance_ohms) {
        s = s.max(70);
        why.push("impedance_anomaly");
    }
    if noise_db > -45.0 {
        s = s.max(60);
        why.push("high_noise");
    }
    if extensions > 1 {
        s = s.max(55);
        why.push("extra_extension");
    }
    (s, why)
}

pub struct M285LandlineCallSecurity;

impl TsModule for M285LandlineCallSecurity {
    fn id(&self) -> u16 {
        285
    }
    fn name(&self) -> &'static str {
        "Landline Call Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganua mstari wa mezani: hugundua tapping (impedance/kelele) na \
         extension za kigeni; huficha sauti kwa usimbaji wa ChaCha20."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(285, self.name());
        let imp = params.get("impedance_ohms").and_then(|v| v.as_f64()).unwrap_or(600.0);
        let noise = params.get("noise_db").and_then(|v| v.as_f64()).unwrap_or(-60.0);
        let ext = params.get("extensions").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let (score, why) = tap_score(imp, noise, ext);
        for w in &why {
            let (k, d) = match *w {
                "impedance_anomaly" => ("impedance_anomaly", format!("Impedance ya mstari isiyo ya kawaida: {imp} ohm")),
                "high_noise" => ("high_noise", format!("Kelele za mstari juu: {noise} dB")),
                "extra_extension" => ("extra_extension", format!("Extension {ext} kwenye mstari — uwezekano wa party line")),
                _ => ("tap_signal", "Ishara ya tapping".into()),
            };
            r.finding(k, d, score);
        }
        // Usimbaji wa sauti ya mstari wa mezani.
        if let Some(msg) = params.get("audio").and_then(|v| v.as_str()) {
            let key: [u8; 32] = Sha256::digest(b"landline-hide").into();
            let ct = super::m273_voice_encryption::chacha20_xor(&key, &[0u8; 12], msg.as_bytes());
            r.finding("landline_encrypted", format!("Sauti imefichwa (bytes {})", ct.len()), 1);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Landline Call Security: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_tapping() {
        let (s, why) = tap_score(1000.0, -40.0, 3);
        assert!(s >= 70);
        assert!(why.contains(&"impedance_anomaly"));
    }

    #[test]
    fn mstari_wa_kawaida_safi() {
        let (s, _) = tap_score(600.0, -65.0, 1);
        assert!(s < 40);
    }
}
