//! mtechos-images — msimamizi wa images za OS
//! (Rust — hakuna Python).
//!
//! Amri:
//!   list                     — orodha (path, bytes, kind)
//!   add <file> [name]        — nakili image kwenye FUNDI_IMAGES + sha256 sidecar
//!   verify [name]            — sha256 + magic bytes (ISO9660/WIM)
//!   remove <name>            — futa image (+ sidecar)
//!
//! Env: `FUNDI_IMAGES` (default `images`).

use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sha2::{Digest, Sha256};

const EXTS: [&str; 4] = ["iso", "wim", "img", "gz"];

fn root() -> PathBuf {
    PathBuf::from(std::env::var("FUNDI_IMAGES").unwrap_or_else(|_| "images".to_string()))
}

fn sha256_file(p: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(p)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// ISO9660 huanza na `CD001` kwenye offset 0x8001; WIM huanza na `MSWIM\0\0\0`
/// (au `WLPWM\0\0\0` kwa split WIM) kwenye offset 0.
fn magic_kind(p: &Path) -> String {
    let mut f = match fs::File::open(p) {
        Ok(f) => f,
        Err(_) => return "unknown".to_string(),
    };
    let mut head = [0u8; 8];
    if f.read_exact(&mut head).is_err() {
        return "unknown".to_string();
    }
    let mut iso = [0u8; 5];
    if f.seek(SeekFrom::Start(0x8001)).is_ok()
        && f.read_exact(&mut iso).is_ok()
        && &iso == b"CD001"
    {
        return "iso".to_string();
    }
    if head.starts_with(b"MSWIM\x00\x00\x00") || head.starts_with(b"WLPWM\x00\x00\x00") {
        return "wim".to_string();
    }
    "unknown".to_string()
}

fn has_ext(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn all_images() -> Vec<PathBuf> {
    let r = root();
    let _ = fs::create_dir_all(&r);
    let mut out = Vec::new();
    let mut stack = vec![r];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() && has_ext(&path) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn list_images() {
    let r = root();
    let items: Vec<serde_json::Value> = all_images()
        .iter()
        .map(|p| {
            serde_json::json!({
                "path": p.strip_prefix(&r).unwrap_or(p).display().to_string(),
                "bytes": fs::metadata(p).map(|m| m.len()).unwrap_or(0),
                "kind": magic_kind(p),
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "images": items,
            "root": r.display().to_string(),
        }))
        .unwrap_or_default()
    );
}

fn add_image(src: &str, name: Option<&str>) -> Result<(), String> {
    let s = Path::new(src);
    if !s.is_file() {
        return Err(format!("Hakuna faili: {src}"));
    }
    let r = root();
    let dst = r.join(name.map(str::to_string).unwrap_or_else(|| {
        s.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }));
    fs::create_dir_all(&r).map_err(|e| e.to_string())?;
    println!("[fundi] nakili {} → {}", s.display(), dst.display());
    fs::copy(s, &dst).map_err(|e| e.to_string())?;
    let digest = sha256_file(&dst).map_err(|e| e.to_string())?;
    let sidecar = PathBuf::from(format!("{}.sha256", dst.display()));
    let dname = dst
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    fs::write(&sidecar, format!("{digest}  {dname}\n")).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "added": dname,
            "bytes": fs::metadata(&dst).map(|m| m.len()).unwrap_or(0),
            "sha256": digest,
        }))
        .unwrap_or_default()
    );
    Ok(())
}

fn verify_image(name: Option<&str>) {
    let targets: Vec<PathBuf> = match name {
        Some(n) => vec![root().join(n)],
        None => all_images(),
    };
    let out: Vec<serde_json::Value> = targets
        .iter()
        .filter(|p| p.is_file())
        .map(|p| {
            let sidecar = PathBuf::from(format!("{}.sha256", p.display()));
            let expected = fs::read_to_string(&sidecar)
                .ok()
                .and_then(|t| t.split_whitespace().next().map(str::to_string));
            let digest = sha256_file(p).unwrap_or_default();
            let matches = expected.as_ref().map(|e| e == &digest);
            serde_json::json!({
                "name": p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                "kind": magic_kind(p),
                "sha256": digest,
                "matches_sidecar": matches,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "verified": out })).unwrap_or_default()
    );
}

fn remove_image(name: &str) {
    let p = root().join(name);
    for f in [p.clone(), PathBuf::from(format!("{}.sha256", p.display()))] {
        if f.exists() {
            let _ = fs::remove_file(&f);
            println!("[fundi] ulifuta {}", f.display());
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("Matumizi: mtechos-images <list|add|verify|remove> [target] [name]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    match cmd {
        "list" => {
            list_images();
            ExitCode::SUCCESS
        }
        "add" => {
            let Some(target) = args.get(1) else {
                eprintln!("add <file> [name]");
                return ExitCode::from(2);
            };
            match add_image(target, args.get(2).map(String::as_str)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        "verify" => {
            verify_image(args.get(1).map(String::as_str));
            ExitCode::SUCCESS
        }
        "remove" => {
            let Some(target) = args.get(1) else {
                eprintln!("remove <name>");
                return ExitCode::from(2);
            };
            remove_image(target);
            ExitCode::SUCCESS
        }
        _ => usage(),
    }
}
