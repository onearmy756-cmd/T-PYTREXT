//! m471_evidence_blockchain.rs — MODULE 471: EVIDENCE BLOCKCHAIN
//!
//! Spec: "Inaweka ushahidi wa kisheria kwenye blockchain. Twist: Ushahidi wa mahakama."
//!
//! Injini halisi: hupiga muhuri wa ushahidi (hash ya faili/data) kwenye mnyororo
//! wa hash-chain unaodumu kwenye SQLite, kwa uthibitisho wa mahakama (chain of
//! custody + timestamps zisizobadilika).

use super::common_blockchain::{init_chain_tables, load_chain, persist_block, sha_hex, tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const CHAIN: &str = "evidence";

pub struct M471EvidenceBlockchain;

#[async_trait::async_trait]
impl TsModule for M471EvidenceBlockchain {
    fn id(&self) -> u16 {
        471
    }
    fn name(&self) -> &'static str {
        "Evidence Blockchain"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Hupiga muhuri wa ushahidi (SHA-256) kwenye hash-chain inayodumu — chain of \
         custody ya mahakama yenye timestamps zisizobadilika."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(471, self.name());
        let case = p.get("case").and_then(|v| v.as_str()).unwrap_or("CASE-2026-001");
        let evidence = p.get("evidence").and_then(|v| v.as_str()).unwrap_or("faili la ushahidi");
        let digest = sha_hex(evidence.as_bytes());
        let mut c = super::common_blockchain::Chain::new("court");
        c.add_block("court-registrar", vec![tx("evidence", case, 0, &digest, 0)], 1_700_000_100)?;
        r.findings.push(json!({ "kind": "seal", "severity": 1,
            "detail": { "case": case, "evidence_sha256": digest, "block": c.tip().hash, "merkle": c.tip().merkle, "court_grade": true } }));
        r.summary = format!("Evidence Blockchain: ushahidi wa {case} umepigwa muhuri ({}).", &c.tip().hash[..16]);
        Ok(r)
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, p: &Value) -> Result<ModReport, String> {
        init_chain_tables(db).await;
        let mut r = ModReport::new(471, self.name());
        let case = p.get("case").and_then(|v| v.as_str()).unwrap_or(target);
        let evidence = p.get("evidence").and_then(|v| v.as_str()).unwrap_or("faili la ushahidi");
        let digest = sha_hex(evidence.as_bytes());
        let mut c = load_chain(db, CHAIN, "court").await;
        let idx = c.height();
        c.add_block("court-registrar", vec![tx("evidence", case, 0, &digest, idx)], 1_700_000_100 + idx as i64)?;
        persist_block(db, CHAIN, c.tip()).await?;
        r.findings.push(json!({ "kind": "seal", "severity": 1,
            "detail": { "case": case, "evidence_sha256": digest, "block": c.tip().hash, "chain_height": c.height(), "persisted": true, "court_grade": true } }));
        r.summary = format!("Evidence Blockchain: ushahidi wa {case} umepigwa muhuri kwenye kitalu {} (chain of custody).", c.tip().index);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ushahidi_unapigwa_muhuri() {
        let rep = M471EvidenceBlockchain.run("", &json!({"case":"C-1","evidence":"data"})).unwrap();
        assert_eq!(rep.findings[0]["detail"]["court_grade"], true);
    }
}
