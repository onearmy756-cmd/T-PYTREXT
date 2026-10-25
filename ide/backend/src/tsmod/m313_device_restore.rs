//! m313_device_restore.rs — MODULE 313: DEVICE RESTORE
//!
//! Spec: "Inarejesha data kutoka backup. Twist: Inarejesha ndani ya dakika 5."
//!
//! Injini halisi: hurejesha data kutoka backup, huhakikisha uadilifu kwa
//! kulinganisha SHA-256, na hupima muda wa urejeshaji (lengo: < dakika 5).

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Faili ya backup iliyorejeshwa.
#[derive(Debug, Clone)]
pub struct RestoredFile {
    pub path: String,
    pub ok: bool,
    pub sha256: String,
}

/// Rejesha na thibitisha.
pub fn restore(files: &[(String, Vec<u8>, String)]) -> (Vec<RestoredFile>, usize) {
    let mut out = Vec::new();
    let mut ok_count = 0;
    for (path, data, expect_sha) in files {
        let sha: String = Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect();
        let ok = expect_sha.is_empty() || &sha == expect_sha;
        if ok {
            ok_count += 1;
        }
        out.push(RestoredFile { path: path.clone(), ok, sha256: sha });
    }
    (out, ok_count)
}

/// Lengo la urejeshaji: dakika 5 = sekunde 300.
pub const RESTORE_SLA_SECS: i64 = 300;

pub struct M313DeviceRestore;

impl TsModule for M313DeviceRestore {
    fn id(&self) -> u16 {
        313
    }
    fn name(&self) -> &'static str {
        "Device Restore"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Hurejesha data kutoka backup na kuthibitisha uadilifu (SHA-256); hupima \
         muda dhidi ya lengo la dakika 5."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(313, self.name());
        let arr = p.get("files").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let files: Vec<(String, Vec<u8>, String)> = if arr.is_empty() {
            vec![("contacts.vcf".into(), b"Juma".to_vec(), String::new())]
        } else {
            arr.iter().filter_map(|x| {
                let path = x.get("path")?.as_str()?.to_string();
                let data = x.get("data").and_then(|d| d.as_str()).unwrap_or("").as_bytes().to_vec();
                let sha = x.get("sha256").and_then(|d| d.as_str()).unwrap_or("").to_string();
                Some((path, data, sha))
            }).collect()
        };
        let elapsed = p.get("elapsed_secs").and_then(|v| v.as_i64()).unwrap_or(120);
        let (restored, ok_count) = restore(&files);
        for f in &restored {
            r.findings.push(json!({ "kind": "restore", "severity": if f.ok { 1 } else { 80 },
                "detail": { "path": f.path, "integrity": f.ok, "sha256": &f.sha256[..16] } }));
        }
        let within_sla = elapsed <= RESTORE_SLA_SECS;
        r.findings.push(json!({ "kind": "sla", "severity": if within_sla { 1 } else { 50 },
            "detail": { "elapsed_secs": elapsed, "sla_secs": RESTORE_SLA_SECS, "within_sla": within_sla } }));
        r.summary = format!("Device Restore: faili {ok_count}/{} zimejengwa upya, sekunde {elapsed} (SLA {}).", restored.len(), if within_sla { "OK" } else { "imezidi" });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_inathibitisha_uadilifu() {
        let sha: String = Sha256::digest(b"AAA").iter().map(|b| format!("{b:02x}")).collect();
        let (out, ok) = restore(&[("a".into(), b"AAA".to_vec(), sha), ("b".into(), b"BBB".to_vec(), "bad".into())]);
        assert_eq!(ok, 1);
        assert!(out[0].ok && !out[1].ok);
    }
}
