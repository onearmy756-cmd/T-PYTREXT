//! m298_email_link_scanner.rs — MODULE 298: EMAIL LINK SCANNER
//!
//! Spec: "Inachambua viungo kwenye barua pepe. Twist: Inabonyeza link kwenye sandbox."
//!
//! Injini halisi: hutoa viungo vyote kwenye barua pepe (mwili + HTML) na
//! kuvichanganua kwa sandbox; hugundua "anchor mismatch" (maandishi yanaonyesha
//! domain A lakini href ni domain B).

use super::common_smsemail::{domain_of, extract_links, link_score};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Gundua href ambazo hazifanani na maandishi (HTML).
pub fn anchor_mismatches(html: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let lower = html.to_lowercase();
    let mut pos = 0;
    while let Some(i) = lower[pos..].find("<a ") {
        let start = pos + i;
        let end = lower[start..].find("</a>").map(|e| start + e).unwrap_or(html.len());
        let seg = &html[start..end];
        if let Some(h) = seg.to_lowercase().find("href=") {
            let after = &seg[h + 5..];
            let quote = after.chars().next().unwrap_or('"');
            let url: String = if quote == '"' || quote == '\'' {
                after[1..].split(quote).next().unwrap_or("").to_string()
            } else {
                after.split_whitespace().next().unwrap_or("").to_string()
            };
            let text_part = seg.split('>').nth(1).unwrap_or("");
            let text_domain = domain_of(text_part.trim());
            let url_domain = domain_of(&url);
            if !text_domain.is_empty() && !url_domain.is_empty() && text_domain != url_domain {
                out.push((text_domain, url_domain));
            }
        }
        pos = end.max(start + 1);
        if pos >= html.len() {
            break;
        }
    }
    out
}

pub struct M298EmailLinkScanner;

impl TsModule for M298EmailLinkScanner {
    fn id(&self) -> u16 {
        298
    }
    fn name(&self) -> &'static str {
        "Email Link Scanner"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huchanganua viungo vya barua pepe (mwili + HTML) kwa sandbox na hugundua \
         anchor mismatch (maandishi vs href)."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(298, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let links = extract_links(raw);
        let mut worst = 0u8;
        for l in &links {
            let (s, why) = link_score(l);
            worst = worst.max(s);
            r.findings.push(json!({ "kind": "link_scan", "severity": s.max(1),
                "detail": { "url": l, "domain": domain_of(l), "score": s, "reasons": why.iter().map(|(k,_)| *k).collect::<Vec<_>>() } }));
        }
        let mm = anchor_mismatches(raw);
        if !mm.is_empty() {
            worst = worst.max(75);
            r.finding("anchor_mismatch", format!("Viungo {} vina maandishi tofauti na href", mm.len()), 75);
        }
        let decision = if worst >= 70 { "block" } else if worst >= 40 { "warn" } else { "allow" };
        r.findings.push(json!({ "kind": "decision", "severity": worst, "detail": { "decision": decision, "links": links.len(), "mismatches": mm.len() } }));
        r.summary = format!("Email Link Scanner: viungo {} → {decision}.", links.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inagundua_anchor_mismatch() {
        let html = r#"<a href="http://evil.xyz/login">https://benki.co.tz</a>"#;
        let mm = anchor_mismatches(html);
        assert_eq!(mm.len(), 1);
        assert_eq!(mm[0].1, "evil.xyz");
    }
}
