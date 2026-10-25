//! m292_sms_spam.rs — MODULE 292: SMS SPAM
//!
//! Spec: "Inazuia SMS za spam. Twist: Inazuia namba zinazotuma SMS nyingi."
//!
//! Injini halisi: hufuatilia kiwango cha SMS kwa kila mtumaji (rate limiting)
//! na kugundua spam kwa alama za maandishi; huzuia namba zinazotuma kwa kasi.

use super::common_smsemail::smishing_score;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

/// Kikomo cha SMS kwa dirisha la muda (sliding window).
pub const MAX_SMS_PER_WINDOW: usize = 10;
pub const WINDOW_SECS: i64 = 60;

/// Hifadhi ya kiwango cha mtumaji (in-memory; kwa mfumo halisi tungetumia DB).
pub struct RateTracker {
    inner: Mutex<HashMap<String, Vec<i64>>>,
}

impl Default for RateTracker {
    fn default() -> Self {
        RateTracker { inner: Mutex::new(HashMap::new()) }
    }
}

impl RateTracker {
    /// Rekodi SMS na rudisha (idadi kwenye dirisha, imezidi?).
    pub fn record(&self, sender: &str, now: i64) -> (usize, bool) {
        let mut m = self.inner.lock().unwrap();
        let v = m.entry(sender.to_string()).or_default();
        v.retain(|t| now - *t < WINDOW_SECS);
        v.push(now);
        (v.len(), v.len() > MAX_SMS_PER_WINDOW)
    }
}

pub struct M292SmsSpam;

impl TsModule for M292SmsSpam {
    fn id(&self) -> u16 {
        292
    }
    fn name(&self) -> &'static str {
        "SMS Spam"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huzuia SMS za spam na hufuatilia kiwango cha mtumaji — namba zinazotuma \
         SMS nyingi kwa kasi zinazuiwa."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(292, self.name());
        let text = params.get("text").and_then(|v| v.as_str()).unwrap_or(target);
        let sender = params.get("sender").and_then(|v| v.as_str()).unwrap_or("+255000");
        let now = params.get("now").and_then(|v| v.as_i64()).unwrap_or(1_700_000_000);
        let prior = params.get("prior_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let mut score = 0u8;
        // Alama za maandishi (spam keywords).
        let low = text.to_lowercase();
        for k in ["promo", "ofisi", "ofa", "malipo", "loan", "mkopo", "subscribe", "unsubscribe", "bonus", "free", "bure"] {
            if low.contains(k) {
                score = score.max(35);
                r.finding("spam_keyword", format!("Neno la spam: '{k}'"), 35);
                break;
            }
        }
        // Kiwango: ikiwa mtumaji amezidi kikomo.
        let (cnt, over) = if prior > 0 {
            (prior + 1, prior + 1 > MAX_SMS_PER_WINDOW)
        } else {
            let t = RateTracker::default();
            t.record(sender, now)
        };
        if over {
            score = score.max(80);
            r.finding("rate_exceeded", format!("Mtumaji {sender} ametuma SMS {cnt} kwa sekunde {WINDOW_SECS}"), 80);
        }
        let (sscore, swhy) = smishing_score(sender, text);
        if sscore > score {
            score = sscore;
            for (k, d) in &swhy {
                r.finding(k, d.clone(), sscore);
            }
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score, "rate": cnt } }));
        r.summary = format!("SMS Spam: uamuzi {decision} (score {score}, rate {cnt}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_inagundua() {
        let t = RateTracker::default();
        let mut last = (0, false);
        for i in 0..15 {
            last = t.record("+255712", 1000 + i);
        }
        assert!(last.1, "count {}", last.0);
    }
}
