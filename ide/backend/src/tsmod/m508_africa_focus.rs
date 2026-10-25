//! m508_africa_focus.rs — MODULE 508: Africa Focus & International
//!
//! Spec: "Africa First policy."
//! Twist: Inatanguliza usalama wa Wafrika kwanza.
//!
//! Injini HALISI: orodha ya nchi za Afrika (Afrika Mashariki kwanza) na washirika wa kimataifa.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
const AFRICA: &[(&str, &str)] = &[
    ("TZ", "Tanzania"), ("KE", "Kenya"), ("UG", "Uganda"), ("RW", "Rwanda"),
    ("BI", "Burundi"), ("CD", "DRC"), ("ZA", "Afrika Kusini"), ("NG", "Nigeria"),
    ("GH", "Ghana"), ("ET", "Ethiopia"), ("ZM", "Zambia"), ("MW", "Malawi"),
];
const EAST_AFRICA: &[&str] = &["TZ", "KE", "UG", "RW", "BI", "CD"];

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(508, "Africa Focus & International");
    let cc = params.get("country").and_then(|v| v.as_str()).unwrap_or("TZ").to_uppercase();
    let name = AFRICA.iter().find(|(c, _)| *c == cc).map(|(_, n)| *n).unwrap_or("(nje ya Afrika)");
    let ea = EAST_AFRICA.contains(&cc.as_str());
    r.finding("africa_first", format!("Nchi {cc} ({name}): {}", if ea { "Afrika Mashariki — kipaumbele cha juu" } else { "Afrika/nje" }), if ea { 0 } else { 20 });
    r.finding("partners", "Washirika: TCRA, INTERPOL, EUROPOL, ISP za Afrika", 0);
    r.summary = format!("Africa Focus: {cc} ({name}).");
    Ok(r)
}

pub struct M508AfricaFocus;

#[async_trait::async_trait]
impl TsModule for M508AfricaFocus {
    fn id(&self) -> u16 { 508 }
    fn name(&self) -> &'static str { "Africa Focus & International" }
    fn group(&self) -> &'static str { "extended" }
    fn about(&self) -> &'static str { "Africa First: hutanguliza nchi za Afrika katika orodha ya ISP/washirika na kushirikiana na INTERPOL/TCRA." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
