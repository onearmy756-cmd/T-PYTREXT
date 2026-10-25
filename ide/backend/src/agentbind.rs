//! agentbind.rs — KUUNGANISHA agents 27 na katalogi ya majukwaa.
//!
//! Hii inafanya **ThreatSwarm kuongoza zana zote**: kila agent anapata
//! (jukwaa, kazi) inayolingana naye, kisha `platforms::plan()` hurudisha amri
//! halisi za jukwaa hilo. Hivyo "AGENT WAKE 27 ndio waongoze zana zote".
//!
//! Lugha zinazoendesha: **Rust** (injini + plan), **Go** (akiliscan),
//! **Elixir** (Akili.Swarm), **R** (analytics).

use crate::platforms;
use crate::threatswarm::AGENTS;
use serde_json::{json, Value};

/// Ramani: agent → (jukwaa, kazi). Hii ni "mikono" ya kila agent.
pub const BINDINGS: &[(&str, &str, &str)] = &[
    ("recon", "kali", "recon"),
    ("exploit", "kali", "exploit"),
    ("post-ex", "kali", "postex"),
    ("web-attacker", "kali", "web"),
    ("api-attacker", "kali", "api"),
    ("active-directory", "windows", "active-directory"),
    ("network-ops", "kali", "network"),
    ("osint", "kali", "recon"),
    ("wireless-attacker", "kali", "wireless"),
    ("cloud-attacker", "kali", "cloud"),
    ("container-attacker", "kali", "cloud"),
    ("mobile-attacker", "android", "dfir"),
    ("social-engineer", "kali", "recon"),
    ("evasion", "windows", "hardening"),
    ("c2-operator", "kali", "network"),
    ("crypto-attacker", "kali", "web"),
    ("iot-attacker", "kali", "dfir"),
    ("password-attacks", "kali", "network"),
    ("vuln-researcher", "kali", "web"),
    ("reverse-engineer", "kali", "dfir"),
    ("malware-analyst", "windows", "dfir"),
    ("dfir", "windows", "dfir"),
    ("threat-hunter", "kali", "network"),
    ("blue-team", "uos", "hardening"),
    ("log-analyst", "uos", "audit"),
    ("compliance-scanner", "uos", "audit"),
    ("report-writer", "kali", "recon"),
];

/// (jukwaa, kazi) ya agent, kama ipo.
pub fn binding_for(agent: &str) -> Option<(&'static str, &'static str)> {
    BINDINGS
        .iter()
        .find(|(a, _, _)| *a == agent)
        .map(|(_, p, t)| (*p, *t))
}

/// Ramani: agent → ops za finshield (ulinzi wa benki/API gateway) anazoongoza.
///
/// Hii inafanya **agents 27 hao hao** waongoze zana za kifedha — hatuongezi agent
/// wa 28. Kila op ni sahihi kwa `finshield::OPS`.
pub const FIN_BINDINGS: &[(&str, &[&str])] = &[
    ("web-attacker", &["gateway_audit", "secrets"]),
    ("api-attacker", &["gateway_audit", "secrets", "strength"]),
    ("crypto-attacker", &["strength", "secrets"]),
    ("compliance-scanner", &["gateway_audit", "wire_audit"]),
    ("malware-analyst", &["secrets", "pci"]),
    ("dfir", &["pci", "secrets"]),
    ("log-analyst", &["pci", "secrets"]),
    ("password-attacks", &["strength"]),
    ("blue-team", &["gateway_audit", "bucket"]),
    ("threat-hunter", &["txn_risk"]),
    ("osint", &["strength"]),
    ("cloud-attacker", &["secrets"]),
    ("report-writer", &["gateway_audit", "wire_audit"]),
];

/// Ops za finshield zinazoongozwa na agent.
pub fn finops_for(agent: &str) -> &'static [&'static str] {
    FIN_BINDINGS
        .iter()
        .find(|(a, _)| *a == agent)
        .map(|(_, o)| *o)
        .unwrap_or(&[])
}

/// Ramani kamili ya agent → ops za finshield.
pub fn finops_catalog() -> Value {
    let rows: Vec<Value> = AGENTS
        .iter()
        .map(|a| {
            let ops = finops_for(a.name);
            json!({"agent": a.name, "class": a.class.as_str(), "finops": ops})
        })
        .filter(|r| !r["finops"].as_array().map(|a| a.is_empty()).unwrap_or(true))
        .collect();
    json!({
        "ok": true,
        "engine": "akili-security/finshield",
        "led_by": "ThreatSwarm (agents 27)",
        "agents_with_finops": rows.len(),
        "bindings": rows,
    })
}

/// Ramani: agent → ops za uthibitisho wa API (JWT/OAuth/Nacos).
/// ThreatSwarm huongoza uchambuzi wa uthibitisho wa API.
pub const AUTH_BINDINGS: &[(&str, &[&str])] = &[
    ("api-attacker", &["jwt", "oauth"]),
    ("web-attacker", &["jwt", "oauth"]),
    ("cloud-attacker", &["oauth", "nacos"]),
    ("crypto-attacker", &["jwt"]),
    ("compliance-scanner", &["oauth", "nacos"]),
    ("blue-team", &["jwt", "oauth", "nacos"]),
    ("threat-hunter", &["jwt"]),
    ("log-analyst", &["jwt"]),
    ("osint", &["oauth"]),
    ("report-writer", &["oauth", "nacos"]),
];

/// Ops za uthibitisho za agent (tupu ikiwa hana).
pub fn authops_for(agent: &str) -> &'static [&'static str] {
    AUTH_BINDINGS
        .iter()
        .find(|(a, _)| *a == agent)
        .map(|(_, o)| *o)
        .unwrap_or(&[])
}

/// Ramani kamili ya agent → ops za uthibitisho.
pub fn authops_catalog() -> Value {
    let rows: Vec<Value> = AGENTS
        .iter()
        .map(|a| json!({"agent": a.name, "class": a.class.as_str(), "authops": authops_for(a.name)}))
        .collect();
    let with: Vec<&str> = rows
        .iter()
        .filter(|r| !r["authops"].as_array().unwrap().is_empty())
        .map(|r| r["agent"].as_str().unwrap())
        .collect();
    json!({
        "ok": true, "engine": "akili-security/agentbind",
        "led_by": "ThreatSwarm (agents 27)",
        "agents_with_authops": with.len(),
        "bindings": rows,
    })
}

/// Plan ya zana kwa agent mmoja: jukwaa + kazi → amri halisi.
pub fn agent_plan(agent: &str, target: &str) -> Value {
    match binding_for(agent) {
        Some((platform, task)) => {
            let mut v = platforms::plan(platform, task, target);
            if let Some(o) = v.as_object_mut() {
                o.insert("agent".into(), json!(agent));
                o.insert("led_by".into(), json!("ThreatSwarm"));
                o.insert("finops".into(), json!(finops_for(agent)));
                o.insert("authops".into(), json!(authops_for(agent)));
            }
            v
        }
        None => json!({
            "ok": false,
            "error": format!("agent `{agent}` haijulikani"),
            "agents": AGENTS.iter().map(|a| a.name).collect::<Vec<_>>(),
        }),
    }
}

/// CONDUCT: agents 27 huongoza zana zote. Hurudisha kila agent + amri zake.
pub fn conduct(target: &str) -> Value {
    let plans: Vec<Value> = AGENTS
        .iter()
        .map(|a| {
            let mut p = agent_plan(a.name, target);
            if let Some(o) = p.as_object_mut() {
                o.insert("class".into(), json!(a.class.as_str()));
                o.insert("title".into(), json!(a.title));
            }
            p
        })
        .collect();
    let total_cmds: u64 = plans
        .iter()
        .filter_map(|p| p["count"].as_u64())
        .sum();
    json!({
        "ok": true,
        "engine": "akili-security/agentbind",
        "leader": "ThreatSwarm (agents 27)",
        "languages": ["rust", "go", "elixir", "r"],
        "target": target,
        "agent_count": plans.len(),
        "total_commands": total_cmds,
        "agents": plans,
        "note": "ThreatSwarm huongoza; amri za hitl=true zinahitaji idhini.",
    })
}

/// Ramani kamili ya agent → jukwaa/kazi.
pub fn catalog() -> Value {
    let rows: Vec<Value> = AGENTS
        .iter()
        .map(|a| {
            let (p, t) = binding_for(a.name).unwrap_or(("", ""));
            json!({"agent": a.name, "class": a.class.as_str(), "platform": p, "task": t})
        })
        .collect();
    json!({"ok": true, "leader": "ThreatSwarm", "count": rows.len(), "bindings": rows})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kila_agent_ina_binding() {
        for a in AGENTS {
            assert!(binding_for(a.name).is_some(), "agent {} haina binding", a.name);
        }
        assert_eq!(BINDINGS.len(), AGENTS.len());
    }

    #[test]
    fn bindings_ni_za_majukwaa_halali() {
        let names = platforms::names();
        for (agent, p, _t) in BINDINGS {
            assert!(names.contains(p), "agent {agent} ana jukwaa batili {p}");
        }
    }

    #[test]
    fn agent_plan_hurudisha_amri() {
        let p = agent_plan("recon", "example.tz");
        assert_eq!(p["ok"], true);
        assert_eq!(p["agent"], "recon");
        assert!(p["count"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn agent_isiyojulikana_hurudisha_kosa() {
        let p = agent_plan("hakuna-agent", "x");
        assert_eq!(p["ok"], false);
    }

    #[test]
    fn conduct_ina_agents_27_na_amri() {
        let c = conduct("127.0.0.1");
        assert_eq!(c["agent_count"], 27);
        assert!(c["total_commands"].as_u64().unwrap() >= 27);
        assert_eq!(c["leader"], "ThreatSwarm (agents 27)");
    }

    #[test]
    fn finbindings_ni_agents_halali_na_ops_halali() {
        for (agent, ops) in FIN_BINDINGS {
            assert!(binding_for(agent).is_some(), "agent {agent} haipo");
            assert!(!ops.is_empty());
            for op in *ops {
                assert!(
                    crate::finshield::OPS.contains(op),
                    "op {op} si sahihi kwa finshield"
                );
            }
        }
    }

    #[test]
    fn auth_bindings_ni_sahihi() {
        let valid = ["jwt", "oauth", "nacos"];
        for (agent, ops) in AUTH_BINDINGS {
            assert!(binding_for(agent).is_some(), "agent {agent} haipo");
            assert!(!ops.is_empty());
            for op in *ops {
                assert!(valid.contains(op), "op {op} si sahihi kwa apiauth");
            }
        }
        let cat = authops_catalog();
        assert!(cat["agents_with_authops"].as_u64().unwrap() >= 10);
    }

    #[test]
    fn api_attacker_ana_finops() {
        let p = agent_plan("api-attacker", "https://api.x.tz");
        let ops = p["finops"].as_array().unwrap();
        assert!(ops.iter().any(|o| o == "gateway_audit"));
        assert!(finops_for("api-attacker").contains(&"strength"));
    }

    #[test]
    fn finops_catalog_ina_bindings() {
        let c = finops_catalog();
        assert!(c["agents_with_finops"].as_u64().unwrap() >= 10);
    }
}
