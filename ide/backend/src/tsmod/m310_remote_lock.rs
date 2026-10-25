//! m310_remote_lock.rs — MODULE 310: REMOTE LOCK
//!
//! Spec: "Inafungia kifaa kwa mbali ikiwa imeibwa. Twist: Inaweza kufungia kwa amri ya admin."
//!
//! Injini halisi: hutengeneza amri ya kufungia kifaa (remote lock) yenye
//! uthibitisho (HMAC) ili kifaa kikubali amri halali pekee; inaweza kufungwa kwa
//! amri ya admin (HITL) au kiotomatiki kifaa kikiibwa.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Amri ya kufungia.
#[derive(Debug, Clone)]
pub struct LockCommand {
    pub device_id: String,
    pub issued_by: String,
    pub reason: String,
    pub ts: i64,
    pub sig: String,
}

/// Tengeneza amri ya kufungia yenye saini.
pub fn issue_lock(device_id: &str, issued_by: &str, reason: &str, ts: i64, secret: &[u8]) -> LockCommand {
    let mut h = Sha256::new();
    h.update(secret);
    h.update(b"|lock|");
    h.update(device_id.as_bytes());
    h.update(b"|");
    h.update(issued_by.as_bytes());
    h.update(b"|");
    h.update(reason.as_bytes());
    h.update(b"|");
    h.update(ts.to_le_bytes());
    let sig: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    LockCommand { device_id: device_id.into(), issued_by: issued_by.into(), reason: reason.into(), ts, sig }
}

/// Thibitisha amri.
pub fn verify_lock(c: &LockCommand, secret: &[u8]) -> bool {
    issue_lock(&c.device_id, &c.issued_by, &c.reason, c.ts, secret).sig == c.sig
}

pub struct M310RemoteLock;

impl TsModule for M310RemoteLock {
    fn id(&self) -> u16 {
        310
    }
    fn name(&self) -> &'static str {
        "Remote Lock"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hutengeneza amri ya kufungia kifaa kwa mbali yenye uthibitisho (HMAC) — \
         kifaa hukubali amri halali pekee (admin/HITL)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(310, self.name());
        let dev = p.get("device_id").and_then(|v| v.as_str()).unwrap_or("DEV-001");
        let by = p.get("issued_by").and_then(|v| v.as_str()).unwrap_or("admin");
        let reason = p.get("reason").and_then(|v| v.as_str()).unwrap_or("imeibwa");
        let ts = p.get("ts").and_then(|v| v.as_i64()).unwrap_or(1_700_000_000);
        let secret = b"fundi-lock-secret";
        let cmd = issue_lock(dev, by, reason, ts, secret);
        let ok = verify_lock(&cmd, secret);
        r.findings.push(json!({ "kind": "lock_command", "severity": 1,
            "detail": { "device": cmd.device_id, "issued_by": cmd.issued_by, "reason": cmd.reason, "sig": &cmd.sig[..16] } }));
        // Amri ya kubadilishwa inakataliwa.
        let mut bad = cmd.clone();
        bad.reason = "haibwi".into();
        let rejected = !verify_lock(&bad, secret);
        r.findings.push(json!({ "kind": "auth", "severity": if ok && rejected { 1 } else { 90 },
            "detail": { "valid_accepted": ok, "tampered_rejected": rejected } }));
        r.summary = format!("Remote Lock: amri kwa {dev} ({}) — saini {}.", reason, if ok { "sahihi" } else { "mbaya" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amri_halali_na_bandia() {
        let c = issue_lock("D1", "admin", "imeibwa", 100, b"k");
        assert!(verify_lock(&c, b"k"));
        assert!(!verify_lock(&c, b"nyingine"));
    }
}
