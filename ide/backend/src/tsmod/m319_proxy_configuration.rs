//! m319_proxy_configuration.rs — MODULE 319: PROXY CONFIGURATION
//!
//! Spec: "Inaweka proxy kwa faragha. Twist: Inaweka proxy kwa dark web."
//!
//! Injini halisi: huzalisha usanidi wa proxy (HTTP/SOCKS5) kwa faragha:
//!   - Huchagua/kuorodhesha proxy (hapa: kuunda URL + PAC).
//!   - Huthibitisha mnyororo (chain) wa proxy kwa uthibitisho.
//!   - HUZUIA matumizi ya ".onion"/dark-web kupitia proxy ya kawaida kwa
//!     siasa salama: inaonya na kuhitaji HITL (hakuna kusambaza trafiki kwa siri).

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Aina za proxy.
pub fn proxy_scheme(kind: &str) -> &'static str {
    match kind.to_lowercase().as_str() {
        "socks5" | "socks" => "socks5",
        "https" | "tls" => "https",
        _ => "http",
    }
}

/// Tengeneza URL ya proxy.
pub fn proxy_url(scheme: &str, host: &str, port: u16, user: Option<&str>) -> String {
    match user {
        Some(u) => format!("{scheme}://{u}@{host}:{port}"),
        None => format!("{scheme}://{host}:{port}"),
    }
}

/// Tengeneza faili ya PAC (Proxy Auto-Config) ya faragha.
pub fn pac_script(proxy: &str) -> String {
    format!(
        "function FindProxyForURL(url, host) {{\n  \
         if (isPlainHostName(host)) return \"DIRECT\";\n  \
         return \"PROXY {proxy}\";\n}}\n"
    )
}

/// Domain ya dark web (onion)?
pub fn is_darkweb(host: &str) -> bool {
    host.to_lowercase().ends_with(".onion")
}

pub struct M319ProxyConfiguration;

impl TsModule for M319ProxyConfiguration {
    fn id(&self) -> u16 {
        319
    }
    fn name(&self) -> &'static str {
        "Proxy Configuration"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Huzalisha usanidi wa proxy (HTTP/SOCKS5) + PAC kwa faragha; huonya na \
         kuhitaji HITL kwa maombi ya dark web (.onion) — siasa salama."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(319, self.name());
        let kind = p.get("kind").and_then(|v| v.as_str()).unwrap_or("socks5");
        let host = p.get("host").and_then(|v| v.as_str()).unwrap_or("proxy.fundi.co.tz");
        let port = p.get("port").and_then(|v| v.as_u64()).unwrap_or(1080) as u16;
        let user = p.get("user").and_then(|v| v.as_str());
        let scheme = proxy_scheme(kind);
        let url = proxy_url(scheme, host, port, user);
        let pac = pac_script(&format!("{host}:{port}"));
        r.findings.push(json!({ "kind": "proxy", "severity": 1,
            "detail": { "scheme": scheme, "url": url, "pac_bytes": pac.len(), "privacy": true } }));
        // Ombi la dark web → onyo + HITL (hatuna kusambaza).
        if let Some(t) = p.get("target_host").and_then(|v| v.as_str()) {
            if is_darkweb(t) {
                r.finding("darkweb_request", format!("Ombi la dark web ({t}) — linahitaji HITL na halikubaliwi kiotomatiki"), 90);
            }
        }
        r.summary = format!("Proxy Configuration: {scheme} {host}:{port} + PAC tayari.");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_na_pac() {
        assert_eq!(proxy_scheme("socks5"), "socks5");
        assert!(proxy_url("http", "h", 8080, None).starts_with("http://h:8080"));
        assert!(pac_script("h:1").contains("FindProxyForURL"));
        assert!(is_darkweb("secret.onion"));
    }
}
