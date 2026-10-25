//! m39_dhcp.rs — MODULE 39: DHCP SECURITY
//!
//! Spec: "Inalinda dhidi ya DHCP spoofing. Twist: inagundua DHCP server bandia
//! ndani ya sekunde."
//!
//! Hii si catalog: inatuma DHCPDISCOVER halisi kwa broadcast (UDP :67) na
//! kukusanya DHCPOFFER kutoka servers zote. Server zaidi ya moja = rogue
//! DHCP. Pia husoma /etc/dhcp/dhcpd.conf kama ipo. Enforcement kwa nftables — HITL.

use super::engine;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

/// Chapa ya DHCP (magic cookie).
const DHCP_MAGIC: [u8; 4] = [99, 130, 83, 99];

pub struct M39Dhcp;

impl TsModule for M39Dhcp {
    fn id(&self) -> u16 {
        39
    }
    fn name(&self) -> &'static str {
        "DHCP Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Hutuma DHCPDISCOVER halisi kwa broadcast na kugundua DHCP server bandia \
         (rogue) mara moja; huzuia kwa nftables kwa idhini."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        let apply = params.get("apply").and_then(|v| v.as_bool()).unwrap_or(false);
        let wait = params.get("wait_ms").and_then(|v| v.as_u64()).unwrap_or(2000);
        match op {
            "audit" => self.audit(),
            "discover" => self.discover(wait),
            "block" => self.block(target, apply),
            other => Err(format!("op '{other}' haipo: audit|discover|block")),
        }
    }
}

/// Jenga pakiti ya DHCPDISCOVER (BOOTP + option 53=1 + option 55).
pub fn build_discover(xid: u32) -> Vec<u8> {
    let mut p = vec![0u8; 236];
    p[0] = 1; // op = BOOTREQUEST
    p[1] = 1; // htype ethernet
    p[2] = 6; // hlen
    p[3] = 0; // hops
    p[4..8].copy_from_slice(&xid.to_be_bytes());
    // ciaddr/yiaddr/siaddr/giaddr = 0
    // chaddr (MAC ya bandia ya kukusanya majibu tu)
    p[28..34].copy_from_slice(&[0x02, 0x4d, 0x54, 0x53, 0x00, 0x01]);
    p.extend_from_slice(&DHCP_MAGIC);
    p.extend_from_slice(&[53, 1, 1]); // DHCP Message Type = DISCOVER
    p.extend_from_slice(&[55, 3, 1, 3, 6]); // Parameter Request List
    p.push(255); // END
    p
}

/// Changanua DHCPOFFER: rudisha (server_id, yiaddr, subnet_mask).
pub fn parse_offer(buf: &[u8]) -> Option<(Option<String>, String, Option<String>)> {
    if buf.len() < 240 || buf[0] != 2 {
        return None; // BOOTREPLY tu
    }
    if buf[236..240] != DHCP_MAGIC {
        return None;
    }
    let yiaddr = format!("{}.{}.{}.{}", buf[16], buf[17], buf[18], buf[19]);
    let mut server_id = None;
    let mut mask = None;
    let mut i = 240;
    while i < buf.len() {
        let opt = buf[i];
        if opt == 255 {
            break;
        }
        if opt == 0 {
            i += 1;
            continue;
        }
        if i + 1 >= buf.len() {
            break;
        }
        let len = buf[i + 1] as usize;
        if i + 2 + len > buf.len() {
            break;
        }
        let val = &buf[i + 2..i + 2 + len];
        if opt == 54 && len == 4 {
            server_id = Some(format!("{}.{}.{}.{}", val[0], val[1], val[2], val[3]));
        } else if opt == 1 && len == 4 {
            mask = Some(format!("{}.{}.{}.{}", val[0], val[1], val[2], val[3]));
        }
        i += 2 + len;
    }
    Some((server_id, yiaddr, mask))
}

impl M39Dhcp {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(39, self.name());
        let paths = [
            "/etc/dhcp/dhcpd.conf",
            "/etc/dhcpd.conf",
            "/etc/dnsmasq.conf",
        ];
        let mut found = Vec::new();
        for p in paths {
            if let Ok(txt) = std::fs::read_to_string(p) {
                found.push(p.to_string());
                let authoritative = txt.contains("authoritative");
                r.finding(
                    "dhcp_config",
                    format!(
                        "{p}: authoritative={authoritative}, mistari {}",
                        txt.lines().count()
                    ),
                    if authoritative { 5 } else { 20 },
                );
            }
        }
        if found.is_empty() {
            r.finding(
                "no_dhcp_config",
                "Hakuna config ya DHCP server kwenye host hii (client pekee?)",
                10,
            );
        }
        r.summary = format!("Config za DHCP zilizopatikana: {}.", found.len());
        Ok(r)
    }

    /// Ugunduzi halisi: broadcast DHCPDISCOVER, kusanya DHCPOFFER.
    fn discover(&self, wait_ms: u64) -> Result<ModReport, String> {
        let mut r = ModReport::new(39, self.name());
        let sock = UdpSocket::bind("0.0.0.0:68").or_else(|_| UdpSocket::bind("0.0.0.0:0"));
        let sock = sock.map_err(|e| format!("bind DHCP: {e}"))?;
        sock.set_broadcast(true).map_err(|e| e.to_string())?;
        sock.set_read_timeout(Some(Duration::from_millis(300)))
            .map_err(|e| e.to_string())?;
        let xid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(1);
        let pkt = build_discover(xid);
        sock.send_to(&pkt, ("255.255.255.255", 67))
            .map_err(|e| format!("DHCPDISCOVER: {e}"))?;
        let mut servers: BTreeSet<String> = BTreeSet::new();
        let mut offers: Vec<Value> = Vec::new();
        let deadline = Instant::now() + Duration::from_millis(wait_ms);
        let mut buf = [0u8; 1024];
        while Instant::now() < deadline {
            match sock.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if let Some((sid, yiaddr, mask)) = parse_offer(&buf[..n]) {
                        let id = sid.unwrap_or_else(|| from.ip().to_string());
                        servers.insert(id.clone());
                        offers.push(json!({
                            "server_id": id, "offered_ip": yiaddr, "mask": mask,
                            "from": from.ip().to_string()
                        }));
                    }
                }
                Err(_) => {}
            }
        }
        for o in &offers {
            r.findings.push(json!({ "kind": "dhcp_offer", "severity": 5, "detail": o }));
        }
        if servers.len() > 1 {
            r.finding(
                "rogue_dhcp",
                format!(
                    "DHCP servers {} zimejibu — ROGUE DHCP inawezekana: [{}]",
                    servers.len(),
                    servers.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
                85,
            );
        }
        r.summary = format!(
            "DHCP servers {} zimejibu kwa {}ms.",
            servers.len(),
            wait_ms
        );
        Ok(r)
    }

    fn block(&self, target: &str, apply: bool) -> Result<ModReport, String> {
        let mut r = ModReport::new(39, self.name());
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
            r.action("dhcp_block", format!("Rogue DHCP {entry} imezuiwa"), true);
            r.summary = format!("Rogue DHCP server {entry} imezuiwa.");
        } else {
            r.action("dhcp_block_planned", format!("Itazuiwa: {entry}"), true);
            r.summary = format!("Dry-run: {entry} itazuiwa ukiweka apply=true.");
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_inajengwa_sahihi() {
        let p = build_discover(0xdeadbeef);
        assert_eq!(p[0], 1);
        assert_eq!(&p[236..240], &DHCP_MAGIC);
        assert_eq!(*p.last().unwrap(), 255);
    }

    #[test]
    fn offer_inaparsika() {
        let mut p = vec![0u8; 240];
        p[0] = 2;
        p[16..20].copy_from_slice(&[192, 168, 1, 50]);
        p[236..240].copy_from_slice(&DHCP_MAGIC);
        p.extend_from_slice(&[53, 1, 2]); // OFFER
        p.extend_from_slice(&[54, 4, 192, 168, 1, 1]); // server id
        p.extend_from_slice(&[1, 4, 255, 255, 255, 0]); // mask
        p.push(255);
        let (sid, yi, mask) = parse_offer(&p).unwrap();
        assert_eq!(sid.as_deref(), Some("192.168.1.1"));
        assert_eq!(yi, "192.168.1.50");
        assert_eq!(mask.as_deref(), Some("255.255.255.0"));
    }

    #[test]
    fn offer_ya_request_inakataliwa() {
        let mut p = vec![0u8; 240];
        p[0] = 1;
        p[236..240].copy_from_slice(&DHCP_MAGIC);
        assert!(parse_offer(&p).is_none());
    }
}
