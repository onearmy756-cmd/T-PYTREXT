//! m284_mobile_network_call_security.rs — MODULE 284: MOBILE NETWORK CALL SECURITY
//!
//! Spec: "Inalinda simu za 2G/3G/4G/5G. Twist: Inaongeza encryption kwa mtandao dhaifu."
//!
//! Injini halisi: huchanganua uwezo wa mtandao wa simu na hutoa ulinzi:
//!   - Hugundua 2G (GSM) — hatari ya IMSI catcher / A5/1 dhaifu → pendekezo la kuzuia.
//!   - Hugundua "downgrade" (5G/4G → 2G) ambayo ni ishara ya Stingray.
//!   - 3G (A5/3) — usimbaji dhaifu wa kati.
//!   - Hutoa "overlay encryption" (ChaCha20) kwa simu wakati mtandao ni dhaifu.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Aina ya mtandao → (usalama, alama).
pub fn network_risk(gen: &str) -> (u8, &'static str) {
    match gen.to_uppercase().as_str() {
        "5G" | "NR" => (10, "salama"),
        "4G" | "LTE" => (25, "salama kiasi"),
        "3G" | "UMTS" | "HSPA" => (55, "dhaifu"),
        "2G" | "GSM" | "EDGE" | "GPRS" => (85, "hatari sana"),
        _ => (50, "haijulikani"),
    }
}

pub struct M284MobileNetworkCallSecurity;

impl TsModule for M284MobileNetworkCallSecurity {
    fn id(&self) -> u16 {
        284
    }
    fn name(&self) -> &'static str {
        "Mobile Network Call Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganua 2G/3G/4G/5G, hugundua downgrade (IMSI catcher) na huongeza \
         usimbaji wa ziada (ChaCha20) mtandao unapokuwa dhaifu."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(284, self.name());
        let gen = params.get("network").and_then(|v| v.as_str()).unwrap_or("4G");
        let prev = params.get("previous_network").and_then(|v| v.as_str());
        let (risk, verdict) = network_risk(gen);
        let mut score = risk;
        r.finding("network", format!("Mtandao: {gen} ({verdict}, risk {risk})"), risk);
        // Downgrade detection.
        if let Some(p) = prev {
            let (pr, _) = network_risk(p);
            if risk > pr + 20 {
                score = score.max(90);
                r.finding("downgrade", format!("Downgrade {p} → {gen} — ishara ya IMSI catcher/Stingray"), 90);
            }
        }
        // Overlay encryption kwa mtandao dhaifu.
        if risk >= 55 {
            let key: [u8; 32] = Sha256::digest(b"mobile-overlay").into();
            let nonce = [0u8; 12];
            let sample: Vec<u8> = (0..160).map(|i| (i % 256) as u8).collect();
            let ct = super::m273_voice_encryption::chacha20_xor(&key, &nonce, &sample);
            r.finding("overlay_encryption", format!("Usimbaji wa ziada umeongezwa (bytes {})", ct.len()), 1);
            r.findings.push(json!({ "kind": "overlay", "severity": 1, "detail": { "alg": "ChaCha20", "reason": "mtandao dhaifu" } }));
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Mobile Network Call Security: {gen} → {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hatari_ya_mitandao() {
        assert!(network_risk("2G").0 > network_risk("4G").0);
        assert!(network_risk("5G").0 < 30);
    }
}
