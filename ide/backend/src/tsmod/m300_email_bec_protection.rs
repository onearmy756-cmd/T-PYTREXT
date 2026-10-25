//! m300_email_bec_protection.rs — MODULE 300: EMAIL BEC PROTECTION
//!
//! Spec: "Inalinda dhidi ya Business Email Compromise. Twist: Inaonya kuhusu uhamisho wa pesa."
//!
//! Injini halisi: huchanganua barua pepe kwa mifumo ya BEC (Business Email
//! Compromise):
//!   - Ombi la uhamisho wa pesa / kubadilisha akaunti ya malipo (payment redirection).
//!   - Uharaka + usiri ("usimwambie mtu").
//!   - Ubadilishaji wa benki/akaunti ("tumia akaunti hii mpya").
//!   - Kuiga mkurugenzi/mtendaji (CEO fraud).

use super::common_smsemail::parse_email;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Alama ya BEC (0-100).
pub fn bec_score(raw: &str) -> (u8, Vec<(&'static str, String)>) {
    let h = parse_email(raw);
    let low = raw.to_lowercase();
    let mut s = 0u8;
    let mut why = Vec::new();
    let has = |w: &str| low.contains(w);

    let money = ["transfer", "uhamisho", "malipo", "payment", "wire", "benki", "account number", "namba ya akaunti", "invoice", "ankara", "fedha", "pesa"];
    let redir = ["new account", "akaunti mpya", "change bank", "badilisha benki", "updated banking", "updated account", "new bank details", "tumia akaunti"];
    let secrecy = ["do not tell", "usimwambie", "confidential", "siri", "between us", "kati yetu", "don't discuss"];
    let urgency = ["urgent", "haraka", "immediately", "sasa hivi", "today", "leo", "asap"];
    let exec = ["ceo", "mkurugenzi", "director", "cfo", "md", "managing director", "boss", "mkuu"];

    let money_hit = money.iter().any(|w| has(w));
    let redir_hit = redir.iter().any(|w| has(w));
    let secrecy_hit = secrecy.iter().any(|w| has(w));
    let urgency_hit = urgency.iter().any(|w| has(w));
    let exec_hit = exec.iter().any(|w| has(w));

    if money_hit {
        s = s.max(40);
        why.push(("money_request", "Ombi la malipo/uhamisho limegundulika".into()));
    }
    if redir_hit {
        s = s.max(80);
        why.push(("payment_redirection", "Kubadilisha akaunti ya malipo (payment redirection)".into()));
    }
    if money_hit && urgency_hit {
        s = s.max(70);
        why.push(("urgent_money", "Ombi la pesa la haraka".into()));
    }
    if money_hit && secrecy_hit {
        s = s.max(85);
        why.push(("secret_money", "Usiri + pesa — mfumo wa BEC".into()));
    }
    if exec_hit && money_hit {
        s = s.max(80);
        why.push(("ceo_fraud", "Kuiga mkurugenzi + ombi la pesa (CEO fraud)".into()));
    }
    (s, why)
}

pub struct M300EmailBecProtection;

impl TsModule for M300EmailBecProtection {
    fn id(&self) -> u16 {
        300
    }
    fn name(&self) -> &'static str {
        "Email BEC Protection"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huchanganua mifumo ya BEC: ombi la pesa la haraka, kubadilisha akaunti, \
         usiri, CEO fraud — humwonya mtumiaji kabla ya kutuma fedha."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(300, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let (score, why) = bec_score(raw);
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        if score >= 70 {
            r.finding("warning", "ONYO: Usifanye uhamisho wa pesa bila kuthibitisha kwa simu/njia nyingine", 90);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Email BEC Protection: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_bec_ya_akaunti_mpya() {
        let raw = "From: md@company.com\r\nSubject: Urgent payment\r\nTumia akaunti mpya kwa malipo ya invoice. Usimwambie mtu, ni siri kati yetu. Transfer leo.\r\n";
        let (s, why) = bec_score(raw);
        assert!(s >= 70, "score {s}");
        assert!(why.iter().any(|(k, _)| *k == "payment_redirection"));
    }
}
