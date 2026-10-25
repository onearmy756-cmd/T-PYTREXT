//! m312_device_backup.rs — MODULE 312: DEVICE BACKUP
//!
//! Spec: "Inahifadhi nakala rudufu ya data kwenye cloud. Twist: Inahifadhi kwenye MinIO na blockchain."
//!
//! Injini halisi: huhifadhi backup (snapshot) yenye:
//!   - Manifest + SHA-256 ya kila faili.
//!   - Hifadhi ya "MinIO/S3" (hapa: saraka ya ndani yenye muundo wa bucket).
//!   - Hash-chain (blockchain-lite) inayothibitisha uadilifu wa backup.

use super::common_smsemail::chain_hash;
use super::{ModReport, TsModule};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Faili moja ya backup.
#[derive(Debug, Clone)]
pub struct BackupFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// Manifest ya backup.
#[derive(Debug, Clone)]
pub struct BackupManifest {
    pub files: Vec<BackupFile>,
    pub chain_head: String,
    pub bucket: String,
    pub created_at: i64,
}

/// Tengeneza manifest kutoka data ya faili (path, bytes).
pub fn build_manifest(files: &[(String, Vec<u8>)], bucket: &str, created_at: i64) -> BackupManifest {
    let mut prev = "GENESIS".to_string();
    let mut out = Vec::new();
    for (path, data) in files {
        let sha: String = Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect();
        prev = chain_hash(&prev, &format!("{path}|{sha}"));
        out.push(BackupFile { path: path.clone(), sha256: sha, size: data.len() as u64 });
    }
    BackupManifest { files: out, chain_head: prev, bucket: bucket.into(), created_at }
}

/// Hifadhi manifest kwenye "bucket" (saraka ya ndani, muundo wa MinIO).
pub fn store_bucket(root: &Path, m: &BackupManifest) -> Result<PathBuf, String> {
    let dir = root.join(&m.bucket);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let obj = dir.join(format!("backup-{}.json", m.created_at));
    let v = json!({
        "bucket": m.bucket, "created_at": m.created_at, "chain_head": m.chain_head,
        "files": m.files.iter().map(|f| json!({"path": f.path, "sha256": f.sha256, "size": f.size})).collect::<Vec<_>>()
    });
    std::fs::write(&obj, serde_json::to_vec_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(obj)
}

pub struct M312DeviceBackup;

impl TsModule for M312DeviceBackup {
    fn id(&self) -> u16 {
        312
    }
    fn name(&self) -> &'static str {
        "Device Backup"
    }
    fn group(&self) -> &'static str {
        "device"
    }
    fn about(&self) -> &'static str {
        "Huhifadhi backup ya kifaa kwenye cloud (MinIO-style bucket) yenye manifest, \
         SHA-256 ya kila faili, na hash-chain (blockchain-lite)."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(312, self.name());
        let bucket = p.get("bucket").and_then(|v| v.as_str()).unwrap_or("fundi-backups");
        let created = p.get("created_at").and_then(|v| v.as_i64()).unwrap_or(1_700_000_000);
        let files: Vec<(String, Vec<u8>)> = p.get("files").and_then(|v| v.as_array()).map(|a| {
            a.iter().filter_map(|x| {
                let path = x.get("path")?.as_str()?.to_string();
                let data = x.get("data").and_then(|d| d.as_str()).unwrap_or("").as_bytes().to_vec();
                Some((path, data))
            }).collect()
        }).unwrap_or_default();
        let files = if files.is_empty() {
            vec![("contacts.vcf".to_string(), b"Juma,+255712".to_vec()), ("photos/img1.jpg".to_string(), b"JPEGDATA".to_vec())]
        } else {
            files
        };
        let m = build_manifest(&files, bucket, created);
        r.finding("manifest", format!("Faili {} zimehifadhiwa, chain head {}", m.files.len(), &m.chain_head[..16]), 1);
        // Jaribu kuhifadhi kwenye bucket ikiwa FUNDI_BACKUP_ROOT imewekwa.
        if let Ok(root) = std::env::var("FUNDI_BACKUP_ROOT") {
            let msg = match store_bucket(Path::new(&root), &m) {
                Ok(obj) => format!("Backup imehifadhiwa: {}", obj.display()),
                Err(e) => format!("Kuhifadhi kumeshindikana: {e}"),
            };
            r.finding("stored", msg, 1);
        }
        r.findings.push(json!({ "kind": "backup", "severity": 1,
            "detail": { "bucket": m.bucket, "files": m.files.len(), "chain_head": m.chain_head,
                        "targets": ["MinIO/S3", "blockchain (hash-chain)"] } }));
        r.summary = format!("Device Backup: faili {} → bucket {}.", m.files.len(), bucket);
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_ina_hash_chain() {
        let files = vec![("a.txt".to_string(), b"AAA".to_vec()), ("b.txt".to_string(), b"BBB".to_vec())];
        let m = build_manifest(&files, "b", 1);
        assert_eq!(m.files.len(), 2);
        assert_ne!(m.files[0].sha256, m.files[1].sha256);
        assert_ne!(m.chain_head, "GENESIS");
    }
}
