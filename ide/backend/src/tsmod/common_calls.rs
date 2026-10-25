//! common_calls.rs — visaidizi vinavyoshirikiwa kwa SEHEMU 8 (Calls, modules 1-10).
//!
//! Uchanganuzi HALISI wa namba za simu, CDR (Call Detail Records), na ubora wa
//! simu (ITU-T E-model / MOS) — sio catalog.

use serde_json::{json, Value};

/// Ainisha namba: (country_code, national, valid_e164).
pub fn parse_e164(num: &str) -> (Option<String>, String, bool) {
    let plus = num.trim().starts_with('+');
    let d: String = num.chars().filter(|c| c.is_ascii_digit()).collect();
    if !plus {
        return (None, d, false);
    }
    let cc_len = if d.starts_with("255") || d.starts_with("254") || d.starts_with("256") {
        3
    } else if d.len() > 10 {
        1
    } else {
        2
    };
    let cut = cc_len.min(d.len());
    let cc = d[..cut].to_string();
    let national = d[cut..].to_string();
    (Some(cc), national, (8..=15).contains(&d.len()))
}

/// Namba za dharura (kimataifa) — hizi HAZUILIKWI kamwe.
pub const EMERGENCY: &[(&str, &str)] = &[
    ("112", "EU/jumla"), ("911", "USA/Canada"), ("999", "UK"), ("110", "Japan/China"),
    ("119", "Japan/Korea"), ("000", "Australia"), ("111", "New Zealand"), ("113", "Tanzania polisi"),
    ("114", "Tanzania zimamoto"), ("115", "Tanzania ambulansi"), ("116", "Kenya"), ("999", "Kenya"),
];

pub fn is_emergency(num: &str) -> Option<&'static str> {
    let d: String = num.chars().filter(|c| c.is_ascii_digit()).collect();
    if let Some((_, c)) = EMERGENCY.iter().find(|(n, _)| *n == d) {
        return Some(c);
    }
    // Namba ya kimataifa: +CC + code (mf. +255113 → 113).
    for (n, c) in EMERGENCY {
        if d.len() > n.len() && d.ends_with(n) {
            let cc = &d[..d.len() - n.len()];
            if (1..=3).contains(&cc.len()) {
                return Some(c);
            }
        }
    }
    None
}

/// ITU-T G.107 E-model: R-factor → MOS (1.0-4.5).
/// `latency_ms`, `jitter_ms`, `loss_pct` ni vipimo halisi vya mtandao.
pub fn e_model_mos(latency_ms: f64, jitter_ms: f64, loss_pct: f64) -> (f64, f64) {
    // R = Ro - Is - Id - Ie_eff  (Ro=93.2, Is=0 hapa; Id = delay impairment)
    let ro = 93.2_f64;
    // Id: impairment ya ucheleweshaji (up to ~30).
    let ta = latency_ms + 2.0 * jitter_ms + 10.0;
    let id = if ta < 100.0 {
        0.024 * ta
    } else if ta < 150.0 {
        0.024 * ta + 0.11 * (ta - 100.0)
    } else {
        0.024 * ta + 0.11 * (ta - 100.0) + 0.0006 * (ta - 150.0).powi(2)
    };
    // Ie_eff: impairment ya upotevu (codec G.711: Ie=0, Bpl=25.1).
    let (ie, bpl) = (0.0_f64, 25.1_f64);
    let ie_eff = ie + (95.0 - ie) * (loss_pct / (loss_pct + bpl));
    let r = (ro - id - ie_eff).clamp(0.0, 100.0);
    // R → MOS (ITU-T G.107).
    let mos = if r < 0.0 {
        1.0
    } else if r > 100.0 {
        4.5
    } else {
        1.0 + 0.035 * r + r * (r - 60.0) * (100.0 - r) * 7e-6
    };
    (r, (mos * 100.0).round() / 100.0)
}

/// Ainisha mwelekeo wa simu (wangiri / IRSF / premium) kutoka CDR.
#[derive(Debug, Clone)]
pub struct Cdr {
    pub cli: String,
    pub cld: String,
    pub duration_s: u64,
    pub direction: String, // "in" | "out"
    pub cost_units: f64,
}

/// Soma CDR kutoka params (array ya objects).
pub fn parse_cdrs(params: &Value) -> Vec<Cdr> {
    params
        .get("cdrs")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|c| Cdr {
                    cli: c.get("cli").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    cld: c.get("cld").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    duration_s: c.get("duration_s").and_then(|v| v.as_u64()).unwrap_or(0),
                    direction: c.get("direction").and_then(|v| v.as_str()).unwrap_or("out").to_string(),
                    cost_units: c.get("cost_units").and_then(|v| v.as_f64()).unwrap_or(0.0),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Wangiri: simu nyingi fupi za kuingia kutoka namba moja (kurudisha simu = gharama).
pub fn wangiri_score(cdrs: &[Cdr]) -> (u8, usize, u64) {
    let mut short_in = 0usize;
    let mut total = 0u64;
    let mut clis = std::collections::BTreeSet::new();
    for c in cdrs.iter().filter(|c| c.direction == "in") {
        total += 1;
        if c.duration_s <= 3 {
            short_in += 1;
        }
        if !c.cli.is_empty() {
            clis.insert(c.cli.clone());
        }
    }
    let score = if total >= 3 && short_in as f64 / total as f64 > 0.7 {
        (60.0 + (short_in as f64 / total as f64) * 40.0) as u8
    } else {
        0
    };
    (score, clis.len(), short_in as u64)
}

pub fn finding_json(kind: &str, detail: impl Into<String>, severity: u8) -> Value {
    json!({ "kind": kind, "detail": detail.into(), "severity": severity })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e164_inatambua_tz() {
        let (cc, nat, ok) = parse_e164("+255712345678");
        assert_eq!(cc.as_deref(), Some("255"));
        assert_eq!(nat, "712345678");
        assert!(ok);
        assert!(!parse_e164("0712345678").2);
    }

    #[test]
    fn mos_ni_juu_kwa_mtandao_mzuri() {
        let (_r, mos) = e_model_mos(20.0, 5.0, 0.0);
        assert!(mos > 4.0, "MOS ilikuwa {mos}");
        let (_r2, mos2) = e_model_mos(400.0, 60.0, 15.0);
        assert!(mos2 < 3.0, "MOS mbaya ilikuwa {mos2}");
    }

    #[test]
    fn dharura_inatambulika() {
        assert!(is_emergency("112").is_some());
        assert!(is_emergency("+255113").is_some());
        assert!(is_emergency("+255712345678").is_none());
    }

    #[test]
    fn wangiri_inagundulika() {
        let cdrs = vec![
            Cdr { cli: "+255600000001".into(), cld: "me".into(), duration_s: 1, direction: "in".into(), cost_units: 0.0 },
            Cdr { cli: "+255600000001".into(), cld: "me".into(), duration_s: 2, direction: "in".into(), cost_units: 0.0 },
            Cdr { cli: "+255600000001".into(), cld: "me".into(), duration_s: 0, direction: "in".into(), cost_units: 0.0 },
            Cdr { cli: "+255600000001".into(), cld: "me".into(), duration_s: 3, direction: "in".into(), cost_units: 0.0 },
        ];
        let (s, _n, short) = wangiri_score(&cdrs);
        assert!(s >= 60, "score {s}");
        assert_eq!(short, 4);
    }
}
