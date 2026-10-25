//! ide_lab.rs — MTECHOS IDE LAB: sehemu HALISI za kujaribu miradi.
//!
//! Inaunganisha:
//!   env probe      — inaonyesha toolchain zilizopo (cargo/rustc/Rscript/node/…)
//!   project test   — hugundua lugha ya mradi na kuendesha build/test HALISI
//!   iot telemetry  — simulation ya sensor + uchambuzi wa R (stats halisi)
//!
//! API:
//!   GET  /api/ide/lab/env        → toolchain + workspace
//!   POST /api/ide/lab/test       → { path, target } → build/test halisi
//!   POST /api/ide/lab/telemetry  → { scenario, points, seed, analyze } → series + R
//!
//! USALAMA: hakuna shell kwa code ya mtumiaji — `tokio::process::Command` kwa
//! arg-array; kila path inathibitishwa kuwa ndani ya workspace (`..` hukataliwa).

use axum::Json;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::ide_api::ide_workspace_root;

const CMD_TIMEOUT: Duration = Duration::from_secs(240);
const MAX_OUTPUT: usize = 512 * 1024;

fn safe_rel(rel: &str) -> Option<PathBuf> {
    let root = ide_workspace_root();
    let rel = rel.trim_start_matches('/');
    if rel.contains("..") {
        return None;
    }
    let full = root.join(rel);
    if full.starts_with(&root) {
        Some(full)
    } else {
        None
    }
}

fn resolve_workdir(path: &str) -> Option<PathBuf> {
    if path.trim().is_empty() {
        return Some(ide_workspace_root());
    }
    let p = safe_rel(path)?;
    if !p.exists() {
        return None;
    }
    Some(if p.is_file() {
        p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
    } else {
        p
    })
}

/// Endesha amri kwa arg-array (hakuna shell). Hupunguza output kubwa.
async fn run_args(cmd: &str, args: &[&str], dir: &Path) -> Value {
    let mut c = tokio::process::Command::new(cmd);
    c.args(args).current_dir(dir);
    match tokio::time::timeout(CMD_TIMEOUT, c.output()).await {
        Err(_) => json!({ "ok": false, "error": format!("{cmd} imesimama (timeout {CMD_TIMEOUT:?})") }),
        Ok(Err(e)) => json!({ "ok": false, "error": format!("{cmd} haipatikani: {e}") }),
        Ok(Ok(o)) => {
            let mut out = String::from_utf8_lossy(&o.stdout).to_string();
            let mut err = String::from_utf8_lossy(&o.stderr).to_string();
            if out.len() > MAX_OUTPUT {
                out.truncate(MAX_OUTPUT);
                out.push_str("\n...[truncated]");
            }
            if err.len() > MAX_OUTPUT {
                err.truncate(MAX_OUTPUT);
                err.push_str("\n...[truncated]");
            }
            json!({
                "ok": o.status.success(),
                "cmd": format!("{} {}", cmd, args.join(" ")),
                "exit": o.status.code(),
                "stdout": out,
                "stderr": err,
            })
        }
    }
}

async fn which(bin: &str) -> Option<String> {
    let out = tokio::process::Command::new("bash")
        .arg("-lc")
        .arg(format!("command -v {bin}"))
        .output()
        .await
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    }
}

/// Mzizi wa mti wa chanzo (repo root) — kwa kupima miradi ya mobile (android/, ios/).
/// `CARGO_MANIFEST_DIR` ni `<repo>/server/agent`; repo root ni ngazi mbili juu.
fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

/// GET /api/ide/lab/env — toolchain zilizopo + workspace.
pub async fn lab_env() -> Json<Value> {
    let root = ide_workspace_root();
    let mut tools = serde_json::Map::new();
    for t in [
        "cargo", "rustc", "rust-analyzer", "Rscript", "R", "node", "npm", "npx", "elixir",
        "mix", "go", "gofmt", "playwright", "adb", "emulator", "tauri", "git", "make", "docker", "gcc",
        "dotnet", "java",
    ] {
        let path = which(t).await;
        tools.insert(t.to_string(), json!({ "found": path.is_some(), "path": path }));
    }

    // Uwezo wa HADHI (grouped) — huonyesha kila eneo la maono kama READY au la.
    let found = |k: &str| tools.get(k).and_then(|v| v["found"].as_bool()).unwrap_or(false);
    let ready = |ks: &[&str]| ks.iter().all(|k| found(k));
    // Playwright ipo ndani ya `<ws>/e2e/node_modules` (si PA global) — hesabu hiyo
    // ili READY ilingane na `ide_e2e::status` (chanzo kimoja cha ukweli).
    let e2e_local = root.join("e2e/node_modules/@playwright/test").exists();
    let e2e_ready = found("node") && (found("playwright") || e2e_local);

    // Miradi ya mobile ipo kwenye mti wa chanzo (sio volume ya kazi). Hii inaonyesha
    // uhalisia: "codegen/design" inaweza kuwa READY hata host isipokuwa macOS/SDK.
    let src_root = source_root();
    let android_project = src_root.join("android/app/src/main/AndroidManifest.xml").exists();
    let ios_project = src_root.join("ios/MtechosApp/Info.plist").exists();

    let capabilities = json!({
        "web":        { "ready": ready(&["node", "npm"]), "needs": ["node", "npm"] },
        "mobile_android": {
            "ready": ready(&["adb"]),
            "needs": ["adb", "emulator"],
            "build_ready": ready(&["adb"]),
            "app_project": android_project,
            "codegen_ready": true,
            "note": "Mradi wa Kotlin/WebView uko `android/`; build huhitaji Android SDK + JDK. Codegen/WebView hufanya kazi bila SDK.",
        },
        "ios": {
            "ready": cfg!(target_os = "macos"),
            "needs": ["xcodebuild (macOS)"],
            "build_ready": cfg!(target_os = "macos"),
            "app_project": ios_project,
            "codegen_ready": true,
            "note": "Mradi wa SwiftUI/WebKit uko `ios/`; build/simulator huhitaji macOS + Xcode. Codegen/WebView hufanya kazi bila Xcode.",
        },
        "desktop_tauri": { "ready": ready(&["cargo", "tauri"]) || found("cargo"), "needs": ["cargo", "tauri"], "note": "cargo ya kutosha kwa `cargo tauri`; tauri CLI inapendekezwa" },
        "iot":        { "ready": ready(&["cargo", "rustc"]), "needs": ["cargo", "rustc"], "note": "IoT huzalishwa kwa Rust (telemetry/analytics); R ikiwepo huchanganua telemetry kwa uhalisia" },
        "gaming_3d":  { "ready": true, "needs": [], "note": "three.js/WebGL kwenye browser (CDN)" },
        "analytics_r_rust": { "ready": ready(&["cargo", "rustc"]), "needs": ["cargo", "rustc"], "r_ready": found("Rscript") },
        "debug_dap":  { "ready": ready(&["node"]), "needs": ["node"], "note": "DAP hupitia node (JavaScript/TypeScript) — Rust/Go/Elixir hupitia LSP zao" },
        "e2e":        { "ready": e2e_ready, "needs": ["node", "playwright"], "local": e2e_local,
                        "note": "Playwright huwekwa ndani ya e2e/ (require('@playwright/test')) — /api/ide/e2e/run huiweka ikiwa haipo" },
        "lsp":        { "ready": found("rust-analyzer") || found("typescript-language-server"), "needs": ["rust-analyzer / typescript-language-server"] },
    });

    Json(json!({
        "ok": true,
        "workspace": root.display().to_string(),
        "tools": tools,
        "capabilities": capabilities,
        "install": {
            "R": "apt-get install -y r-base-core r-cran-jsonlite",
            "elixir": "apt-get install -y erlang elixir",
            "playwright": "npm i -g @playwright/test && npx playwright install --with-deps chromium",
            "android": "apt-get install -y adb android-sdk (au Android Studio)",
            "rust-analyzer": "rustup component add rust-analyzer",
            "tauri": "cargo install tauri-cli",
        },
        "note": "Rust (cargo/rustc) + Go + Elixir + R (Rscript) ni injini za uchambuzi; kila uwezo huonyesha READY/la kwa uhalisia.",
    }))
}

fn detect_project(dir: &Path) -> Vec<Value> {
    let checks: &[(&str, &str, &str)] = &[
        ("Cargo.toml", "rust", "cargo"),
        ("package.json", "node", "npm"),
        ("mix.exs", "elixir", "mix"),
        ("DESCRIPTION", "r-package", "R"),
        ("renv.lock", "r-project", "R"),
        ("go.mod", "go", "go"),
        ("Makefile", "make", "make"),
        ("pom.xml", "java-maven", "mvn"),
        ("build.gradle", "java-gradle", "gradle"),
        ("composer.json", "php", "composer"),
        ("Dockerfile", "docker", "docker"),
    ];
    let mut found = Vec::new();
    for (f, kind, runner) in checks {
        if dir.join(f).exists() {
            found.push(json!({ "file": f, "kind": kind, "runner": runner }));
        }
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let is_r = p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("r"))
                .unwrap_or(false);
            if is_r {
                found.push(json!({
                    "file": p.file_name().unwrap_or_default().to_string_lossy(),
                    "kind": "r-script",
                    "runner": "Rscript",
                }));
                break;
            }
        }
    }
    found
}

fn has_kind(detected: &[Value], kind: &str) -> bool {
    detected.iter().any(|d| d["kind"].as_str() == Some(kind))
}

/// POST /api/ide/lab/test — hugundua mradi na kuendesha build/test halisi.
pub async fn lab_test(Json(body): Json<Value>) -> Json<Value> {
    let path = body["path"].as_str().unwrap_or("");
    let target = body["target"].as_str().unwrap_or("test").to_lowercase();
    let Some(dir) = resolve_workdir(path) else {
        return Json(json!({ "ok": false, "error": "workdir haipatikani ndani ya workspace" }));
    };
    let detected = detect_project(&dir);

    let result = if has_kind(&detected, "rust") {
        let args: Vec<&str> = match target.as_str() {
            "build" => vec!["build", "--release"],
            "check" => vec!["check"],
            "run" => vec!["run", "--release"],
            _ => vec!["test"],
        };
        run_args("cargo", &args, &dir).await
    } else if has_kind(&detected, "node") {
        let script = match target.as_str() {
            "build" => "build",
            "run" => "start",
            _ => "test",
        };
        run_args("npm", &["run", script, "--silent"], &dir).await
    } else if has_kind(&detected, "elixir") {
        let args: Vec<&str> = match target.as_str() {
            "build" => vec!["compile"],
            "run" => vec!["run", "--no-halt"],
            _ => vec!["test"],
        };
        run_args("mix", &args, &dir).await
    } else if has_kind(&detected, "go") {
        let args: Vec<&str> = match target.as_str() {
            "build" => vec!["build", "./..."],
            "check" => vec!["vet", "./..."],
            "run" => vec!["run", "."],
            _ => vec!["test", "./..."],
        };
        run_args("go", &args, &dir).await
    } else if has_kind(&detected, "make") {
        run_args("make", &[target.as_str()], &dir).await
    } else if has_kind(&detected, "r-script")
        || has_kind(&detected, "r-project")
        || has_kind(&detected, "r-package")
    {
        run_r_project(&dir).await
    } else {
        json!({
            "ok": false,
            "error": "hakuna mradi unaotambulika (Cargo.toml / package.json / mix.exs / DESCRIPTION / *.R / go.mod / Makefile)",
        })
    };

    Json(json!({
        "ok": true,
        "workdir": dir.display().to_string(),
        "target": target,
        "detected": detected,
        "result": result,
    }))
}

/// Endesha faili za .R za mradi (hadi 5) kwa `Rscript --vanilla`.
async fn run_r_project(dir: &Path) -> Value {
    let mut runs = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            let is_r = p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("r"))
                .unwrap_or(false);
            if !is_r {
                continue;
            }
            let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let r = run_args("Rscript", &["--vanilla", name.as_str()], dir).await;
            runs.push(json!({ "file": name, "result": r }));
            if runs.len() >= 5 {
                break;
            }
        }
    }
    if runs.is_empty() {
        json!({ "ok": false, "error": "hakuna faili .R" })
    } else {
        json!({ "ok": true, "runs": runs })
    }
}

// ------------------------- IoT TELEMETRY SIM + R -------------------------

struct Rng {
    s: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { s: seed | 1 }
    }
    /// xorshift64* — deterministic, hakuna crate ya nje.
    fn next_f64(&mut self) -> f64 {
        let mut x = self.s;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.s = x;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Box–Muller normal.
    fn normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-12);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
}

fn simulate(scenario: &str, points: usize, seed: u64) -> Vec<Value> {
    let mut rng = Rng::new(seed);
    let n = points.clamp(8, 512);
    let (base_t, amp_t, base_h, unit) = match scenario {
        "afya" | "cold-chain" => (5.0, 2.0, 70.0, "°C"),
        "kilimo" => (27.0, 5.0, 55.0, "°C"),
        "nyumbani" => (24.0, 1.5, 50.0, "°C"),
        "usalama" => (30.0, 6.0, 40.0, "°C"),
        _ => (23.0, 3.0, 55.0, "°C"),
    };
    let anomaly_at = (rng.next_f64() * n as f64) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let phase = i as f64 / 6.0;
        let mut temp = base_t + amp_t * phase.sin() + rng.normal() * 0.4;
        let hum = (base_h + 8.0 * (phase / 2.0).cos() + rng.normal() * 1.2).clamp(0.0, 100.0);
        let mut anomaly = false;
        if i == anomaly_at {
            temp += amp_t * 3.0 + 6.0;
            anomaly = true;
        }
        out.push(json!({
            "t": i,
            "temperature": (temp * 100.0).round() / 100.0,
            "humidity": (hum * 100.0).round() / 100.0,
            "unit": unit,
            "anomaly": anomaly,
        }));
    }
    out
}

async fn run_r_code(code: &str) -> Value {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("mtechos_lab_{nanos}.R"));
    if let Err(e) = std::fs::write(&path, code) {
        return json!({ "ok": false, "error": e.to_string() });
    }
    let p = path.display().to_string();
    let res = run_args("Rscript", &["--vanilla", p.as_str()], &ide_workspace_root()).await;
    let _ = std::fs::remove_file(&path);
    res
}

/// POST /api/ide/lab/telemetry — series ya sensor + uchambuzi wa R.
pub async fn lab_telemetry(Json(body): Json<Value>) -> Json<Value> {
    let scenario = body["scenario"].as_str().unwrap_or("greenhouse");
    let points = body["points"].as_u64().unwrap_or(48) as usize;
    let seed = body["seed"].as_u64().unwrap_or(42);
    let analyze = body["analyze"].as_bool().unwrap_or(true);

    let series = simulate(scenario, points, seed);
    let temps: Vec<String> = series
        .iter()
        .filter_map(|p| p["temperature"].as_f64())
        .map(|v| format!("{v:.4}"))
        .collect();

    let r = if analyze {
        let code = format!(
            r#"x <- c({})
x <- x[is.finite(x)]
out <- list(n = length(x), mean = round(mean(x), 2), sd = round(sd(x), 3),
  min = round(min(x), 2), max = round(max(x), 2), median = round(median(x), 2))
cat(jsonlite::toJSON(out, auto_unbox = TRUE))
"#,
            temps.join(",")
        );
        run_r_code(&code).await
    } else {
        json!({ "skipped": true })
    };

    let anomalies = series
        .iter()
        .filter(|p| p["anomaly"].as_bool().unwrap_or(false))
        .count();

    Json(json!({
        "ok": true,
        "scenario": scenario,
        "points": series.len(),
        "anomalies": anomalies,
        "series": series,
        "r": r,
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/lab/env", get(lab_env))
        .route("/api/ide/lab/test", post(lab_test))
        .route("/api/ide/lab/telemetry", post(lab_telemetry))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Kila eneo la maono LAZIMA liwe na `ready` (bool) — hakuna "gap" inayofichwa.
    #[tokio::test]
    async fn lab_env_capabilities_zina_ready_bool() {
        let Json(v) = lab_env().await;
        let caps = v["capabilities"].as_object().expect("capabilities ni object");
        for id in [
            "web", "mobile_android", "ios", "desktop_tauri", "iot", "gaming_3d",
            "analytics_r_rust", "debug_dap", "e2e", "lsp",
        ] {
            let c = caps.get(id).unwrap_or_else(|| panic!("capability `{id}` haipo"));
            assert!(c["ready"].is_boolean(), "`{id}.ready` lazima iwe bool");
            assert!(c["needs"].is_array(), "`{id}.needs` lazima iwe array");
        }
        assert!(v["tools"]["node"]["found"].is_boolean());
        assert!(v["install"]["R"].is_string());
    }
}
