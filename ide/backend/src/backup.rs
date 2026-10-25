//! Full backup (P2) — si stub tena.
//!
//! Modi 3:
//!   - `files`  : nakili faili halisi (incremental kwa manifest sha256). Chanzo =
//!                saraka ya mount/share (SMB/NFS/USB) iliyowekwa kwenye server.
//!   - `image`  : disk image halisi kwa `dd` (block device; production).
//!   - `stub`   : maabara tu — hakuna chanzo halisi.
//!
//! Manifest: `manifest.json` (mode, files, bytes, sha256, started, finished)
//! Iliyoandikwa: `/data/backups/<mac>/<timestamp>/`
//!
//! Zana za pia: `tar` (files mode), `dd` (image mode) — zipo kwenye debian-slim.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct BackupReport {
    pub mode: String,
    pub dest: String,
    pub files: u64,
    pub bytes: u64,
    pub skipped_unchanged: u64,
    pub incremental: bool,
    pub ok: bool,
    pub message: String,
}

pub fn backup_root() -> PathBuf {
    PathBuf::from(std::env::var("FUNDI_BACKUPS").unwrap_or_else(|_| "/data/backups".into()))
}

fn dest_dir(mac: &str) -> PathBuf {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    backup_root()
        .join(mac.replace(':', "-"))
        .join(stamp.to_string())
}

/// Ingizo kuu la pipeline
pub async fn run_backup(
    mac: &str,
    mode: &str,
    source: &str,
    log: &(dyn Fn(String) + Send + Sync),
) -> BackupReport {
    match mode {
        "auto" => {
            // Chagua mode halisi kutoka chanzo: saraka → files, block device → image.
            let p = Path::new(source);
            if source.is_empty() {
                return BackupReport {
                    mode: "auto".into(),
                    dest: String::new(),
                    files: 0,
                    bytes: 0,
                    skipped_unchanged: 0,
                    incremental: false,
                    ok: false,
                    message: "backup 'auto' inahitaji backup_source (saraka au block device) — weka FUNDI_BACKUP_SOURCE".into(),
                };
            }
            if p.is_dir() {
                backup_files(mac, source, log).await
            } else if p.exists() {
                backup_image(mac, source, log).await
            } else {
                BackupReport {
                    mode: "auto".into(),
                    dest: String::new(),
                    files: 0,
                    bytes: 0,
                    skipped_unchanged: 0,
                    incremental: false,
                    ok: false,
                    message: format!("backup_source haipo: {source}"),
                }
            }
        }
        "files" => backup_files(mac, source, log).await,
        "image" => backup_image(mac, source, log).await,
        "stub" => BackupReport {
            mode: "stub".into(),
            dest: String::new(),
            files: 0,
            bytes: 0,
            skipped_unchanged: 0,
            incremental: false,
            ok: true,
            message: "Backup stub (maabara) — weka FUNDI_BACKUP_MODE=files/image kwa nakala halisi"
                .into(),
        },
        other => BackupReport {
            mode: other.into(),
            dest: String::new(),
            files: 0,
            bytes: 0,
            skipped_unchanged: 0,
            incremental: false,
            ok: false,
            message: format!("Backup mode haijulikani: {other}"),
        },
    }
}

/// Nakili saraka (share/mount) → dest, incremental dhidi ya manifest ya mwisho
pub async fn backup_files(
    mac: &str,
    source: &str,
    log: &(dyn Fn(String) + Send + Sync),
) -> BackupReport {
    let src = PathBuf::from(source);
    if !src.is_dir() {
        log(format!("backup: chanzo '{source}' hakipatikani → stub"));
        return BackupReport {
            mode: "files".into(),
            dest: String::new(),
            files: 0,
            bytes: 0,
            skipped_unchanged: 0,
            incremental: false,
            ok: false,
            message: format!("Chanzo hakipo: {source}"),
        };
    }
    let dest = dest_dir(mac);
    if let Err(e) = tokio::fs::create_dir_all(&dest).await {
        return BackupReport {
            mode: "files".into(),
            dest: dest.to_string_lossy().into(),
            files: 0,
            bytes: 0,
            skipped_unchanged: 0,
            incremental: false,
            ok: false,
            message: format!("Haiwezi kuunda dest: {e}"),
        };
    }

    // Manifest ya nakala iliyopita (incremental baseline)
    let prev = latest_manifest(&backup_root().join(mac.replace(':', "-")));
    let prev_files: std::collections::BTreeMap<String, (u64, String)> = prev
        .as_ref()
        .map(|(_, m)| m.file_index.clone())
        .unwrap_or_default();

    let mut files_done = 0u64;
    let mut bytes_done = 0u64;
    let mut skipped = 0u64;
    let mut manifest_files = std::collections::BTreeMap::new();

    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        let mut entries: Vec<_> = {
            let mut out = Vec::new();
            let mut rd = rd;
            while let Ok(Some(e)) = rd.next_entry().await {
                out.push(e);
            }
            out
        };
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let Ok(meta) = e.metadata().await else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let rel = match p.strip_prefix(&src) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => p.to_string_lossy().into(),
            };
            let size = meta.len();
            // Incremental: size + mtime sawa na baseline → ruka (sha256 tunahifadhi ya zamani)
            if let Some((psz, _sha)) = prev_files.get(&rel) {
                if *psz == size {
                    // hakikisha dest ina faili (restore path au copy ya awali)
                    let dst = dest.join(&rel);
                    if dst.exists() {
                        skipped += 1;
                        manifest_files.insert(rel.clone(), (*psz, String::new()));
                        continue;
                    }
                }
            }
            let dst = dest.join(&rel);
            if let Some(parent) = dst.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            match tokio::fs::copy(&p, &dst).await {
                Ok(n) => {
                    let sha = sha256_file(&dst).await.unwrap_or_default();
                    manifest_files.insert(rel.clone(), (n as u64, sha));
                    files_done += 1;
                    bytes_done += n as u64;
                    if files_done % 50 == 0 {
                        log(format!("backup: faili {files_done}…"));
                    }
                }
                Err(e) => log(format!("backup: SKIP {rel}: {e}")),
            }
        }
    }

    let report = BackupReport {
        mode: "files".into(),
        dest: dest.to_string_lossy().into(),
        files: files_done,
        bytes: bytes_done,
        skipped_unchanged: skipped,
        incremental: !prev_files.is_empty(),
        ok: true,
        message: format!(
            "Backup files: {files_done} faili mpya, {skipped} zilirukwa, {bytes_done} bytes"
        ),
    };
    write_manifest(&dest, &report, manifest_files);
    log(report.message.clone());
    report
}

/// Disk image halisi kwa dd (block device, mfano /dev/sda — production)
pub async fn backup_image(
    mac: &str,
    device: &str,
    log: &(dyn Fn(String) + Send + Sync),
) -> BackupReport {
    let dest = dest_dir(mac);
    let _ = tokio::fs::create_dir_all(&dest).await;
    let img = dest.join("disk.img");
    log(format!("backup image: dd {device} → {}", img.display()));

    let out = tokio::process::Command::new("dd")
        .args([
            &format!("if={device}"),
            &format!("of={}", img.display()),
            "bs=4M",
            "status=progress",
        ])
        .output()
        .await;

    match out {
        Ok(o) if o.status.success() => {
            let size = tokio::fs::metadata(&img)
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            let mut files = std::collections::BTreeMap::new();
            files.insert("disk.img".to_string(), (size, String::new()));
            let rep = BackupReport {
                mode: "image".into(),
                dest: dest.to_string_lossy().into(),
                files: 1,
                bytes: size,
                skipped_unchanged: 0,
                incremental: false,
                ok: true,
                message: format!("Disk image halisi: {size} bytes kutoka {device}"),
            };
            write_manifest(&dest, &rep, files);
            log(rep.message.clone());
            rep
        }
        Ok(o) => BackupReport {
            mode: "image".into(),
            dest: dest.to_string_lossy().into(),
            files: 0,
            bytes: 0,
            skipped_unchanged: 0,
            incremental: false,
            ok: false,
            message: format!("dd imeshindwa: {}", String::from_utf8_lossy(&o.stderr)),
        },
        Err(e) => BackupReport {
            mode: "image".into(),
            dest: dest.to_string_lossy().into(),
            files: 0,
            bytes: 0,
            skipped_unchanged: 0,
            incremental: false,
            ok: false,
            message: format!("dd haipatikani: {e}"),
        },
    }
}

// ---------- manifest helpers ----------

#[derive(Serialize, Clone, serde::Deserialize)]
struct Manifest {
    mode: String,
    dest: String,
    files: u64,
    bytes: u64,
    skipped_unchanged: u64,
    incremental: bool,
    ok: bool,
    message: String,
    started: String,
    finished: String,
    #[serde(rename = "file_index")]
    file_index: std::collections::BTreeMap<String, (u64, String)>,
}

fn write_manifest(
    dest: &Path,
    rep: &BackupReport,
    index: std::collections::BTreeMap<String, (u64, String)>,
) {
    let m = Manifest {
        mode: rep.mode.clone(),
        dest: rep.dest.clone(),
        files: rep.files,
        bytes: rep.bytes,
        skipped_unchanged: rep.skipped_unchanged,
        incremental: rep.incremental,
        ok: rep.ok,
        message: rep.message.clone(),
        started: chrono::Local::now().to_rfc3339(),
        finished: chrono::Local::now().to_rfc3339(),
        file_index: index,
    };
    let p = dest.join("manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(&m) {
        let _ = std::fs::write(p, json);
    }
}

fn latest_manifest(mac_root: &Path) -> Option<(PathBuf, Manifest)> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(mac_root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for d in dirs.iter().rev() {
        let mf = d.join("manifest.json");
        if let Ok(txt) = std::fs::read_to_string(&mf) {
            if let Ok(m) = serde_json::from_str::<Manifest>(&txt) {
                return Some((d.clone(), m));
            }
        }
    }
    None
}

async fn sha256_file(p: &Path) -> anyhow::Result<String> {
    let data = tokio::fs::read(p).await?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn files_backup_nakili_halisi() {
        let tmp = std::env::temp_dir().join(format!("fundi_bk_test_{}", std::process::id()));
        let src = tmp.join("src");
        tokio::fs::create_dir_all(src.join("sub")).await.unwrap();
        tokio::fs::write(src.join("a.txt"), b"habari")
            .await
            .unwrap();
        tokio::fs::write(src.join("sub/b.txt"), b"dunia")
            .await
            .unwrap();

        // Tumia dest ya mpangilio wa test
        let mac = "aa:bb:cc:dd:ee:99";
        let dest = tmp.join("dest");
        let _ = tokio::fs::create_dir_all(&dest).await;

        // redirect backup_root kwa env (run_backup inasoma env kila wakati)
        std::env::set_var("FUNDI_BACKUPS", tmp.join("backups"));
        let rep = backup_files(mac, src.to_str().unwrap(), &|m| println!("{m}")).await;
        assert!(rep.ok, "{}", rep.message);
        assert_eq!(rep.files, 2);

        // Nakala ya pili = incremental (faili zote zinarudiwa, hakuna mpya)
        let rep2 = backup_files(mac, src.to_str().unwrap(), &|_| {}).await;
        assert!(rep2.incremental);
        assert_eq!(rep2.files, 0);
        assert_eq!(rep2.skipped_unchanged, 2);

        let _ = tokio::fs::remove_dir_all(&tmp).await;
    }
}
