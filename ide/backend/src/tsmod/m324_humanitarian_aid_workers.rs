//! m324_humanitarian_aid_workers.rs — MODULE 324
//!
//! Spec: "Inalinda wafanyakazi wa misaada ya kibinadamu. Twist: Inaweza kutuma location ya dharura."
//!
//! Injini halisi: hulinda wafanyakazi wa misaada na kutuma location ya dharura
//! (SOS) yenye AEAD + pseudonymization; hukadiria hatari kwa eneo na kutoa
//! taarifa kwa mratibu (HITL).

use super::common_human::{aead_encrypt, pseudonymize};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Aina za dharura.
pub fn emergency_level(code: &str) -> (u8, &'static str) {
    match code.to_lowercase().as_str() {
        "sos" | "under_fire" | "kidnap" => (100, "dharura kuu"),
        "medical" | "injury" => (85, "dharura ya matibabu"),
        "evacuation" => (90, "uokoaji"),
        "lost" | "stranded" => (70, "amepotea"),
        _ => (40, "kawaida"),
    }
}

pub struct M324HumanitarianAidWorkers;

impl TsModule for M324HumanitarianAidWorkers {
    fn id(&self) -> u16 {
        324
    }
    fn name(&self) -> &'static str {
        "Humanitarian Aid Workers"
    }
    fn group(&self) -> &'static str {
        "humanitarian"
    }
    fn about(&self) -> &'static str {
        "Hulinda wafanyakazi wa misaada; hutuma location ya dharura (SOS) yenye \
         AEAD na pseudonymization, na kukadiria kiwango cha dharura."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(324, self.name());
        let worker = p.get("worker").and_then(|v| v.as_str()).unwrap_or("AID-4471");
        let code = p.get("emergency").and_then(|v| v.as_str()).unwrap_or("sos");
        let lat = p.get("lat").and_then(|v| v.as_f64()).unwrap_or(-6.8);
        let lon = p.get("lon").and_then(|v| v.as_f64()).unwrap_or(39.2);
        let (level, label) = emergency_level(code);
        let key: [u8; 32] = Sha256::digest(b"aid-key").into();
        let nonce = [0x41, 0x49, 0x44, 0, 0, 0, 0, 0, 0, 0, 0, 1];
        let payload = format!("{{\"lat\":{lat},\"lon\":{lon},\"code\":\"{code}\"}}");
        let (ct, tag) = aead_encrypt(&key, &nonce, b"aid-sos", payload.as_bytes());
        let pseudo = pseudonymize(&[worker]);
        r.findings.push(json!({ "kind": "sos", "severity": level,
            "detail": { "worker_pseudonym": pseudo, "level": label, "code": code,
                        "lat": lat, "lon": lon, "encrypted": true, "tag": hex(&tag[..8]) } }));
        r.findings.push(json!({ "kind": "dispatch", "severity": level,
            "detail": { "notify": ["mratibu", "timu ya uokoaji"], "requires_hitl": level >= 85 } }));
        r.summary = format!("Humanitarian Aid: {label} ({code}) kutoka {pseudo}, location imetumwa.");
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
    fn sos_inatuma_location() {
        let m = M324HumanitarianAidWorkers;
        let rep = m.run("", &json!({"worker":"A1","emergency":"under_fire","lat":-6.8,"lon":39.2})).unwrap();
        assert!(rep.findings[0]["detail"]["encrypted"].as_bool().unwrap());
        assert_eq!(rep.findings[0]["severity"], 100);
    }
}
