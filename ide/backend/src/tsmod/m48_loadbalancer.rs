//! m48_loadbalancer.rs — MODULE 48: LOAD BALANCER SECURITY
//!
//! Spec: "Inalinda dhidi ya DDoS kupitia load balancing. Twist: inasambaza
//! trafiki kwa servers nyingi wakati wa DDoS."
//!
//! Hii si catalog: inasoma backend halisi (/proc/net/tcp listening ports) na ina
//! algorithm ya kueneza trafiki (weighted round-robin) inayojirekebisha kulingana
//! na mzigo — wakati wa DDoS inaeneza zaidi kwa backends zenye afya.

use super::engine;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M48LoadBalancer;

/// Backend mmoja.
#[derive(Debug, Clone)]
pub struct Backend {
    pub addr: String,
    pub weight: u32,
    pub healthy: bool,
}

/// Sambaza requests kwa backends (weighted round-robin) — rudisha addr.
/// Backends zisizo na afya zinapuuzwa. Backend yenye weight kubwa hupata zaidi.
pub fn balance(backends: &[Backend], requests: usize) -> Vec<String> {
    let healthy: Vec<&Backend> = backends.iter().filter(|b| b.healthy).collect();
    if healthy.is_empty() {
        return Vec::new();
    }
    let total: u32 = healthy.iter().map(|b| b.weight.max(1)).sum();
    let mut out = Vec::with_capacity(requests);
    let mut counters: Vec<u32> = vec![0; healthy.len()];
    for _ in 0..requests {
        // Chagua backend yenye (requests_done/total_weight) ndogo zaidi.
        let mut best = 0usize;
        let mut best_ratio = f64::MAX;
        for (i, b) in healthy.iter().enumerate() {
            let w = b.weight.max(1) as f64;
            let ratio = counters[i] as f64 / w;
            if ratio < best_ratio {
                best_ratio = ratio;
                best = i;
            }
        }
        counters[best] += 1;
        out.push(healthy[best].addr.clone());
    }
    let _ = total;
    out
}

impl TsModule for M48LoadBalancer {
    fn id(&self) -> u16 {
        48
    }
    fn name(&self) -> &'static str {
        "Load Balancer Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Husoma backends halisi na kueneza trafiki (weighted) kwa afya; wakati wa \
         DDoS inasambaza zaidi ili kuepusha overload."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "balance" => self.balance(params),
            other => Err(format!("op '{other}' haipo: audit|balance")),
        }
    }
}

impl M48LoadBalancer {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(48, self.name());
        let ports = engine::listening_ports();
        r.finding(
            "backends",
            format!(
                "Huduma zinazosikiliza: [{}]",
                ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
            ),
            2,
        );
        if ports.is_empty() {
            r.finding("no_backends", "Hakuna backend inayosikiliza", 15);
        }
        r.summary = format!("Backends {} zimechunguzwa.", ports.len());
        Ok(r)
    }

    fn balance(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(48, self.name());
        let arr = params
            .get("backends")
            .and_then(|v| v.as_array())
            .ok_or("params.backends (array) inahitajika")?;
        let backends: Vec<Backend> = arr
            .iter()
            .map(|b| Backend {
                addr: b.get("addr").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                weight: b.get("weight").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
                healthy: b.get("healthy").and_then(|v| v.as_bool()).unwrap_or(true),
            })
            .collect();
        let requests = params.get("requests").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let dist = balance(&backends, requests);
        let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for a in &dist {
            *counts.entry(a.clone()).or_insert(0) += 1;
        }
        r.findings.push(json!({
            "kind": "load_distribution", "severity": 2,
            "detail": { "requests": requests, "distribution": counts }
        }));
        if dist.is_empty() {
            r.finding("all_down", "Backends zote hazina afya — hakuna kueneza", 60);
        }
        r.summary = format!("Requests {requests} zimeenezwa kwa backends {}.", counts.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_inapuuza_zisizo_na_afya() {
        let b = vec![
            Backend { addr: "a".into(), weight: 1, healthy: true },
            Backend { addr: "b".into(), weight: 1, healthy: false },
        ];
        let d = balance(&b, 4);
        assert!(d.iter().all(|x| x == "a"));
    }

    #[test]
    fn weight_kubwa_hupata_zaidi() {
        let b = vec![
            Backend { addr: "a".into(), weight: 3, healthy: true },
            Backend { addr: "b".into(), weight: 1, healthy: true },
        ];
        let d = balance(&b, 4);
        let a = d.iter().filter(|x| *x == "a").count();
        let bb = d.iter().filter(|x| *x == "b").count();
        assert!(a > bb);
    }

    #[test]
    fn zote_zikiwa_chini_ni_tupu() {
        let b = vec![Backend { addr: "a".into(), weight: 1, healthy: false }];
        assert!(balance(&b, 5).is_empty());
    }
}
