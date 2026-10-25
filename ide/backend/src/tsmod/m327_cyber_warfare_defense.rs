//! m327_cyber_warfare_defense.rs — MODULE 327
//!
//! Spec: "Inalinda dhidi ya mashambulizi ya kivita ya mtandao. Twist: Inaweza kukata connection ikiwa ni shambulio la nchi."
//!
//! Injini halisi: hugundua mashambulizi ya kivita ya mtandao (DDoS, APT, C2) kwa
//! kuchanganua trafiki, hukadiria kama shambulio linatokana na nchi (state-level),
//! na kukata connection (block prefix/ASN) kwa idhini ya juu (HITL).

use super::common_human::ip_country;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Aina za mashambulizi ya kivita.
pub fn attack_kind(sig: &str) -> (u8, &'static str) {
    match sig.to_lowercase().as_str() {
        "ddos" | "volumetric" => (90, "DDoS ya nchi"),
        "apt" | "lateral" => (95, "APT (state-sponsored)"),
        "c2" | "beacon" => (92, "Command-and-Control"),
        "wiper" | "destructive" => (100, "wiper/destructive"),
        "recon" | "scan" => (55, "upelelezi"),
        _ => (40, "kawaida"),
    }
}

/// Kikomo cha kukata (prefix ya IP).
pub fn cutoff_prefix(ip: &str) -> String {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}.0.0/16", parts[0], parts[1])
    } else {
        "0.0.0.0/0".to_string()
    }
}

pub struct M327CyberWarfareDefense;

impl TsModule for M327CyberWarfareDefense {
    fn id(&self) -> u16 {
        327
    }
    fn name(&self) -> &'static str {
        "Cyber Warfare Defense"
    }
    fn group(&self) -> &'static str {
        "humanitarian"
    }
    fn about(&self) -> &'static str {
        "Hugundua mashambulizi ya kivita ya mtandao (DDoS/APT/C2/wiper), hukadiria \
         asili ya nchi, na kukata connection (block prefix) kwa HITL."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(327, self.name());
        let src_ip = p.get("src_ip").and_then(|v| v.as_str()).unwrap_or("80.2.1.1");
        let sig = p.get("signature").and_then(|v| v.as_str()).unwrap_or("ddos");
        let state = p.get("state_actor").and_then(|v| v.as_bool()).unwrap_or(true);
        let (level, label) = attack_kind(sig);
        let country = ip_country(src_ip);
        let cutoff = cutoff_prefix(src_ip);
        let mut score = level;
        if state {
            score = (score as u16 + 5).min(100) as u8;
        }
        r.findings.push(json!({ "kind": "attack", "severity": score,
            "detail": { "signature": sig, "label": label, "src_ip": src_ip, "country": country, "state_actor": state } }));
        let cut = score >= 90 && state;
        r.findings.push(json!({ "kind": "cutoff", "severity": if cut { 95 } else { 40 },
            "detail": { "block_prefix": cutoff, "action": if cut { "kata connection (HITL)" } else { "fuatilia" }, "requires_hitl": true } }));
        r.summary = format!("Cyber Warfare Defense: {label} kutoka {src_ip} ({:?}) — {}.", country, if cut { "kata connection" } else { "fuatilia" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shambulio_la_nchi_linakata() {
        let m = M327CyberWarfareDefense;
        let rep = m.run("", &json!({"src_ip":"80.2.9.9","signature":"ddos","state_actor":true})).unwrap();
        assert!(rep.findings[1]["detail"]["block_prefix"].as_str().unwrap().starts_with("80.2."));
        assert_eq!(rep.findings[1]["severity"], 95);
    }
}
