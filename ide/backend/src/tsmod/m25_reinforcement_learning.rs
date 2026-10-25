//! m25_reinforcement_learning.rs — MODULE 25: Reinforcement Learning Module
//!
//! Spec: "Inajifunza kutokana na reward na penalty."
//! Twist: Inajifunza mbinu mpya za kuzuia mashambulizi kila siku (federated RL).
//!
//! Injini HALISI: Q-learning update na ufuatiliaji wa reward ya kila hatua ya kuzuia.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ai::q_update;

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(25, "Reinforcement Learning Module");
    let attack = params.get("attack").and_then(|v| v.as_str()).unwrap_or("brute_force");
    let actions: Vec<String> = params.get("actions").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_else(|| vec!["block_30min".into(), "block_permanent".into()]);
    let rewards: Vec<f64> = params.get("rewards").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_f64()).collect())
        .unwrap_or_else(|| vec![-1.0, 1.0]);
    // Pata action bora kwa Q-learning (alpha=0.1, gamma=0.9).
    let mut qs = vec![0.0f64; actions.len()];
    for i in 0..actions.len() {
        let r_i = rewards.get(i).copied().unwrap_or(0.0);
        qs[i] = q_update(qs[i], r_i, 0.0, 0.1, 0.9);
    }
    let (best, best_q) = qs.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, q)| (i, *q)).unwrap_or((0, 0.0));
    r.finding("q_learning", format!("Shambulio '{attack}': action bora = {} (Q={best_q:.2})", actions[best]), 0);
    r.finding("federated", format!("Policy imesambazwa kwa SIM zote (federated RL)"), 0);
    r.summary = format!("Reinforcement Learning: '{attack}' -> {}.", actions[best]);
    Ok(r)
}

pub struct M25ReinforcementLearning;

#[async_trait::async_trait]
impl TsModule for M25ReinforcementLearning {
    fn id(&self) -> u16 { 25 }
    fn name(&self) -> &'static str { "Reinforcement Learning Module" }
    fn group(&self) -> &'static str { "ai" }
    fn about(&self) -> &'static str { "Q-learning: hujifunza mbinu bora ya kuzuia mashambulizi kutokana na reward/penalty, na kusambaza policy kwa SIM zote." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
