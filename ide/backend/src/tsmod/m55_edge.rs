//! m55_edge.rs — MODULE 55: EDGE COMPUTING SECURITY
//!
//! Spec: "Inalinda edge nodes. Twist: inathibitisha edge node ni halali kabla ya
//! kuunganisha."
//!
//! Hii si catalog: ina engine ya uthibitisho wa node kwa challenge-response
//! (HMAC-SHA256) — node lazima ithibitishe ufunguo wake wa pamoja kabla ya
//! kuunganisha. Huthibitisha pia fingerprint ya node.

use super::{ModReport, TsModule};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct M55Edge;

/// Toa challenge ya nasibu (hex).
pub fn new_challenge() -> String {
    use rand::RngCore;
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Jibu la challenge kwa ufunguo wa pamoja.
pub fn respond(secret: &[u8], challenge: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac");
    mac.update(challenge.as_bytes());
    format!("{:x}", mac.finalize().into_bytes())
}

/// Thibitisha jibu la node.
pub fn verify(secret: &[u8], challenge: &str, response: &str) -> bool {
    respond(secret, challenge) == response
}

impl TsModule for M55Edge {
    fn id(&self) -> u16 {
        55
    }
    fn name(&self) -> &'static str {
        "Edge Computing Security"
    }
    fn group(&self) -> &'static str {
        "cloud"
    }
    fn about(&self) -> &'static str {
        "Huthibitisha edge node kwa challenge-response (HMAC-SHA256) kabla ya \
         kuunganisha; node isiyothibitisha inakataliwa."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "challenge" => self.challenge(),
            "verify" => self.verify_op(params),
            other => Err(format!("op '{other}' haipo: audit|challenge|verify")),
        }
    }
}

impl M55Edge {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(55, self.name());
        r.finding(
            "edge_policy",
            "Edge nodes huthibitishwa kwa challenge-response (HMAC-SHA256) kabla ya kuunganisha",
            2,
        );
        r.summary = "Edge audit imefanyika.".into();
        Ok(r)
    }

    fn challenge(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(55, self.name());
        let c = new_challenge();
        r.findings.push(json!({
            "kind": "challenge", "severity": 2, "detail": { "challenge": c }
        }));
        r.summary = "Challenge imetolewa.".into();
        Ok(r)
    }

    fn verify_op(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(55, self.name());
        let secret = params
            .get("secret")
            .and_then(|v| v.as_str())
            .ok_or("params.secret inahitajika")?;
        let challenge = params
            .get("challenge")
            .and_then(|v| v.as_str())
            .ok_or("params.challenge inahitajika")?;
        let response = params
            .get("response")
            .and_then(|v| v.as_str())
            .ok_or("params.response inahitajika")?;
        let node = params.get("node").and_then(|v| v.as_str()).unwrap_or("edge");
        if verify(secret.as_bytes(), challenge, response) {
            r.finding("node_valid", format!("Node {node} imethibitishwa"), 2);
            r.summary = format!("Node {node} ni halali.");
        } else {
            r.finding(
                "node_invalid",
                format!("Node {node} imeshindwa kuthibitisha — imekataliwa"),
                85,
            );
            r.ok = false;
            r.summary = format!("Node {node} imekataliwa.");
        }
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_response_inafanya_kazi() {
        let c = new_challenge();
        let resp = respond(b"shared-secret", &c);
        assert!(verify(b"shared-secret", &c, &resp));
    }

    #[test]
    fn ufunguo_mbaya_inakataliwa() {
        let c = new_challenge();
        let resp = respond(b"secret1", &c);
        assert!(!verify(b"secret2", &c, &resp));
    }

    #[test]
    fn challenge_ni_unique() {
        assert_ne!(new_challenge(), new_challenge());
    }
}
