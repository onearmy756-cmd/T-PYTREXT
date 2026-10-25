//! m482_blockchain_explorer.rs — MODULE 482: BLOCKCHAIN EXPLORER
//!
//! Spec: "Kuchunguza blockchain. Twist: Kama Etherscan."
//!
//! Injini halisi: huchunguza blockchain kama Etherscan — kutafuta kitalu kwa
//! index/hash, muamala kwa hash, na kuonyesha historia ya anwani.

use super::common_blockchain::{block_json, Chain, Tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Historia ya anwani (kama Etherscan).
pub fn address_history(chain: &Chain, addr: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for b in &chain.blocks {
        for t in &b.txs {
            if t.from == addr || t.to == addr {
                out.push(json!({ "block": b.index, "from": t.from, "to": t.to, "amount": t.amount, "hash": t.hash(), "direction": if t.from == addr { "out" } else { "in" } }));
            }
        }
    }
    out
}

pub struct M482BlockchainExplorer;

impl TsModule for M482BlockchainExplorer {
    fn id(&self) -> u16 {
        482
    }
    fn name(&self) -> &'static str {
        "Blockchain Explorer"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Huchunguza blockchain kama Etherscan: kitalu kwa index/hash, muamala, na \
         historia ya anwani."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(482, self.name());
        let addr = p.get("address").and_then(|v| v.as_str()).unwrap_or("wallet-a");
        let mut c = Chain::new("genesis");
        c.add_block("miner", vec![
            Tx { from: "wallet-a".into(), to: "wallet-b".into(), amount: 250, data: "malipo".into(), nonce: 0 },
            Tx { from: "wallet-c".into(), to: "wallet-a".into(), amount: 75, data: "marejesho".into(), nonce: 1 },
        ], 1_700_000_010).unwrap();
        let hist = address_history(&c, addr);
        let txh = c.tip().txs[0].hash();
        let found = c.find_tx(&txh);
        r.findings.push(json!({ "kind": "block", "severity": 1, "detail": block_json(c.tip()) }));
        r.findings.push(json!({ "kind": "address", "severity": 1,
            "detail": { "address": addr, "tx_count": hist.len(), "history": hist } }));
        r.findings.push(json!({ "kind": "tx_lookup", "severity": 1,
            "detail": { "tx_hash": txh, "found_at": found.map(|(b, j)| json!({"block": b, "index": j})), "etherscan_style": true } }));
        r.summary = format!("Blockchain Explorer: anwani {addr} ina miamala {}, muamala umepatikana.", hist.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn historia_ya_anwani() {
        let mut c = Chain::new("g");
        c.add_block("m", vec![Tx { from: "a".into(), to: "b".into(), amount: 1, data: "".into(), nonce: 0 }], 1).unwrap();
        let h = address_history(&c, "a");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0]["direction"], "out");
    }
}
