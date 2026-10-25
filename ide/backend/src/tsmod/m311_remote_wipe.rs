//! m311_remote_wipe.rs — MODULE 311: REMOTE WIPE
//!
//! Spec: "Inafuta data zote kwenye kifaa kwa mbali. Twist: Inafuta data kabla ya kuwa mikononi mwa mwizi."
//!
//! Injini halisi: hutengeneza amri ya kufuta data (remote wipe) yenye uthibitisho
//! na "dead-man switch": kifaa kikikosa kuwasiliana kwa muda (au kikiibwa), data
//! hufutwa kabla ya kuwa mikononi mwa mwizi.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Amri ya kufuta.
#[derive(Debug, Clone)]
pub struct WipeCommand {
    pub device_id: String,
    pub scope: String, // "full" | "keys" | "selective"
    pub reason: String,
    pub ts: i64,
    pub sig: String,
}

pub fn issue_wipe(device_id: &str, scope: &str, reason: &str, ts: i64, secret: &[u8]) -> WipeCommand {
    let mut h = Sha256::new();
    h.update(secret);
    h.update(b"|wipe|");
    h.update(device_id.as_bytes());
    h.update(b"|");
    h.update(scope.as_bytes());
    h.update(b"|");
    h.update(reason.as_bytes());
    h.update(b"|");
    h.update(ts.to_le_bytes());
    let sig: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    WipeCommand { device_id: device_id.into(), scope: scope.into(), reason: reason.into(), ts, sig }
}

pub fn verify_wipe(c: &WipeCommand, secret: &[u8]) -> bool {
    issue_wipe(&c.device_id, &c.scope, &c.reason, c.ts, secret).sig == c.sig
}

/// Dead-man switch: kifaa kikikosa kuwasiliana > threshold, futa.
pub fn deadman_trigger(last_seen: i64, now: i64, threshold_secs: i64) -> bool {
    now - last_seen > threshold_secs
}

pub struct M311RemoteWipe;

impl TsModule for M311RemoteWipe {
    fn id(&self) -> u16 {
        311
    }
    fn name(&self) -> &'static str {
        "Remote Wipe"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Amri ya kufuta data kwa mbali yenye uthibitisho + dead-man switch: kifaa \
         kikikosa kuwasiliana, data hufutwa kabla ya kufikia mwizi."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(311, self.name());
        let dev = p.get("device_id").and_then(|v| v.as_str()).unwrap_or("DEV-001");
        let scope = p.get("scope").and_then(|v| v.as_str()).unwrap_or("full");
        let reason = p.get("reason").and_then(|v| v.as_str()).unwrap_or("imeibwa");
        let ts = p.get("ts").and_then(|v| v.as_i64()).unwrap_or(1_700_000_000);
        let last_seen = p.get("last_seen").and_then(|v| v.as_i64()).unwrap_or(ts);
        let threshold = p.get("deadman_secs").and_then(|v| v.as_i64()).unwrap_or(86_400);
        let secret = b"fundi-wipe-secret";
        let cmd = issue_wipe(dev, scope, reason, ts, secret);
        let ok = verify_wipe(&cmd, secret);
        let deadman = deadman_trigger(last_seen, ts, threshold);
        r.findings.push(json!({ "kind": "wipe_command", "severity": 1,
            "detail": { "device": cmd.device_id, "scope": cmd.scope, "reason": cmd.reason, "sig": &cmd.sig[..16] } }));
        r.findings.push(json!({ "kind": "deadman", "severity": if deadman { 80 } else { 1 },
            "detail": { "triggered": deadman, "last_seen": last_seen, "threshold_secs": threshold } }));
        // Wipe inatekelezwa ikiwa amri ni halali NA (imeombwa au dead-man imeamka).
        let execute = ok && (reason == "imeibwa" || deadman);
        r.findings.push(json!({ "kind": "decision", "severity": if execute { 90 } else { 1 },
            "detail": { "execute_wipe": execute, "signed": ok } }));
        r.summary = format!("Remote Wipe: {dev} scope {scope} — {} (dead-man {}).", if execute { "TAFSIRI: futa" } else { "hakuna" }, deadman);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipe_ina_uthibitisho() {
        let c = issue_wipe("D1", "full", "imeibwa", 100, b"k");
        assert!(verify_wipe(&c, b"k"));
        assert!(!verify_wipe(&c, b"x"));
    }

    #[test]
    fn deadman_inamka() {
        assert!(deadman_trigger(0, 100_000, 86_400));
        assert!(!deadman_trigger(99_000, 100_000, 86_400));
    }
}
