//! ide_build.rs — BUILD MATRIX ya kila jukwaa + release artifacts + kasi.
//!
//! Inatoa:
//!   targets  — orodha ya malengo yote (Windows, macOS, Linux, ARM64, Android, iOS, Web/WASM)
//!   script   — hutengeneza script HALISI ya kujenga (Rust/Tauri/Node) kwa jukwaa
//!   run      — huendesha build HALISI (kama toolchain ipo)
//!   speed    — kipimo cha kasi (benchmark halisi ya codegen + I/O)

use axum::Json;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;

use crate::ide_api::ide_workspace_root;

struct Platform {
    id: &'static str,
    label: &'static str,
    os: &'static str,
    arch: &'static str,
    rust_target: &'static str,
    ext: &'static str,
}

/// Malengo ya Rust/Tauri kwa majukwaa yote.
const PLATFORMS: &[Platform] = &[
    Platform { id: "windows-x64", label: "Windows x64", os: "windows", arch: "x86_64", rust_target: "x86_64-pc-windows-msvc", ext: ".exe" },
    Platform { id: "windows-arm64", label: "Windows ARM64", os: "windows", arch: "aarch64", rust_target: "aarch64-pc-windows-msvc", ext: ".exe" },
    Platform { id: "macos-intel", label: "macOS Intel", os: "macos", arch: "x86_64", rust_target: "x86_64-apple-darwin", ext: "" },
    Platform { id: "macos-arm64", label: "macOS Apple Silicon (M-series)", os: "macos", arch: "aarch64", rust_target: "aarch64-apple-darwin", ext: "" },
    Platform { id: "macos-universal", label: "macOS Universal", os: "macos", arch: "universal", rust_target: "universal-apple-darwin", ext: "" },
    Platform { id: "linux-x64-gnu", label: "Linux x64 (glibc)", os: "linux", arch: "x86_64", rust_target: "x86_64-unknown-linux-gnu", ext: "" },
    Platform { id: "linux-x64-musl", label: "Linux x64 (musl/static)", os: "linux", arch: "x86_64", rust_target: "x86_64-unknown-linux-musl", ext: "" },
    Platform { id: "linux-arm64", label: "Linux ARM64 (RPi/server)", os: "linux", arch: "aarch64", rust_target: "aarch64-unknown-linux-gnu", ext: "" },
    Platform { id: "linux-armv7", label: "Linux ARMv7", os: "linux", arch: "arm", rust_target: "armv7-unknown-linux-gnueabihf", ext: "" },
    Platform { id: "android-arm64", label: "Android ARM64", os: "android", arch: "aarch64", rust_target: "aarch64-linux-android", ext: ".so" },
    Platform { id: "android-x64", label: "Android x64", os: "android", arch: "x86_64", rust_target: "x86_64-linux-android", ext: ".so" },
    Platform { id: "ios-arm64", label: "iOS ARM64", os: "ios", arch: "aarch64", rust_target: "aarch64-apple-ios", ext: "" },
    Platform { id: "ios-sim", label: "iOS Simulator", os: "ios", arch: "aarch64", rust_target: "aarch64-apple-ios-sim", ext: "" },
    Platform { id: "web-wasm", label: "Web (WASM)", os: "web", arch: "wasm32", rust_target: "wasm32-unknown-unknown", ext: ".wasm" },
];

/// Hutafuta jukwaa kwa ID kamili AU jina fupi la OS.
///
/// Schema ya zana ilitangaza `android | ios | linux | windows | macos | wasm`,
/// lakini IDs halisi ni `linux-x64-gnu` n.k. Modeli ilitii schema → "jukwaa
/// halijulikani" kila mara. Sasa majina yote mawili yanakubaliwa; jina fupi
/// huchagua chaguo la kwanza (la kawaida) la OS hiyo.
fn find(id: &str) -> Option<&'static Platform> {
    let want = id.trim().to_ascii_lowercase();
    if want.is_empty() {
        return None;
    }
    if let Some(p) = PLATFORMS.iter().find(|p| p.id == want) {
        return Some(p);
    }
    let os = match want.as_str() {
        "linux" => "linux",
        "windows" | "win" => "windows",
        "macos" | "mac" | "darwin" | "osx" => "macos",
        "android" => "android",
        "ios" => "ios",
        "web" | "wasm" | "web-wasm" => "web",
        // Majina ya kawaida ya Rust targets pia yanakubaliwa.
        _ => {
            return PLATFORMS
                .iter()
                .find(|p| p.rust_target == want)
        }
    };
    // Chaguo la kwanza la OS hiyo ni la kawaida (mf. linux → linux-x64-gnu).
    PLATFORMS.iter().find(|p| p.os == os)
}

pub async fn targets() -> Json<Value> {
    let list: Vec<Value> = PLATFORMS
        .iter()
        .map(|p| {
            json!({
                "id": p.id, "label": p.label, "os": p.os, "arch": p.arch,
                "rust_target": p.rust_target, "artifact_ext": p.ext,
            })
        })
        .collect();
    Json(json!({ "ok": true, "total": PLATFORMS.len(), "platforms": list }))
}

/// Hutengeneza script ya kujenga kwa jukwaa + stack (Rust au Node/Tauri).
fn build_script(p: &Platform, stack: &str) -> String {
    if stack == "tauri" {
        format!(
            r#"#!/usr/bin/env bash
# Jenga {label} (Tauri desktop) — target {t}
set -euo pipefail
rustup target add {t}
cd "$(dirname "$0")/.."
cargo tauri build --target {t}
echo "Artifact: src-tauri/target/{t}/release/bundle/"
"#,
            label = p.label, t = p.rust_target
        )
    } else {
        format!(
            r#"#!/usr/bin/env bash
# Jenga {label} — target {t}
set -euo pipefail
rustup target add {t}
cargo build --release --target {t}
echo "Artifact: target/{t}/release/<bin>{ext}"
"#,
            label = p.label, t = p.rust_target, ext = p.ext
        )
    }
}

/// POST /api/ide/build/script — { platform, stack } → script halisi.
pub async fn script(Json(body): Json<Value>) -> Json<Value> {
    let id = body["platform"].as_str().unwrap_or("linux-x64-gnu");
    let stack = body["stack"].as_str().unwrap_or("rust");
    let Some(p) = find(id) else {
        return Json(json!({ "ok": false, "error": format!("jukwaa halijulikani: {id}") }));
    };
    let script = build_script(p, stack);
    // Andika kwenye scripts/build-<platform>.sh (ndani ya workspace) kama path imetolewa.
    let mut written = Value::Null;
    if let Some(path) = body["path"].as_str().filter(|s| !s.is_empty()) {
        if let Some(dir) = crate::ide_api::safe_path_in_root(&ide_workspace_root(), path) {
            let _ = std::fs::create_dir_all(dir.join("scripts"));
            let file = dir.join("scripts").join(format!("build-{id}.sh"));
            let _ = std::fs::write(&file, &script);
            written = json!(file.display().to_string());
        }
    }
    Json(json!({
        "ok": true, "platform": p.label, "rust_target": p.rust_target,
        "script": script, "written": written,
    }))
}

async fn run_capture(cmd: &str, dir: &Path) -> Value {
    let started = Instant::now();
    let full = format!("{}{}", crate::ide_api::tool_path_prefix(), cmd);
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(900),
        tokio::process::Command::new("bash").arg("-lc").arg(&full).current_dir(dir).output(),
    )
    .await;
    let ms = started.elapsed().as_millis() as u64;
    match out {
        Err(_) => json!({ "ok": false, "cmd": cmd, "ms": ms, "error": "timeout" }),
        Ok(Err(e)) => json!({ "ok": false, "cmd": cmd, "ms": ms, "error": format!("{e}") }),
        Ok(Ok(o)) => json!({
            "ok": o.status.success(), "cmd": cmd, "ms": ms, "exit": o.status.code(),
            "stdout": String::from_utf8_lossy(&o.stdout).chars().take(8000).collect::<String>(),
            "stderr": String::from_utf8_lossy(&o.stderr).chars().take(8000).collect::<String>(),
        }),
    }
}

/// POST /api/ide/build/run — endesha build HALISI kwa jukwaa.
pub async fn run(Json(body): Json<Value>) -> Json<Value> {
    let id = body["platform"].as_str().unwrap_or("");
    let path = body["path"].as_str().unwrap_or("");
    let stack = body["stack"].as_str().unwrap_or("rust");
    let Some(p) = find(id) else {
        return Json(json!({ "ok": false, "error": "jukwaa halijulikani" }));
    };
    let root = ide_workspace_root();
    let dir = crate::ide_api::safe_path_in_root(&root, path).unwrap_or_else(|| root.clone());
    let cmd = if stack == "tauri" {
        format!("rustup target add {t} && cargo tauri build --target {t}", t = p.rust_target)
    } else {
        format!("rustup target add {t} && cargo build --release --target {t}", t = p.rust_target)
    };
    let res = run_capture(&cmd, &dir).await;
    Json(json!({ "ok": res["ok"], "platform": p.label, "target": p.rust_target, "result": res }))
}

/// POST /api/ide/build/speed — kipimo cha KASI HALISI (I/O + codegen-ish).
/// Hupima: kuandika faili 200, kuzisoma, na regex audit — kwa millisecond.
pub async fn speed(Json(body): Json<Value>) -> Json<Value> {
    let n = body["files"].as_u64().unwrap_or(200).min(2000) as usize;
    let size = body["size"].as_u64().unwrap_or(2048).min(65536) as usize;
    let root = ide_workspace_root().join(format!(".speedbench-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&root);
    let sample = "fn main() { let x = \"sk-test\"; println!(\"{}\", x); }\n".repeat(size / 40 + 1);

    let t0 = Instant::now();
    for i in 0..n {
        let _ = std::fs::write(root.join(format!("f{i}.rs")), &sample);
    }
    let write_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let mut total = 0usize;
    for i in 0..n {
        if let Ok(c) = std::fs::read_to_string(root.join(format!("f{i}.rs"))) {
            total += c.len();
        }
    }
    let read_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let t2 = Instant::now();
    let mut findings = 0usize;
    for i in 0..n {
        if let Ok(c) = std::fs::read_to_string(root.join(format!("f{i}.rs"))) {
            findings += crate::code_audit::analyze_content("rust", &c).len();
        }
    }
    let audit_ms = t2.elapsed().as_secs_f64() * 1000.0;

    let _ = std::fs::remove_dir_all(&root);

    let mb = (total as f64) / (1024.0 * 1024.0);
    Json(json!({
        "ok": true,
        "files": n,
        "bytes": total,
        "write_ms": (write_ms * 100.0).round() / 100.0,
        "read_ms": (read_ms * 100.0).round() / 100.0,
        "audit_ms": (audit_ms * 100.0).round() / 100.0,
        "total_ms": ((write_ms + read_ms + audit_ms) * 100.0).round() / 100.0,
        "throughput_mb_s": ((mb / ((write_ms + read_ms) / 1000.0)) * 10.0).round() / 10.0,
        "findings": findings,
        "note": "Kipimo halisi cha I/O + uchambuzi wa usalama (millisecond).",
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/build/targets", get(targets))
        .route("/api/ide/build/script", post(script))
        .route("/api/ide/build/run", post(run))
        .route("/api/ide/build/speed", post(speed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_windows_macos_linux_arm() {
        let ids: Vec<&str> = PLATFORMS.iter().map(|p| p.id).collect();
        for want in ["windows-x64", "macos-arm64", "linux-arm64", "android-arm64", "ios-arm64", "web-wasm"] {
            assert!(ids.contains(&want), "missing {want}");
        }
    }

    #[test]
    fn script_mentions_target() {
        let p = find("macos-arm64").unwrap();
        let s = build_script(p, "rust");
        assert!(s.contains("aarch64-apple-darwin"));
        assert!(s.contains("rustup target add"));
    }

    #[test]
    fn ids_unique() {
        let mut ids: Vec<&str> = PLATFORMS.iter().map(|p| p.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
