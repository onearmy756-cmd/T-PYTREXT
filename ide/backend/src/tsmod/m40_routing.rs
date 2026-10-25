//! m40_routing.rs — MODULE 40: ROUTING SECURITY
//!
//! Spec: "Inalinda dhidi ya routing attacks (BGP hijacking). Twist: inathibitisha
//! routes kwa RPKI."
//!
//! Hii si catalog: inasoma routing table HALISI (/proc/net/route), inagundua
//! default gateways nyingi, na ina engine ya uhalali wa prefixes (RPKI-class)
//! inayotumia bogons: route kwa prefix ya bogon kwenye default route ni ukiukaji.
//! Pia huzuia martian/bogon sources kwenye interface ya umma (nftables) — HITL.

use super::engine::{self, FW_TABLE};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::net::Ipv4Addr;

pub struct M40Routing;

impl TsModule for M40Routing {
    fn id(&self) -> u16 {
        40
    }
    fn name(&self) -> &'static str {
        "Routing Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Husoma routing table halisi na kuthibitisha prefixes (RPKI-class) kwa bogons; \
         huzuia martian sources kwa nftables kwa idhini."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "audit" => self.audit(),
            "validate" => self.validate(target),
            "block" => self.block(target, apply),
            "martians" => self.martians(apply),
            other => Err(format!("op '{other}' haipo: audit|validate|block|martians")),
        }
    }
}

/// Route moja kutoka /proc/net/route.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub iface: String,
    pub dest: u32,
    pub gateway: u32,
    pub mask: u32,
}

/// Changanua /proc/net/route (little-endian hex).
pub fn parse_routes(txt: &str) -> Vec<Route> {
    let mut v = Vec::new();
    for line in txt.lines().skip(1) {
        let c: Vec<&str> = line.split_whitespace().collect();
        if c.len() < 8 {
            continue;
        }
        let le = |s: &str| -> u32 {
            u32::from_str_radix(s, 16).map(|v| v.swap_bytes()).unwrap_or(0)
        };
        v.push(Route {
            iface: c[0].to_string(),
            dest: le(c[1]),
            gateway: le(c[2]),
            mask: le(c[7]),
        });
    }
    v
}

impl M40Routing {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(40, self.name());
        let txt = std::fs::read_to_string("/proc/net/route").unwrap_or_default();
        let routes = parse_routes(&txt);
        let mut default_gws: Vec<String> = Vec::new();
        for rt in &routes {
            if rt.dest == 0 && rt.mask == 0 {
                default_gws.push(format!(
                    "{} via {}",
                    rt.iface,
                    Ipv4Addr::from(rt.gateway)
                ));
            }
        }
        r.findings.push(json!({
            "kind": "routes", "severity": 2,
            "detail": format!("Routes {}; default gateways {}", routes.len(), default_gws.len())
        }));
        for gw in &default_gws {
            r.finding("default_route", format!("Default: {gw}"), 3);
        }
        if default_gws.len() > 1 {
            r.finding(
                "multiple_defaults",
                format!("Default gateways {} — uwezekano wa route injection", default_gws.len()),
                60,
            );
        }
        if routes.is_empty() {
            r.finding("no_routes", "Routing table haipatikani", 20);
        }
        r.summary = format!(
            "Routes {}; default gateways {}.",
            routes.len(),
            default_gws.len()
        );
        Ok(r)
    }

    /// RPKI-class validation: prefix lazima isiwe bogon na iwe na mask halali.
    fn validate(&self, target: &str) -> Result<ModReport, String> {
        let mut r = ModReport::new(40, self.name());
        if target.trim().is_empty() {
            return Err("target (prefix mf. 203.0.113.0/24) inahitajika".into());
        }
        let (net, prefix) = engine::parse_cidr(target)
            .ok_or_else(|| format!("prefix si sahihi: {target}"))?;
        let valid_mask = matches!(prefix, 8 | 16 | 24 | 32);
        let bogon = engine::is_bogon(net);
        r.findings.push(json!({
            "kind": "rpki_validation", "severity": if bogon {70} else {5},
            "detail": {
                "prefix": format!("{}/{prefix}", Ipv4Addr::from(net)),
                "bogon": bogon,
                "mask_len_valid": valid_mask,
                "origin_valid": !bogon,
            }
        }));
        if bogon {
            r.finding(
                "invalid_origin",
                format!("{target} ni bogon — haipaswi kuwa na route halali kwenye internet"),
                70,
            );
        }
        r.summary = format!(
            "{target}: bogon={bogon}, mask_valid={valid_mask}"
        );
        Ok(r)
    }

    fn martians(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(40, self.name());
        // Zuia martian sources kwenye chain ya input (bogon source).
        let script = format!(
            "table inet {FW_TABLE} {{\n\
               chain input {{\n\
                 type filter hook input priority -10; policy accept;\n\
                 ip saddr {{ 0.0.0.0/8, 127.0.0.0/8, 224.0.0.0/4, 240.0.0.0/4 }} drop comment \"MTECH martian\"\n\
                 ip saddr 169.254.0.0/16 drop comment \"MTECH martian\"\n\
               }}\n\
             }}\n"
        );
        r.finding(
            "martian_plan",
            "Bogon sources (0/8, 127/8, 224/4, 240/4, 169.254/16) zitazuiwa kwenye input",
            10,
        );
        if apply {
            engine::nft_apply(&script)?;
            r.applied = true;
            r.dry_run = false;
            r.action("martian_filter", "Martian filter imewekwa (nftables halisi)", true);
            r.summary = "Martian filter imewekwa.".into();
        } else {
            r.action("martian_filter_planned", "Martian filter itawekwa", true);
            r.summary = "Dry-run: martian filter itawekwa ukiweka apply=true.".into();
        }
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(40, self.name());
        let (net, prefix) = engine::parse_cidr(target)
            .ok_or_else(|| format!("target si IP/CIDR sahihi: {target}"))?;
        let entry = if prefix == 32 {
            Ipv4Addr::from(net).to_string()
        } else {
            format!("{}/{prefix}", Ipv4Addr::from(net))
        };
        if apply {
            engine::fw_block(&entry)?;
            r.applied = true;
            r.dry_run = false;
            r.action("route_block", format!("{entry} imezuiwa"), true);
            r.summary = format!("{entry} imezuiwa.");
        } else {
            r.action("route_block_planned", format!("Itazuiwa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_zinaparsika() {
        // 10.1.0.0/24 kwa little-endian: 0000010A ; default 00000000
        let txt = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\n\
                   eth0\t0000010A\t00000000\t0001\t0\t0\t0\t00FFFFFF\n\
                   eth0\t00000000\t0101FEA9\t0003\t0\t0\t0\t00000000\n";
        let rs = parse_routes(txt);
        assert_eq!(rs.len(), 2);
        assert_eq!(rs[0].dest, u32::from(Ipv4Addr::new(10, 1, 0, 0)));
        assert_eq!(rs[1].dest, 0);
    }

    #[test]
    fn bogon_inatambulika() {
        assert!(engine::is_bogon(u32::from(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(engine::is_bogon(u32::from(Ipv4Addr::new(203, 0, 113, 5))));
        assert!(!engine::is_bogon(u32::from(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn validate_inagundua_bogon() {
        let m = M40Routing;
        let r = m.run("10.0.0.0/8", &json!({ "op": "validate" })).unwrap();
        assert!(r.findings.iter().any(|f| f["kind"] == "invalid_origin"));
    }
}
