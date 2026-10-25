//! engine.rs — primitives zinazotumiwa na modules zote: nftables halisi,
//! uainishaji wa IP/CIDR, na utekelezaji wa amri bila shell (arg-array pekee).

use serde_json::Value;
use std::net::{Ipv4Addr, ToSocketAddrs};
use std::process::Command;
use std::sync::OnceLock;

/// Table + set za MTECH zinazotumiwa na firewall (m35) na IPS (m36).
pub const FW_TABLE: &str = "mtech_ts";
pub const FW_SET: &str = "blocked";

/// Script inayohakikisha table/set/chain ya MTECH ipo (idempotent).
pub fn fw_ensure_script() -> String {
    format!(
        "table inet {FW_TABLE} {{\n\
           set {FW_SET} {{ type ipv4_addr; flags interval; comment \"MTECH blocked\"; }}\n\
           chain input {{\n\
             type filter hook input priority 0; policy accept;\n\
             ip saddr @{FW_SET} drop\n\
           }}\n\
         }}\n"
    )
}

/// Zuia IP/CIDR kwenye set ya MTECH (halisi). Huhakikisha table ipo kwanza.
pub fn fw_block(entry: &str) -> Result<(), String> {
    nft_apply(&fw_ensure_script())?;
    nft_apply(&format!("add element inet {FW_TABLE} {FW_SET} {{ {entry} }}\n"))?;
    Ok(())
}

/// Amri za nftables zinaweza kuhitaji root. Tunapima mara moja tu.
static NFT_PREFIX: OnceLock<Vec<&'static str>> = OnceLock::new();

fn nft_prefix() -> &'static [&'static str] {
    NFT_PREFIX.get_or_init(|| {
        // Jaribio la kwanza: nft moja kwa moja (mara nyingi server ni root).
        if Command::new("nft")
            .args(["-j", "list", "ruleset"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return vec!["nft"];
        }
        // Kisha: sudo -n nft (bila prompt).
        vec!["sudo", "-n", "nft"]
    })
}

/// Tekeleza amri ya nft kwa arg-array (HAKUNA shell). Hurudisha stdout.
pub fn nft(args: &[&str]) -> Result<String, String> {
    let prefix = nft_prefix();
    let mut cmd = Command::new(prefix[0]);
    for a in &prefix[1..] {
        cmd.arg(a);
    }
    let out = cmd
        .args(args)
        .output()
        .map_err(|e| format!("nft haikupatikana: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("nft imeshindwa: {:?}", out.status.code())
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Ruleset nzima kama JSON (nft -j list ruleset).
pub fn nft_ruleset() -> Result<Value, String> {
    let s = nft(&["-j", "list", "ruleset"])?;
    serde_json::from_str(&s).map_err(|e| format!("ruleset si JSON sahihi: {e}"))
}

/// Tekeleza script ya nft (transaction moja) kwa stdin — `nft -f -`.
pub fn nft_apply(script: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::Stdio;
    let prefix = nft_prefix();
    let mut cmd = Command::new(prefix[0]);
    for a in &prefix[1..] {
        cmd.arg(a);
    }
    let mut child = cmd
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("nft haikupatikana: {e}"))?;
    child
        .stdin
        .as_mut()
        .ok_or("stdin ya nft haipatikani")?
        .write_all(script.as_bytes())
        .map_err(|e| format!("kuandika kwa nft: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("nft wait: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("nft -f imeshindwa: {:?}", out.status.code())
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Soma sysctl (mf. net.ipv4.conf.all.rp_filter) kutoka /proc/sys.
pub fn read_sysctl(key: &str) -> Option<String> {
    let path = format!("/proc/sys/{}", key.replace('.', "/"));
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Weka sysctl (inahitaji root). Arg-array, hakuna shell.
pub fn write_sysctl(key: &str, val: &str) -> Result<(), String> {
    let path = format!("/proc/sys/{}", key.replace('.', "/"));
    if std::fs::write(&path, val).is_ok() {
        return Ok(());
    }
    let out = Command::new("sudo")
        .args(["-n", "sh", "-c", &format!("echo {val} > {path}")])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("kuweka {key}: {}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

/// Soma faili ya kernel inayohitaji root (mf. /proc/net/nf_conntrack).
/// Hujaribu moja kwa moja kwanza, kisha `sudo -n cat`.
pub fn read_root_file(path: &str) -> Result<String, String> {
    if let Ok(s) = std::fs::read_to_string(path) {
        return Ok(s);
    }
    let out = Command::new("sudo")
        .args(["-n", "cat", path])
        .output()
        .map_err(|e| format!("cat {path}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{path} haisomeki (root inahitajika)"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Bandari za TCP zinazosikiliza sasa (kutoka /proc/net/tcp{,6}).
/// Huhakikisha hardening HAIJIFUNGI kwenye bandari za huduma zetu wenyewe.
pub fn listening_ports() -> Vec<u16> {
    let mut ports = Vec::new();
    for path in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(txt) = std::fs::read_to_string(path) else {
            continue;
        };
        for line in txt.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 4 {
                continue;
            }
            // st == 0A (LISTEN); local_address = HEXIP:HEXPORT
            if cols[3] != "0A" {
                continue;
            }
            if let Some(p) = cols[1].split(':').nth(1) {
                if let Ok(v) = u16::from_str_radix(p, 16) {
                    if v != 0 && !ports.contains(&v) {
                        ports.push(v);
                    }
                }
            }
        }
    }
    ports.sort_unstable();
    ports
}


/// Vipande vya ruleset (array ya objects) kutoka `nft -j`.
pub fn ruleset_objects(rs: &Value) -> Vec<Value> {
    rs.get("nftables")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

// ---------- IP / CIDR ----------

/// Rudisha (network, prefix) kwa CIDR ya IPv4. `10.0.0.5` → /32.
pub fn parse_cidr(s: &str) -> Option<(u32, u8)> {
    let s = s.trim();
    let (ip_s, pfx_s) = match s.split_once('/') {
        Some((a, b)) => (a, Some(b)),
        None => (s, None),
    };
    let ip: Ipv4Addr = ip_s.parse().ok()?;
    let prefix: u8 = match pfx_s {
        Some(p) => p.parse().ok()?,
        None => 32,
    };
    if prefix > 32 {
        return None;
    }
    let bits = u32::from(ip);
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
    Some((bits & mask, prefix))
}

pub fn cidr_contains(net: u32, prefix: u8, ip: u32) -> bool {
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
    (ip & mask) == net
}

/// Tafsiri lengo kuwa IPs halisi (hostname → DNS).
pub fn resolve(target: &str) -> Result<Vec<Ipv4Addr>, String> {
    let host = target.split('/').next().unwrap_or(target).trim();
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Ok(vec![ip]);
    }
    let addrs: Vec<Ipv4Addr> = (host, 0u16)
        .to_socket_addrs()
        .map_err(|e| format!("DNS imeshindwa kwa {host}: {e}"))?
        .filter_map(|a| match a.ip() {
            std::net::IpAddr::V4(v4) => Some(v4),
            _ => None,
        })
        .collect();
    if addrs.is_empty() {
        return Err(format!("{host} haina anwani ya IPv4"));
    }
    Ok(addrs)
}

pub fn valid_target(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 253
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_/:".contains(c))
}

/// Vipimo vya anwani zisizopaswa kutokea kwenye interface ya umma (bogons).
pub const BOGONS: &[(&str, u8)] = &[
    ("0.0.0.0", 8),
    ("10.0.0.0", 8),
    ("100.64.0.0", 10),
    ("127.0.0.0", 8),
    ("169.254.0.0", 16),
    ("172.16.0.0", 12),
    ("192.0.0.0", 24),
    ("192.0.2.0", 24),
    ("192.168.0.0", 16),
    ("198.18.0.0", 15),
    ("198.51.100.0", 24),
    ("203.0.113.0", 24),
    ("224.0.0.0", 4),
    ("240.0.0.0", 4),
];

/// Je, IP ni bogon (haipaswi kuvuka interface ya umma)?
pub fn is_bogon(ip: u32) -> bool {
    BOGONS
        .iter()
        .any(|(s, p)| parse_cidr(&format!("{s}/{p}")).map(|(n, pp)| cidr_contains(n, pp, ip)).unwrap_or(false))
}

/// Tafuta `comment` ndani ya expression ya nft rule (JSON).
pub fn rule_comment(rule: &Value) -> Option<String> {
    find_key(rule, "comment").and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Tafuta dport ndani ya rule (payload match).
pub fn rule_dport(rule: &Value) -> Option<u16> {
    if let Some(v) = find_key(rule, "dport") {
        if let Some(n) = v.as_u64() {
            return Some(n as u16);
        }
    }
    None
}

fn find_key(v: &Value, key: &str) -> Option<Value> {
    match v {
        Value::Object(m) => {
            if let Some(x) = m.get(key) {
                return Some(x.clone());
            }
            for (_k, val) in m {
                if let Some(found) = find_key(val, key) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(a) => a.iter().find_map(|x| find_key(x, key)),
        _ => None,
    }
}

/// Tarakimu salama: h = hash fupi ya kamba (kwa comment za idempotency).
pub fn short_hash(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let d = h.finalize();
    format!("{:x}", &d[..4].iter().fold(0u32, |acc, b| (acc << 8) | *b as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidr_inatambua_mipaka() {
        let (net, p) = parse_cidr("10.0.0.0/24").unwrap();
        assert_eq!(p, 24);
        assert!(cidr_contains(net, p, u32::from(Ipv4Addr::new(10, 0, 0, 5))));
        assert!(!cidr_contains(net, p, u32::from(Ipv4Addr::new(10, 0, 1, 5))));
    }

    #[test]
    fn cidr_bila_prefix_ni_host() {
        let (net, p) = parse_cidr("192.168.1.7").unwrap();
        assert_eq!(p, 32);
        assert_eq!(net, u32::from(Ipv4Addr::new(192, 168, 1, 7)));
    }

    #[test]
    fn cidr_prefix_kubwa_zaidi_ya_32_inakataliwa() {
        assert!(parse_cidr("10.0.0.0/33").is_none());
    }

    #[test]
    fn resolve_ya_ip_direct() {
        let v = resolve("127.0.0.1").unwrap();
        assert_eq!(v, vec![Ipv4Addr::new(127, 0, 0, 1)]);
    }

    #[test]
    fn short_hash_ni_taratibu() {
        assert_eq!(short_hash("abc"), short_hash("abc"));
        assert_ne!(short_hash("abc"), short_hash("abd"));
    }
}
