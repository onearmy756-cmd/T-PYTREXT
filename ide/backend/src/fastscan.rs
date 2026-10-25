//! fastscan.rs — SCANNER YA MILLISECOND (Rust, tokio).
//!
//! Hukagua hosti na bandari NYINGI kwa wakati mmoja kwa kutumia tokio tasks.
//! Muundo: `spawn` ya kazi moja kwa (host, port), semaphore ya concurrency,
//! na timeout ya kila muunganisho. Hii hutumika kama "mikono ya haraka" ya
//! ThreatSwarm — inaweza kuchunguza mamia ya hosti kwa milliseconds.
//!
//! Kanuni:
//!   - **Scope-gated**: hosti LAZIMA ziwe kwenye scope (au `allow_any` iwe `true`
//!     kwa lab pekee). Bila idhini, hukagua loopback pekee.
//!   - Bila shell; ni TCP connect tu (hakuna command injection).
//!   - HITL: matokeo ni ya uchunguzi (read-only) — hakuna mabadiliko ya mfumo.

use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use std::sync::Arc;

/// Mipangilio ya scan.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub concurrency: usize,
    pub timeout_ms: u64,
    /// Ruhusu hosti yoyote (lab pekee). Bila hii, loopback/scope pekee.
    pub allow_any: bool,
    /// Scope: hosti/CIDR zinazoruhusiwa (tupu = loopback pekee).
    pub scope: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            concurrency: 1024,
            timeout_ms: 800,
            allow_any: false,
            scope: Vec::new(),
        }
    }
}

/// Matokeo ya bandari moja.
#[derive(Debug, Clone)]
pub struct PortResult {
    pub host: String,
    pub port: u16,
    pub open: bool,
    pub ms: u128,
    pub error: Option<String>,
}

/// Matokeo ya scan kamili.
#[derive(Debug, Clone)]
pub struct ScanReport {
    pub results: Vec<PortResult>,
    pub hosts_scanned: usize,
    pub ports_per_host: usize,
    pub open_count: usize,
    pub elapsed_ms: u128,
    pub blocked_hosts: Vec<String>,
}

impl ScanReport {
    pub fn to_json(&self) -> Value {
        let open: Vec<Value> = self
            .results
            .iter()
            .filter(|r| r.open)
            .map(|r| json!({ "host": r.host, "port": r.port, "ms": r.ms }))
            .collect();
        json!({
            "ok": true,
            "hosts_scanned": self.hosts_scanned,
            "ports_per_host": self.ports_per_host,
            "open_count": self.open_count,
            "elapsed_ms": self.elapsed_ms,
            "pps": if self.elapsed_ms > 0 {
                ((self.hosts_scanned * self.ports_per_host) as f64 / (self.elapsed_ms as f64 / 1000.0)) as u64
            } else { 0 },
            "open": open,
            "blocked_hosts": self.blocked_hosts,
        })
    }
}

/// Hukagua kama host iko kwenye scope (au ni loopback).
pub fn in_scope(cfg: &ScanConfig, host: &str) -> bool {
    if cfg.allow_any {
        return true;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if ip.is_loopback() {
            return true;
        }
    }
    if host == "localhost" {
        return true;
    }
    for entry in &cfg.scope {
        if entry == host {
            return true;
        }
        // CIDR ya IPv4 rahisi: 10.0.0.0/24
        if let Some((net, bits)) = entry.split_once('/') {
            if let (Ok(net_ip), Ok(bits), Ok(host_ip)) = (
                net.parse::<IpAddr>(),
                bits.parse::<u32>(),
                host.parse::<IpAddr>(),
            ) {
                if cidr_contains(net_ip, bits, host_ip) {
                    return true;
                }
            }
        }
    }
    false
}

fn cidr_contains(net: IpAddr, bits: u32, host: IpAddr) -> bool {
    match (net, host) {
        (IpAddr::V4(n), IpAddr::V4(h)) => {
            if bits > 32 {
                return false;
            }
            let mask: u32 = if bits == 0 { 0 } else { u32::MAX << (32 - bits) };
            (u32::from(n) & mask) == (u32::from(h) & mask)
        }
        _ => false,
    }
}

/// Panua CIDR kuwa hosti (IPv4 pekee, kikomo cha usalama).
pub fn expand_cidr(cidr: &str, max_hosts: usize) -> Vec<String> {
    let mut out = Vec::new();
    let (net, bits) = match cidr.split_once('/') {
        Some(x) => x,
        None => return vec![cidr.to_string()],
    };
    let (net, bits) = match (net.parse::<std::net::Ipv4Addr>(), bits.parse::<u32>()) {
        (Ok(n), Ok(b)) if b <= 32 => (n, b),
        _ => return vec![cidr.to_string()],
    };
    let base = u32::from(net);
    let mask: u32 = if bits == 0 { 0 } else { u32::MAX << (32 - bits) };
    let network = base & mask;
    let count = if bits == 32 { 1u64 } else { 1u64 << (32 - bits) };
    for i in 0..count.min(max_hosts as u64) {
        let ip = std::net::Ipv4Addr::from(network + i as u32);
        out.push(ip.to_string());
    }
    out
}

/// Hukagua hosti nyingi × bandari nyingi kwa wakati mmoja.
pub async fn scan(hosts: &[String], ports: &[u16], cfg: &ScanConfig) -> ScanReport {
    let start = Instant::now();
    let sem = Arc::new(Semaphore::new(cfg.concurrency.max(1)));
    let timeout = Duration::from_millis(cfg.timeout_ms.max(1));

    let mut blocked_hosts = Vec::new();
    let mut resolved: Vec<String> = Vec::new();
    for h in hosts {
        if in_scope(cfg, h) {
            resolved.push(h.clone());
        } else {
            blocked_hosts.push(h.clone());
        }
    }

    let hosts_scanned = resolved.len();
    let ports_per_host = ports.len();
    let mut handles = Vec::new();
    for host in &resolved {
        let host = host.clone();
        for &port in ports {
            let sem = sem.clone();
            let host = host.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok();
                let t = Instant::now();
                let target = host.clone();
                let res = tokio::time::timeout(timeout, async move {
                    // Resolve kwa kutumia SocketAddr (host:port).
                    let addr = format!("{target}:{port}");
                    let resolved = match tokio::net::lookup_host(&addr).await {
                        Ok(it) => it.collect::<Vec<_>>(),
                        Err(e) => return Err(e.to_string()),
                    };
                    match resolved.into_iter().next() {
                        Some(sa) => connect(sa).await,
                        None => Err("DNS haikupata anwani".to_string()),
                    }
                })
                .await;
                let ms = t.elapsed().as_millis();
                match res {
                    Ok(Ok(())) => PortResult { host, port, open: true, ms, error: None },
                    Ok(Err(e)) => PortResult { host, port, open: false, ms, error: Some(e) },
                    Err(_) => PortResult { host, port, open: false, ms, error: Some("timeout".into()) },
                }
            }));
        }
    }

    let mut results = Vec::with_capacity(handles.len());
    for h in handles {
        if let Ok(r) = h.await {
            results.push(r);
        }
    }
    let open_count = results.iter().filter(|r| r.open).count();
    ScanReport {
        results,
        hosts_scanned,
        ports_per_host,
        open_count,
        elapsed_ms: start.elapsed().as_millis(),
        blocked_hosts,
    }
}

async fn connect(sa: SocketAddr) -> Result<(), String> {
    match TcpStream::connect(sa).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_iko_kwenye_scope() {
        let cfg = ScanConfig::default();
        assert!(in_scope(&cfg, "127.0.0.1"));
        assert!(in_scope(&cfg, "localhost"));
        assert!(!in_scope(&cfg, "8.8.8.8"));
    }

    #[test]
    fn cidr_inashirikisha_members() {
        let cfg = ScanConfig { scope: vec!["10.0.0.0/24".into()], ..Default::default() };
        assert!(in_scope(&cfg, "10.0.0.55"));
        assert!(!in_scope(&cfg, "10.0.1.5"));
    }

    #[test]
    fn expand_cidr_inapunguza_kwa_kikomo() {
        let v = expand_cidr("10.0.0.0/24", 5);
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], "10.0.0.0");
    }

    #[tokio::test]
    async fn scan_ya_loopback_inafanya_kazi() {
        // Bandari isiyofunguliwa: haitafunguliwa, lakini scan ikamilike.
        let cfg = ScanConfig { concurrency: 16, timeout_ms: 200, ..Default::default() };
        let rep = scan(&["127.0.0.1".into()], &[1], &cfg).await;
        assert_eq!(rep.hosts_scanned, 1);
        assert_eq!(rep.ports_per_host, 1);
    }

    #[test]
    fn hosti_nje_ya_scope_inazuiwa() {
        let cfg = ScanConfig::default();
        let blocked = !in_scope(&cfg, "203.0.113.7");
        assert!(blocked);
    }
}
