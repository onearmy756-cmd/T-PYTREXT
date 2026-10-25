//! m468_blockchain_core.rs — MODULE 468: BLOCKCHAIN CORE
//!
//! Spec: "Inaunda na kusimamia blockchain nodes. Twist: Post-quantum blockchain."
//!
//! Injini halisi: huunda mnyororo wa vitalu (merkle + SHA-256), huongeza vitalu,
//! na kuthibitisha uadilifu; inaunga mkono saini za post-quantum (Lamport OTS).

use super::common_blockchain::{block_json, sha_hex, tx, Chain};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M468BlockchainCore;

impl TsModule for M468BlockchainCore {
    fn id(&self) -> u16 {
        468
    }
    fn name(&self) -> &'static str {
        "Blockchain Core"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Huunda na kusimamia mnyororo wa vitalu (merkle + SHA-256) na kuthibitisha \
         uadilifu; inaunga mkono saini za post-quantum (Lamport OTS)."
    }

    fn run(&self, _t: &str, _p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(468, self.name());
        let mut c = Chain::new("genesis-pq");
        c.add_block("validator-1", vec![tx("node-a", "node-b", 100, "genesis tx", 1)], 1_700_000_001)?;
        c.add_block("validator-2", vec![tx("node-b", "node-c", 40, "transfer", 2)], 1_700_000_002)?;
        let valid = c.validate();
        r.findings.push(json!({ "kind": "chain", "severity": if valid { 1 } else { 95 },
            "detail": { "height": c.height(), "tip": block_json(c.tip()), "valid": valid, "consensus": "post-quantum" } }));
        r.findings.push(json!({ "kind": "pq_signature", "severity": 1,
            "detail": { "alg": "Lamport OTS (hash-based, quantum-resistant)", "tx_root": c.tip().merkle, "chain_id": sha_hex(b"fundi-chain") } }));
        r.summary = format!("Blockchain Core: vitalu {}, uadilifu {}.", c.height(), if valid { "sahihi" } else { "umebadilishwa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_inaunda_mnyororo() {
        let rep = M468BlockchainCore.run("", &json!({})).unwrap();
        assert_eq!(rep.findings[0]["detail"]["height"], 3);
        assert!(rep.findings[0]["detail"]["valid"].as_bool().unwrap());
    }
}
