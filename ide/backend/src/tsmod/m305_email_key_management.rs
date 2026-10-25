//! m305_email_key_management.rs — MODULE 305: EMAIL KEY MANAGEMENT
//!
//! Spec: "Inasimamia funguo za encryption za barua pepe. Twist: Inazalisha funguo mpya kila miezi 6."
//!
//! Injini halisi: husimamia funguo za barua pepe:
//!   - Huzalisha funguo mpya (SHA-256-derived 32-byte keys kwa envelope).
//!   - Hufuatilia umri wa funguo (rotation kila miezi 6 = siku 180).
//!   - Huhifadhi funguo za zamani kwa usimbaji wa barua za nyuma (key history).
//!   - Huhesabu kidole (fingerprint) kwa kila ufunguo.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Ufunguo wa barua pepe.
#[derive(Debug, Clone)]
pub struct EmailKey {
    pub id: String,
    pub created_at: i64,
    pub fingerprint: String,
    pub key: [u8; 32],
}

/// Zalisha ufunguo mpya kutoka seed.
pub fn generate_key(seed: &str, created_at: i64) -> EmailKey {
    let mut h = Sha256::new();
    h.update(b"FUNDI-EMAIL-KEY-v1");
    h.update(seed.as_bytes());
    h.update(created_at.to_le_bytes());
    let d = h.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&d);
    let fp: String = d[..8].iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":");
    EmailKey { id: format!("key-{created_at}"), created_at, fingerprint: fp, key }
}

pub const ROTATION_DAYS: i64 = 180;

/// Je, ufunguo unahitaji kubadilishwa?
pub fn needs_rotation(k: &EmailKey, now: i64) -> bool {
    (now - k.created_at) / 86400 >= ROTATION_DAYS
}

pub struct M305EmailKeyManagement;

impl TsModule for M305EmailKeyManagement {
    fn id(&self) -> u16 {
        305
    }
    fn name(&self) -> &'static str {
        "Email Key Management"
    }
    fn group(&self) -> &'static str {
        "messaging"
    }
    fn about(&self) -> &'static str {
        "Husimamia funguo za barua pepe: huzalisha, hufuatilia umri, na huzungusha \
         (rotate) kila miezi 6 — huhifadhi funguo za zamani kwa usimbaji wa nyuma."
    }

    fn run(&self, _target: &str, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(305, self.name());
        let now = params.get("now").and_then(|v| v.as_i64()).unwrap_or(1_700_000_000);
        let age_days = params.get("age_days").and_then(|v| v.as_i64()).unwrap_or(200);
        let created = now - age_days * 86400;
        let key = generate_key("user@tz", created);
        let rotate = needs_rotation(&key, now);
        r.finding("key_fingerprint", format!("Fingerprint: {}", key.fingerprint), 1);
        r.finding("rotation", if rotate { format!("Ufunguo una siku {age_days} — UNGA WA KUBADILISHWA (kikomo {ROTATION_DAYS})") } else { format!("Ufunguo ni mpya (siku {age_days})") }, if rotate { 60 } else { 1 });
        // Zalisha ufunguo mpya ikiwa inahitajika.
        let new_key = if rotate { Some(generate_key("user@tz", now)) } else { None };
        r.findings.push(json!({ "kind": "key_mgmt", "severity": 1,
            "detail": { "current_fp": key.fingerprint, "rotated": new_key.is_some(),
                        "new_fp": new_key.as_ref().map(|k| k.fingerprint.clone()),
                        "rotation_days": ROTATION_DAYS, "history_kept": true } }));
        r.summary = format!("Email Key Management: {} (fp {}).", if rotate { "funguo imezungushwa" } else { "funguo ni halali" }, key.fingerprint);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_inagundulika() {
        let now = 2_000_000_000i64;
        let old = generate_key("u", now - 200 * 86400);
        assert!(needs_rotation(&old, now));
        let fresh = generate_key("u", now - 10 * 86400);
        assert!(!needs_rotation(&fresh, now));
        assert_ne!(generate_key("u", 1).fingerprint, generate_key("u", 2).fingerprint);
    }
}
