//! m279_recording_protection.rs — MODULE 279: RECORDING PROTECTION
//!
//! Spec: "Inagundua kama simu inarekodiwa. Twist: Inaonya ikiwa upande mwingine anarekodi."
//!
//! Injini halisi: huchanganua sifa za mstari wa simu kugundua kurekodiwa:
//!   - Beep ya mara kwa mara (recording notification) kwenye sauti (PCM).
//!   - Kushuka kwa kiwango cha sauti (attenuation) mwanzoni — ishara ya rekodi.
//!   - Uwepo wa sauti ya tatu (mchanganyiko) kwa uhusiano wa chaneli.
//!   - Ishara za maandishi ("this call is being recorded").

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Rudisha idadi ya "beep" (mlipuko mfupi wa sauti) kwenye PCM.
pub fn count_beeps(pcm: &[i16], sample_rate: u32) -> usize {
    let win = (sample_rate / 100).max(1) as usize; // 10ms
    let mut levels: Vec<f64> = Vec::new();
    for c in pcm.chunks(win) {
        if c.is_empty() {
            continue;
        }
        let rms = (c.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / c.len() as f64).sqrt();
        levels.push(rms);
    }
    let mean = if levels.is_empty() { 0.0 } else { levels.iter().sum::<f64>() / levels.len() as f64 };
    let thr = mean * 3.0 + 500.0;
    // Beep = mlipuko mfupi (<= 30 windows) unaozidi kizingiti.
    let mut beeps = 0;
    let mut run = 0;
    for &l in &levels {
        if l > thr {
            run += 1;
        } else {
            if run > 0 && run <= 30 {
                beeps += 1;
            }
            run = 0;
        }
    }
    if run > 0 && run <= 30 {
        beeps += 1;
    }
    beeps
}

pub struct M279RecordingProtection;

impl TsModule for M279RecordingProtection {
    fn id(&self) -> u16 {
        279
    }
    fn name(&self) -> &'static str {
        "Recording Protection"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Hugundua kama simu inarekodiwa (beep, attenuation, chaneli ya tatu) na \
         kumwonya mtumiaji ikiwa upande mwingine anarekodi."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(279, self.name());
        let sr = params.get("sample_rate").and_then(|v| v.as_u64()).unwrap_or(8000) as u32;
        let mut score = 0u8;
        let mut reasons: Vec<&'static str> = Vec::new();

        if let Some(arr) = params.get("pcm").and_then(|v| v.as_array()) {
            let pcm: Vec<i16> = arr.iter().filter_map(|v| v.as_i64()).map(|v| v as i16).collect();
            let beeps = count_beeps(&pcm, sr);
            if beeps >= 2 {
                score = score.max(80);
                reasons.push("periodic_beep");
                r.finding("periodic_beep", format!("Beep {beeps} zimegundulika — ishara ya kurekodi"), 80);
            } else if beeps == 1 {
                score = score.max(45);
                reasons.push("single_beep");
                r.finding("single_beep", "Beep moja imegundulika", 45);
            }
        }
        if params.get("attenuation_db").and_then(|v| v.as_f64()).unwrap_or(0.0) >= 3.0 {
            score = score.max(60);
            reasons.push("attenuation");
            r.finding("attenuation", "Kiwango cha sauti kimepungua — dalili ya rekodi", 60);
        }
        if params.get("third_party_channel").and_then(|v| v.as_bool()).unwrap_or(false) {
            score = score.max(85);
            reasons.push("third_party");
            r.finding("third_party", "Chaneli ya tatu imegundulika — mtu wa tatu anasikiliza", 85);
        }
        if let Some(t) = params.get("transcript").and_then(|v| v.as_str()) {
            let low = t.to_lowercase();
            if low.contains("being recorded") || low.contains("inarekodiwa") || low.contains("this call is recorded") {
                score = score.max(75);
                reasons.push("spoken_notice");
                r.finding("spoken_notice", "Taarifa ya kurekodi imetajwa", 75);
            }
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score, "reasons": reasons } }));
        r.summary = format!(
            "Recording Protection: uamuzi {decision} (score {score}){}.",
            if score >= 40 { " — ONYO: simu inaweza kurekodiwa" } else { "" }
        );
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_beep_za_mara_kwa_mara() {
        let mut pcm: Vec<i16> = vec![0; 8000];
        // Weka beep 3 (mlipuko mfupi wa sauti kubwa).
        for b in [1000usize, 3000, 5000] {
            for i in 0..80 {
                pcm[b + i] = 20000;
            }
        }
        assert!(count_beeps(&pcm, 8000) >= 2);
    }
}
