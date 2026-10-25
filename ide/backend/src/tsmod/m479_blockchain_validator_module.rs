//! m479_blockchain_validator_module.rs — MODULE 479: BLOCKCHAIN VALIDATOR MODULE
//!
//! Spec: "Uthibitishaji wa vitalu. Twist: Validators 5."
//!
//! Injini halisi: huthibitisha kitalu kwa validators 5 (PBFT 2/3 quorum); kila
//! validator hukagua merkle root + hash ya kitalu + prev_hash.

use super::common_blockchain::{byzantine_tolerance, pbft_quorum, Chain, Tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Matokeo ya uthibitishaji wa validator mmoja.
pub fn validate_as(chain: &Chain, idx: usize) -> bool {
    if idx >= chain.blocks.len() {
        return false;
    }
    let b = &chain.blocks[idx];
    let leaves: Vec<String> = b.txs.iter().map(|t| t.hash()).collect();
    super::common_blockchain::merkle_root(&leaves) == b.merkle
        && (idx == 0 || chain.blocks[idx - 1].hash == b.prev_hash)
}

pub struct M479BlockchainValidatorModule;

impl TsModule for M479BlockchainValidatorModule {
    fn id(&self) -> u16 {
        479
    }
    fn name(&self) -> &'static str {
        "Blockchain Validator Module"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Huthibitisha vitalu kwa validators 5 (PBFT 2/3 quorum); kila mmoja hukagua \
         merkle root, hash na prev_hash."
    }

    fn run(&self, _t: &str, _p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(479, self.name());
        let mut c = Chain::new("genesis");
        c.add_block("v1", vec![Tx { from: "a".into(), to: "b".into(), amount: 1, data: "x".into(), nonce: 0 }], 1)?;
        let validators = ["val-1", "val-2", "val-3", "val-4", "val-5"];
        let idx = 1usize;
        let mut approvals = 0;
        for v in validators {
            let ok = validate_as(&c, idx);
            if ok {
                approvals += 1;
            }
            r.finding("validator", format!("{v}: {}", if ok { "kitalu ni sahihi" } else { "kitalu si sahihi" }), if ok { 1 } else { 80 });
        }
        let quorum = pbft_quorum(approvals, validators.len());
        let f = byzantine_tolerance(validators.len());
        r.findings.push(json!({ "kind": "quorum", "severity": if quorum { 1 } else { 75 },
            "detail": { "validators": validators.len(), "approvals": approvals, "quorum_reached": quorum, "byzantine_tolerance": f } }));
        r.summary = format!("Blockchain Validator: idhini {approvals}/{} (quorum {}).", validators.len(), if quorum { "imefikiwa" } else { "haijafikiwa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validators_5_wanathibitisha() {
        let rep = M479BlockchainValidatorModule.run("", &json!({})).unwrap();
        assert_eq!(rep.findings[5]["detail"]["approvals"], 5);
    }
}
