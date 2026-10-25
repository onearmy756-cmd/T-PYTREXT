//! common_sms.rs — visaidizi vinavyoshirikiwa kwa SEHEMU 9 (SMS, modules 11-20).
//!
//! Uchanganuzi HALISI wa SMS: maneno ya smishing, uchambuzi wa URL, PII
//! (Luhn/NIDA), uthibitisho wa SMSC, na hash-chain ya ushahidi.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Maneno ya hatari ya smishing (Kiswahili + Kiingereza).
pub const SMISH_WORDS: &[&str] = &[
    "hongera", "umeshinda", "zawadi", "tuma", "kutuma", "akaunti", "imefungwa",
    "blocked", "verify", "thibitisha", "otp", "password", "namba ya siri", "pin",
    "refund", "fidia", "loan", "mkopo", "hela", "cash", "benki", "bank",
    "click", "bofya", "link", "winner", "lottery", "bahati nasibu", "urgent",
    "haraka", "sasa hivi", "expire", "maliza", "suspended", "activated",
];

/// Ainisha maandishi ya smishing: rudisha (score 0-100, sababu).
pub fn smish_score(text: &str) -> (u8, Vec<(&'static str, String)>) {
    let low = text.to_lowercase();
    let mut hits = Vec::new();
    for w in SMISH_WORDS {
        if low.contains(w) {
            hits.push(*w);
        }
    }
    let mut why: Vec<(&'static str, String)> = Vec::new();
    let mut score = (hits.len() as u8 * 15).min(75);
    if !hits.is_empty() {
        why.push(("smish_words", format!("Maneno ya hatari: {}", hits.join(", "))));
    }
    // Ombi la pesa ni alama kali.
    if low.contains("tuma") && (low.contains("tsh") || low.contains("shiling") || low.contains("kwa ")) {
        score = score.max(80);
        why.push(("money_request", "Ombi la kutuma pesa — utapeli wa kawaida".into()));
    }
    // Namba ndani ya maandishi (lengo la kutuma).
    if has_phone(&low) && (low.contains("tuma") || low.contains("piga")) {
        score = score.max(85);
        why.push(("target_number", "SMS ina namba ya kutuma pesa".into()));
    }
    // Viungo.
    for u in extract_urls(text) {
        let (us, uw) = url_risk(&u);
        if us > score {
            score = us;
        }
        why.push(("suspicious_url", format!("{u}: {}", uw)));
    }
    (score, why)
}

/// Viungo ndani ya maandishi.
pub fn extract_urls(text: &str) -> Vec<String> {
    let re_like: Vec<String> = text
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '/' && c != '.' && c != '-' && c != ':' && c != '_').to_string())
        .filter(|w| w.starts_with("http://") || w.starts_with("https://") || w.starts_with("www.") || (w.contains('.') && w.split('/').next().map(|h| h.contains('.')).unwrap_or(false) && !w.contains(' ')))
        .collect();
    re_like
}

/// Hatari ya URL: IP literal, TLD hatari, homoglyph, urefu, @ trick.
pub fn url_risk(url: &str) -> (u8, String) {
    let low = url.to_lowercase();
    let mut score = 0u8;
    let mut why = Vec::new();
    let host = low.trim_start_matches("http://").trim_start_matches("https://");
    let host = host.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.split('@').last().unwrap_or(host);
    // IP literal badala ya jina.
    let first = host.split(':').next().unwrap_or("");
    if first.chars().all(|c| c.is_ascii_digit() || c == '.') && first.contains('.') {
        score = score.max(70);
        why.push("host ni IP (si jina)".into());
    }
    for tld in [".tk", ".ml", ".ga", ".cf", ".gq", ".xyz", ".top", ".club", ".work", ".zip", ".mov"] {
        if host.ends_with(tld) {
            score = score.max(55);
            why.push(format!("TLD hatari {tld}"));
        }
    }
    if low.contains('@') {
        score = score.max(60);
        why.push("'@' ndani ya URL (hila)".into());
    }
    if host.len() > 40 {
        score = score.max(45);
        why.push("jina refu sana".into());
    }
    if !low.starts_with("https://") {
        score = score.max(30);
        why.push("si HTTPS".into());
    }
    (score, if why.is_empty() { "sahihi".to_string() } else { why.join("; ") })
}

fn has_phone(text: &str) -> bool {
    let d: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    // Mlolongo wa tarakimu 9+ unaofanana na namba ya simu.
    let mut run = 0;
    for c in text.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= 9 {
                return true;
            }
        } else if c == '+' || c == ' ' || c == '-' {
            // endelea
        } else {
            run = 0;
        }
    }
    d.len() >= 9
}

/// Luhn check (kadi za benki / namba za simu).
pub fn luhn_ok(num: &str) -> bool {
    let d: Vec<u32> = num.chars().filter(|c| c.is_ascii_digit()).filter_map(|c| c.to_digit(10)).collect();
    if d.len() < 8 {
        return false;
    }
    let mut sum = 0u32;
    let mut alt = false;
    for &n in d.iter().rev() {
        let mut v = n;
        if alt {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
        alt = !alt;
    }
    sum % 10 == 0
}

/// NIDA (Tanzania) — tarakimu 20 (huonekana kama namba ndefu).
pub fn find_nida(text: &str) -> Option<String> {
    let digits: Vec<char> = text.chars().filter(|c| c.is_ascii_digit()).collect();
    for w in text.split(|c: char| !c.is_ascii_digit()) {
        if w.len() == 20 {
            return Some(w.to_string());
        }
    }
    let _ = digits;
    None
}

/// Hash ya SMS kwa blockchain (ushahidi bila maudhui).
pub fn sms_hash(from: &str, to: &str, body: &str) -> String {
    let mut h = Sha256::new();
    h.update(from.as_bytes());
    h.update(b"|");
    h.update(to.as_bytes());
    h.update(b"|");
    h.update(body.as_bytes());
    format!("{:x}", h.finalize())
}

/// Mnyororo wa hash (kila SMS inaunganisha na ya awali).
pub fn chain(prev: &str, entry: &str) -> String {
    let mut h = Sha256::new();
    h.update(prev.as_bytes());
    h.update(entry.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn finding_json(kind: &str, detail: impl Into<String>, severity: u8) -> Value {
    json!({ "kind": kind, "detail": detail.into(), "severity": severity })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smishing_inagundulika() {
        let (s, _) = smish_score("Hongera! Umeshinda TSh 5,000,000. Tuma TSh 50,000 kwa 0700001234");
        assert!(s >= 80, "score {s}");
    }

    #[test]
    fn url_hatari_inagundulika() {
        let (s, _) = url_risk("http://192.168.1.1/login");
        assert!(s >= 60);
        let (s2, _) = url_risk("http://malipo.tk/verify");
        assert!(s2 >= 50);
    }

    #[test]
    fn luhn_inafanya_kazi() {
        assert!(luhn_ok("4111111111111111"));
        assert!(!luhn_ok("4111111111111112"));
    }

    #[test]
    fn hash_ni_taratibu() {
        assert_eq!(sms_hash("a", "b", "c"), sms_hash("a", "b", "c"));
        assert_ne!(sms_hash("a", "b", "c"), sms_hash("a", "b", "d"));
    }
}
