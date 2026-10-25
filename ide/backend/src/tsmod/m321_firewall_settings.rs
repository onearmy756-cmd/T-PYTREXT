//! m321_firewall_settings.rs — MODULE 321: FIREWALL SETTINGS
//!
//! Spec: "Inaweka firewall kwenye kifaa. Twist: Inazuia ports zisizohitajika."
//!
//! Injini halisi: hukagua sheria za firewall na kuzuia ports zisizohitajika
//! (default-deny); hutoa usanidi halisi wa sheria (iptables/ufw-style).

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Ports zinazopaswa kufungwa (hatarishi).
const DANGEROUS_PORTS: &[u16] = &[23, 21, 135, 137, 138, 139, 445, 1433, 1521, 3306, 3389, 5900, 6667, 11211];
/// Ports zinazoruhusiwa (outbound/inbound salama).
const SAFE_PORTS: &[u16] = &[22, 53, 80, 443, 51820];

pub fn is_dangerous(port: u16) -> bool {
    DANGEROUS_PORTS.contains(&port)
}

pub fn is_safe(port: u16) -> bool {
    SAFE_PORTS.contains(&port)
}

/// Tengeneza sheria (default-deny + ruhusa salama).
pub fn build_rules(extra_open: &[u16]) -> Vec<String> {
    let mut rules = vec![
        "iptables -P INPUT DROP".to_string(),
        "iptables -P FORWARD DROP".to_string(),
        "iptables -P OUTPUT ACCEPT".to_string(),
        "iptables -A INPUT -i lo -j ACCEPT".to_string(),
        "iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT".to_string(),
    ];
    for p in SAFE_PORTS {
        rules.push(format!("iptables -A INPUT -p tcp --dport {p} -j ACCEPT"));
    }
    for p in extra_open {
        rules.push(format!("iptables -A INPUT -p tcp --dport {p} -j ACCEPT"));
    }
    for p in DANGEROUS_PORTS {
        rules.push(format!("iptables -A INPUT -p tcp --dport {p} -j DROP"));
    }
    rules
}

pub struct M321FirewallSettings;

impl TsModule for M321FirewallSettings {
    fn id(&self) -> u16 {
        321
    }
    fn name(&self) -> &'static str {
        "Firewall Settings"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Huweka firewall ya kifaa (default-deny) na kuzuia ports zisizohitajika \
         (23, 445, 3389, n.k.)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(321, self.name());
        let open: Vec<u16> = p.get("open_ports").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_u64().map(|u| u as u16)).collect())
            .unwrap_or_default();
        let dangerous: Vec<u16> = open.iter().copied().filter(|p| is_dangerous(*p)).collect();
        for d in &dangerous {
            r.finding("dangerous_port", format!("Port {d} iko wazi — hatarishi"), 70);
        }
        let extra: Vec<u16> = open.iter().copied().filter(|p| !is_safe(*p) && !is_dangerous(*p)).collect();
        let rules = build_rules(&extra);
        r.findings.push(json!({ "kind": "firewall", "severity": if dangerous.is_empty() { 1 } else { 65 },
            "detail": { "default": "deny", "rules": rules.len(), "dangerous_open": dangerous, "allowed": SAFE_PORTS } }));
        r.summary = format!("Firewall Settings: default-deny, sheria {} (hatarishi {}).", rules.len(), dangerous.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deny_na_drop() {
        let rules = build_rules(&[]);
        assert!(rules.iter().any(|r| r.contains("INPUT DROP")));
        assert!(rules.iter().any(|r| r.contains("--dport 445 -j DROP")));
        assert!(is_dangerous(3389));
        assert!(is_safe(443));
    }
}
