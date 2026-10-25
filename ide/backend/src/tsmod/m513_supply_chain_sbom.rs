//! m513_supply_chain_sbom.rs — MODULE 513: Supply Chain Security (SBOM)
//!
//! Spec: "Software Bill of Materials, binary authorization."
//! Twist: Kila module ina SBOM halisi.
//!
//! Injini HALISI: SBOM Merkle root kutoka component hashes na ulinganisho wa baseline.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ext::{component_hash, merkle_root};

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(513, "Supply Chain Security (SBOM)");
    let comps: Vec<(String, String)> = params.get("components").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| {
            Some((c.get("name")?.as_str()?.to_string(), c.get("version")?.as_str()?.to_string()))
        }).collect())
        .unwrap_or_else(|| vec![("fundi-deploy".into(), "1.0".into())]);
    let leaves: Vec<String> = comps.iter().map(|(n, v)| component_hash(n, v)).collect();
    let root = merkle_root(&leaves);
    let baseline = params.get("baseline_root").and_then(|v| v.as_str()).unwrap_or("");
    r.finding("sbom", format!("SBOM: components {}, Merkle root {}", comps.len(), &root[..16]), 0);
    if !baseline.is_empty() && baseline != root {
        r.finding("tampered", "SBOM imebadilika kutoka baseline — uwezekano wa supply chain attack", 85);
    }
    r.summary = format!("Supply Chain SBOM: {} components, root {}...", comps.len(), &root[..12]);
    Ok(r)
}

pub struct M513SupplyChainSbom;

#[async_trait::async_trait]
impl TsModule for M513SupplyChainSbom {
    fn id(&self) -> u16 { 513 }
    fn name(&self) -> &'static str { "Supply Chain Security (SBOM)" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "SBOM: hutengeneza Merkle root ya components (jina@toleo), hulinganisha na baseline na kugundua mabadiliko (supply chain attack)." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
