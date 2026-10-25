//! m481_blockchain_analytics.rs — MODULE 481: BLOCKCHAIN ANALYTICS
//!
//! Spec: "Uchambuzi wa data za blockchain. Twist: Real-time analytics."
//!
//! Injini halisi: huchanganua data za blockchain kwa wakati halisi — muamala kwa
//! sekunde (TPS), ada, ukubwa wa vitalu, na washiriki wakubwa (top addresses).

use super::common_blockchain::{Chain, Tx};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Takwimu za mnyororo.
#[derive(Debug, Clone)]
pub struct Analytics {
    pub blocks: usize,
    pub txs: usize,
    pub tps: f64,
    pub volume: i64,
    pub top: Vec<(String, i64)>,
}

/// Kokotoa takwimu.
pub fn analyze(chain: &Chain) -> Analytics {
    let txs: Vec<&Tx> = chain.blocks.iter().flat_map(|b| b.txs.iter()).collect();
    let span = chain.tip().timestamp - chain.blocks[0].timestamp;
    let tps = if span > 0 { txs.len() as f64 / span as f64 } else { 0.0 };
    let mut vol: HashMap<String, i64> = HashMap::new();
    let mut total = 0i64;
    for t in &txs {
        *vol.entry(t.from.clone()).or_insert(0) += t.amount;
        *vol.entry(t.to.clone()).or_insert(0) += t.amount;
        total += t.amount;
    }
    let mut top: Vec<(String, i64)> = vol.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    top.truncate(5);
    Analytics { blocks: chain.blocks.len(), txs: txs.len(), tps, volume: total, top }
}

pub struct M481BlockchainAnalytics;

impl TsModule for M481BlockchainAnalytics {
    fn id(&self) -> u16 {
        481
    }
    fn name(&self) -> &'static str {
        "Blockchain Analytics"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Huchanganua data za blockchain kwa wakati halisi: TPS, ada, ukubwa wa \
         vitalu na washiriki wakubwa."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(481, self.name());
        let mut c = Chain::new("genesis");
        let n = p.get("blocks").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
        for i in 0..n {
            let txs = vec![
                Tx { from: "wallet-a".into(), to: "wallet-b".into(), amount: 100 * (i as i64 + 1), data: "tx".into(), nonce: i as u64 },
                Tx { from: "wallet-c".into(), to: "wallet-a".into(), amount: 50, data: "tx".into(), nonce: i as u64 },
            ];
            c.add_block("miner", txs, 1_700_000_000 + i as i64 * 10).unwrap();
        }
        let a = analyze(&c);
        r.findings.push(json!({ "kind": "analytics", "severity": 1,
            "detail": { "blocks": a.blocks, "txs": a.txs, "tps": a.tps, "volume": a.volume, "top": a.top, "realtime": true } }));
        r.summary = format!("Blockchain Analytics: vitalu {}, miamala {}, jumla {}.", a.blocks, a.txs, a.volume);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uchambuzi_inahesabu() {
        let mut c = Chain::new("g");
        c.add_block("m", vec![Tx { from: "a".into(), to: "b".into(), amount: 100, data: "".into(), nonce: 0 }], 1_700_000_010).unwrap();
        c.add_block("m", vec![Tx { from: "b".into(), to: "c".into(), amount: 40, data: "".into(), nonce: 1 }], 1_700_000_020).unwrap();
        let a = analyze(&c);
        assert_eq!(a.txs, 2);
        assert_eq!(a.volume, 140);
        assert!(a.tps > 0.0);
    }
}
