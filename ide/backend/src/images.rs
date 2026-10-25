//! Images za OS (P2) — orodha, validation (sha256 + magic bytes), metadata.
//!
//! Zana: `sha256sum` (podman container) au pure-Rust hashing.
//! Magic bytes:
//!   - ISO9660: "CD001" kwenye offset 0x8001
//!   - WIM:     "MSWIM\0\0\0" / "WLPWM\0\0\0" kwenye offset 0

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct OsImage {
    pub name: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub kind: String, // iso | wim | img | unknown
    pub valid: bool,
    pub note: String,
}

pub fn images_root() -> PathBuf {
    PathBuf::from(std::env::var("FUNDI_IMAGES").unwrap_or_else(|_| "./images".into()))
}

pub fn list_images() -> Vec<OsImage> {
    let root = images_root();
    let _ = std::fs::create_dir_all(&root);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&root) else {
        return out;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .map(|x| x.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !matches!(ext.as_str(), "iso" | "wim" | "img" | "gz") {
            continue;
        }
        out.push(inspect(&p));
    }
    out
}

/// Kagua image moja: size, sha256 (kama < FUNDI_HASH_LIMIT_MB), magic bytes
pub fn inspect(p: &Path) -> OsImage {
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().into())
        .unwrap_or_default();
    let meta = std::fs::metadata(p);
    let bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let ext = p
        .extension()
        .map(|x| x.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let limit_mb: u64 = std::env::var("FUNDI_HASH_LIMIT_MB")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2048); // hashing ya GB nyingi ni ghali — limit default 2GB

    let mut sha = String::new();
    let mut note = String::new();
    if bytes <= limit_mb * 1024 * 1024 {
        sha = sha256_path(p).unwrap_or_default();
    } else {
        note.push_str(&format!(
            "sha256 alirukwa (>{limit_mb}MB; onyesha FUNDI_HASH_LIMIT_MB)"
        ));
    }

    let kind = match ext.as_str() {
        "iso" => "iso".into(),
        "wim" => "wim".into(),
        "img" | "gz" => ext.clone(),
        _ => "unknown".into(),
    };

    // Magic bytes validation
    let magic_ok = match kind.as_str() {
        "iso" => has_iso_magic(p),
        "wim" => has_wim_magic(p),
        _ => true, // img/gz: hakuna magic rahisi
    };
    if !magic_ok && note.is_empty() {
        note.push_str("magic bytes hazilingani — faili inaweza si ISO/WIM halisi");
    }

    let valid = bytes > 0 && magic_ok;
    OsImage {
        name,
        path: p.to_string_lossy().into(),
        bytes,
        sha256: sha,
        kind,
        valid,
        note,
    }
}

fn has_iso_magic(p: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(p) else {
        return false;
    };
    if f.seek(SeekFrom::Start(0x8001)).is_err() {
        return false;
    }
    let mut buf = [0u8; 5];
    f.read_exact(&mut buf).is_ok() && &buf == b"CD001"
}

fn has_wim_magic(p: &Path) -> bool {
    let mut buf = [0u8; 8];
    let Ok(mut f) = std::fs::File::open(p) else {
        return false;
    };
    f.read_exact(&mut buf).is_ok() && (&buf == b"MSWIM\0\0\0" || &buf == b"WLPWM\0\0\0")
}

fn sha256_path(p: &Path) -> anyhow::Result<String> {
    let data = std::fs::read(p)?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

/// Pata image kwa jina (pipeline inaitwa hii kabla ya multicast)
pub fn find_image(name: &str) -> Option<OsImage> {
    list_images().into_iter().find(|i| i.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_root_hakuna_panic() {
        // Lengo: haina panic hata root haiipo/ingine — env shared na tests nyingine
        std::env::set_var(
            "FUNDI_IMAGES",
            std::env::temp_dir().join("fundi_img_none_x"),
        );
        let _ = list_images();
    }

    #[test]
    fn iso_magic_halisi() {
        let tmp = std::env::temp_dir().join("fundi_iso_test.iso");
        let mut data = vec![0u8; 0x8001 + 5];
        data[0x8001..0x8006].copy_from_slice(b"CD001");
        std::fs::write(&tmp, &data).unwrap();
        std::env::set_var("FUNDI_IMAGES", tmp.parent().unwrap());
        let img = inspect(&tmp);
        assert_eq!(img.kind, "iso");
        assert!(img.valid, "{}", img.note);
        let _ = std::fs::remove_file(&tmp);
    }
}
