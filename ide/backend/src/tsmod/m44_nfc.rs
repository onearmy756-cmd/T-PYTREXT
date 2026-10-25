//! m44_nfc.rs — MODULE 44: NFC SECURITY
//!
//! Spec: "Inalinda NFC transactions, inahitaji uthibitisho. Twist: inahitaji
//! fingerprint kabla ya NFC payment > TSh 50,000."
//!
//! Hii si catalog: inasoma adapters halisi (/sys/class/nfc) na ina engine ya
//! uamuzi wa malipo (TSh) inayohitaji uthibitisho wa kibaiometriki juu ya kikomo.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Kikomo cha malipo bila fingerprint (TSh).
pub const BIOMETRIC_LIMIT_TSH: u64 = 50_000;

pub struct M44Nfc;

/// Uamuzi wa malipo ya NFC.
#[derive(Debug, Clone, PartialEq)]
pub enum PayDecision {
    /// Ruhusa moja kwa moja (chini ya kikomo).
    Allow,
    /// Inahitaji fingerprint.
    RequireBiometric,
    /// Imekataliwa (fingerprint haikuthibitishwa au kikomo cha juu).
    Deny,
}

/// Amua malipo: kikomo + uthibitisho wa kibaiometriki.
pub fn decide_payment(amount_tsh: u64, biometric_ok: bool, daily_total_tsh: u64) -> PayDecision {
    // Kikomo cha siku kwa malipo yasiyo na uthibitisho: 3x kikomo cha moja.
    let daily_cap = BIOMETRIC_LIMIT_TSH * 3;
    if amount_tsh > BIOMETRIC_LIMIT_TSH {
        if biometric_ok {
            PayDecision::Allow
        } else {
            PayDecision::RequireBiometric
        }
    } else if daily_total_tsh + amount_tsh > daily_cap {
        if biometric_ok {
            PayDecision::Allow
        } else {
            PayDecision::Deny
        }
    } else {
        PayDecision::Allow
    }
}

impl TsModule for M44Nfc {
    fn id(&self) -> u16 {
        44
    }
    fn name(&self) -> &'static str {
        "NFC Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Husoma adapters halisi za NFC na kutekeleza sera ya malipo: fingerprint \
         inahitajika kwa malipo zaidi ya TSh 50,000."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "payment" => self.payment(params),
            other => Err(format!("op '{other}' haipo: audit|payment")),
        }
    }
}

impl M44Nfc {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(44, self.name());
        match std::fs::read_dir("/sys/class/nfc") {
            Ok(rd) => {
                let n = rd.flatten().count();
                if n == 0 {
                    r.finding("no_nfc", "Hakuna adapter ya NFC kwenye host hii", 5);
                } else {
                    r.finding("nfc_adapter", format!("Adapters za NFC: {n}"), 2);
                }
            }
            Err(_) => {
                r.finding("nfc_unavailable", "NFC haipo kwenye host hii", 5);
            }
        }
        r.finding(
            "biometric_policy",
            format!("Kikomo cha malipo bila fingerprint: TSh {BIOMETRIC_LIMIT_TSH}"),
            2,
        );
        r.summary = "Hali ya NFC imechunguzwa.".into();
        Ok(r)
    }

    fn payment(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(44, self.name());
        let amount = params.get("amount_tsh").and_then(|v| v.as_u64()).unwrap_or(0);
        let biometric = params
            .get("biometric_ok")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let daily = params
            .get("daily_total_tsh")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let decision = decide_payment(amount, biometric, daily);
        let (label, sev) = match decision {
            PayDecision::Allow => ("ALLOW", 2),
            PayDecision::RequireBiometric => ("REQUIRE_BIOMETRIC", 40),
            PayDecision::Deny => ("DENY", 70),
        };
        r.findings.push(json!({
            "kind": "payment_decision", "severity": sev,
            "detail": {
                "amount_tsh": amount,
                "biometric_ok": biometric,
                "daily_total_tsh": daily,
                "decision": label,
                "limit_tsh": BIOMETRIC_LIMIT_TSH,
            }
        }));
        r.summary = format!("Malipo TSh {amount}: {label}.");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chini_ya_kikomo_inaruhusiwa() {
        assert_eq!(decide_payment(20_000, false, 0), PayDecision::Allow);
    }

    #[test]
    fn juu_ya_kikomo_inahitaji_fingerprint() {
        assert_eq!(
            decide_payment(80_000, false, 0),
            PayDecision::RequireBiometric
        );
        assert_eq!(decide_payment(80_000, true, 0), PayDecision::Allow);
    }

    #[test]
    fn kikomo_cha_siku_kinakataliwa() {
        // amount 40k (chini ya kikomo cha moja) lakini jumla ya siku 130k +
        // 40k = 170k > cap 150k -> Deny bila biometric.
        assert_eq!(decide_payment(40_000, false, 130_000), PayDecision::Deny);
        assert_eq!(decide_payment(40_000, true, 130_000), PayDecision::Allow);
    }
}
