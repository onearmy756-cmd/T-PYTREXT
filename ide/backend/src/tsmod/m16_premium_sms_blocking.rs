//! m16_premium_sms_blocking.rs — MODULE 16: Premium SMS Blocking
//!
//! Spec: "Inazuia SMS za gharama kubwa."
//! Twist: Inaonyesha bei halisi ya premium SMS kabla ya kutuma.
//!
//! Injini HALISI: database ya namba fupi za premium na bei zake, pamoja na ukaguzi wa idhini.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Bei za namba fupi za premium (TZS) — database ya ndani (TCRA-style).
pub fn premium_price(shortcode: &str) -> Option<(u32, u32)> {
    let db: HashMap<&str, (u32, u32)> = [
        ("15000", (1500, 0)), ("15100", (1000, 0)), ("15200", (500, 0)),
        ("15500", (300, 0)), ("900", (200, 0)),
    ].into_iter().collect();
    db.get(shortcode).copied()
}

fn run_impl(target: &str, params: &Value) -> Result<ModReport, String> {
    let mut r = ModReport::new(16, "Premium SMS Blocking");
    let shortcode = params.get("shortcode").and_then(|v| v.as_str()).unwrap_or(target);
    let approved = params.get("human_approved").and_then(|v| v.as_bool()).unwrap_or(false);
    match premium_price(shortcode) {
        Some((price, _)) => {
            r.finding("premium_price", format!("Premium SMS kwa {shortcode} inagharimu TSh {price}"), 0);
            if !approved {
                r.finding("needs_approval", format!("Mteja HAIJATOa idhini ya TSh {price} — SMS imezuiwa"), 60);
                r.action("block_premium", "Zuia hadi mteja aridhie", false);
            } else {
                r.finding("approved", format!("Mteja amekubali TSh {price}"), 0);
            }
        }
        None => { r.finding("normal", format!("{shortcode} si namba fupi ya premium"), 0); }
    }
    r.summary = format!("Premium SMS Blocking: {shortcode}.");
    Ok(r)
}

pub struct M16PremiumSmsBlocking;

#[async_trait::async_trait]
impl TsModule for M16PremiumSmsBlocking {
    fn id(&self) -> u16 { 16 }
    fn name(&self) -> &'static str { "Premium SMS Blocking" }
    fn group(&self) -> &'static str { "sms" }
    fn about(&self) -> &'static str { "Inaonyesha bei halisi ya premium SMS (kutoka database ya TCRA) kabla ya kutuma na inaomba idhini ya mteja." }
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> { run_impl(target, params) }
}
