//! m299_email_impersonation.rs — MODULE 299: EMAIL IMPERSONATION
//!
//! Spec: "Inagundua ikiwa barua pepe inaiga mtu mwingine. Twist: Inalinganisha anwani na jina."
//!
//! Injini halisi: hulinganisha jina la mwonekano (display name) na anwani halisi:
//!   - Jina la mtu/taasisi lakini anwani ya bure (gmail/yahoo/hotmail) → kuiga.
//!   - Jina la mtu maarufu/taasisi likiiga domain nyingine (lookalike).
//!   - Levenshtein ya domain dhidi ya domain halisi ya taasisi (homograph).

use super::common_smsemail::{domain_of, parse_email};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Levenshtein (umbali wa uhariri).
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Free mail providers.
const FREE_MAIL: &[&str] = &["gmail.com", "yahoo.com", "hotmail.com", "outlook.com", "live.com", "aol.com", "mail.ru", "protonmail.com", "gmx.com", "yandex.com"];

/// Taasisi zinazojulikana (jina → domain halisi).
pub fn official_domain(brand: &str) -> Option<&'static str> {
    Some(match brand.to_lowercase().as_str() {
        "crdb" => "crdb.co.tz",
        "nmb" => "nmbbank.co.tz",
        "vodacom" | "mpesa" | "m-pesa" => "vodacom.co.tz",
        "airtel" => "airtel.co.tz",
        "tigo" | "mixx" => "tigo.co.tz",
        "halotel" => "halotel.co.tz",
        "ttcl" => "ttcl.co.tz",
        "nida" => "nida.go.tz",
        "tra" => "tra.go.tz",
        "tanesco" => "tanesco.co.tz",
        "nhif" => "nhif.or.tz",
        _ => return None,
    })
}

/// Alama ya impersonation (0-100).
pub fn impersonation_score(raw: &str) -> (u8, Vec<(&'static str, String)>) {
    let h = parse_email(raw);
    let mut s = 0u8;
    let mut why = Vec::new();
    let fd = domain_of(&format!("http://{}", h.from_addr));
    let name_low = h.from_name.to_lowercase();
    // 1) Jina la taasisi lakini anwani ya bure.
    for brand in ["crdb", "nmb", "vodacom", "airtel", "tigo", "nida", "tra", "nhif", "tanesco", "benki", "bank"] {
        if name_low.contains(brand) {
            if FREE_MAIL.contains(&fd.as_str()) {
                s = s.max(85);
                why.push(("brand_on_freemail", format!("Jina '{brand}' kwenye anwani ya bure ({fd})")));
            } else if let Some(off) = official_domain(brand) {
                if fd != off {
                    let dist = levenshtein(&fd, off);
                    let sev = if dist <= 2 { 90 } else { 70 };
                    s = s.max(sev);
                    why.push(("lookalike_domain", format!("Domain '{fd}' inaiga '{off}' (umbali {dist})")));
                }
            }
            break;
        }
    }
    // 2) Display name ni barua pepe ya mtu mwingine (rare).
    if name_low.contains('@') && !name_low.contains(&h.from_addr) {
        s = s.max(60);
        why.push(("name_is_email", "Jina linaonyesha anwani nyingine".into()));
    }
    (s, why)
}

pub struct M299EmailImpersonation;

impl TsModule for M299EmailImpersonation {
    fn id(&self) -> u16 {
        299
    }
    fn name(&self) -> &'static str {
        "Email Impersonation"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Hulinganisha jina la mwonekano na anwani halisi, na hugundua lookalike \
         domain (Levenshtein) kwa kuiga taasisi."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(299, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let (score, why) = impersonation_score(raw);
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Email Impersonation: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_kuiga_benki() {
        let raw = "From: \"CRDB Bank\" <crdb.secure@mail.ru>\r\nSubject: Malipo\r\n";
        let (s, why) = impersonation_score(raw);
        assert!(s >= 70, "score {s}");
        assert!(why.iter().any(|(k, _)| *k == "brand_on_freemail"));
    }

    #[test]
    fn levenshtein_sahihi() {
        assert_eq!(levenshtein("crdb.co.tz", "crdb.co.tz"), 0);
        assert!(levenshtein("crdb-bank.co.tz", "crdb.co.tz") <= 5);
    }
}
