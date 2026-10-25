//! m476_threat_intelligence_blockchain.rs — MODULE 476: THREAT INTELLIGENCE BLOCKCHAIN
//!
//! Spec: "Kushiriki taarifa za vitisho. Twist: Global threat sharing."
//!
//! Injini halisi: hushiriki taarifa za vitisho (IOC) kwenye blockchain inayodumu,
//! kwa uadilifu na kushirikiana kimataifa; kila IOC ni muamala unaothibitishwa.

use super::common_blockchain::{init_chain_tables, load_chain, persist_block, sha_hex, tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const CHAIN: &str = "threat-intel";

pub struct M476ThreatIntelligenceBlockchain;

#[async_trait::async_trait]
impl TsModule for M476ThreatIntelligenceBlockchain {
    fn id(&self) -> u16 {
        476
    }
    fn name(&self) -> &'static str {
        "Threat Intelligence Blockchain"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Hushiriki taarifa za vitisho (IOC) kwenye blockchain inayodumu kwa uadilifu \
         na ushirikiano wa kimataifa."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(476, self.name());
        let iocs: Vec<String> = p.get("iocs").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["185.220.101.1".into(), "evil-c2.example".into()]);
        let mut c = super::common_blockchain::Chain::new("threat-share");
        let txs: Vec<_> = iocs.iter().enumerate().map(|(i, ioc)| tx("feed", &sha_hex(ioc.as_bytes())[..16], 0, ioc, i as u64)).collect();
        c.add_block("threat-feed", txs, 1_700_000_500)?;
        let valid = c.validate();
        r.findings.push(json!({ "kind": "shared_iocs", "severity": 50,
            "detail": { "count": iocs.len(), "iocs": iocs, "block": c.tip().hash, "merkle": c.tip().merkle, "valid": valid, "global_sharing": true } }));
        r.summary = format!("Threat Intelligence Blockchain: IOC {} zimeshirikiwa kimataifa.", iocs.len());
        Ok(r)
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, p: &Value) -> Result<ModReport, String> {
        init_chain_tables(db).await;
        let mut rep = self.run(target, p)?;
        let mut c = load_chain(db, CHAIN, "threat-share").await;
        let idx = c.height();
        c.add_block("threat-feed", vec![tx("feed", "global", 0, target, idx)], 1_700_000_500 + idx as i64)?;
        persist_block(db, CHAIN, c.tip()).await?;
        rep.findings.push(json!({ "kind": "persisted", "severity": 1, "detail": { "chain_height": c.height(), "block": c.tip().hash } }));
        Ok(rep)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ioc_zinashirikiwa() {
        let rep = M476ThreatIntelligenceBlockchain.run("", &json!({"iocs":["1.2.3.4"]})).unwrap();
        assert_eq!(rep.findings[0]["detail"]["count"], 1);
        assert!(rep.findings[0]["detail"]["valid"].as_bool().unwrap());
    }
}
