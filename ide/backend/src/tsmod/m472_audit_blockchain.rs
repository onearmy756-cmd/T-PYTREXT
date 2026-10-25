//! m472_audit_blockchain.rs — MODULE 472: AUDIT BLOCKCHAIN
//!
//! Spec: "Inarekodi shughuli za ukaguzi. Twist: Immutable audit logs."
//!
//! Injini halisi: hurekodi kila shughuli ya ukaguzi kwenye mnyororo wa hash-chain
//! unaodumu (immutable); huthibitisha kwamba hakuna rekodi iliyobadilishwa.

use super::common_blockchain::{init_chain_tables, load_chain, persist_block, tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const CHAIN: &str = "audit";

pub struct M472AuditBlockchain;

#[async_trait::async_trait]
impl TsModule for M472AuditBlockchain {
    fn id(&self) -> u16 {
        472
    }
    fn name(&self) -> &'static str {
        "Audit Blockchain"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Hurekodi shughuli za ukaguzi kwenye hash-chain inayodumu (immutable) na \
         kuthibitisha kwamba hakuna rekodi iliyobadilishwa."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(472, self.name());
        let actor = p.get("actor").and_then(|v| v.as_str()).unwrap_or("admin");
        let action = p.get("action").and_then(|v| v.as_str()).unwrap_or("login");
        let mut c = super::common_blockchain::Chain::new("audit");
        c.add_block(actor, vec![tx(actor, "audit-log", 0, action, 0)], 1_700_000_200)?;
        let valid = c.validate();
        r.findings.push(json!({ "kind": "audit_entry", "severity": 1,
            "detail": { "actor": actor, "action": action, "block": c.tip().hash, "immutable": true, "chain_valid": valid } }));
        r.summary = format!("Audit Blockchain: shughuli ya {actor} imerekodiwa (immutable).");
        Ok(r)
    }

    async fn run_db(&self, db: &SqlitePool, target: &str, p: &Value) -> Result<ModReport, String> {
        init_chain_tables(db).await;
        let mut r = ModReport::new(472, self.name());
        let actor = p.get("actor").and_then(|v| v.as_str()).unwrap_or(target);
        let action = p.get("action").and_then(|v| v.as_str()).unwrap_or("login");
        let mut c = load_chain(db, CHAIN, "audit").await;
        let idx = c.height();
        c.add_block(actor, vec![tx(actor, "audit-log", 0, action, idx)], 1_700_000_200 + idx as i64)?;
        persist_block(db, CHAIN, c.tip()).await?;
        let valid = c.validate();
        r.findings.push(json!({ "kind": "audit_entry", "severity": if valid { 1 } else { 90 },
            "detail": { "actor": actor, "action": action, "block": c.tip().hash, "chain_height": c.height(), "immutable": true, "chain_valid": valid } }));
        r.summary = format!("Audit Blockchain: shughuli ya {actor} kwenye kitalu {} (chain {}).", c.tip().index, if valid { "sahihi" } else { "umebadilishwa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_inarekodi() {
        let rep = M472AuditBlockchain.run("", &json!({"actor":"admin","action":"deploy"})).unwrap();
        assert!(rep.findings[0]["detail"]["chain_valid"].as_bool().unwrap());
    }
}
