//! m28_behavioral_analysis.rs — MODULE 28: Behavioral Analysis Module
//!
//! Spec: "Inachambua tabia ya mtumiaji."
//! Twist: Inagundua hisia za mteja (hasira, hofu, huzuni) na kupendekeza hatua.
//!
//! Injini HALISI: sentiment ya Kiswahili na z-score ya tabia kupitia Welford streaming stats.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_ai::{sentiment_sw, Welford};

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(28, "Behavioral Analysis");
    let text = params.get("text").and_then(|v| v.as_str()).unwrap_or(target);
    let (s, label) = sentiment_sw(text);
    r.finding("sentiment", format!("Hisia: {label} ({s:+.2})"), if s < -0.2 { 50 } else { 0 });
    let history: Vec<f64> = params.get("history").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_f64()).collect()).unwrap_or_else(|| vec![1.0, 2.0, 3.0]);
    let current = params.get("current").and_then(|v| v.as_f64()).unwrap_or(3.0);
    let mut w = Welford::default();
    for x in &history { w.push(*x); }
    let z = w.zscore(current);
    r.finding("anomaly", format!("Tabia ya sasa z={z:.2} (wastani {:.2})", w.mean), if z.abs() > 3.0 { 60 } else { 0 });
    r.summary = format!("Behavioral Analysis: {label}, z={z:.2}.");
    Ok(r)
}

pub struct M28BehavioralAnalysis;

#[async_trait::async_trait]
impl TsModule for M28BehavioralAnalysis {
    fn id(&self) -> u16 { 28 }
    fn name(&self) -> &'static str { "Behavioral Analysis Module" }
    fn group(&self) -> &'static str { "ai" }
    fn about(&self) -> &'static str { "Uchambuzi wa tabia: hisia za Kiswahili (sentiment), ufuatiliaji wa mwenendo, na ugunduzi wa tabia isiyo ya kawaida (z-score)." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
