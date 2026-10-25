//! m469_consensus_module.rs — MODULE 469: CONSENSUS MODULE
//!
//! Spec: "Inatekeleza consensus algorithms. Twist: PoS + PBFT."
//!
//! Injini halisi: huchagua proposer kwa Proof-of-Stake (uzito wa stake) na
//! kuthibitisha makubaliano kwa PBFT (2/3 quorum); hukokotoa uvumilivu wa
//! Byzantine (f = (n-1)/3).

use super::common_blockchain::{byzantine_tolerance, pbft_quorum, pos_select};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct M469ConsensusModule;

impl TsModule for M469ConsensusModule {
    fn id(&self) -> u16 {
        469
    }
    fn name(&self) -> &'static str {
        "Consensus Module"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Huchagua proposer kwa PoS (uzito wa stake) na kuthibitisha PBFT (2/3 \
         quorum) pamoja na uvumilivu wa Byzantine."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(469, self.name());
        let mut stakes: HashMap<String, u64> = HashMap::new();
        stakes.insert("validator-a".into(), 100);
        stakes.insert("validator-b".into(), 60);
        stakes.insert("validator-c".into(), 40);
        stakes.insert("validator-d".into(), 25);
        if let Some(obj) = p.get("stakes").and_then(|v| v.as_object()) {
            for (k, v) in obj {
                if let Some(n) = v.as_u64() {
                    stakes.insert(k.clone(), n);
                }
            }
        }
        let seed = p.get("seed").and_then(|v| v.as_u64()).unwrap_or(137);
        let proposer = pos_select(&stakes, seed).unwrap_or_default();
        let total = stakes.len();
        let votes = p.get("votes").and_then(|v| v.as_u64()).unwrap_or((total as u64 * 3 + 3) / 4);
        let quorum = pbft_quorum(votes as usize, total);
        let f = byzantine_tolerance(total);
        r.findings.push(json!({ "kind": "pos", "severity": 1,
            "detail": { "proposer": proposer, "seed": seed, "total_stake": stakes.values().sum::<u64>(), "validators": total } }));
        r.findings.push(json!({ "kind": "pbft", "severity": if quorum { 1 } else { 60 },
            "detail": { "votes": votes, "total": total, "quorum_reached": quorum, "byzantine_tolerance": f } }));
        r.summary = format!("Consensus: PoS proposer {proposer}, PBFT quorum {} (f={f}).", if quorum { "imefikiwa" } else { "haijafikiwa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pos_na_pbft_inafanya_kazi() {
        let rep = M469ConsensusModule.run("", &json!({"seed": 137, "votes": 3})).unwrap();
        assert!(!rep.findings[0]["detail"]["proposer"].as_str().unwrap().is_empty());
        assert!(rep.findings[1]["detail"]["quorum_reached"].as_bool().unwrap());
    }
}
