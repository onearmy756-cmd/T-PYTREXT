//! vpn.rs — WIREGUARD VPN: safari moja salama ya kazi ZOTE za mbali.
//!
//! Mahitaji ya mtumiaji: "kazi zote zitafanyika remote kwa kutumia vpn ya
//! wireguard" — kwa hiyo kila kazi ya mbali (discovery, OS install, apps,
//! drivers, ripoti) inaendesha kupitia tunnel ya WireGuard (wg0). Bila VPN:
//! hakuna remote work (policy.block_when_vpn_down).
//!
//! HALISI vs MAABARA (hakuna uongo):
//!   - Keys: `wg genkey`/`wg pubkey` kama zipo; vinginevyo x25519-dalek
//!     (pure Rust) — matokeo yote ni keys halisi za Curve25519.
//!   - Conf: wg0.conf halisi (server + per-peer client conf za kudownload).
//!   - up/down: `wg-quick` halisi — inahitaji NET_ADMIN (docker-compose
//!     tayari ina cap_add + /dev/net/tun). Error halisi inarudishwa.
//!   - Status: `wg show wg0 dump` halisi (handshake, rx, tx kwa peer).
//!   - Scan: TCP connect halisi kwenye subnet ya VPN.
//!
//! Data-driven: config kutoka data/deploy/wireguard.json (+ env overrides).

use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::PathBuf;

// ---------- config (data-driven) ----------

#[derive(Debug, Clone)]
pub struct Config {
    pub interface: String,
    pub subnet: String,    // 10.66.66.0/24
    pub server_ip: String, // 10.66.66.1
    pub port: u16,         // 51820
    pub endpoint: String,  // FUNDI_WG_ENDPOINT (public IP/domain ya server)
    pub dns: String,
    pub mtu: u32,
    pub keepalive: u32,
    pub client_allowed_ips: String,
    pub block_when_vpn_down: bool,
}

fn json_path() -> PathBuf {
    // Kama bundles_path ya remote.rs: repo data dir, au /data (docker)
    let cands = [
        std::env::var("FUNDI_WG_JSON").unwrap_or_default(),
        "../../../data/deploy/wireguard.json".into(),
        "../../data/deploy/wireguard.json".into(),
        "data/deploy/wireguard.json".into(),
        "/data/wireguard.json".into(),
    ];
    for c in cands {
        if !c.is_empty() {
            let p = PathBuf::from(&c);
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from("../../../data/deploy/wireguard.json")
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Self {
            interface: "wg0".into(),
            subnet: "10.66.66.0/24".into(),
            server_ip: "10.66.66.1".into(),
            port: 51820,
            endpoint: std::env::var("FUNDI_WG_ENDPOINT").unwrap_or_default(),
            dns: "10.66.66.1".into(),
            mtu: 1420,
            keepalive: 25,
            client_allowed_ips: "0.0.0.0/0, ::/0".into(),
            block_when_vpn_down: true,
        };
        if let Ok(txt) = std::fs::read_to_string(json_path()) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(x) = v["interface"].as_str() {
                    cfg.interface = x.into();
                }
                if let Some(x) = v["subnet"].as_str() {
                    cfg.subnet = x.into();
                }
                if let Some(x) = v["server_ip"].as_str() {
                    cfg.server_ip = x.into();
                }
                if let Some(x) = v["port"].as_u64() {
                    cfg.port = x as u16;
                }
                if let Some(x) = v["dns"].as_str() {
                    cfg.dns = x.into();
                }
                if let Some(x) = v["mtu"].as_u64() {
                    cfg.mtu = x as u32;
                }
                if let Some(x) = v["keepalive"].as_u64() {
                    cfg.keepalive = x as u32;
                }
                if let Some(x) = v["endpoint"].as_str() {
                    if !x.is_empty() {
                        cfg.endpoint = x.into();
                    }
                }
                if let Some(x) = v["client_allowed_ips"].as_str() {
                    cfg.client_allowed_ips = x.into();
                }
                if let Some(b) = v["policy"]["block_when_vpn_down"].as_bool() {
                    cfg.block_when_vpn_down = b;
                }
            }
        }
        cfg
    }

    /// Base ya subnet (mf 10.66.66) kutoka "a.b.c.d/len"
    pub fn subnet_base(&self) -> (u8, u8, u8) {
        let ip = self.subnet.split('/').next().unwrap_or("10.66.66.0");
        let o: Vec<u8> = ip.split('.').filter_map(|x| x.parse().ok()).collect();
        if o.len() == 4 {
            (o[0], o[1], o[2])
        } else {
            (10, 66, 66)
        }
    }

    pub fn endpoint_or(&self) -> String {
        if self.endpoint.is_empty() {
            "WEKA-FUNDI_WG_ENDPOINT-public-IP-au-domain".into()
        } else {
            self.endpoint.clone()
        }
    }
}

pub fn dir() -> PathBuf {
    PathBuf::from(std::env::var("FUNDI_WG_DIR").unwrap_or_else(|_| "/data/wireguard".into()))
}

// ---------- keys ----------

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn wg_binary_genkey() -> Option<String> {
    let o = std::process::Command::new("wg")
        .arg("genkey")
        .output()
        .ok()?;
    if o.status.success() {
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        None
    }
}

fn wg_binary_pubkey(priv_key: &str) -> Option<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut c = Command::new("wg")
        .arg("pubkey")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    c.stdin.as_mut()?.write_all(priv_key.as_bytes()).ok()?;
    let o = c.wait_with_output().ok()?;
    if o.status.success() {
        Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        None
    }
}

/// Keypair moja ya WireGuard (Curve25519): (private_b64, public_b64).
/// Kwanza `wg genkey` (HALISI ya tool rasmi); kama ipo → fallback x25519-dalek.
pub fn keypair() -> (String, String) {
    if let Some(priv_key) = wg_binary_genkey() {
        if let Some(pub_key) = wg_binary_pubkey(&priv_key) {
            return (priv_key, pub_key);
        }
    }
    let secret = x25519_dalek::StaticSecret::random_from_rng(rand::rngs::OsRng);
    let public = x25519_dalek::PublicKey::from(&secret);
    (b64(&secret.to_bytes()), b64(&public.to_bytes()))
}

fn fingerprint(pub_key: &str) -> String {
    let mut h = Sha256::new();
    h.update(pub_key.as_bytes());
    format!("{:x}", h.finalize()).chars().take(12).collect()
}

// ---------- DB peers ----------

#[derive(Debug, Clone, Serialize)]
pub struct Peer {
    pub name: String,
    pub ip: String,
    pub public_key: String,
    pub created_at: String,
    pub enabled: bool,
    #[serde(skip_serializing)]
    pub private_key: String,
}

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS wg_peers (
            name TEXT PRIMARY KEY,
            ip TEXT NOT NULL,
            public_key TEXT NOT NULL,
            private_key TEXT NOT NULL,
            created_at TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1
        )",
    )
    .execute(db)
    .await;
}

pub async fn list_peers(db: &SqlitePool) -> Vec<Peer> {
    let rows: Vec<(String, String, String, String, String, i64)> = sqlx::query_as(
        "SELECT name, ip, public_key, private_key, created_at, enabled FROM wg_peers ORDER BY created_at",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    rows.into_iter()
        .map(
            |(name, ip, public_key, private_key, created_at, enabled)| Peer {
                name,
                ip,
                public_key,
                private_key,
                created_at,
                enabled: enabled != 0,
            },
        )
        .collect()
}

/// IP ya bure inayofuata kwenye subnet (server_ip = .1 ni ya server)
async fn free_ip(db: &SqlitePool, cfg: &Config) -> Option<String> {
    let (a, b, c) = cfg.subnet_base();
    let rows: Vec<(String,)> = sqlx::query_as("SELECT ip FROM wg_peers")
        .fetch_all(db)
        .await
        .unwrap_or_default();
    let used: Vec<String> = rows.into_iter().map(|r| r.0).collect();
    let mut out = None;
    for i in 2..=254u8 {
        let ip = format!("{a}.{b}.{c}.{i}");
        if ip == cfg.server_ip {
            continue;
        }
        if !used.contains(&ip) {
            out = Some(ip);
            break;
        }
    }
    out
}

pub async fn add_peer(db: &SqlitePool, name: &str, note: &str) -> Result<Peer, String> {
    let cfg = Config::load();
    let name = name.trim();
    if name.is_empty() {
        return Err("Jina la peer ni lazima".into());
    }
    let exists: Option<(String,)> = sqlx::query_as("SELECT name FROM wg_peers WHERE name = ?")
        .bind(name)
        .fetch_optional(db)
        .await
        .unwrap_or(None);
    if exists.is_some() {
        return Err(format!("Peer '{name}' tayari ipo"));
    }
    let Some(ip) = free_ip(db, &cfg).await else {
        return Err("Subnet ya VPN imejaa (253 peers max)".into());
    };
    let (priv_key, pub_key) = keypair();
    let created = chrono::Local::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO wg_peers (name, ip, public_key, private_key, created_at, enabled) VALUES (?,?,?,?,?,1)",
    )
    .bind(name)
    .bind(&ip)
    .bind(&pub_key)
    .bind(&priv_key)
    .bind(&created)
    .execute(db)
    .await;
    tracing::info!(
        "vpn: peer '{name}' ({ip}) {note} imeongezwa — conf: /api/vpn/peers/{name}/conf"
    );
    Ok(Peer {
        name: name.into(),
        ip,
        public_key: pub_key,
        private_key: priv_key,
        created_at: created,
        enabled: true,
    })
}

pub async fn remove_peer(db: &SqlitePool, name: &str) -> bool {
    sqlx::query("DELETE FROM wg_peers WHERE name = ?")
        .bind(name)
        .execute(db)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

// ---------- config rendering ----------

pub fn server_conf(cfg: &Config, priv_key: &str, peers: &[Peer]) -> String {
    let mut s = format!(
        "[Interface]\nAddress = {}\nListenPort = {}\nPrivateKey = {}\nMTU = {}\n\n",
        cfg.server_ip, cfg.port, priv_key, cfg.mtu
    );
    for p in peers {
        if !p.enabled {
            continue;
        }
        s.push_str(&format!(
            "[Peer]\n# {name}\nPublicKey = {pk}\nAllowedIPs = {ip}/32\nPersistentKeepalive = {ka}\n\n",
            name = p.name,
            pk = p.public_key,
            ip = p.ip,
            ka = cfg.keepalive,
        ));
    }
    s
}

pub fn client_conf(cfg: &Config, peer: &Peer, server_pub: &str) -> String {
    format!(
        "[Interface]\nPrivateKey = {priv}\nAddress = {ip}/32\nDNS = {dns}\nMTU = {mtu}\n\n[Peer]\nPublicKey = {spub}\nEndpoint = {ep}:{port}\nAllowedIPs = {allowed}\nPersistentKeepalive = {ka}\n",
        priv = peer.private_key,
        ip = peer.ip,
        dns = cfg.dns,
        mtu = cfg.mtu,
        spub = server_pub,
        ep = cfg.endpoint_or(),
        port = cfg.port,
        allowed = cfg.client_allowed_ips,
        ka = cfg.keepalive,
    )
}

// ---------- server keys + apply ----------

fn read_key(p: &PathBuf) -> Option<String> {
    std::fs::read_to_string(p)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Server keypair: inahifadhiwa <dir>/server.key (0600) + server.pub — inaendelea kwenye volume.
pub fn server_keys() -> Result<(String, String), String> {
    let d = dir();
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let kp = d.join("server.key");
    let pp = d.join("server.pub");
    if let (Some(pk), Some(_)) = (read_key(&kp), read_key(&pp)) {
        let pub_key = read_key(&pp).unwrap_or_default();
        return Ok((pk, pub_key));
    }
    let (priv_key, pub_key) = keypair();
    std::fs::write(&kp, &priv_key).map_err(|e| e.to_string())?;
    std::fs::write(&pp, &pub_key).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&kp, std::fs::Permissions::from_mode(0o600));
    }
    Ok((priv_key, pub_key))
}

/// Andika wg0.conf halisi (server + peers zote) kwenye <dir>/wg0.conf
pub async fn write_server_conf(db: &SqlitePool) -> Result<PathBuf, String> {
    let cfg = Config::load();
    let (priv_key, _) = server_keys()?;
    let peers = list_peers(db).await;
    let conf = server_conf(&cfg, &priv_key, &peers);
    let path = dir().join(format!("{}.conf", cfg.interface));
    std::fs::write(&path, &conf).map_err(|e| e.to_string())?;
    Ok(path)
}

/// wg-quick up halisi (inahitaji NET_ADMIN + /dev/net/tun + wireguard-tools)
pub async fn up(conf_path: Option<String>) -> Result<String, String> {
    let cfg = Config::load();
    let path = match conf_path {
        Some(p) => PathBuf::from(p),
        None => dir().join(format!("{}.conf", cfg.interface)),
    };
    if !path.is_file() {
        return Err(format!(
            "Conf haipo: {} — ita /api/vpn/init kwanza",
            path.display()
        ));
    }
    let o = tokio::process::Command::new("wg-quick")
        .arg("up")
        .arg(&path)
        .output()
        .await
        .map_err(|e| format!("wg-quick haipatikani ({e}) — install wireguard-tools + NET_ADMIN"))?;
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    if o.status.success() {
        Ok(format!(
            "Tunnel {iface} IMEWASHWA — kazi za mbali sasa zinaenda kupitia VPN.\n{out}",
            iface = cfg.interface
        ))
    } else {
        Err(format!("wg-quick up imeshindikana (halisi):\n{out}"))
    }
}

pub async fn down() -> Result<String, String> {
    let cfg = Config::load();
    let o = tokio::process::Command::new("wg-quick")
        .arg("down")
        .arg(&cfg.interface)
        .output()
        .await
        .map_err(|e| format!("wg-quick haipatikani ({e})"))?;
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    if o.status.success() {
        Ok(format!("Tunnel {iface} IMEZIMWA.", iface = cfg.interface))
    } else {
        Err(format!("wg-quick down imeshindikana (halisi):\n{out}"))
    }
}

// ---------- status (wg show dump — halisi) ----------

#[derive(Debug, Clone, Serialize)]
pub struct WgPeerStatus {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: String,
    pub latest_handshake_unix: i64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub keepalive: Option<u32>,
}

async fn wg_show_dump(iface: &str) -> Result<Vec<WgPeerStatus>, String> {
    let o = tokio::process::Command::new("wg")
        .args(["show", iface, "dump"])
        .output()
        .await
        .map_err(|e| format!("wg haipatikani ({e}) — install wireguard-tools"))?;
    if !o.status.success() {
        return Err(format!(
            "wg show {iface} imeshindikana: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        ));
    }
    let txt = String::from_utf8_lossy(&o.stdout);
    let mut peers = Vec::new();
    for (i, line) in txt.lines().enumerate() {
        if i == 0 {
            continue; // interface line
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 8 {
            continue;
        }
        peers.push(WgPeerStatus {
            public_key: f[0].into(),
            endpoint: if f[2].is_empty() {
                None
            } else {
                Some(f[2].into())
            },
            allowed_ips: f[3].into(),
            latest_handshake_unix: f[4].parse().unwrap_or(0),
            rx_bytes: f[5].parse().unwrap_or(0),
            tx_bytes: f[6].parse().unwrap_or(0),
            keepalive: f[7].parse().ok(),
        });
    }
    Ok(peers)
}

pub async fn status(db: &SqlitePool) -> serde_json::Value {
    let cfg = Config::load();
    let peers = list_peers(db).await;
    let server_keys_ok = read_key(&dir().join("server.key")).is_some();
    let conf_path = dir().join(format!("{}.conf", cfg.interface));
    let (active, wg_error, wg_peers) = match wg_show_dump(&cfg.interface).await {
        Ok(ps) => {
            let now = chrono::Utc::now().timestamp();
            let up = ps
                .iter()
                .filter(|p| now - p.latest_handshake_unix < 180)
                .count();
            (
                true,
                None,
                serde_json::json!({ "total": ps.len(), "handshake_fresh": up }),
            )
        }
        Err(e) => (
            false,
            Some(e),
            serde_json::json!({ "total": 0, "handshake_fresh": 0 }),
        ),
    };
    serde_json::json!({
        "interface": cfg.interface,
        "subnet": cfg.subnet,
        "server_ip": cfg.server_ip,
        "port": cfg.port,
        "endpoint": cfg.endpoint_or(),
        "endpoint_configured": !cfg.endpoint.is_empty(),
        "server_keys": server_keys_ok,
        "conf_file": conf_path.to_string_lossy(),
        "conf_exists": conf_path.is_file(),
        "active": active, // wg show ilifanikiwa = interface up
        "wg_error": wg_error,
        "peers_db": peers.len(),
        "peers": peers.iter().map(|p| serde_json::json!({
            "name": p.name, "ip": p.ip, "fp": fingerprint(&p.public_key),
            "created_at": p.created_at, "enabled": p.enabled,
        })).collect::<Vec<_>>(),
        "wg_peers_live": wg_peers,
        "policy": {
            "kazi_zote_kwa_vpn": true,
            "block_when_vpn_down": cfg.block_when_vpn_down,
            "note_sw": "Kazi zote za mbali (discovery, OS, apps, drivers, ripoti) ZINAENDA kupitia WireGuard. Bila VPN — remote work haianzi (block_when_vpn_down)."
        }
    })
}

// ---------- discovery juu ya VPN (TCP halisi) ----------

/// Scan subnet ya VPN kwa services za remote (SSH/SMB/RDP) — TCP connect halisi.
pub async fn scan_vpn_subnet(cfg: &Config, timeout_ms: u64) -> Vec<(String, Vec<u16>)> {
    let (a, b, c) = cfg.subnet_base();
    let ports = [22u16, 445, 135, 3389];
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(64));
    let mut handles = Vec::new();
    for i in 1..=254u8 {
        let ip = format!("{a}.{b}.{c}.{i}");
        let permit = sem.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            let mut open = Vec::new();
            for p in ports {
                let fut = tokio::net::TcpStream::connect((
                    std::net::IpAddr::from(std::net::Ipv4Addr::new(a, b, c, i)),
                    p,
                ));
                if tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), fut)
                    .await
                    .map(|r| r.is_ok())
                    .unwrap_or(false)
                {
                    open.push(p);
                }
            }
            drop(permit);
            (ip, open)
        }));
    }
    let mut out = Vec::new();
    for h in handles {
        if let Ok((ip, open)) = h.await {
            if !open.is_empty() {
                out.push((ip, open));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_halisi_au_wg() {
        let (priv_key, pub_key) = keypair();
        assert_eq!(priv_key.len(), 44, "wg/x25519 private key = 44 chars b64");
        assert_eq!(pub_key.len(), 44);
        assert!(priv_key.ends_with('=') && pub_key.ends_with('='));
    }

    #[test]
    fn keypair_unique() {
        let (a, _) = keypair();
        let (b, _) = keypair();
        assert_ne!(
            a, b,
            "keys mbili zinazotolewa mfululizo lazima zitofautiane"
        );
    }

    #[test]
    fn server_conf_ina_peers() {
        let cfg = Config::load();
        let peers = vec![Peer {
            name: "hr".into(),
            ip: "10.66.66.2".into(),
            public_key: "PUB_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
            private_key: String::new(),
            created_at: String::new(),
            enabled: true,
        }];
        let (priv_key, _) = keypair();
        let conf = server_conf(&cfg, &priv_key, &peers);
        assert!(conf.contains("[Interface]"));
        assert!(conf.contains("Address = 10.66.66.1"));
        assert!(conf.contains("ListenPort = 51820"));
        assert!(conf.contains("AllowedIPs = 10.66.66.2/32"));
        assert!(conf.contains("PersistentKeepalive = 25"));
        // Private key ya PEER hairuhusiwi kwenye server conf (ni ya client tu)
        assert!(!conf.contains("PrivateKey = PUB_"));
    }

    #[test]
    fn client_conf_ina_server_pub() {
        let cfg = Config::load();
        let (_, server_pub) = keypair();
        let peer = Peer {
            name: "account".into(),
            ip: "10.66.66.3".into(),
            public_key: String::new(),
            private_key: "PRIV_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
            created_at: String::new(),
            enabled: true,
        };
        let conf = client_conf(&cfg, &peer, &server_pub);
        assert!(conf.contains("Address = 10.66.66.3/32"));
        assert!(conf.contains(&format!("PublicKey = {server_pub}")));
        assert!(conf.contains("AllowedIPs = 0.0.0.0/0"));
    }

    #[test]
    fn subnet_base() {
        assert_eq!(Config::load().subnet_base(), (10, 66, 66));
    }
}
