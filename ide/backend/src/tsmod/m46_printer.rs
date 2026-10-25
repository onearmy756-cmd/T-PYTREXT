//! m46_printer.rs — MODULE 46: PRINTER SECURITY
//!
//! Spec: "Inalinda printing jobs na data nyeti. Twist: inachambua hati kabla ya
//! kuchapisha kwa DLP."
//!
//! Hii si catalog: inasoma CUPS config halisi (/etc/cups) na ina engine ya DLP
//! inayochanganua maudhui ya hati kwa siri (NIDA, kadi za benki, nywila, keys)
//! kabla ya kuchapisha.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M46Printer;

/// Kategoria ya data nyeti iliyogundulika.
#[derive(Debug, Clone, PartialEq)]
pub struct DlpHit {
    pub kind: &'static str,
    pub sample: String,
}

/// Changanua maandishi kwa data nyeti (DLP).
pub fn dlp_scan(text: &str) -> Vec<DlpHit> {
    let mut hits = Vec::new();
    // Namba za kadi za benki: tarakimu 13-19 mfululizo (space/dash zinaruhusiwa
    // kati ya tarakimu). Tunakusanya runs za tarakimu.
    let mut runs: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if (c == ' ' || c == '-') && !cur.is_empty() {
            // ruhusu muendelezo (usi flush)
        } else {
            if !cur.is_empty() {
                runs.push(std::mem::take(&mut cur));
            }
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    for d in &runs {
        if (13..=19).contains(&d.len()) && luhn(d) {
            hits.push(DlpHit {
                kind: "card_number",
                sample: mask(d),
            });
            break;
        }
    }
    let lower = text.to_lowercase();
    for kw in [
        "password", "nywila", "secret", "api_key", "apikey", "private key",
        "begin rsa", "begin openssh", "bearer ",
    ] {
        if lower.contains(kw) {
            hits.push(DlpHit {
                kind: "credential",
                sample: kw.to_string(),
            });
        }
    }
    // NIDA/kitambulisho cha Tanzania: tarakimu 20 (mf. 19900101-12345-00001-01).
    if runs.iter().any(|d| d.len() >= 20) {
        if let Some(d) = runs.iter().find(|d| d.len() >= 20) {
            hits.push(DlpHit {
                kind: "national_id",
                sample: mask(d),
            });
        }
    }
    hits
}

fn mask(d: &str) -> String {
    if d.len() <= 4 {
        return "****".into();
    }
    format!("****{}", &d[d.len() - 4..])
}

/// Luhn checksum (kadi halali).
pub fn luhn(d: &str) -> bool {
    let mut sum = 0u32;
    let mut alt = false;
    for c in d.chars().rev() {
        let Some(mut n) = c.to_digit(10) else {
            return false;
        };
        if alt {
            n *= 2;
            if n > 9 {
                n -= 9;
            }
        }
        sum += n;
        alt = !alt;
    }
    sum % 10 == 0
}

impl TsModule for M46Printer {
    fn id(&self) -> u16 {
        46
    }
    fn name(&self) -> &'static str {
        "Printer Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Husoma CUPS halisi na kuchanganua hati kwa data nyeti (DLP) kabla ya \
         kuchapisha — kadi, nywila, vitambulisho."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "dlp" => self.dlp(params),
            other => Err(format!("op '{other}' haipo: audit|dlp")),
        }
    }
}

impl M46Printer {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(46, self.name());
        let mut found = false;
        for p in ["/etc/cups/cupsd.conf", "/etc/cups/printers.conf"] {
            if let Ok(txt) = std::fs::read_to_string(p) {
                found = true;
                let allow_all = txt.contains("Allow All");
                r.finding(
                    "cups_config",
                    format!(
                        "{p}: mistari {}; Allow All={allow_all}",
                        txt.lines().count()
                    ),
                    if allow_all { 40 } else { 5 },
                );
            }
        }
        if !found {
            r.finding("no_cups", "CUPS haipo kwenye host hii", 5);
        }
        r.summary = "Hali ya uchapishaji imechunguzwa.".into();
        Ok(r)
    }

    fn dlp(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(46, self.name());
        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("params.text inahitajika")?;
        let hits = dlp_scan(text);
        if hits.is_empty() {
            r.summary = "DLP: hakuna data nyeti — kuchapisha ni salama.".into();
        } else {
            for h in &hits {
                r.finding(
                    "dlp_hit",
                    format!("Data nyeti ({}): {}", h.kind, h.sample),
                    75,
                );
            }
            r.action("dlp_block", "Uchapishaji umezuiwa (data nyeti)", true);
            r.summary = format!("DLP: vitu {} nyeti — uchapishaji umezuiwa.", hits.len());
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_inatambua_kadi() {
        assert!(luhn("4111111111111111"));
        assert!(!luhn("4111111111111112"));
    }

    #[test]
    fn dlp_inagundua_kadi_na_nywila() {
        let t = "Card: 4111 1111 1111 1111 password=hunter2";
        let h = dlp_scan(t);
        assert!(h.iter().any(|x| x.kind == "card_number"));
        assert!(h.iter().any(|x| x.kind == "credential"));
    }

    #[test]
    fn maandishi_salama_hayagundui() {
        assert!(dlp_scan("Hello world, meeting at 3pm").is_empty());
    }
}
