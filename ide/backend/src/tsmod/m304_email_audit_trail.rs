//! m304_email_audit_trail.rs — MODULE 304: EMAIL AUDIT TRAIL
//!
//! Spec: "Inaweka rekodi ya shughuli zote za barua pepe. Twist: Inaweka kwenye blockchain."
//!
//! Injini halisi: hurekodi kila tukio la barua pepe (send/receive/open/delete) kwa
//! hash-chain inayoweza kuthibitishwa — audit trail isiyobadilika.

use super::common_smsemail::chain_hash;
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Tukio moja la audit.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub seq: u64,
    pub action: String,
    pub actor: String,
    pub msg_id: String,
    pub ts: i64,
    pub prev: String,
    pub hash: String,
}

pub fn log_event(chain: &mut Vec<AuditEntry>, action: &str, actor: &str, msg_id: &str, ts: i64) -> AuditEntry {
    let prev = chain.last().map(|e| e.hash.clone()).unwrap_or_else(|| "GENESIS".into());
    let payload = format!("{action}|{actor}|{msg_id}|{ts}");
    let hash = chain_hash(&prev, &payload);
    let e = AuditEntry { seq: chain.len() as u64, action: action.into(), actor: actor.into(), msg_id: msg_id.into(), ts, prev, hash };
    chain.push(e.clone());
    e
}

pub fn verify(chain: &[AuditEntry]) -> bool {
    let mut prev = "GENESIS".to_string();
    for e in chain {
        if e.prev != prev {
            return false;
        }
        let expect = chain_hash(&prev, &format!("{}|{}|{}|{}", e.action, e.actor, e.msg_id, e.ts));
        if expect != e.hash {
            return false;
        }
        prev = e.hash.clone();
    }
    true
}

pub struct M304EmailAuditTrail;

impl TsModule for M304EmailAuditTrail {
    fn id(&self) -> u16 {
        304
    }
    fn name(&self) -> &'static str {
        "Email Audit Trail"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Hurekodi shughuli zote za barua pepe kwenye hash-chain inayothibitishika \
         (audit trail isiyobadilika)."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(304, self.name());
        let mut chain: Vec<AuditEntry> = Vec::new();
        log_event(&mut chain, "receive", "juma@tz", "msg-001", 1_700_000_000);
        log_event(&mut chain, "open", "juma@tz", "msg-001", 1_700_000_050);
        log_event(&mut chain, "send", "juma@tz", "msg-002", 1_700_000_100);
        log_event(&mut chain, "delete", "juma@tz", "msg-002", 1_700_000_200);
        let ok = verify(&chain);
        r.finding("audit_valid", if ok { "Audit trail ni sahihi" } else { "Imevunjika" }, if ok { 1 } else { 95 });
        let mut tampered = chain.clone();
        tampered[2].action = "forward".into();
        let detected = !verify(&tampered);
        r.finding("tamper_detect", if detected { "Kubadilisha kunagundulika" } else { "HAIKUGUNDULIKA" }, if detected { 1 } else { 95 });
        r.findings.push(json!({ "kind": "audit", "severity": 1,
            "detail": { "events": chain.len(), "head": chain.last().map(|e| e.hash.clone()), "storage": "blockchain-ready" } }));
        r.summary = format!("Email Audit Trail: matukio {} yamerekodiwa (mnyororo {ok}).", chain.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_ni_imara() {
        let mut c = Vec::new();
        log_event(&mut c, "send", "a", "m1", 1);
        log_event(&mut c, "open", "a", "m1", 2);
        assert!(verify(&c));
        c[0].actor = "b".into();
        assert!(!verify(&c));
    }
}
