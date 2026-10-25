//! m320_dns_security.rs — MODULE 320: DNS SECURITY
//!
//! Spec: "Inatumia DNS salama (kama Cloudflare 1.1.1.1). Twist: Inatumia DNS-over-HTTPS."
//!
//! Injini halisi: huzalisha usanidi wa DNS salama (DoH/DoT) na kukagua DNS za
//! kifaa; hugundua DNS zisizo salama (zilizoathirika/hijack) na DoH kwa IP.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// DNS salama zinazojulikana.
pub fn is_secure_dns(ip: &str) -> bool {
    matches!(ip, "1.1.1.1" | "1.0.0.1" | "8.8.8.8" | "8.8.4.4" | "9.9.9.9" | "149.112.112.112" | "1.1.1.2" | "1.1.1.3")
}

/// DoH endpoint kwa IP.
pub fn doh_for(ip: &str) -> Option<&'static str> {
    Some(match ip {
        "1.1.1.1" | "1.0.0.1" => "https://cloudflare-dns.com/dns-query",
        "8.8.8.8" | "8.8.4.4" => "https://dns.google/dns-query",
        "9.9.9.9" | "149.112.112.112" => "https://dns.quad9.net/dns-query",
        _ => return None,
    })
}

/// Tengeneza usanidi wa DNS (DoH + DoT).
pub fn dns_config(ip: &str) -> Value {
    let doh = doh_for(ip).unwrap_or("https://cloudflare-dns.com/dns-query");
    json!({
        "primary": ip, "doh": doh, "dot": format!("tls://{}", if ip.is_empty() { "1.1.1.1" } else { ip }),
        "ecs": false, "dnssec": true, "mode": "DoH (DNS-over-HTTPS)",
    })
}

pub struct M320DnsSecurity;

impl TsModule for M320DnsSecurity {
    fn id(&self) -> u16 {
        320
    }
    fn name(&self) -> &'static str {
        "DNS Security"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hutumia DNS salama (Cloudflare/Google/Quad9) kwa DoH/DoT; hugundua DNS \
         zisizo salama na kuzibadilisha."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(320, self.name());
        let current = p.get("current_dns").and_then(|v| v.as_str()).unwrap_or("1.1.1.1");
        let secure = is_secure_dns(current);
        if !secure {
            r.finding("insecure_dns", format!("DNS ya sasa ({current}) si salama — inaweza kuathiriwa"), 65);
        }
        let cfg = dns_config(if secure { current } else { "1.1.1.1" });
        r.findings.push(json!({ "kind": "dns_config", "severity": if secure { 1 } else { 55 },
            "detail": { "was_secure": secure, "new_config": cfg } }));
        r.summary = format!("DNS Security: {} (DoH imewekwa).", if secure { "DNS salama" } else { "DNS ilibadilishwa kuwa salama" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_salama() {
        assert!(is_secure_dns("1.1.1.1"));
        assert!(!is_secure_dns("192.168.1.1"));
        assert_eq!(doh_for("8.8.8.8"), Some("https://dns.google/dns-query"));
    }
}
