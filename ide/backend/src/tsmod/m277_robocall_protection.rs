//! m277_robocall_protection.rs — MODULE 277: ROBOCALL PROTECTION
//!
//! Spec: "Inagundua na kuzuia simu za kiotomatiki. Twist: Inagundua sauti iliyorekodiwa."
//!
//! Injini halisi ya uchanganuzi wa sauti (PCM i16 mono):
//!   - Kugundua SEGMENTI ZINAZORUDIA (sauti iliyorekodiwa/loop) kwa kulinganisha
//!     hash za vipande (framing 100ms).
//!   - Kutofautiana kwa sauti (RMS variance) — robocall ina sauti tambarare.
//!   - Muda mrefu wa ukimya usio wa kawaida (IVR pause).
//!   - Ishara za maandishi za IVR ("press 1", "bonyeza 1", "call is important").

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Ugunduzi mmoja.
#[derive(Debug, Clone)]
pub struct RoboFind {
    pub kind: &'static str,
    pub severity: u8,
    pub detail: String,
}

/// Changanua PCM i16 → sifa za sauti.
pub fn pcm_features(pcm: &[i16], sample_rate: u32) -> (f64, f64, f64, f64) {
    // Rudisha (rms, rms_variance, silence_ratio, zcr_mean).
    let win = (sample_rate / 10).max(1) as usize; // 100ms
    let mut rms_v = Vec::new();
    let mut zcr_v = Vec::new();
    let mut silent = 0usize;
    let mut total = 0usize;
    for chunk in pcm.chunks(win) {
        if chunk.is_empty() {
            continue;
        }
        let sum_sq: f64 = chunk.iter().map(|&s| (s as f64) * (s as f64)).sum();
        let rms = (sum_sq / chunk.len() as f64).sqrt();
        rms_v.push(rms);
        let zc = chunk.windows(2).filter(|w| (w[0] >= 0) != (w[1] >= 0)).count();
        zcr_v.push(zc as f64 / chunk.len() as f64);
        if rms < 200.0 {
            silent += 1;
        }
        total += 1;
    }
    let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
    let rms_mean = mean(&rms_v);
    let var = if rms_v.len() > 1 {
        rms_v.iter().map(|x| (x - rms_mean).powi(2)).sum::<f64>() / rms_v.len() as f64
    } else {
        0.0
    };
    let silence_ratio = if total == 0 { 0.0 } else { silent as f64 / total as f64 };
    (rms_mean, var, silence_ratio, mean(&zcr_v))
}

/// Kugundua vipande vinavyorudia (recorded loop) kwa hash ya kila kipande.
pub fn repeated_segments(pcm: &[i16], sample_rate: u32) -> usize {
    let win = (sample_rate / 10).max(1) as usize;
    let mut hashes: Vec<[u8; 8]> = Vec::new();
    for chunk in pcm.chunks(win) {
        // Quantize (kupunguza kelele) kabla ya hash ili sauti ile ile igundulike.
        let q: Vec<i16> = chunk.iter().map(|&s| s / 500).collect();
        let h = Sha256::digest(bytemuck_i16(&q));
        let mut t = [0u8; 8];
        t.copy_from_slice(&h[..8]);
        hashes.push(t);
    }
    let mut repeats = 0;
    for i in 0..hashes.len() {
        for j in (i + 1)..hashes.len() {
            if hashes[i] == hashes[j] {
                repeats += 1;
            }
        }
    }
    repeats
}

fn bytemuck_i16(v: &[i16]) -> Vec<u8> {
    let mut o = Vec::with_capacity(v.len() * 2);
    for &s in v {
        o.extend_from_slice(&s.to_le_bytes());
    }
    o
}

/// Ishara za maandishi za robocall/IVR.
pub fn text_signals(text: &str) -> Vec<RoboFind> {
    let low = text.to_lowercase();
    let mut out = Vec::new();
    for (k, words, sev, d) in [
        ("ivr_press", &["press 1", "bonyeza 1", "press one", "bonyeza moja"][..], 60u8, "Mfumo wa IVR (bonyeza namba)"),
        ("ivr_important", &["your call is important", "simu yako ni muhimu", "please hold", "tafadhali subiri"][..], 55, "Ujumbe wa kiotomatiki wa kusubiri"),
        ("recorded_notice", &["this call may be recorded", "simu hii inaweza kurekodiwa"][..], 40, "Taarifa ya kurekodi"),
        ("scripted_offer", &["congratulations you have been selected", "umeteuliwa", "special offer for you"][..], 65, "Ofa iliyorekodiwa ya kiotomatiki"),
        ("press_zero", &["press 0 to speak", "bonyeza 0"][..], 45, "Uhamisho wa kiotomatiki"),
    ] {
        if words.iter().any(|w| low.contains(w)) {
            out.push(RoboFind { kind: k, severity: sev, detail: d.into() });
        }
    }
    out
}

pub struct M277RobocallProtection;

impl TsModule for M277RobocallProtection {
    fn id(&self) -> u16 {
        277
    }
    fn name(&self) -> &'static str {
        "Robocall Protection"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganua sauti (PCM) kugundua vipande vinavyorudia, sauti tambarare na \
         ukimya wa IVR — huzuia simu za kiotomatiki zilizorekodiwa."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(277, self.name());
        let sr = params.get("sample_rate").and_then(|v| v.as_u64()).unwrap_or(8000) as u32;
        let mut score = 0u8;

        if let Some(arr) = params.get("pcm").and_then(|v| v.as_array()) {
            let pcm: Vec<i16> = arr.iter().filter_map(|v| v.as_i64()).map(|v| v as i16).collect();
            let (rms, var, sil, zcr) = pcm_features(&pcm, sr);
            let repeats = repeated_segments(&pcm, sr);
            if repeats > 0 {
                score = score.max(75);
                r.finding("recorded_loop", format!("Vipande {repeats} vinarudia — sauti iliyorekodiwa"), 75);
            }
            if var < 1000.0 && rms > 200.0 {
                score = score.max(50);
                r.finding("flat_audio", format!("Sauti tambarare (variance {var:.0}) — mfumo wa kiotomatiki"), 50);
            }
            if sil > 0.4 {
                score = score.max(40);
                r.finding("ivr_silence", format!("Ukimya usio wa kawaida ({:.0}%)", sil * 100.0), 40);
            }
            r.findings.push(json!({
                "kind": "audio_stats", "severity": 1,
                "detail": { "rms": rms, "rms_variance": var, "silence_ratio": sil, "zcr": zcr, "repeats": repeats }
            }));
        }
        if let Some(t) = params.get("transcript").and_then(|v| v.as_str()) {
            for f in text_signals(t) {
                score = score.max(f.severity);
                r.finding(f.kind, f.detail, f.severity);
            }
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Robocall Protection: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_sauti_iliyorekodiwa() {
        // Sauti inayorudia kipande kimoja mara nyingi = recorded loop.
        let seg: Vec<i16> = (0..800).map(|i| ((i * 40) % 3000) as i16).collect();
        let mut pcm = Vec::new();
        for _ in 0..5 {
            pcm.extend_from_slice(&seg);
        }
        let reps = repeated_segments(&pcm, 8000);
        assert!(reps > 0, "vipande vinarudia: {reps}");
    }

    #[test]
    fn inagundua_ivr_kwenye_maandishi() {
        let f = text_signals("Your call is important, please hold. Press 1 for English.");
        assert!(f.iter().any(|x| x.kind == "ivr_press"));
        assert!(f.iter().any(|x| x.kind == "ivr_important"));
    }

    #[test]
    fn sauti_ya_kawaida_haina_alama_nyingi() {
        let f = text_signals("Habari Juma, tunaongea kuhusu mkutano wa kesho.");
        assert!(f.is_empty());
    }
}
