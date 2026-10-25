//! engine.rs — injini ya uchanganuzi inayotumia HIFADHIDATA (dynamic).
//!
//! Sheria haziko ndani ya code: zinasomwa kutoka `ts_rules` (SQLite) kila
//! wakati. Admin anaweza kuongeza/kubadilisha/kuzima sheria bila recompile, na
//! mfumo hujifunza kutoka feedback (`ts_feedback`) kupitia `store::add_feedback`.
//!
//! Injini hii hutumika kwa kila jukwaa la kijamii (Facebook..YouTube).

use crate::tsmod::store::{self, Rule};
use sqlx::SqlitePool;

/// Rule iliyogusa maandishi/URL.
#[derive(Debug, Clone)]
pub struct RuleHit {
    pub rule_id: i64,
    pub kind: String,
    pub pattern: String,
    pub severity: u8,
}

impl RuleHit {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "rule_id": self.rule_id, "kind": self.kind,
            "pattern": self.pattern, "severity": self.severity
        })
    }
}

/// Changanua maandishi kwa sheria za category "content" za platform hii.
pub async fn match_text(db: &SqlitePool, platform: &str, text: &str) -> Vec<RuleHit> {
    let rules = store::rules_for(db, platform, "content").await;
    match_text_rules(&rules, text)
}

/// Toleo la dhahiri la kulinganisha (kwa majaribio bila DB).
pub fn match_text_rules(rules: &[Rule], text: &str) -> Vec<RuleHit> {
    let low = text.to_lowercase();
    let mut hits = Vec::new();
    for r in rules {
        if low.contains(&r.pattern) {
            hits.push(RuleHit {
                rule_id: r.id,
                kind: r.kind.clone(),
                pattern: r.pattern.clone(),
                severity: r.effective_severity(),
            });
        }
    }
    hits
}

/// Matokeo ya uchanganuzi wa URL.
#[derive(Debug, Clone, Default)]
pub struct UrlVerdict {
    pub score: u32,
    pub hits: Vec<RuleHit>,
    pub notes: Vec<String>,
}

/// Changanua URL kwa sheria za category "link" + uchanganuzi wa kimuundo.
pub async fn match_url(db: &SqlitePool, platform: &str, url: &str) -> UrlVerdict {
    let rules = store::rules_for(db, platform, "link").await;
    let mut v = match_url_rules(&rules, url);
    let host = host_of(url);
    if let Some((_, kind, sev)) = store::ioc_lookup(db, &host).await {
        v.score = (v.score + sev as u32).min(100);
        v.notes.push(format!("host {host} ni IOC ({kind})"));
    }
    v
}

pub fn match_url_rules(rules: &[Rule], url: &str) -> UrlVerdict {
    let low = url.to_lowercase();
    let after = low.split("://").nth(1).unwrap_or(&low);
    let host = after.split(['/', '?']).next().unwrap_or("");
    let host_no_port = host.split(':').next().unwrap_or(host);
    let mut v = UrlVerdict::default();

    if !low.starts_with("https://") {
        v.score += 15;
        v.notes.push("si HTTPS".into());
    }
    if host_no_port.parse::<std::net::Ipv4Addr>().is_ok() {
        v.score += 30;
        v.notes.push("IP-literal (hakuna domain)".into());
    }
    if host_no_port.contains("xn--") {
        v.score += 40;
        v.notes.push("punycode/homoglyph".into());
    }
    if after.contains('@') {
        v.score += 30;
        v.notes.push("'@' kwenye URL (userinfo trick)".into());
    }
    let labels: Vec<&str> = host_no_port.split('.').collect();
    for r in rules {
        let matched = match r.kind.as_str() {
            "bad_tld" => host_no_port.ends_with(&r.pattern),
            "shortener" => host_no_port == r.pattern || host_no_port.ends_with(&format!(".{}", r.pattern)),
            "brand_impersonation" => {
                labels.len() >= 3 && labels[..labels.len() - 2].iter().any(|l| l.contains(&r.pattern))
            }
            _ => low.contains(&r.pattern),
        };
        if matched {
            v.score += r.effective_severity() as u32 / 2;
            v.hits.push(RuleHit {
                rule_id: r.id,
                kind: r.kind.clone(),
                pattern: r.pattern.clone(),
                severity: r.effective_severity(),
            });
        }
    }
    v.score = v.score.min(100);
    v
}

/// Toa host kutoka URL.
pub fn host_of(url: &str) -> String {
    let low = url.to_lowercase();
    let after = low.split("://").nth(1).unwrap_or(&low);
    let host = after.split(['/', '?']).next().unwrap_or("");
    host.split(':').next().unwrap_or(host).to_string()
}

/// Changanua payload ya QR/Snapcode.
pub async fn match_qr(db: &SqlitePool, platform: &str, payload: &str) -> UrlVerdict {
    let low = payload.to_lowercase();
    if low.starts_with("http://") || low.starts_with("https://") {
        match_url(db, platform, payload).await
    } else {
        let mut v = UrlVerdict::default();
        v.score += 10;
        v.notes.push("QR si URL — maudhui ya maandishi".into());
        if low.starts_with("wifi:") || low.contains("password") {
            v.score += 30;
            v.notes.push("QR ina credentials/WiFi".into());
        }
        v
    }
}

/// Umbali wa haversine (km).
pub fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0_f64;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Kuingia moja.
#[derive(Debug, Clone)]
pub struct Login {
    pub lat: f64,
    pub lon: f64,
    pub ts: i64,
    pub device: String,
}

/// Gundua impossible travel (mwendo > 900 km/h).
pub fn impossible_travel(logins: &[Login]) -> Vec<String> {
    let mut out = Vec::new();
    for w in logins.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let dt = (b.ts - a.ts).abs() as f64 / 3600.0;
        if dt <= 0.0 {
            continue;
        }
        let km = haversine_km(a.lat, a.lon, b.lat, b.lon);
        let speed = km / dt;
        if speed > 900.0 {
            out.push(format!(
                "Mwendo {:.0} km/h kati ya ({:.2},{:.2}) na ({:.2},{:.2})",
                speed, a.lat, a.lon, b.lat, b.lon
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tsmod::store::Rule;

    fn rule(id: i64, kind: &str, pat: &str, sev: u8) -> Rule {
        Rule {
            id, platform: "*".into(), category: "content".into(), kind: kind.into(),
            pattern: pat.into(), severity: sev, source: "seed".into(), enabled: true,
            hits: 0, tp: 0, fp: 0, created_at: String::new(),
        }
    }

    #[test]
    fn text_matching() {
        let rules = vec![rule(1, "threat", "nitakuua", 90), rule(2, "profanity", "pumbavu", 60)];
        let h = match_text_rules(&rules, "Nitakuua wewe pumbavu");
        assert_eq!(h.len(), 2);
        assert!(h.iter().any(|x| x.rule_id == 1));
    }

    #[test]
    fn url_brand_impersonation() {
        let rules = vec![rule(1, "brand_impersonation", "facebook", 35), rule(2, "bad_tld", ".xyz", 40)];
        let v = match_url_rules(&rules, "http://facebook.login.verify.xyz/x");
        assert!(v.score >= 40, "score {}", v.score);
        assert_eq!(v.hits.len(), 2);
    }

    #[test]
    fn host_extraction() {
        assert_eq!(host_of("https://a.b.com:8443/p?q=1"), "a.b.com");
        assert_eq!(host_of("http://1.2.3.4/x"), "1.2.3.4");
    }

    #[test]
    fn impossible_travel_test() {
        let l = vec![
            Login { lat: -6.79, lon: 39.28, ts: 0, device: "a".into() },
            Login { lat: 51.5, lon: -0.12, ts: 600, device: "b".into() },
        ];
        assert!(!impossible_travel(&l).is_empty());
    }
}
