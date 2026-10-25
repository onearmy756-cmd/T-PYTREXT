//! m275_spoofing_detection.rs — MODULE 275: SPOOFING DETECTION
//!
//! Spec: "Inagundua namba bandia. Twist: Inaonya kabla ya kuitisha."
//!
//! Injini halisi: huchanganua namba ya mpigaji (CLI) kabla ya simu kuita:
//!   - STIR/SHAKEN attestation (A kamili, B sehemu, C gateway → hatari).
//!   - Jina linaloonyeshwa likiiga taasisi (benki/telco) wakati namba ni ya kawaida.
//!   - Namba iliyofichwa/private/withheld.
//!   - Namba yenye country code tofauti na eneo la mtumiaji.
//!   - Namba inayojulikana kwenye orodha ya IOC (DB).
//!
//! Uwazi: STIR/SHAKEN ni ishara ya mtoa huduma; hatufanyi carrier lookup halisi
//! (hakuna API ya nje), tunatumia attestation + heuristics.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// Ainisha namba: rudisha (country_code, national_number, valid).
pub fn parse_e164(num: &str) -> (Option<String>, String, bool) {
    let digits: String = num.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
    let d: String = digits.chars().filter(|c| c.is_ascii_digit()).collect();
    if !digits.starts_with('+') {
        return (None, d, false);
    }
    // Namba fupi za kitaifa (255 + 9 = 12 kwa TZ; kwa ujumla 8-15).
    let cc_len = if d.starts_with("255") { 3 } else if d.len() > 10 { 1 } else { 2 };
    let cc = d[..cc_len.min(d.len())].to_string();
    let national = d[cc_len.min(d.len())..].to_string();
    let valid = (8..=15).contains(&d.len());
    (Some(cc), national, valid)
}

/// Majina ya taasisi yanayotumiwa kuiga (impersonation).
const INSTITUTIONS: &[&str] = &["bank", "benki", "crdb", "nmb", "vodacom", "airtel", "tigo", "halotel",
    "nhif", "nida", "tra", "tcra", "police", "polisi", "msd", "tanesco", "luku", "mpesa", "m-pesa"];

/// Alama ya uongo (spoof score 0-100).
pub fn spoof_score(number: &str, presented_name: &str, attestation: &str, region_cc: &str) -> (u8, Vec<(&'static str, String)>) {
    let mut score = 0u8;
    let mut why = Vec::new();
    let low = number.to_lowercase();
    if low.contains("private") || low.contains("withheld") || low.contains("anonymous") || low.contains("unknown") {
        score = score.max(55);
        why.push(("cli_withheld", "Namba imefichwa (private/withheld)".into()));
    }
    match attestation.to_uppercase().as_str() {
        "A" => {}
        "B" => {
            score = score.max(30);
            why.push(("attest_b", "STIR/SHAKEN attestation B — sehemu tu".into()));
        }
        "C" => {
            score = score.max(70);
            why.push(("attest_c", "STIR/SHAKEN attestation C — gateway ya nje (uwezekano wa kuiga)".into()));
        }
        "" => {
            score = score.max(25);
            why.push(("attest_none", "Hakuna attestation ya STIR/SHAKEN".into()));
        }
        _ => {}
    }
    let (cc, national, valid) = parse_e164(number);
    if !valid {
        score = score.max(45);
        why.push(("invalid_e164", format!("Namba si E.164 sahihi: {number}")));
    }
    // Kuiga taasisi: jina linaitaasisi lakini namba ni ya kawaida ya mkononi.
    let name_low = presented_name.to_lowercase();
    if INSTITUTIONS.iter().any(|i| name_low.contains(i)) {
        let mobile = national.len() >= 9 && matches!(national.as_bytes().first(), Some(b'6' | b'7'));
        if mobile {
            score = score.max(80);
            why.push(("brand_cli_spoof", format!("Jina '{presented_name}' linaiga taasisi lakini namba ni ya mkononi")));
        } else {
            score = score.max(35);
            why.push(("brand_name", format!("Jina la taasisi '{presented_name}' — thibitisha")));
        }
    }
    // Country code tofauti na eneo.
    if let Some(cc) = cc {
        if !region_cc.is_empty() && cc != region_cc {
            score = score.max(40);
            why.push(("foreign_cc", format!("Country code {cc} tofauti na eneo {region_cc}")));
        }
    }
    (score, why)
}

pub struct M275SpoofingDetection;

#[async_trait::async_trait]
impl TsModule for M275SpoofingDetection {
    fn id(&self) -> u16 {
        275
    }
    fn name(&self) -> &'static str {
        "Spoofing Detection"
    }
    fn group(&self) -> &'static str {
        "calls"
    }
    fn about(&self) -> &'static str {
        "Huchanganua namba ya mpigaji (CLI) kabla ya kuitisha: STIR/SHAKEN, kuiga \
         taasisi, namba iliyofichwa, country code — hutoa tahadhari mapema."
    }

    fn run(&self, _target: &str, _params: &Value) -> Result<ModReport, String> {
        Err("tumia run_db (module hii inahitaji DB kwa IOC)".into())
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(275, self.name());
        let number = params.get("number").and_then(|v| v.as_str()).unwrap_or(target);
        let name = params.get("presented_name").and_then(|v| v.as_str()).unwrap_or("");
        let att = params.get("attestation").and_then(|v| v.as_str()).unwrap_or("");
        let region = params.get("region_cc").and_then(|v| v.as_str()).unwrap_or("255");
        let (score, why) = spoof_score(number, name, att, region);
        // IOC lookup (namba inayojulikana kuwa ya ulaghai).
        let mut ioc_hit = false;
        if let Some((_, kind, sev)) = crate::tsmod::store::ioc_lookup(db, number).await {
            ioc_hit = true;
            r.findings.push(json!({ "kind": "ioc_hit", "severity": sev, "detail": format!("Namba ipo kwenye IOC ({kind})") }));
        }
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "spoof_score": score, "ioc": ioc_hit } }));
        r.summary = format!("Spoofing Detection: {number} → {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_kuiga_benki() {
        let (s, why) = spoof_score("+255712345678", "CRDB Bank", "C", "255");
        assert!(s >= 70);
        assert!(why.iter().any(|(k, _)| *k == "brand_cli_spoof"));
        assert!(why.iter().any(|(k, _)| *k == "attest_c"));
    }

    #[test]
    fn namba_halali_haina_alama() {
        let (s, _) = spoof_score("+255712345678", "Juma M.", "A", "255");
        assert!(s < 40, "score ilikuwa {s}");
    }

    #[test]
    fn e164_parsing() {
        let (cc, nat, ok) = parse_e164("+255712345678");
        assert_eq!(cc.as_deref(), Some("255"));
        assert_eq!(nat, "712345678");
        assert!(ok);
        assert!(!parse_e164("0712345678").2);
    }
}
