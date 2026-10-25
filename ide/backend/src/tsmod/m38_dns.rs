//! m38_dns.rs — MODULE 38: DNS SECURITY
//!
//! Spec: "Inachuja DNS queries hatari, inatumia DNS-over-HTTPS. Twist: inazuia
//! DNS tunneling na C2 communication."
//!
//! Hii si catalog: inatuma DNS queries HALISI (UDP, protocol imeandikwa hapa)
//! kwa resolvers kadhaa na kulinganisha majibu → kugundua DNS poisoning/hijack.
//! Pia huchanganua conntrack kwa DNS tunneling (server mmoja, queries nyingi)
//! na DNS kwenye bandari isiyo ya 53. Enforcement ni nftables — HITL.

use super::engine::{self, FW_TABLE};
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::Duration;

/// Resolvers za kawaida za kulinganisha.
const RESOLVERS: &[(&str, &str)] = &[
    ("8.8.8.8", "Google"),
    ("1.1.1.1", "Cloudflare"),
    ("9.9.9.9", "Quad9"),
];
/// Kikomo cha queries kwa server mmoja kabla ya kuitwa tunneling.
const TUNNEL_QUERY_THRESHOLD: usize = 50;

pub struct M38Dns;

impl TsModule for M38Dns {
    fn id(&self) -> u16 {
        38
    }
    fn name(&self) -> &'static str {
        "DNS Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Hutuma DNS queries halisi kwa resolvers kadhaa na kulinganisha; hugundua \
         poisoning, DNS tunneling na C2. Enforcement kwa nftables kwa idhini."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        match op {
            "audit" => self.audit(),
            "resolve" => self.resolve(target),
            "tunnels" => self.tunnels(),
            "block" => self.block(target, apply),
            other => Err(format!("op '{other}' haipo: audit|resolve|tunnels|block")),
        }
    }
}

/// Jenga DNS query (A record) kwa jina.
pub fn build_query(id: u16, name: &str) -> Vec<u8> {
    let mut q = Vec::with_capacity(32);
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&0x0100u16.to_be_bytes()); // RD=1
    q.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    q.extend_from_slice(&0u16.to_be_bytes());
    q.extend_from_slice(&0u16.to_be_bytes());
    q.extend_from_slice(&0u16.to_be_bytes());
    for label in name.trim_end_matches('.').split('.') {
        let b = label.as_bytes();
        if b.len() > 63 {
            continue;
        }
        q.push(b.len() as u8);
        q.extend_from_slice(b);
    }
    q.push(0);
    q.extend_from_slice(&1u16.to_be_bytes()); // TYPE A
    q.extend_from_slice(&1u16.to_be_bytes()); // CLASS IN
    q
}

/// Changanua jibu: rudisha IPs za A records.
pub fn parse_answer(buf: &[u8]) -> Vec<String> {
    if buf.len() < 12 {
        return Vec::new();
    }
    let ancount = u16::from_be_bytes([buf[6], buf[7]]) as usize;
    let mut pos = 12;
    // Ruka question
    if !skip_name(buf, &mut pos) {
        return Vec::new();
    }
    pos += 4; // qtype+qclass
    let mut ips = Vec::new();
    for _ in 0..ancount {
        if !skip_name(buf, &mut pos) {
            break;
        }
        if pos + 10 > buf.len() {
            break;
        }
        let rtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
        let rdlen = u16::from_be_bytes([buf[pos + 8], buf[pos + 9]]) as usize;
        pos += 10;
        if pos + rdlen > buf.len() {
            break;
        }
        if rtype == 1 && rdlen == 4 {
            ips.push(format!(
                "{}.{}.{}.{}",
                buf[pos],
                buf[pos + 1],
                buf[pos + 2],
                buf[pos + 3]
            ));
        }
        pos += rdlen;
    }
    ips
}

fn skip_name(buf: &[u8], pos: &mut usize) -> bool {
    loop {
        if *pos >= buf.len() {
            return false;
        }
        let len = buf[*pos] as usize;
        if len == 0 {
            *pos += 1;
            return true;
        }
        if len & 0xc0 == 0xc0 {
            // compression pointer
            *pos += 2;
            return true;
        }
        *pos += 1 + len;
    }
}

/// Uliza resolver mmoja; rudisha IPs (au error).
pub fn query_resolver(resolver: &str, name: &str, timeout_ms: u64) -> Result<Vec<String>, String> {
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_millis(timeout_ms)))
        .map_err(|e| e.to_string())?;
    let id = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
        & 0xffff) as u16;
    let q = build_query(id, name);
    sock.send_to(&q, (resolver, 53)).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 512];
    let (n, _) = sock.recv_from(&mut buf).map_err(|e| e.to_string())?;
    Ok(parse_answer(&buf[..n]))
}

impl M38Dns {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(38, self.name());
        let resolv = std::fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
        let servers: Vec<String> = resolv
            .lines()
            .filter_map(|l| l.trim().strip_prefix("nameserver ").map(|s| s.trim().to_string()))
            .collect();
        if servers.is_empty() {
            r.finding("no_resolv", "/etc/resolv.conf haina nameserver", 20);
        } else {
            r.finding(
                "resolv_conf",
                format!("Resolvers: [{}]", servers.join(", ")),
                2,
            );
            // Onya kama resolver haijulikani (sio ya kawaida).
            let known = ["8.8.8.8", "8.8.4.4", "1.1.1.1", "1.0.0.1", "9.9.9.9", "127.0.0.53"];
            for s in &servers {
                if !known.contains(&s.as_str()) && !s.starts_with("10.") && !s.starts_with("192.168.") {
                    r.finding(
                        "resolver_unknown",
                        format!("Resolver {s} haijulikani — hakikisha ni ya kuaminika"),
                        25,
                    );
                }
            }
        }
        r.summary = format!("Resolvers {} zimechunguzwa.", servers.len());
        Ok(r)
    }

    /// Uliza resolvers zote; linganisha majibu (poisoning detection).
    fn resolve(&self, target: &str) -> Result<ModReport, String> {
        let mut r = ModReport::new(38, self.name());
        if target.trim().is_empty() {
            return Err("target (domain) inahitajika".into());
        }
        let mut answers: HashMap<String, Vec<String>> = HashMap::new();
        for (ip, label) in RESOLVERS {
            match query_resolver(ip, target, 1500) {
                Ok(ips) if !ips.is_empty() => {
                    r.finding(
                        "resolved",
                        format!("{label} ({ip}): {}", ips.join(", ")),
                        1,
                    );
                    answers.insert(label.to_string(), ips);
                }
                Ok(_) => {
                    r.finding("no_answer", format!("{label} ({ip}): hakuna jibu"), 10);
                }
                Err(e) => {
                    r.finding("query_fail", format!("{label} ({ip}): {e}"), 15);
                }
            }
        }
        // Linganisha: sets tofauti = poisoning/hijack.
        let sets: Vec<std::collections::BTreeSet<&String>> =
            answers.values().map(|v| v.iter().collect()).collect();
        let mut inconsistent = false;
        if sets.len() >= 2 {
            for i in 1..sets.len() {
                if sets[i] != sets[0] {
                    inconsistent = true;
                }
            }
        }
        if inconsistent {
            r.finding(
                "dns_poisoning",
                "Resolvers zinarudisha majibu TOFAUTI — dalili ya DNS poisoning/hijack",
                70,
            );
        }
        r.summary = format!(
            "Resolvers {} zimejibu kwa {target}; tofauti: {}.",
            answers.len(),
            if inconsistent { "NDIYO" } else { "hapana" }
        );
        Ok(r)
    }

    /// Tunneling: server moja ya DNS yenye queries nyingi, au DNS kwenye bandari nyingine.
    fn tunnels(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(38, self.name());
        let txt = engine::read_root_file("/proc/net/nf_conntrack")?;
        let mut per_server: HashMap<String, usize> = HashMap::new();
        let mut odd_port: Vec<String> = Vec::new();
        for line in txt.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                continue;
            }
            let mut dst = "";
            let mut dport = 0u16;
            for c in &cols {
                if let Some(v) = c.strip_prefix("dst=") {
                    if dst.is_empty() {
                        dst = v;
                    }
                } else if let Some(v) = c.strip_prefix("dport=") {
                    if dport == 0 {
                        dport = v.parse().unwrap_or(0);
                    }
                }
            }
            if dport == 53 {
                *per_server.entry(dst.to_string()).or_insert(0) += 1;
            } else if (dport == 5353 || dport == 8053) && !dst.is_empty() {
                odd_port.push(format!("{dst}:{dport}"));
            }
        }
        let mut tunnels = 0;
        for (srv, n) in &per_server {
            if *n >= TUNNEL_QUERY_THRESHOLD {
                tunnels += 1;
                r.finding(
                    "dns_tunnel",
                    format!("{srv}: flows {n} kwa bandari 53 (uwezekano wa tunneling)"),
                    55,
                );
            }
        }
        for o in &odd_port {
            r.finding("dns_odd_port", format!("DNS kwenye bandari isiyo ya kawaida: {o}"), 30);
        }
        r.summary = format!(
            "DNS servers {}; tunnels {}; bandari za kawaida {}.",
            per_server.len(),
            tunnels,
            odd_port.len()
        );
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(38, self.name());
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
            r.action("dns_block", format!("{entry} imezuiwa"), true);
            r.summary = format!("DNS server {entry} imezuiwa.");
        } else {
            r.action("dns_block_planned", format!("Itazuiwa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_inajengwa_sahihi() {
        let q = build_query(0x1234, "example.com");
        assert_eq!(&q[0..2], &[0x12, 0x34]);
        assert_eq!(u16::from_be_bytes([q[4], q[5]]), 1); // QDCOUNT
        assert_eq!(*q.last().unwrap(), 1); // QCLASS IN low byte
    }

    #[test]
    fn parse_answer_inarudisha_ip() {
        // Header + question + answer 1.2.3.4
        let mut p = vec![0u8, 1, 0x81, 0x80, 0, 1, 0, 1, 0, 0, 0, 0];
        p.extend_from_slice(&[7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0]);
        p.extend_from_slice(&[0, 1, 0, 1]);
        p.extend_from_slice(&[0xc0, 0x0c]); // pointer
        p.extend_from_slice(&[0, 1, 0, 1]); // type A, class IN
        p.extend_from_slice(&[0, 0, 0, 60]); // ttl
        p.extend_from_slice(&[0, 4]); // rdlength
        p.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(parse_answer(&p), vec!["1.2.3.4"]);
    }

    #[test]
    fn skip_name_inashughulikia_pointer() {
        let buf = vec![0xc0, 0x0c, 0, 1];
        let mut pos = 0;
        assert!(skip_name(&buf, &mut pos));
        assert_eq!(pos, 2);
    }
}
