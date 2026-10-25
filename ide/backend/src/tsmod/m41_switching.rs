//! m41_switching.rs — MODULE 41: SWITCHING SECURITY
//!
//! Spec: "Inalinda dhidi ya MAC flooding na ARP spoofing. Twist: inazuia ARP
//! spoofing kwa kuthibitisha kila ARP."
//!
//! Hii si catalog: inasoma ARP cache HALISI (/proc/net/arp) na kugundua MAC
//! moja inayohusishwa na IP nyingi (ARP spoofing / MAC flooding kwenye L2).
//! Pia inaweka ulinzi wa kernel (arp_ignore, arp_announce, rp_filter) — HITL.

use super::engine;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub struct M41Switching;

/// Entry moja ya ARP.
#[derive(Debug, Clone, PartialEq)]
pub struct ArpEntry {
    pub ip: String,
    pub mac: String,
    pub iface: String,
}

/// Changanua /proc/net/arp.
pub fn parse_arp(txt: &str) -> Vec<ArpEntry> {
    let mut v = Vec::new();
    for line in txt.lines().skip(1) {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() < 6 {
            continue;
        }
        v.push(ArpEntry {
            ip: c[0].to_string(),
            mac: c[3].to_string(),
            iface: c[5].to_string(),
        });
    }
    v
}

impl TsModule for M41Switching {
    fn id(&self) -> u16 {
        41
    }
    fn name(&self) -> &'static str {
        "Switching Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Husoma ARP cache halisi na kugundua MAC spoofing/flooding; huweka ulinzi wa \
         kernel (arp_ignore/arp_announce/rp_filter) kwa idhini."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "audit" => self.audit(),
            "spoof" => self.spoof(),
            "harden" => self.harden(apply),
            "block" => self.block(target, apply),
            other => Err(format!("op '{other}' haipo: audit|spoof|harden|block")),
        }
    }
}

impl M41Switching {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(41, self.name());
        let txt = std::fs::read_to_string("/proc/net/arp").unwrap_or_default();
        let entries = parse_arp(&txt);
        r.findings.push(json!({
            "kind": "arp_table", "severity": 2,
            "detail": format!("ARP entries {} kwenye interfaces", entries.len())
        }));
        for e in &entries {
            if e.mac == "00:00:00:00:00:00" || e.mac == "ee:ee:ee:ee:ee:ee" {
                r.finding(
                    "arp_placeholder",
                    format!("{} → {} (MAC ya placeholder)", e.ip, e.mac),
                    5,
                );
            }
        }
        r.summary = format!("ARP entries {} zimechunguzwa.", entries.len());
        Ok(r)
    }

    /// Twist: thibitisha kila ARP. MAC moja → IP nyingi = spoofing.
    fn spoof(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(41, self.name());
        let txt = std::fs::read_to_string("/proc/net/arp").unwrap_or_default();
        let entries = parse_arp(&txt);
        let mut mac_to_ips: HashMap<String, HashSet<String>> = HashMap::new();
        let mut ip_to_macs: HashMap<String, HashSet<String>> = HashMap::new();
        for e in &entries {
            if e.mac == "00:00:00:00:00:00" {
                continue;
            }
            mac_to_ips.entry(e.mac.clone()).or_default().insert(e.ip.clone());
            ip_to_macs.entry(e.ip.clone()).or_default().insert(e.mac.clone());
        }
        let mut alerts = 0;
        for (mac, ips) in &mac_to_ips {
            if ips.len() > 1 {
                alerts += 1;
                r.finding(
                    "arp_spoof",
                    format!(
                        "MAC {mac} imehusishwa na IP {} — ARP spoofing/MAC flooding",
                        ips.len()
                    ),
                    80,
                );
            }
        }
        for (ip, macs) in &ip_to_macs {
            if macs.len() > 1 {
                alerts += 1;
                r.finding(
                    "ip_conflict",
                    format!("IP {ip} ina MAC {} tofauti — mgongano/spoof", macs.len()),
                    75,
                );
            }
        }
        r.summary = format!("ARP entries {}; alerts {alerts}.", entries.len());
        Ok(r)
    }

    fn harden(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(41, self.name());
        let settings: &[(&str, &str)] = &[
            ("net.ipv4.conf.all.arp_ignore", "1"),
            ("net.ipv4.conf.all.arp_announce", "2"),
            ("net.ipv4.conf.all.rp_filter", "1"),
            ("net.ipv4.conf.default.rp_filter", "1"),
        ];
        for (k, v) in settings {
            let cur = engine::read_sysctl(k).unwrap_or_default();
            r.findings.push(json!({
                "kind": "sysctl", "severity": if cur == *v {3} else {20},
                "detail": format!("{k} = {cur} (inayotakiwa {v})")
            }));
        }
        if apply {
            let mut done = 0;
            for (k, v) in settings {
                if engine::write_sysctl(k, v).is_ok() {
                    done += 1;
                }
            }
            r.applied = true;
            r.dry_run = false;
            r.action("arp_harden", format!("Sysctl {done}/{} zimewekwa", settings.len()), true);
            r.summary = format!("Ulinzi wa ARP: sysctl {done} zimewekwa.");
        } else {
            r.action("arp_harden_planned", "Sysctl zitawekwa", true);
            r.summary = "Dry-run: ulinzi wa ARP utawekwa ukiweka apply=true.".into();
        }
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(41, self.name());
        let (net, prefix) = engine::parse_cidr(target)
            .ok_or_else(|| format!("target si IP/CIDR sahihi: {target}"))?;
        let entry = if prefix == 32 {
            std::net::Ipv4Addr::from(net).to_string()
        } else {
            format!("{}/{prefix}", std::net::Ipv4Addr::from(net))
        };
        if apply {
            engine::fw_block(&entry)?;
            r.applied = true;
            r.dry_run = false;
            r.action("l2_block", format!("{entry} imezuiwa"), true);
            r.summary = format!("{entry} imezuiwa.");
        } else {
            r.action("l2_block_planned", format!("Itazuiwa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_inaparsika() {
        let txt = "IP address       HW type     Flags       HW address            Mask     Device\n\
                   10.1.0.201       0x1         0x2         ee:ee:ee:ee:ee:ee     *        eth0\n\
                   10.1.0.5         0x1         0x2         aa:bb:cc:dd:ee:ff     *        eth0\n";
        let e = parse_arp(txt);
        assert_eq!(e.len(), 2);
        assert_eq!(e[1].mac, "aa:bb:cc:dd:ee:ff");
        assert_eq!(e[1].iface, "eth0");
    }
}
