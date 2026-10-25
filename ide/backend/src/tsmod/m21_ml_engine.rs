//! m21_ml_engine.rs — MODULE 21: Machine Learning Module
//!
//! Spec: "Inajifunza tabia na kugundua vitisho kwa AI."
//! Twist: Inachukua digital fingerprint ya mtumiaji (vipimo 7).
//!
//! Injini HALISI: umbali wa fingerprint (L1 relative), uamuzi wa hatua (allow/step_up/deny), na Welford stats.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ai::{fingerprint_distance, fingerprint_decision, Welford};

pub const FEATURES: &[&str] = &["typing", "voice", "gait", "device", "location", "time", "social"];

fn vec_of(v: Option<&Value>, n: usize) -> Vec<f64> {
    v.and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|y| y.as_f64()).collect::<Vec<f64>>())
        .filter(|x| x.len() == n).unwrap_or_else(|| vec![0.0; n])
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(21, "Machine Learning Module");
    let profile = vec_of(params.get("profile"), FEATURES.len());
    let current = vec_of(params.get("current"), FEATURES.len());
    let d = fingerprint_distance(&profile, &current);
    let decision = fingerprint_decision(d);
    r.finding("fingerprint", format!("Umbali wa fingerprint: {:.0}% (vipimo 7)", d * 100.0), if decision == "deny" { 80 } else if decision == "step_up" { 45 } else { 0 });
    r.finding("decision", format!("Kuingia: {decision}"), if decision == "deny" { 80 } else { 0 });
    // Ufuatiliaji wa muda (Welford) kwa vipimo vya umbali wa nyuma.
    let mut w = Welford::default();
    for i in 0..FEATURES.len() { w.push(current[i]); }
    r.finding("stats", format!("Vipimo {}: wastani {:.2}, std {:.2}", w.n, w.mean, w.std()), 0);
    r.summary = format!("ML Engine: umbali {:.0}% -> {decision}.", d * 100.0);
    Ok(r)
}

pub struct M21MlEngine;

#[async_trait::async_trait]
impl TsModule for M21MlEngine {
    fn id(&self) -> u16 { 21 }
    fn name(&self) -> &'static str { "Machine Learning Module" }
    fn group(&self) -> &'static str { "ai" }
    fn about(&self) -> &'static str { "Digital fingerprint ya vipimo 7 (typing, voice, gait, device, location, time, social); hulinganisha na profile na kuamua allow/step-up/deny." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
