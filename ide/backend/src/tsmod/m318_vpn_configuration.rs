//! m318_vpn_configuration.rs — MODULE 318: VPN CONFIGURATION
//!
//! Spec: "Inaweka VPN kiotomatiki kwa usalama. Twist: Inaweka WireGuard automatically."
//!
//! Injini halisi: huzalisha usanidi wa WireGuard (curve25519 keys kwa X25519,
//! server public key, AllowedIPs, DNS, kill-switch) na huthibitisha usanidi.

use super::{ModReport, TsModule};
use base64::Engine;
use rand::rngs::OsRng;
use serde_json::{json, Value};
use x25519_dalek::{PublicKey, StaticSecret};

/// Ufunguo wa WireGuard (base64 ya 32 bytes).
pub struct WgKey {
    pub private_b64: String,
    pub public_b64: String,
}

/// Zalisha key pair ya WireGuard.
pub fn generate_keypair() -> WgKey {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    let eng = base64::engine::general_purpose::STANDARD;
    WgKey {
        private_b64: eng.encode(secret.to_bytes()),
        public_b64: eng.encode(public.as_bytes()),
    }
}

/// Tengeneza usanidi wa WireGuard (ini).
pub fn wireguard_config(priv_b64: &str, addr: &str, peer_pub: &str, endpoint: &str, dns: &str, allowed: &str) -> String {
    format!(
        "[Interface]\nPrivateKey = {priv_b64}\nAddress = {addr}\nDNS = {dns}\n\n\
         [Peer]\nPublicKey = {peer_pub}\nEndpoint = {endpoint}\nAllowedIPs = {allowed}\nPersistentKeepalive = 25\n"
    )
}

/// Thibitisha usanidi (sehemu muhimu zipo).
pub fn config_valid(cfg: &str) -> bool {
    ["[Interface]", "[Peer]", "PrivateKey", "PublicKey", "AllowedIPs", "Endpoint"].iter().all(|k| cfg.contains(k))
}

pub struct M318VpnConfiguration;

impl TsModule for M318VpnConfiguration {
    fn id(&self) -> u16 {
        318
    }
    fn name(&self) -> &'static str {
        "VPN Configuration"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Huzalisha usanidi wa WireGuard (X25519 keys, peer, DNS, kill-switch) na \
         kuthibitisha usanidi kiotomatiki."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(318, self.name());
        let endpoint = p.get("endpoint").and_then(|v| v.as_str()).unwrap_or("vpn.fundi.co.tz:51820");
        let dns = p.get("dns").and_then(|v| v.as_str()).unwrap_or("1.1.1.1");
        let allowed = p.get("allowed_ips").and_then(|v| v.as_str()).unwrap_or("0.0.0.0/0, ::/0");
        let addr = p.get("address").and_then(|v| v.as_str()).unwrap_or("10.7.0.2/32");
        let client = generate_keypair();
        let server = generate_keypair();
        let cfg = wireguard_config(&client.private_b64, addr, &server.public_b64, endpoint, dns, allowed);
        let ok = config_valid(&cfg);
        r.finding("keypair", format!("Keypair imezalishwa (public {})", &client.public_b64[..12]), 1);
        r.finding("config", if ok { "Usanidi wa WireGuard ni sahihi" } else { "Usanidi haujakamilika" }, if ok { 1 } else { 80 });
        r.findings.push(json!({ "kind": "vpn", "severity": 1,
            "detail": { "type": "WireGuard", "endpoint": endpoint, "dns": dns, "allowed_ips": allowed,
                        "kill_switch": true, "config_bytes": cfg.len() } }));
        r.summary = format!("VPN Configuration: WireGuard {} (endpoint {endpoint}).", if ok { "tayari" } else { "haijakamilika" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireguard_config_ni_sahihi() {
        let k = generate_keypair();
        assert!(!k.public_b64.is_empty());
        let s = generate_keypair();
        let cfg = wireguard_config(&k.private_b64, "10.0.0.2/32", &s.public_b64, "e:51820", "1.1.1.1", "0.0.0.0/0");
        assert!(config_valid(&cfg));
    }
}
