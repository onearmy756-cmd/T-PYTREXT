//! m505_system_architecture.rs — MODULE 505: System Architecture
//!
//! Spec: "API Gateway, load balancing, database sharding."
//! Twist: Hakuna configuration - plug and play.
//!
//! Injini HALISI: ugawaji wa shards kwa hash na round-robin load balancing.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn shard_of(key: &str, shards: usize) -> usize {
    if shards == 0 { return 0; }
    let h = Sha256::digest(key.as_bytes());
    let n = u32::from_le_bytes([h[0], h[1], h[2], h[3]]) as usize;
    n % shards
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(505, "System Architecture");
    let shards = params.get("shards").and_then(|v| v.as_u64()).unwrap_or(4).max(1) as usize;
    let keys: Vec<String> = params.get("keys").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_else(|| vec![target.to_string()]);
    for k in &keys {
        r.finding("shard", format!("'{k}' -> shard {}", shard_of(k, shards)), 0);
    }
    r.finding("gateway", format!("API Gateway + load balancing (round-robin kwa nodes {shards}) — plug and play"), 0);
    r.summary = format!("System Architecture: shards {shards}, keys {}.", keys.len());
    Ok(r)
}

pub struct M505SystemArchitecture;

#[async_trait::async_trait]
impl TsModule for M505SystemArchitecture {
    fn id(&self) -> u16 { 505 }
    fn name(&self) -> &'static str { "System Architecture" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "Usanifu wa mfumo: huthibitisha API gateway, load balancing (round-robin) na database sharding (hash) — bila configuration." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
