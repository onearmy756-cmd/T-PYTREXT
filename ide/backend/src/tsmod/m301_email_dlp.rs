//! m301_email_dlp.rs — MODULE 301: EMAIL DLP (Data Loss Prevention)
//!
//! Spec: "Inazuia uvujaji wa data kwenye barua pepe. Twist: Inagundua namba za kadi, NIDA."
//!
//! Injini halisi: huchanganua barua pepe inayotoka kwa PII:
//!   - Namba za kadi (Luhn), NIDA (tarakimu 20), namba za simu, CVV patterns.
//!   - Hutumia masking (haisambazi PII); huzuia kutoka ikiwa PII imegundulika.

use super::common_smsemail::{detect_pii, pii_json};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// CVV/CVC pattern (3-4 tarakimu karibu na "cvv").
pub fn has_cvv(text: &str) -> bool {
    let low = text.to_lowercase();
    if let Some(i) = low.find("cvv").or_else(|| low.find("cvc")) {
        let rest = &text[i..];
        let digits: String = rest.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).take(4).collect();
        return (3..=4).contains(&digits.len());
    }
    false
}

pub struct M301EmailDlp;

impl TsModule for M301EmailDlp {
    fn id(&self) -> u16 {
        301
    }
    fn name(&self) -> &'static str {
        "Email DLP"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huzuia uvujaji wa data: hugundua namba za kadi (Luhn), NIDA, simu na CVV \
         kwenye barua pepe; huweka masking bila kusambaza PII."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(301, self.name());
        let body = params.get("body").and_then(|v| v.as_str()).unwrap_or(target);
        let mut score = 0u8;
        let pii = detect_pii(body);
        for p in &pii {
            let sev = match p.kind {
                "card_number" => 85,
                "nida_number" => 75,
                "phone_tz" => 45,
                _ => 40,
            };
            score = score.max(sev);
            r.findings.push(json!({ "kind": "pii", "severity": sev, "detail": pii_json(p) }));
        }
        if has_cvv(body) {
            score = score.max(90);
            r.finding("cvv", "CVV imegundulika — hatari kubwa ya uvujaji", 90);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score,
            "detail": { "decision": decision, "pii_count": pii.len(), "note": "PII imefichwa (masked)" } }));
        r.summary = format!("Email DLP: uamuzi {decision} (PII {}, score {score}).", pii.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_pii_na_cvv() {
        let r = M301EmailDlp.run("kadi yangu 4111111111111111 cvv 123 nida 19900101123456789012", &json!({})).unwrap();
        let dec = r.findings.iter().find(|f| f["kind"] == "decision").unwrap();
        assert_eq!(dec["detail"]["decision"], "block");
        assert!(r.findings.iter().any(|f| f["kind"] == "pii"));
    }
}
