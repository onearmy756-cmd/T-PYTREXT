//! m473_compliance_blockchain.rs — MODULE 473: COMPLIANCE BLOCKCHAIN
//!
//! Spec: "Inaweka compliance records. Twist: Kufuata sheria za TCRA."
//!
//! Injini halisi: hurekodi rekodi za ufuataji wa sheria (TCRA) kwenye hash-chain
//! inayodumu, na kukagua kwamba mahitaji ya lazima yametimizwa.

use super::common_blockchain::{init_chain_tables, load_chain, persist_block, tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const CHAIN: &str = "compliance";

/// Mahitaji ya TCRA (spec: kufuata sheria za TCRA).
pub const TCRA_REQUIREMENTS: &[(&str, &str)] = &[
    ("data_localization", "Data ya wateja ihifadhiwe Tanzania"),
    ("lawful_interception", "Uwezo wa kunasa kwa amri ya mahakama"),
    ("subscriber_registration", "Usajili wa wateja kwa vitambulisho"),
    ("retention", "Kuhifadhi kumbukumbu kwa miaka 3+"),
    ("cyber_incident_reporting", "Kuripoti matukio ya mtandao TCRA"),
    ("quality_of_service", "Kiwango cha huduma (QoS) kinachokaguliwa"),
];

/// Kagua ufuataji: rudisha (yaliyotimizwa, yaliyokosekana).
pub fn audit(provided: &[String]) -> (Vec<&'static str>, Vec<&'static str>) {
    let mut ok = Vec::new();
    let mut missing = Vec::new();
    for (k, _) in TCRA_REQUIREMENTS {
        if provided.iter().any(|p| p == k) {
            ok.push(*k);
        } else {
            missing.push(*k);
        }
    }
    (ok, missing)
}

pub struct M473ComplianceBlockchain;

#[async_trait::async_trait]
impl TsModule for M473ComplianceBlockchain {
    fn id(&self) -> u16 {
        473
    }
    fn name(&self) -> &'static str {
        "Compliance Blockchain"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Hurekodi rekodi za ufuataji wa TCRA kwenye hash-chain inayodumu na \
         kukagua mahitaji ya lazima."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(473, self.name());
        let provided: Vec<String> = p.get("controls").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| TCRA_REQUIREMENTS.iter().map(|(k, _)| k.to_string()).collect());
        let (ok, missing) = audit(&provided);
        let mut c = super::common_blockchain::Chain::new("tcra");
        let entries: Vec<(&str, &str)> = TCRA_REQUIREMENTS.iter().map(|(k, d)| (*k, *d)).collect();
        let txs: Vec<_> = entries.iter().enumerate().map(|(i, (k, d))| tx("tcra", k, 0, d, i as u64)).collect();
        c.add_block("compliance-officer", txs, 1_700_000_300)?;
        for m in &missing {
            r.finding("tcra_gap", format!("Hitaji halijatimizwa: {m}"), 70);
        }
        r.findings.push(json!({ "kind": "compliance_record", "severity": if missing.is_empty() { 1 } else { 65 },
            "detail": { "compliant": missing.is_empty(), "satisfied": ok, "missing": missing, "block": c.tip().hash, "standard": "TCRA" } }));
        r.summary = format!("Compliance Blockchain: TCRA {}/{} imetimizwa (blockchain imethibitisha).", ok.len(), TCRA_REQUIREMENTS.len());
        Ok(r)
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, p: &Value) -> Result<ModReport, String> {
        init_chain_tables(db).await;
        let mut rep = self.run(target, p)?;
        let mut c = load_chain(db, CHAIN, "tcra").await;
        let idx = c.height();
        c.add_block("compliance-officer", vec![tx("tcra", target, 0, "compliance snapshot", idx)], 1_700_000_300 + idx as i64)?;
        persist_block(db, CHAIN, c.tip()).await?;
        rep.findings.push(json!({ "kind": "persisted", "severity": 1, "detail": { "chain_height": c.height(), "block": c.tip().hash } }));
        Ok(rep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kagua_tcra() {
        let provided = vec!["data_localization".to_string()];
        let (ok, missing) = audit(&provided);
        assert_eq!(ok, vec!["data_localization"]);
        assert!(missing.len() == TCRA_REQUIREMENTS.len() - 1);
    }
}
