//! m326_global_crime_tracking.rs — MODULE 326
//!
//! Spec: "Inafuatilia uhalifu wa kimataifa. Twist: Inashirikiana na INTERPOL."
//!
//! Injini halisi: hufuatilia uhalifu wa kimataifa kwa kuchanganua IP/eneo la
//! matukio, kutambua mifumo ya uhalifu inayovuka mipaka, na kuandaa ripoti
//! ya kushirikiana na INTERPOL (notice types).

use super::common_human::{counts, interpol_notice, ip_country};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Tukio la uhalifu.
#[derive(Debug, Clone)]
pub struct CrimeEvent {
    pub kind: String,
    pub ip: String,
    pub country: Option<String>,
}

/// Changanua matukio → mifumo.
pub fn analyze(events: &[CrimeEvent]) -> (Vec<(String, usize)>, Vec<String>) {
    let kinds: Vec<String> = events.iter().map(|e| e.kind.clone()).collect();
    let mut top: Vec<(String, usize)> = counts(&kinds).into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    let mut countries: Vec<String> = events.iter().filter_map(|e| e.country.clone()).collect();
    countries.sort();
    countries.dedup();
    (top, countries)
}

pub struct M326GlobalCrimeTracking;

impl TsModule for M326GlobalCrimeTracking {
    fn id(&self) -> u16 {
        326
    }
    fn name(&self) -> &'static str {
        "Global Crime Tracking"
    }
    fn group(&self) -> &'static str {
        "humanitarian"
    }
    fn about(&self) -> &'static str {
        "Hufuatilia uhalifu wa kimataifa (kwa IP/eneo), hutambua mifumo inayovuka \
         mipaka, na kuandaa ripoti ya INTERPOL (notice types)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(326, self.name());
        let events: Vec<CrimeEvent> = p.get("events").and_then(|v| v.as_array()).map(|a| {
            a.iter().map(|x| {
                let ip = x.get("ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
                CrimeEvent {
                    kind: x.get("kind").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                    country: ip_country(&ip).map(|c| c.to_string()),
                    ip,
                }
            }).collect()
        }).unwrap_or_else(|| vec![
            CrimeEvent { kind: "ransomware".into(), ip: "80.2.1.1".into(), country: ip_country("80.2.1.1").map(|s| s.to_string()) },
            CrimeEvent { kind: "ransomware".into(), ip: "41.59.1.1".into(), country: ip_country("41.59.1.1").map(|s| s.to_string()) },
        ]);
        let (top, countries) = analyze(&events);
        let notice = interpol_notice("RED").unwrap();
        for (k, n) in &top {
            r.finding("pattern", format!("{k}: matukio {n}"), (n * 20).min(90) as u8);
        }
        r.findings.push(json!({ "kind": "interpol", "severity": 50,
            "detail": { "share_with": "INTERPOL", "notice": notice, "countries": countries, "cross_border": countries.len() > 1 } }));
        r.summary = format!("Global Crime Tracking: matukio {}, nchi {}, INTERPOL {} .", events.len(), countries.len(), notice);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_mifumo() {
        let ev = vec![
            CrimeEvent { kind: "fraud".into(), ip: "80.2.1.1".into(), country: ip_country("80.2.1.1").map(|s| s.to_string()) },
            CrimeEvent { kind: "fraud".into(), ip: "41.59.1.1".into(), country: ip_country("41.59.1.1").map(|s| s.to_string()) },
        ];
        let (top, countries) = analyze(&ev);
        assert_eq!(top[0].0, "fraud");
        assert_eq!(top[0].1, 2);
        assert!(countries.len() >= 1);
    }
}
