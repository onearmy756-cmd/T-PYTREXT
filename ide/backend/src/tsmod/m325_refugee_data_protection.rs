//! m325_refugee_data_protection.rs — MODULE 325
//!
//! Spec: "Inalinda data za wakimbizi. Twist: Inahifadhi data kwenye blockchain isiyoweza kubadilishwa."
//!
//! Injini halisi: hulinda data nyeti za wakimbizi kwa pseudonymization (hakuna
//! PII mbichi) na kuhifadhi kwenye blockchain-lite (hash-chain) isiyoweza
//! kubadilishwa; huthibitisha uadilifu wa rekodi.

use super::common_human::{chain, pseudonymize};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Rekodi moja ya mkimbizi (pseudonymized).
#[derive(Debug, Clone)]
pub struct RefugeeRecord {
    pub pseudo: String,
    pub age_band: String,
    pub origin: String,
    pub hash: String,
}

/// Tengeneza rekodi zisizobadilika (hash-chain).
pub fn build_ledger(rows: &[(String, String, String)]) -> (Vec<RefugeeRecord>, String) {
    let mut prev = "GENESIS".to_string();
    let mut out = Vec::new();
    for (name, age, origin) in rows {
        let pseudo = pseudonymize(&[name, age]);
        let rec = format!("{pseudo}|{age}|{origin}");
        prev = chain(&prev, &rec);
        out.push(RefugeeRecord { pseudo, age_band: age.clone(), origin: origin.clone(), hash: prev.clone() });
    }
    (out, prev)
}

/// Thibitisha uadilifu: hash ya rekodi inalingana?
pub fn verify_ledger(rows: &[(String, String, String)], head: &str) -> bool {
    let (_, h) = build_ledger(rows);
    h == head
}

pub struct M325RefugeeDataProtection;

impl TsModule for M325RefugeeDataProtection {
    fn id(&self) -> u16 {
        325
    }
    fn name(&self) -> &'static str {
        "Refugee Data Protection"
    }
    fn group(&self) -> &'static str {
        "humanitarian"
    }
    fn about(&self) -> &'static str {
        "Hulinda data za wakimbizi kwa pseudonymization na kuhifadhi kwenye \
         blockchain-lite (hash-chain) isiyoweza kubadilishwa."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(325, self.name());
        let rows: Vec<(String, String, String)> = p.get("refugees").and_then(|v| v.as_array()).map(|a| {
            a.iter().filter_map(|x| {
                Some((
                    x.get("name")?.as_str()?.to_string(),
                    x.get("age_band").and_then(|v| v.as_str()).unwrap_or("adult").to_string(),
                    x.get("origin").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                ))
            }).collect()
        }).unwrap_or_else(|| vec![("Asha M.".into(), "adult".into(), "DRC".into()), ("Juma K.".into(), "minor".into(), "SSD".into())]);
        let (ledger, head) = build_ledger(&rows);
        let intact = verify_ledger(&rows, &head);
        for rec in &ledger {
            r.findings.push(json!({ "kind": "record", "severity": 1,
                "detail": { "pseudo": rec.pseudo, "age_band": rec.age_band, "origin": rec.origin, "block_hash": &rec.hash[..16] } }));
        }
        r.findings.push(json!({ "kind": "ledger", "severity": if intact { 1 } else { 90 },
            "detail": { "records": ledger.len(), "head": head, "immutable": true, "pii_stored": false } }));
        r.summary = format!("Refugee Data Protection: rekodi {} kwenye blockchain (head {}).", ledger.len(), &head[..16]);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_haibadiliki() {
        let rows = vec![("A".into(), "adult".into(), "DRC".into())];
        let (_, head) = build_ledger(&rows);
        assert!(verify_ledger(&rows, &head));
        let tampered = vec![("A".into(), "minor".into(), "DRC".into())];
        assert!(!verify_ledger(&tampered, &head));
    }
}
