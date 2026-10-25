//! m316_screen_lock_security.rs — MODULE 316: SCREEN LOCK SECURITY
//!
//! Spec: "Inahakikisha screen lock imewashwa. Twist: Inazuia kutumia simu bila screen lock."
//!
//! Injini halisi: hukagua usanidi wa screen lock (imewashwa, aina, muda wa
//! kufunga, idadi ya majaribio) na kuzuia matumizi ya kifaa kisicho na screen lock.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Usanidi wa screen lock.
#[derive(Debug, Clone)]
pub struct LockConfig {
    pub enabled: bool,
    pub method: String, // "none" | "pin" | "password" | "pattern" | "biometric"
    pub timeout_secs: i64,
    pub max_attempts: u32,
    pub biometric_fallback: bool,
}

/// Kagua usanidi.
pub fn evaluate(c: &LockConfig) -> (u8, bool, Vec<String>) {
    let mut issues = Vec::new();
    let mut score = 0u8;
    if !c.enabled {
        issues.push("Screen lock HAIJAWASHWA".into());
    } else {
        score += 40;
    }
    match c.method.as_str() {
        "password" => score += 30,
        "pin" => score += 25,
        "pattern" => score += 20,
        "biometric" => score += 15,
        _ => issues.push("Njia ya kufunga haitoshi (none)".into()),
    }
    if c.timeout_secs <= 30 {
        score += 15;
    } else if c.timeout_secs <= 300 {
        score += 8;
        issues.push(format!("Muda wa kufunga ni mrefu ({}s)", c.timeout_secs));
    } else {
        issues.push(format!("Muda wa kufunga ni mrefu mno ({}s)", c.timeout_secs));
    }
    if c.max_attempts <= 5 {
        score += 15;
    } else {
        issues.push(format!("Majaribio {} ni mengi", c.max_attempts));
    }
    if c.biometric_fallback {
        issues.push("Biometric inaruhusu kufungua bila PIN".into());
        score = score.saturating_sub(10);
    }
    let block = !c.enabled || score < 60;
    (score.min(100), block, issues)
}

pub struct M316ScreenLockSecurity;

impl TsModule for M316ScreenLockSecurity {
    fn id(&self) -> u16 {
        316
    }
    fn name(&self) -> &'static str {
        "Screen Lock Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hukagua screen lock (imewashwa, aina, muda, majaribio) na kuzuia matumizi \
         ya kifaa kisicho na screen lock."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(316, self.name());
        let c = LockConfig {
            enabled: p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            method: p.get("method").and_then(|v| v.as_str()).unwrap_or("pin").to_string(),
            timeout_secs: p.get("timeout_secs").and_then(|v| v.as_i64()).unwrap_or(30),
            max_attempts: p.get("max_attempts").and_then(|v| v.as_u64()).unwrap_or(5) as u32,
            biometric_fallback: p.get("biometric_fallback").and_then(|v| v.as_bool()).unwrap_or(false),
        };
        let (score, block, issues) = evaluate(&c);
        for i in &issues {
            r.finding("lock_issue", i.clone(), 45);
        }
        r.findings.push(json!({ "kind": "decision", "severity": if block { 75 } else { 1 },
            "detail": { "block_device": block, "score": score, "method": c.method, "enabled": c.enabled } }));
        r.summary = format!("Screen Lock Security: {} (score {score}).", if block { "kifaa kimezuiwa" } else { "sawa" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bila_lock_inazuia() {
        let c = LockConfig { enabled: false, method: "none".into(), timeout_secs: 600, max_attempts: 10, biometric_fallback: false };
        let (_, block, _) = evaluate(&c);
        assert!(block);
    }
}
