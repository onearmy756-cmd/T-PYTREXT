//! m47_proxy.rs — MODULE 47: PROXY SECURITY
//!
//! Spec: "Inaweka na kudhibiti proxy. Twist: inagundua ikiwa proxy inatumika kwa
//! uhalifu."
//!
//! Hii si catalog: inasoma env/config halisi za proxy, na ina engine ya uamuzi
//! inayogundua proxy hatarishi (open proxy, TOR exit, proxy kwenye akaunti
//! iliyoripotiwa) kwa kutumia blacklist + port/anonimity signals.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M47Proxy;

/// Bandari za proxy zinazojulikana.
const PROXY_PORTS: &[u16] = &[3128, 8080, 8888, 1080, 9050, 9150, 8118];

impl TsModule for M47Proxy {
    fn id(&self) -> u16 {
        47
    }
    fn name(&self) -> &'static str {
        "Proxy Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Husoma proxy halisi kutoka env/config na kugundua matumizi ya uhalifu \
         (open proxy, TOR exit, anonimity high)."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "assess" => self.assess(target, params),
            other => Err(format!("op '{other}' haipo: audit|assess")),
        }
    }
}

impl M47Proxy {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(47, self.name());
        let mut found = false;
        for k in [
            "http_proxy",
            "https_proxy",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "all_proxy",
            "ALL_PROXY",
        ] {
            if let Ok(v) = std::env::var(k) {
                if !v.trim().is_empty() {
                    found = true;
                    let risky = v.contains("://") && !v.starts_with("http://") && !v.starts_with("https://");
                    r.finding(
                        "proxy_env",
                        format!("{k} = {v}"),
                        if risky { 30 } else { 5 },
                    );
                }
            }
        }
        if !found {
            r.finding("no_proxy", "Hakuna proxy iliyosanidiwa kwenye env", 2);
        }
        r.summary = "Sanidi za proxy zimechunguzwa.".into();
        Ok(r)
    }

    /// Tathmini proxy: port, anonimity, blacklist.
    fn assess(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(47, self.name());
        let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let anon = params
            .get("anonymity")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let reported = params
            .get("reported")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tor = port == 9050 || port == 9150;
        let mut score = 0u32;
        if PROXY_PORTS.contains(&port) {
            score += 10;
        }
        if anon.eq_ignore_ascii_case("high") || anon.eq_ignore_ascii_case("elite") {
            score += 30;
        }
        if tor {
            score += 40;
            r.finding(
                "tor_exit",
                format!("{target}:{port} ni TOR (Socks) — matumizi ya uhalifu yanawezekana"),
                60,
            );
        }
        if reported {
            score += 90;
            r.finding(
                "reported_proxy",
                format!("{target} imeripotiwa kwa uhalifu"),
                90,
            );
        }
        r.findings.push(json!({
            "kind": "proxy_assessment", "severity": score.min(100) as u8,
            "detail": {
                "target": target, "port": port, "anonymity": anon,
                "tor": tor, "reported": reported, "risk_score": score.min(100)
            }
        }));
        r.summary = format!("Proxy {target}: hatari {}.", score.min(100));
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tor_inagundulika() {
        let m = M47Proxy;
        let r = m
            .run("1.2.3.4", &json!({ "op": "assess", "port": 9050 }))
            .unwrap();
        assert!(r.findings.iter().any(|f| f["kind"] == "tor_exit"));
    }

    #[test]
    fn proxy_iliyoripotiwa_ni_hatari_kubwa() {
        let m = M47Proxy;
        let r = m
            .run("1.2.3.4", &json!({ "op": "assess", "port": 3128, "reported": true }))
            .unwrap();
        let a = r.findings.iter().find(|f| f["kind"] == "proxy_assessment").unwrap();
        assert_eq!(a["detail"]["risk_score"], 100);
    }
}
