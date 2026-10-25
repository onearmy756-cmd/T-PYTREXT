//! m10_international_fraud.rs — MODULE 10: International Call Fraud Detection
//!
//! Spec: "Inagundua ulaghai wa simu za kimataifa."
//! Twist: Inagundua namba za nje zinazotumika kwa ulaghai wa kimataifa.
//!
//! Injini HALISI: uchambuzi wa CDR za kimataifa: marudio, muda, na namba za premium za nje.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use super::common_calls::{parse_cdrs, wangiri_score};

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(10, "International Call Fraud Detection");
    let home_cc = params.get("home_cc").and_then(|v| v.as_str()).unwrap_or("255");
    let cdrs = parse_cdrs(params);
    let (wang, _clis, short) = wangiri_score(&cdrs);
    let intl: Vec<_> = cdrs.iter().filter(|c| {
        let d: String = c.cli.chars().filter(|x| x.is_ascii_digit()).collect();
        !d.starts_with(home_cc) && d.len() > 6
    }).collect();
    let mut score = wang;
    if intl.len() >= 3 { score = score.max(55); }
    if intl.len() >= 3 && short >= 3 { score = score.max(75); }
    r.finding("intl_calls", format!("{} simu za kimataifa; {short} fupi (wangiri)", intl.len()), score);
    r.finding("decision", format!("{}", if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" }), score);
    r.summary = format!("International Call Fraud: {} simu za nje, score {score}.", intl.len());
    Ok(r)
}

pub struct M10InternationalFraud;

#[async_trait::async_trait]
impl TsModule for M10InternationalFraud {
    fn id(&self) -> u16 { 10 }
    fn name(&self) -> &'static str { "International Call Fraud Detection" }
    fn group(&self) -> &'static str { "calls" }
    fn about(&self) -> &'static str { "Inagundua ulaghai wa simu za kimataifa: namba za nje, marudio ya simu fupi, na namba za premium." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
