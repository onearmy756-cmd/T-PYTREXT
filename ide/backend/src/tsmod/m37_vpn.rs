//! m37_vpn.rs — MODULE 37: VPN SECURITY
//!
//! Spec: "Inaweka na kudhibiti VPN (WireGuard/OpenVPN). Twist: inagundua ikiwa
//! VPN inatumika kwa uhalifu."
//!
//! Hii si catalog: inasoma interfaces HALISI (/sys/class/net), inachanganua
//! conntrack kwa tunnels (WG 51820, OpenVPN 1194, IPsec 500/4500, PPTP 1723),
//! na inaweza kuweka sera (kuzuia VPN zisizoidhinishwa) kwenye nftables — HITL.
//! Ufunguo wa WireGuard wetu unatoka vpn.rs (uliopo).

use super::engine::{self, FW_TABLE};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Bandari za VPN zinazojulikana (tunnel detection).
const VPN_PORTS: &[(u16, &str)] = &[
    (51820, "WireGuard"),
    (1194, "OpenVPN"),
    (500, "IKE/IPsec"),
    (4500, "IPsec NAT-T"),
    (1723, "PPTP"),
    (1701, "L2TP"),
];

/// Interface za VPN zinazojulikana kwa jina.
fn is_vpn_iface(name: &str) -> bool {
    ["wg", "tun", "tap", "ipsec", "ppp", "nordlynx", "utun"]
        .iter()
        .any(|p| name.starts_with(p))
}

pub struct M37Vpn;

impl TsModule for M37Vpn {
    fn id(&self) -> u16 {
        37
    }
    fn name(&self) -> &'static str {
        "VPN Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Huchunguza interfaces halisi na tunnels (WG/OpenVPN/IPsec) kwenye conntrack; \
         huzuia VPN zisizoidhinishwa kwa nftables kwa idhini."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "audit" => self.audit(),
            "tunnels" => self.tunnels(),
            "policy" => self.policy(apply),
            "block" => self.block(target, apply),
            other => Err(format!("op '{other}' haipo: audit|tunnels|policy|block")),
        }
    }
}

/// Interfaces halisi kutoka /sys/class/net.
pub fn interfaces() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir("/sys/class/net")
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

impl M37Vpn {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(37, self.name());
        let ifaces = interfaces();
        let vpn: Vec<&String> = ifaces.iter().filter(|n| is_vpn_iface(n)).collect();
        r.finding(
            "interfaces",
            format!("Interfaces zote: [{}]", ifaces.join(", ")),
            2,
        );
        if vpn.is_empty() {
            r.finding(
                "no_vpn_iface",
                "Hakuna interface ya VPN (wg/tun/...) — WireGuard haijawashwa",
                15,
            );
        } else {
            r.finding(
                "vpn_iface",
                format!(
                    "VPN interfaces: [{}]",
                    vpn.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                ),
                5,
            );
        }
        r.summary = format!("Interfaces {}; VPN {}.", ifaces.len(), vpn.len());
        Ok(r)
    }

    /// Tunnels halisi kutoka conntrack (UDP/TCP kwa bandari za VPN).
    fn tunnels(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(37, self.name());
        let txt = engine::read_root_file("/proc/net/nf_conntrack")?;
        let mut found: Vec<(String, String, u16, String)> = Vec::new();
        for line in txt.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                continue;
            }
            let proto = cols[2];
            let mut src = "";
            let mut dst = "";
            let mut dport = 0u16;
            for c in &cols {
                if let Some(v) = c.strip_prefix("src=") {
                    if src.is_empty() {
                        src = v;
                    }
                } else if let Some(v) = c.strip_prefix("dst=") {
                    if dst.is_empty() {
                        dst = v;
                    }
                } else if let Some(v) = c.strip_prefix("dport=") {
                    if dport == 0 {
                        dport = v.parse().unwrap_or(0);
                    }
                }
            }
            for (p, name) in VPN_PORTS {
                if dport == *p {
                    found.push((proto.to_string(), src.to_string(), *p, name.to_string()));
                    let _ = dst;
                }
            }
        }
        found.sort();
        found.dedup();
        for (proto, src, p, name) in &found {
            r.finding(
                "vpn_tunnel",
                format!("{proto} kutoka {src} → bandari {p} ({name})"),
                10,
            );
        }
        r.summary = format!("Tunnels zilizogundulika: {}.", found.len());
        Ok(r)
    }

    /// Sera: zuia bandari za VPN zisizoidhinishwa (isipokuwa WG yetu) — HITL.
    fn policy(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(37, self.name());
        // Tunazindua: tunaruhusu WG yetu (51820) kwa interface wg0; tunazuia
        // bandari nyingine za VPN zinazotoka (outbound) — kuzuia matumizi ya VPN
        // isiyoidhinishwa kwenye mtandao wetu.
        let blocked: Vec<u16> = VPN_PORTS
            .iter()
            .filter(|(p, _)| *p != 51820)
            .map(|(p, _)| *p)
            .collect();
        let list = blocked
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let script = format!(
            "table inet {FW_TABLE} {{\n\
               chain output {{\n\
                 type filter hook output priority 0; policy accept;\n\
                 oifname != \"wg0\" udp dport {{ {list} }} drop comment \"MTECH vpn-policy\"\n\
                 oifname != \"wg0\" tcp dport {{ {list} }} drop comment \"MTECH vpn-policy\"\n\
               }}\n\
             }}\n"
        );
        r.finding(
            "vpn_policy_plan",
            format!("Outbound VPN ports zitazuiwa: [{list}] (isipokuwa wg0)"),
            10,
        );
        if apply {
            engine::nft_apply(&script)?;
            r.applied = true;
            r.dry_run = false;
            r.action("vpn_policy", "Sera ya VPN imewekwa (nftables halisi)", true);
            r.summary = format!("Sera imewekwa: bandari {} zimezuiwa.", blocked.len());
        } else {
            r.action("vpn_policy_planned", "Sera itawekwa", true);
            r.summary = "Dry-run: sera itawekwa ukiweka apply=true.".into();
        }
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(37, self.name());
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
            r.action("vpn_block", format!("{entry} imezuiwa"), true);
            r.summary = format!("{entry} imezuiwa.");
        } else {
            r.action("vpn_block_planned", format!("Itazuiwa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vpn_iface_inatambulika() {
        assert!(is_vpn_iface("wg0"));
        assert!(is_vpn_iface("tun0"));
        assert!(!is_vpn_iface("eth0"));
    }

    #[test]
    fn interfaces_zinapatikana() {
        let ifs = interfaces();
        assert!(!ifs.is_empty());
    }

    #[test]
    fn policy_dry_run() {
        let m = M37Vpn;
        let r = m.run("", &json!({ "op": "policy" })).unwrap();
        assert!(r.dry_run);
        assert_eq!(r.module, 37);
    }
}
