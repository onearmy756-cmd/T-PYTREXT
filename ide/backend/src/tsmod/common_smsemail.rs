//! common_smsemail.rs — visaidizi vinavyoshirikiwa kwa SEHEMU 15 (SMS & Email).
//!
//! Hapa ni uchanganuzi wa kweli: headers za barua pepe, viungo, PII (Luhn, NIDA),
//! na alama za smishing/spam. Modules 288-305 zinatumia hizi.

use serde_json::json;
use sha2::{Digest, Sha256};

/// Headers muhimu za barua pepe.
#[derive(Debug, Default, Clone)]
pub struct EmailHeaders {
    pub from_addr: String,
    pub from_name: String,
    pub reply_to: String,
    pub return_path: String,
    pub subject: String,
    pub spf: String,
    pub dkim: String,
    pub dmarc: String,
    pub received_count: usize,
    pub has_dkim_sig: bool,
}

/// Changanua barua pepe mbichi (raw) → headers.
pub fn parse_email(raw: &str) -> EmailHeaders {
    let mut h = EmailHeaders::default();
    for line in raw.lines() {
        let l = line.trim();
        let low = l.to_lowercase();
        if low.starts_with("from:") {
            let v = l[5..].trim();
            let (name, addr) = split_name_addr(v);
            h.from_name = name;
            h.from_addr = addr;
        } else if low.starts_with("reply-to:") {
            h.reply_to = split_name_addr(l[9..].trim()).1;
        } else if low.starts_with("return-path:") {
            h.return_path = split_name_addr(l[12..].trim()).1;
        } else if low.starts_with("subject:") {
            h.subject = l[8..].trim().to_string();
        } else if low.starts_with("received:") {
            h.received_count += 1;
        } else if low.starts_with("dkim-signature:") {
            h.has_dkim_sig = true;
        } else if low.starts_with("authentication-results:") || low.contains("spf=") || low.contains("dkim=") || low.contains("dmarc=") {
            for (k, v) in [("spf=", &mut h.spf), ("dkim=", &mut h.dkim), ("dmarc=", &mut h.dmarc)] {
                if let Some(i) = low.find(k) {
                    let rest = &low[i + k.len()..];
                    let val: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                    *v = val;
                }
            }
        }
    }
    h
}

/// "Jina <addr>" → (jina, addr).
pub fn split_name_addr(v: &str) -> (String, String) {
    let v = v.trim().trim_matches('"');
    if let (Some(a), Some(b)) = (v.find('<'), v.find('>')) {
        let name = v[..a].trim().trim_matches('"').trim().to_string();
        let addr = v[a + 1..b].trim().to_lowercase();
        (name, addr)
    } else {
        (String::new(), v.trim_matches(|c| c == '<' || c == '>').to_lowercase())
    }
}

/// Toa viungo kutoka kwenye maandishi.
pub fn extract_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in text.split_whitespace() {
        let t = tok.trim_matches(|c: char| "()[]<>,;\"'".contains(c));
        let low = t.to_lowercase();
        if low.starts_with("http://") || low.starts_with("https://") || low.starts_with("www.") {
            out.push(t.to_string());
        }
    }
    out
}

/// Domain kutoka URL (huondoa userinfo `user@` na www).
pub fn domain_of(url: &str) -> String {
    let u = url.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.");
    let host = u.split(['/', '?', '#']).next().unwrap_or("");
    // Ondoa userinfo (kila kitu kabla ya '@' ya mwisho).
    let host = host.rsplit('@').next().unwrap_or(host);
    host.to_lowercase()
}

/// Vifupisho vya URL vinavyojulikana.
const SHORTENERS: &[&str] = &["bit.ly", "tinyurl.com", "t.co", "goo.gl", "ow.ly", "is.gd", "buff.ly", "cutt.ly", "rb.gy", "shorturl.at", "rebrand.ly", "s.id"];

pub fn is_shortener(url: &str) -> bool {
    let d = domain_of(url);
    SHORTENERS.iter().any(|s| d == *s || d.ends_with(&format!(".{s}")))
}

/// TLD hatarishi.
const RISKY_TLDS: &[&str] = &[".zip", ".mov", ".top", ".xyz", ".tk", ".gq", ".ml", ".cf", ".ga", ".work", ".click", ".link", ".country", ".kim"];

pub fn risky_tld(url: &str) -> Option<&'static str> {
    let d = domain_of(url);
    RISKY_TLDS.iter().find(|t| d.ends_with(*t)).copied()
}

/// URL ni IP address moja kwa moja?
pub fn is_ip_url(url: &str) -> bool {
    let d = domain_of(url);
    d.split('.').count() == 4 && d.split('.').all(|p| p.parse::<u8>().is_ok())
}

/// Punycode (homograph attack).
pub fn is_punycode(url: &str) -> bool {
    domain_of(url).contains("xn--")
}

/// Alama za kuchanganua kiungo (0-100).
pub fn link_score(url: &str) -> (u8, Vec<(&'static str, String)>) {
    let mut s = 0u8;
    let mut why = Vec::new();
    if is_shortener(url) {
        s = s.max(45);
        why.push(("url_shortener", format!("Kiungo kimefupishwa: {url}")));
    }
    if let Some(t) = risky_tld(url) {
        s = s.max(60);
        why.push(("risky_tld", format!("TLD hatarishi {t}: {url}")));
    }
    if is_ip_url(url) {
        s = s.max(75);
        why.push(("ip_url", format!("Kiungo ni IP moja kwa moja: {url}")));
    }
    if is_punycode(url) {
        s = s.max(80);
        why.push(("punycode", format!("Punycode/homograph: {url}")));
    }
    if url.to_lowercase().contains("@") && !url.starts_with("mailto:") {
        s = s.max(65);
        why.push(("userinfo_trick", format!("URL yenye '@' (ujanja): {url}")));
    }
    (s, why)
}

/// Namba ya kadi inakidhi Luhn?
pub fn luhn_valid(s: &str) -> bool {
    let d: Vec<u32> = s.chars().filter(|c| c.is_ascii_digit()).filter_map(|c| c.to_digit(10)).collect();
    if d.len() < 12 || d.len() > 19 {
        return false;
    }
    let mut sum = 0u32;
    let mut double = false;
    for &n in d.iter().rev() {
        let mut v = n;
        if double {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
        double = !double;
    }
    sum % 10 == 0
}

/// PII inayogundulika.
#[derive(Debug, Clone)]
pub struct Pii {
    pub kind: &'static str,
    pub masked: String,
}

fn mask(s: &str, keep: usize) -> String {
    if s.len() <= keep {
        return "*".repeat(s.len());
    }
    format!("{}{}", "*".repeat(s.len() - keep), &s[s.len() - keep..])
}

/// Gundua PII: kadi (Luhn), NIDA (tarakimu 20), namba ya simu TZ, barua pepe.
pub fn detect_pii(text: &str) -> Vec<Pii> {
    let mut out = Vec::new();
    let digits: Vec<String> = split_runs(text, |c| c.is_ascii_digit());
    for d in &digits {
        if d.len() >= 13 && d.len() <= 19 && luhn_valid(d) {
            out.push(Pii { kind: "card_number", masked: mask(d, 4) });
        }
        if d.len() == 20 {
            out.push(Pii { kind: "nida_number", masked: mask(d, 4) });
        }
    }
    for tok in text.split_whitespace() {
        let t = tok.trim_matches(|c: char| "()[]<>,;\"'".contains(c));
        if (t.starts_with("+255") || t.starts_with("0")) && t.chars().filter(|c| c.is_ascii_digit()).count() >= 9 && t.chars().all(|c| c.is_ascii_digit() || c == '+') {
            out.push(Pii { kind: "phone_tz", masked: mask(&t.chars().filter(|c| c.is_ascii_digit()).collect::<String>(), 3) });
        }
        if t.contains('@') && t.contains('.') && !t.contains(' ') {
            let (_, addr) = split_name_addr(t);
            if !addr.is_empty() {
                out.push(Pii { kind: "email", masked: mask_email(&addr) });
            }
        }
    }
    out
}

fn mask_email(addr: &str) -> String {
    match addr.split_once('@') {
        Some((u, d)) => format!("{}@{}", mask(u, 1), d),
        None => mask(addr, 2),
    }
}

fn split_runs<F: Fn(char) -> bool>(text: &str, pred: F) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if pred(c) {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Maneno ya haraka/ushawishi (urgency) kwa smishing/spam.
const URGENCY: &[&str] = &["haraka", "urgent", "immediately", "sasa hivi", "within 24", "expire", "imeisha", "imesimama", "suspended", "blocked", "verify now", "thibitisha", "re-activate", "fungua"];

/// Alama za smishing (0-100) kwa SMS.
pub fn smishing_score(sender: &str, text: &str) -> (u8, Vec<(&'static str, String)>) {
    let mut s = 0u8;
    let mut why = Vec::new();
    let low = text.to_lowercase();
    for u in URGENCY {
        if low.contains(u) {
            s = s.max(40);
            why.push(("urgency", format!("Uharaka: '{u}'")));
            break;
        }
    }
    if low.contains("otp") || low.contains("code ya") || low.contains("namba ya siri") || low.contains("pin yako") {
        s = s.max(85);
        why.push(("otp_request", "Ombi la OTP/PIN — sim-swap".into()));
    }
    if low.contains("shinda") || low.contains("umeshinda") || low.contains("congratulations") || low.contains("zawadi") || low.contains("bonus") {
        s = s.max(65);
        why.push(("prize_bait", "Kishawishi cha zawadi/shinda".into()));
    }
    for l in extract_links(text) {
        let (ls, lw) = link_score(&l);
        if ls > s {
            s = ls;
            why.extend(lw);
        }
    }
    // Sender ya kigeni/haijulikani.
    let sd = sender.to_lowercase();
    if sd.contains("+" ) && !sd.starts_with("+255") {
        s = s.max(50);
        why.push(("foreign_sender", format!("Mtumaji wa kigeni: {sender}")));
    }
    (s, why)
}

/// Hash chain (blockchain-lite) kwa audit/archiving.
pub fn chain_hash(prev: &str, payload: &str) -> String {
    let mut h = Sha256::new();
    h.update(prev.as_bytes());
    h.update(b"|");
    h.update(payload.as_bytes());
    let d = h.finalize();
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON kwa PII (kwa findings).
pub fn pii_json(p: &Pii) -> serde_json::Value {
    json!({ "kind": p.kind, "masked": p.masked })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_inafanya_kazi() {
        assert!(luhn_valid("4111111111111111"));
        assert!(!luhn_valid("4111111111111112"));
    }

    #[test]
    fn email_headers_zinachanganuliwa() {
        let raw = "From: \"CRDB Bank\" <no-reply@crdb-bank.co.tz>\r\nReply-To: scam@evil.xyz\r\nSubject: Verify now\r\nAuthentication-Results: spf=fail dkim=none dmarc=fail\r\n";
        let h = parse_email(raw);
        assert_eq!(h.from_addr, "no-reply@crdb-bank.co.tz");
        assert_eq!(h.from_name, "CRDB Bank");
        assert_eq!(h.reply_to, "scam@evil.xyz");
        assert_eq!(h.spf, "fail");
        assert_eq!(h.dmarc, "fail");
    }

    #[test]
    fn links_na_pii_zinagundulika() {
        let links = extract_links("Bofya https://bit.ly/abc na http://192.168.0.1/x");
        assert_eq!(links.len(), 2);
        assert!(is_shortener(&links[0]));
        assert!(is_ip_url(&links[1]));
        let pii = detect_pii("kadi 4111111111111111 nida 19900101123456789012");
        assert!(pii.iter().any(|p| p.kind == "card_number"));
        assert!(pii.iter().any(|p| p.kind == "nida_number"));
    }
}
