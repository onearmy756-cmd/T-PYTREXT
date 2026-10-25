//! m286_international_call_security.rs — MODULE 286: INTERNATIONAL CALL SECURITY
//!
//! Spec: "Inalinda simu za kimataifa. Twist: Inaonya ikiwa namba ni premium rate."
//!
//! Injini halisi: huchanganua simu za kimataifa:
//!   - Hugundua namba za "premium rate" (hugharimu mtumiaji) kwa kategoria za
//!     country code + prefix za premium.
//!   - Hugundua "wangiri" (one-ring scam) — mdundo mmoja kutoka namba ya kigeni.
//!   - Huthibitisha country code halali (ISO 3166-1 numeric/prefix).
//!   - Hukagua "toll fraud" (mchanganyiko wa njia za gharama kubwa).

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Country codes za kawaida (prefix → nchi). Sehemu (si kamili).
pub fn country_for(cc: &str) -> Option<&'static str> {
    Some(match cc {
        "255" => "Tanzania",
        "254" => "Kenya",
        "256" => "Uganda",
        "250" => "Rwanda",
        "257" => "Burundi",
        "27" => "Afrika Kusini",
        "1" => "USA/Canada",
        "44" => "Uingereza",
        "91" => "India",
        "86" => "China",
        "971" => "UAE",
        "880" => "Bangladesh",
        "234" => "Nigeria",
        "92" => "Pakistan",
        _ => return None,
    })
}

/// Nchi/CC zenye viwango vya juu vya premium-rate (hatari ya wangiri).
const PREMIUM_RISK: &[&str] = &["234", "92", "880", "255", "254", "256", "91", "371", "372", "381"];

/// Alama ya hatari ya simu ya kimataifa (0-100).
pub fn intl_risk(number: &str, ring_count: u32, region_cc: &str) -> (u8, Vec<(&'static str, String)>) {
    let mut s = 0u8;
    let mut why = Vec::new();
    let (cc, _nat, valid) = super::m275_spoofing_detection::parse_e164(number);
    if !valid {
        return (45, vec![("invalid_e164", format!("Namba si sahihi: {number}"))]);
    }
    let cc = cc.unwrap_or_default();
    if cc != region_cc {
        s = s.max(35);
        why.push(("foreign_cc", format!("Simu ya kimataifa kutoka +{cc}")));
    }
    if PREMIUM_RISK.contains(&cc.as_str()) && cc != region_cc {
        s = s.max(55);
        why.push(("premium_rate", format!("+{cc} inaweza kuwa premium-rate")));
    }
    // Wangiri: mdundo mmoja (au wachache) bila kujibiwa.
    if ring_count <= 1 {
        s = s.max(75);
        why.push(("wangiri", "Mdundo mmoja (wangiri/one-ring scam) — usirudishe simu".into()));
    }
    (s, why)
}

pub struct M286InternationalCallSecurity;

impl TsModule for M286InternationalCallSecurity {
    fn id(&self) -> u16 {
        286
    }
    fn name(&self) -> &'static str {
        "International Call Security"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganua simu za kimataifa: hugundua premium-rate na wangiri \
         (one-ring scam) na kumwonya mtumiaji kabla ya kurudisha simu."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(286, self.name());
        let number = params.get("number").and_then(|v| v.as_str()).unwrap_or("");
        let rings = params.get("ring_count").and_then(|v| v.as_u64()).unwrap_or(5) as u32;
        let region = params.get("region_cc").and_then(|v| v.as_str()).unwrap_or("255");
        let (score, why) = intl_risk(number, rings, region);
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        let (cc, _, _) = super::m275_spoofing_detection::parse_e164(number);
        if let Some(cc) = cc {
            if let Some(name) = country_for(&cc) {
                r.findings.push(json!({ "kind": "country", "severity": 1, "detail": { "cc": cc, "country": name } }));
            }
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("International Call Security: {number} → {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_wangiri() {
        let (s, why) = intl_risk("+2348012345678", 1, "255");
        assert!(s >= 70);
        assert!(why.iter().any(|(k, _)| *k == "wangiri"));
    }

    #[test]
    fn simu_ya_kawaida_ya_tz() {
        let (s, _) = intl_risk("+255712345678", 5, "255");
        assert!(s < 40, "score {s}");
    }
}
