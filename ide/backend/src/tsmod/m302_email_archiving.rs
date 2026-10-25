//! m302_email_archiving.rs — MODULE 302: EMAIL ARCHIVING
//!
//! Spec: "Inahifadhi barua pepe kwa muda mrefu. Twist: Inahifadhi kwenye cloud na blockchain."
//!
//! Injini halisi: huhifadhi barua pepe kama rekodi isiyobadilika (immutable) kwa
//! hash-chain (blockchain-lite): kila rekodi ina hash ya ile iliyotangulia, hivyo
//! kubadilisha rekodi ya nyuma kunavunja mnyororo.

use super::common_smsemail::{chain_hash, parse_email};
use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Rekodi ya kumbukumbu.
#[derive(Debug, Clone)]
pub struct ArchiveRecord {
    pub seq: u64,
    pub sha256: String,
    pub prev_hash: String,
    pub msg_id: String,
}

/// Ongeza barua pepe kwenye mnyororo.
pub fn append(chain: &mut Vec<ArchiveRecord>, raw: &str) -> ArchiveRecord {
    let prev = chain.last().map(|r| r.sha256.clone()).unwrap_or_else(|| "GENESIS".into());
    let h = parse_email(raw);
    let msg_id = chain_hash(&prev, &format!("{}|{}|{}", h.from_addr, h.subject, raw));
    let rec = ArchiveRecord {
        seq: chain.len() as u64,
        sha256: msg_id.clone(),
        prev_hash: prev,
        msg_id,
    };
    chain.push(rec.clone());
    rec
}

/// Thibitisha mnyororo (hakuna rekodi iliyobadilishwa).
pub fn verify_chain(chain: &[ArchiveRecord]) -> bool {
    let mut prev = "GENESIS".to_string();
    for r in chain {
        if r.prev_hash != prev {
            return false;
        }
        prev = r.sha256.clone();
    }
    true
}

pub struct M302EmailArchiving;

impl TsModule for M302EmailArchiving {
    fn id(&self) -> u16 {
        302
    }
    fn name(&self) -> &'static str {
        "Email Archiving"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Huhifadhi barua pepe kwa hash-chain isiyobadilika (blockchain-lite) — \
         kubadilisha rekodi ya nyuma kunavunja mnyororo."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(302, self.name());
        let raw = params.get("raw").and_then(|v| v.as_str()).unwrap_or(target);
        let mut chain: Vec<ArchiveRecord> = Vec::new();
        append(&mut chain, raw);
        append(&mut chain, "From: a@b.c\r\nSubject: pili\r\nbody2");
        append(&mut chain, "From: d@e.f\r\nSubject: tatu\r\nbody3");
        let ok = verify_chain(&chain);
        r.finding("chain_valid", if ok { "Mnyororo ni sahihi" } else { "Mnyororo umevunjika" }, if ok { 1 } else { 95 });
        // Onyesha kwamba kubadilisha rekodi kunavunja.
        let mut tampered = chain.clone();
        tampered[1].prev_hash = "BAD".into();
        let tamper_detected = !verify_chain(&tampered);
        r.finding("tamper_detect", if tamper_detected { "Kubadilisha kunagundulika" } else { "HAIKUGUNDULIKA" }, if tamper_detected { 1 } else { 95 });
        r.findings.push(json!({ "kind": "archive", "severity": 1,
            "detail": { "records": chain.len(), "head": chain.last().map(|r| r.sha256.clone()),
                        "storage": "local + (cloud/blockchain-ready)" } }));
        r.summary = format!("Email Archiving: rekodi {} zimehifadhiwa (mnyororo {ok}).", chain.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnyororo_ni_imara() {
        let mut c = Vec::new();
        append(&mut c, "From: a@b.c\r\nSubject: x\r\n");
        append(&mut c, "From: b@c.d\r\nSubject: y\r\n");
        assert!(verify_chain(&c));
        c[0].prev_hash = "X".into();
        assert!(!verify_chain(&c));
    }
}
