//! m296_email_spam.rs — MODULE 296: EMAIL SPAM
//!
//! Spec: "Inazuia barua taka. Twist: Inazuia anwani za spam."
//!
//! Injini halisi: hutoa alama ya spam (Bayesian-style keyword + heuristics) na
//! huzuia anwani (blocklist) zinazojulikana kwa spam.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashSet;

/// Maneno ya spam (Kiswahili + Kiingereza).
const SPAM_WORDS: &[&str] = &["winner", "umeshinda", "lottery", "bahati nasibu", "viagra", "casino",
    "loan", "mkopo wa haraka", "click here", "bofya hapa", "100% free", "bure kabisa", "million",
    "milioni", "inheritance", "urithi", "bitcoin", "crypto", "forex", "double your", "act now",
    "limited offer", "ofa ya kikomo", "unsubscribe", "bulk mail", "no-reply-", "prize", "zawadi"];

/// Anwani zinazojulikana kwa spam (sehemu).
pub fn is_known_spam_addr(addr: &str) -> bool {
    let a = addr.to_lowercase();
    a.contains("spam") || a.contains("bulk") || a.ends_with("@mail.ru") || a.ends_with("@tempmail") || a.contains("temp-mail")
}

/// Alama ya spam ya barua pepe (0-100).
pub fn email_spam_score(raw: &str, blocklist: &HashSet<String>) -> (u8, Vec<(&'static str, String)>) {
    let h = super::common_smsemail::parse_email(raw);
    let low = raw.to_lowercase();
    let mut s = 0u8;
    let mut why = Vec::new();
    let mut hits = 0;
    for w in SPAM_WORDS {
        if low.contains(w) {
            hits += 1;
        }
    }
    if hits > 0 {
        s = s.max((hits as u8 * 18).min(75));
        why.push(("spam_words", format!("Maneno {hits} ya spam yamegundulika")));
    }
    if is_known_spam_addr(&h.from_addr) || blocklist.contains(&h.from_addr) {
        s = s.max(80);
        why.push(("blocked_addr", format!("Anwani imezuiwa: {}", h.from_addr)));
    }
    if h.subject.chars().filter(|c| c.is_uppercase()).count() > h.subject.len() / 2 && h.subject.len() > 5 {
        s = s.max(45);
        why.push(("all_caps_subject", "Subject ina herufi kubwa nyingi".into()));
    }
    if h.received_count > 8 {
        s = s.max(40);
        why.push(("many_relays", format!("Received relays {} — uwezekano wa bulk", h.received_count)));
    }
    (s, why)
}

pub struct M296EmailSpam;

impl TsModule for M296EmailSpam {
    fn id(&self) -> u16 {
        296
    }
    fn name(&self) -> &'static str {
        "Email Spam"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Hutoa alama ya spam (maneno + heuristics) na huzuia anwani za spam \
         zilizojulikana (blocklist)."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(296, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let blocklist: HashSet<String> = params.get("blocklist").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(|s| s.to_lowercase())).collect())
            .unwrap_or_default();
        let (score, why) = email_spam_score(raw, &blocklist);
        for (k, d) in &why {
            r.finding(k, d.clone(), score);
        }
        let decision = if score >= 70 { "block" } else if score >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": score, "detail": { "decision": decision, "score": score } }));
        r.summary = format!("Email Spam: uamuzi {decision} (score {score}).");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_spam() {
        let raw = "From: x@mail.ru\r\nSubject: WINNER!!! Umeshindwa lottery million\r\nbofya hapa casino bitcoin\r\n";
        let (s, _) = email_spam_score(raw, &HashSet::new());
        assert!(s >= 70, "score {s}");
    }
}
