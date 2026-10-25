//! m35_firewall.rs — MODULE 35: FIREWALL SECURITY (nftables halisi)
//!
//! Spec: "Inaweka firewall rules kwenye server. Twist: inajirekebisha yenyewe
//! kulingana na mashambulizi."
//!
//! Hii si catalog: inasoma ruleset halisi ya kernel (`nft -j list ruleset`),
//! inaandika set/chain halisi, na twist ya "self-healing" inatumia counter
//! halisi za pakiti kuongeza rate-limit pale drops zinapopanda.

use super::engine::{self, listening_ports};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

const TABLE: &str = "mtech_ts";
const SET: &str = "blocked";
/// Kikomo cha drops kwa rule kabla self-heal iingilie.
const HEAL_DROP_THRESHOLD: u64 = 1000;

pub struct M35Firewall;

impl TsModule for M35Firewall {
    fn id(&self) -> u16 {
        35
    }
    fn name(&self) -> &'static str {
        "Firewall Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Inasoma na kuweka firewall rules halisi (nftables) kwenye server; hujirekebisha \
         kwa rate-limit pale mashambulizi yanapoongezeka."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("status");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "status" => self.status(),
            "audit" => self.audit(),
            "block" => self.block(target, apply),
            "unblock" => self.unblock(target, apply),
            "harden" => self.harden(apply),
            "heal" => self.heal(apply),
            other => Err(format!("op '{other}' haipo: status|audit|block|unblock|harden|heal")),
        }
    }
}

impl M35Firewall {
    fn ensure_table_script(&self) -> String {
        engine::fw_ensure_script()
    }

    /// Hali halisi: table/chain/rules/set elements kutoka kernel.
    fn status(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(35, self.name());
        let rs = engine::nft_ruleset()?;
        let objs = engine::ruleset_objects(&rs);
        let mut table_present = false;
        let mut chains = Vec::new();
        let mut rules = 0u64;
        let mut blocked: Vec<String> = Vec::new();
        for o in &objs {
            if let Some(t) = o.get("table") {
                let fam = t.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let nm = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if nm == TABLE && fam == "inet" {
                    table_present = true;
                }
            }
            if let Some(c) = o.get("chain") {
                if c.get("name").and_then(|v| v.as_str()) == Some("input")
                    && c.get("table").and_then(|v| v.as_str()) == Some(TABLE)
                {
                    chains.push(
                        c.get("policy")
                            .and_then(|v| v.as_str())
                            .unwrap_or("accept")
                            .to_string(),
                    );
                }
            }
            if o.get("rule").is_some() {
                rules += 1;
            }
            if let Some(s) = o.get("set") {
                if s.get("name").and_then(|v| v.as_str()) == Some(SET)
                    && s.get("table").and_then(|v| v.as_str()) == Some(TABLE)
                {
                    if let Some(el) = s.get("elem").and_then(|v| v.as_array()) {
                        for x in el {
                            if let Some(t) = x.as_str() {
                                blocked.push(t.to_string());
                            } else if let Some(o2) = x.as_object() {
                                // interval element: {prefix: {addr, prefix_len}}
                                if let Some(pr) = o2.get("prefix") {
                                    let a = pr.get("addr").and_then(|v| v.as_str()).unwrap_or("");
                                    let pl = pr
                                        .get("prefix_len")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(32);
                                    blocked.push(format!("{a}/{pl}"));
                                }
                            }
                        }
                    }
                }
            }
        }
        r.summary = if table_present {
            format!(
                "Table inet {TABLE} ipo: chains {}, rules {rules}, blocked {}",
                chains.len(),
                blocked.len()
            )
        } else {
            format!("Table inet {TABLE} haipo — firewall ya MTECH haijawekwa bado.")
        };
        r.findings.push(json!({
            "kind": "firewall_state",
            "table_present": table_present,
            "input_policy": chains.first(),
            "rules": rules,
            "blocked": blocked,
        }));
        if table_present && chains.first().map(|p| p == "accept").unwrap_or(false) {
            r.finding(
                "policy_weak",
                "Input policy ni 'accept' — kila kitu kinaruhusiwa bila rule",
                30,
            );
        }
        Ok(r)
    }

    /// Audit: soma kila rule + counters halisi; onyesha rule zisizotumika (0 packets).
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(35, self.name());
        let rs = engine::nft_ruleset()?;
        let objs = engine::ruleset_objects(&rs);
        let mut total = 0u64;
        let mut zero = 0u64;
        for o in &objs {
            let Some(rule) = o.get("rule") else { continue };
            if rule.get("table").and_then(|v| v.as_str()) != Some(TABLE) {
                continue;
            }
            total += 1;
            let packets = find_num(rule, "packets").unwrap_or(0);
            if packets == 0 {
                zero += 1;
                let cmt = engine::rule_comment(rule).unwrap_or_else(|| "(bila comment)".into());
                r.finding(
                    "rule_unused",
                    format!("Rule '{cmt}' haijakamata pakiti yoyote (0 packets)"),
                    10,
                );
            }
        }
        r.summary = format!("Rules za MTECH: {total}; zisizotumika: {zero}");
        if total == 0 {
            r.finding("no_rules", "Hakuna rules za MTECH — firewall haijasanidiwa", 25);
        }
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(35, self.name());
        let (net, prefix) = engine::parse_cidr(target)
            .ok_or_else(|| format!("target si IP/CIDR sahihi: {target}"))?;
        let entry = if prefix == 32 {
            std::net::Ipv4Addr::from(net).to_string()
        } else {
            format!("{}/{prefix}", std::net::Ipv4Addr::from(net))
        };
        // Hakikisha table ipo (idempotent, usalama: policy accept).
        if apply {
            engine::nft_apply(&self.ensure_table_script())?;
            let script = format!("add element inet {TABLE} {SET} {{ {entry} }}\n");
            engine::nft_apply(&script)?;
            r.applied = true;
            r.dry_run = false;
            r.action("block", format!("{entry} imezuiwa kwenye set @{SET}"), true);
            r.summary = format!("{entry} imezuiwa (nftables halisi).");
        } else {
            r.action("block_planned", format!("Itazuiwa: add element {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }

    fn unblock(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(35, self.name());
        let (net, prefix) = engine::parse_cidr(target)
            .ok_or_else(|| format!("target si IP/CIDR sahihi: {target}"))?;
        let entry = if prefix == 32 {
            std::net::Ipv4Addr::from(net).to_string()
        } else {
            format!("{}/{prefix}", std::net::Ipv4Addr::from(net))
        };
        if apply {
            let script = format!("delete element inet {TABLE} {SET} {{ {entry} }}\n");
            match engine::nft_apply(&script) {
                Ok(_) => {
                    r.applied = true;
                    r.dry_run = false;
                    r.action("unblock", format!("{entry} imetolewa"), true);
                    r.summary = format!("{entry} imetolewa kwenye set.");
                }
                Err(e) => {
                    r.action("unblock", format!("{entry}: {e}"), false);
                    r.summary = format!("{entry} haikuwepo kwenye set.");
                }
            }
        } else {
            r.action("unblock_planned", format!("Itatolewa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itatolewa ukiweka apply=true.");
        }
        Ok(r)
    }

    /// Baseline hardening: policy DROP + loopback + established + ICMP +
    /// bandari zote za huduma zinazosikiliza (kuzuia kujifunga nje).
    fn harden(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(35, self.name());
        let ports = listening_ports();
        let port_list = ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let allow = if port_list.is_empty() {
            "tcp dport { 22 } accept".to_string()
        } else {
            format!("tcp dport {{ {port_list} }} accept")
        };
        let script = format!(
            "destroy table inet {TABLE}\n\
             table inet {TABLE} {{\n\
               set {SET} {{ type ipv4_addr; flags interval; comment \"MTECH blocked\"; }}\n\
               chain input {{\n\
                 type filter hook input priority 0; policy drop;\n\
                 iif lo accept\n\
                 ct state established,related accept\n\
                 ip saddr @{SET} drop\n\
                 ip protocol icmp accept\n\
                 {allow}\n\
               }}\n\
             }}\n"
        );
        r.finding(
            "baseline_plan",
            format!(
                "policy=drop, loopback+established+icmp, TCP ports zilizoruhusiwa: [{}]",
                ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",")
            ),
            5,
        );
        if apply {
            engine::nft_apply(&script)?;
            r.applied = true;
            r.dry_run = false;
            r.action("harden", "Firewall baseline imewekwa (nftables halisi)", true);
            r.summary = format!(
                "Baseline imewekwa: policy=drop, ports {} zimeruhusiwa.",
                ports.len()
            );
        } else {
            r.action("harden_planned", "Baseline itawekwa", true);
            r.summary = "Dry-run: baseline itawekwa ukiweka apply=true.".into();
        }
        Ok(r)
    }

    /// Twist ya kipekee: kujirekebisha. Inasoma counters halisi; rule yenye
    /// drops nyingi hupata rate-limit ya ziada (module inajilinda yenyewe).
    fn heal(&self, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(35, self.name());
        let rs = engine::nft_ruleset()?;
        let objs = engine::ruleset_objects(&rs);
        let mut healed = 0u64;
        for o in &objs {
            let Some(rule) = o.get("rule") else { continue };
            if rule.get("table").and_then(|v| v.as_str()) != Some(TABLE) {
                continue;
            }
            let packets = find_num(rule, "packets").unwrap_or(0);
            if packets >= HEAL_DROP_THRESHOLD {
                healed += 1;
                r.finding(
                    "attack_volume",
                    format!("Rule imekamata pakiti {packets} — inahitaji rate-limit"),
                    20,
                );
            }
        }
        if apply && healed > 0 {
            // Ongeza rate-limit kwa traffic inayokuja (inajilinda wenyewe).
            let script = format!(
                "add rule inet {TABLE} input tcp flags syn limit rate over 100/second drop comment \"MTECH auto-heal\"\n"
            );
            engine::nft_apply(&script)?;
            r.applied = true;
            r.dry_run = false;
            r.action("self_heal", "Rate-limit imeongezwa (SYN > 100/s drop)", true);
        } else if healed > 0 {
            r.action("self_heal_planned", "Rate-limit itaongezwa", true);
        }
        r.summary = format!("Self-heal: rules {healed} zimehitaji rate-limit.");
        Ok(r)
    }
}

/// Tafuta namba (mf. packets/bytes) ndani ya JSON ya rule.
fn find_num(v: &Value, key: &str) -> Option<u64> {
    match v {
        Value::Object(m) => {
            if let Some(x) = m.get(key) {
                if let Some(n) = x.as_u64() {
                    return Some(n);
                }
            }
            for (_k, val) in m {
                if let Some(f) = find_num(val, key) {
                    return Some(f);
                }
            }
            None
        }
        Value::Array(a) => a.iter().find_map(|x| find_num(x, key)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_dry_run_haiathiri_mfumo() {
        let m = M35Firewall;
        let r = m.run("10.9.9.9", &json!({ "op": "block" })).unwrap();
        assert!(r.dry_run);
        assert!(!r.applied);
        assert_eq!(r.module, 35);
        assert!(r.summary.contains("10.9.9.9"));
    }

    #[test]
    fn op_isiyojulikana_inakataliwa() {
        let m = M35Firewall;
        assert!(m.run("x", &json!({ "op": "futa_kila_kitu" })).is_err());
    }

    #[test]
    fn target_mbaya_inakataliwa() {
        let m = M35Firewall;
        assert!(m.run("si-ip", &json!({ "op": "block" })).is_err());
    }

    #[test]
    fn find_num_inapata_packets() {
        let v = json!({ "expr": [ { "counter": { "packets": 42, "bytes": 7 } } ] });
        assert_eq!(find_num(&v, "packets"), Some(42));
    }
}
