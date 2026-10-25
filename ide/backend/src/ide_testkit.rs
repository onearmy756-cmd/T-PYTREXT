//! ide_testkit.rs — TESTI HALISI kwa kila stack (desktop + web + mobile + R).
//!
//! Hugundua aina ya mradi kwa faili zake, kisha huendesha amri HALISI za
//! test/lint/format/build. Hii ndiyo "real project testing" ya IDE.
//!
//! Stacks: Rust, Node/TS, Python, Go, Java (maven/gradle), PHP, Ruby, R, C/C++,
//! Docker, Tauri (desktop). Amri zinaendeshwa kwa `bash -lc` ndani ya workdir
//! iliyothibitishwa (ndani ya workspace pekee), zenye timeout.

use axum::Json;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::ide_api::ide_workspace_root;

const TEST_TIMEOUT: Duration = Duration::from_secs(600);

struct Suite {
    id: &'static str,
    label: &'static str,
    marker: &'static str,
    test: &'static str,
    lint: Option<&'static str>,
    fmt: Option<&'static str>,
}

/// Stack zote zinazotambulika. `marker` = faili inayoashiria stack.
const SUITES: &[Suite] = &[
    Suite { id: "rust", label: "Rust (cargo)", marker: "Cargo.toml", test: "cargo test --all --quiet", lint: Some("cargo clippy --all-targets --quiet"), fmt: Some("cargo fmt --all --check") },
    Suite { id: "node", label: "Node / TypeScript", marker: "package.json", test: "npm test --silent", lint: Some("npx --yes tsc --noEmit"), fmt: None },
    // JS/TS bila `package.json` (node:test ya kizazi kipya) — la sivyo mradi
    // wa faili moja haugunduliki na `run_tests` hushindwa bure.
    Suite { id: "node-test", label: "Node (node:test)", marker: "*.test.js", test: "node --test", lint: None, fmt: None },
    Suite { id: "go", label: "Go", marker: "go.mod", test: "go test ./...", lint: Some("go vet ./..."), fmt: Some("gofmt -l .") },
    Suite { id: "java-maven", label: "Java (Maven)", marker: "pom.xml", test: "mvn -q test", lint: None, fmt: None },
    Suite { id: "java-gradle", label: "Java (Gradle)", marker: "build.gradle", test: "gradle test --quiet", lint: None, fmt: None },
    Suite { id: "php", label: "PHP (PHPUnit)", marker: "composer.json", test: "vendor/bin/phpunit --colors=never", lint: None, fmt: None },
    Suite { id: "ruby", label: "Ruby (RSpec)", marker: "Gemfile", test: "bundle exec rspec --format progress", lint: None, fmt: None },
    Suite { id: "r-package", label: "R package (testthat)", marker: "DESCRIPTION", test: "Rscript -e 'testthat::test_dir(\"tests/testthat\")'", lint: None, fmt: None },
    Suite { id: "r-project", label: "R project", marker: "renv.lock", test: "Rscript -e 'source(\"tests/run_all.R\")'", lint: None, fmt: None },
    Suite { id: "cpp", label: "C/C++ (make)", marker: "Makefile", test: "make test", lint: None, fmt: None },
    Suite { id: "tauri", label: "Tauri desktop", marker: "tauri.conf.json", test: "cargo test --manifest-path src-tauri/Cargo.toml --quiet", lint: Some("cargo tauri build --debug"), fmt: None },
    Suite { id: "elixir", label: "Elixir (Mix)", marker: "mix.exs", test: "mix test", lint: Some("mix compile --warnings-as-errors"), fmt: Some("mix format --check-formatted") },
    Suite { id: "ml-rust", label: "Rust ML (Candle)", marker: "Cargo.toml", test: "cargo test --quiet", lint: Some("cargo check --quiet"), fmt: Some("cargo fmt --all --check") },
    Suite { id: "r-plain", label: "R (base scripts)", marker: "main.R", test: "Rscript -e 'invisible(lapply(list.files(\".\", pattern=\"[.]R$\", recursive=TRUE, full.names=TRUE), parse))'", lint: None, fmt: None },
    Suite { id: "kotlin", label: "Kotlin (Gradle/Ktor)", marker: "build.gradle.kts", test: "gradle test --quiet", lint: None, fmt: None },
    Suite { id: "swift", label: "Swift (SwiftPM/Vapor)", marker: "Package.swift", test: "swift test", lint: Some("swift build"), fmt: None },
    Suite { id: "csharp", label: "C# (.NET)", marker: "*.csproj", test: "dotnet test --nologo", lint: Some("dotnet build --nologo"), fmt: None },
    Suite { id: "cpp-cmake", label: "C++ (CMake)", marker: "CMakeLists.txt", test: "cmake -S . -B build && cmake --build build", lint: None, fmt: None },
];

fn safe_dir(path: &str) -> Option<std::path::PathBuf> {
    let root = ide_workspace_root();
    // `safe_path_in_root` hushughulikia path relatif NA kamili ndani ya root,
    // hivyo `/data/ide` haijoin maradufu kuwa `/data/ide/data/ide` (bug ya awali
    // iliyovunja run_tests kwa path yoyote kamili).
    let p = crate::ide_api::safe_path_in_root(&root, path)?;
    Some(if p.is_file() {
        p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
    } else {
        p
    })
}

async fn run_cmd(cmd: &str, dir: &Path) -> Value {
    let started = Instant::now();
    let full = format!("{}{}", crate::ide_api::tool_path_prefix(), cmd);
    let out = tokio::time::timeout(
        TEST_TIMEOUT,
        tokio::process::Command::new("bash")
            .arg("-lc")
            .arg(&full)
            .current_dir(dir)
            .output(),
    )
    .await;
    let ms = started.elapsed().as_millis() as u64;
    match out {
        Err(_) => json!({ "ok": false, "cmd": cmd, "ms": ms, "error": "timeout" }),
        Ok(Err(e)) => json!({ "ok": false, "cmd": cmd, "ms": ms, "error": format!("{e}") }),
        Ok(Ok(o)) => {
            let mut stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let mut stderr = String::from_utf8_lossy(&o.stderr).to_string();
            if stdout.len() > 12000 {
                stdout.truncate(12000);
                stdout.push_str("\n...[truncated]");
            }
            if stderr.len() > 12000 {
                stderr.truncate(12000);
                stderr.push_str("\n...[truncated]");
            }
            json!({
                "ok": o.status.success(),
                "cmd": cmd,
                "ms": ms,
                "exit": o.status.code(),
                "stdout": stdout,
                "stderr": stderr,
            })
        }
    }
}

/// Tambua stacks zilizopo kwenye workdir (pamoja na subdirs moja chini).
fn detect(dir: &Path) -> Vec<Value> {
    let mut found = Vec::new();
    let mut dirs: Vec<std::path::PathBuf> = vec![dir.to_path_buf()];
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let n = p.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                if !matches!(n.as_str(), "node_modules" | "target" | ".git" | "dist" | "build") {
                    dirs.push(p);
                }
            }
        }
    }
    for d in dirs {
        for s in SUITES {
            let hit = if let Some(pat) = s.marker.strip_prefix('*') {
                // `*.csproj` au `*.test.js`: linganisha mwisho wa jina la faili
                // (`extension()` hurudisha "js" pekee kwa `calc.test.js`).
                std::fs::read_dir(&d).ok().map(|rd| {
                    rd.flatten().any(|e| {
                        e.path()
                            .file_name()
                            .map(|x| x.to_string_lossy().ends_with(pat))
                            .unwrap_or(false)
                    })
                }).unwrap_or(false)
            } else {
                d.join(s.marker).exists()
            };
            if hit {
                let rel = d
                    .strip_prefix(dir)
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or_default();
                found.push(json!({
                    "id": s.id, "label": s.label, "marker": s.marker,
                    "subdir": if rel.is_empty() { ".".to_string() } else { rel },
                }));
            }
        }
    }
    // Dedupe kwa id+subdir
    let mut seen = std::collections::HashSet::new();
    found.retain(|f| {
        let k = format!("{}|{}", f["id"].as_str().unwrap_or(""), f["subdir"].as_str().unwrap_or(""));
        seen.insert(k)
    });
    found
}

fn suite_by_id(id: &str) -> Option<&'static Suite> {
    SUITES.iter().find(|s| s.id == id)
}

/// GET /api/ide/test/suites — orodha ya stacks + zilizogundulika.
pub async fn suites(Json(body): Json<Value>) -> Json<Value> {
    let path = body["path"].as_str().unwrap_or("");
    let Some(dir) = safe_dir(path) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace" }));
    };
    let detected = detect(&dir);
    Json(json!({
        "ok": true,
        "workdir": dir.display().to_string(),
        "available": SUITES.iter().map(|s| json!({
            "id": s.id, "label": s.label, "marker": s.marker,
            "has_test": !s.test.is_empty(),
            "has_lint": s.lint.is_some(),
            "has_fmt": s.fmt.is_some(),
        })).collect::<Vec<_>>(),
        "detected": detected,
    }))
}

/// POST /api/ide/test/run — endesha testi HALISI.
/// body: { path, only?: "rust", with_lint?: bool, with_fmt?: bool }
pub async fn run(Json(body): Json<Value>) -> Json<Value> {
    let path = body["path"].as_str().unwrap_or("");
    let only = body["only"].as_str().unwrap_or("");
    let with_lint = body["with_lint"].as_bool().unwrap_or(false);
    let with_fmt = body["with_fmt"].as_bool().unwrap_or(false);

    let Some(dir) = safe_dir(path) else {
        return Json(json!({ "ok": false, "error": "path nje ya workspace" }));
    };
    let mut detected = detect(&dir);
    if !only.is_empty() {
        detected.retain(|d| d["id"].as_str() == Some(only));
    }
    if detected.is_empty() {
        return Json(json!({
            "ok": false,
            "error": "hakuna stack inayotambulika — angalia /api/ide/test/suites",
            "workdir": dir.display().to_string(),
        }));
    }

    let mut results = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    for d in &detected {
        let id = d["id"].as_str().unwrap_or("");
        let sub = d["subdir"].as_str().unwrap_or(".");
        let Some(s) = suite_by_id(id) else { continue };
        let workdir = if sub == "." { dir.clone() } else { dir.join(sub) };
        let mut entry = json!({ "id": s.id, "label": s.label, "subdir": sub });

        let test = run_cmd(s.test, &workdir).await;
        let test_ok = test["ok"].as_bool().unwrap_or(false);
        if test_ok { passed += 1; } else { failed += 1; }
        entry["test"] = test;

        if with_lint {
            if let Some(lint) = s.lint {
                entry["lint"] = run_cmd(lint, &workdir).await;
            }
        }
        if with_fmt {
            if let Some(fmt) = s.fmt {
                entry["fmt"] = run_cmd(fmt, &workdir).await;
            }
        }
        results.push(entry);
    }

    Json(json!({
        "ok": true,
        "workdir": dir.display().to_string(),
        "suites": results.len(),
        "passed": passed,
        "failed": failed,
        "results": results,
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/test/suites", get(suites_get))
        .route("/api/ide/test/run", post(run))
}

async fn suites_get() -> Json<Value> {
    suites(Json(json!({}))).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suites_cover_major_stacks() {
        let ids: Vec<&str> = SUITES.iter().map(|s| s.id).collect();
        for want in ["rust", "node", "go", "java-maven", "r-package", "tauri", "elixir", "ml-rust"] {
            assert!(ids.contains(&want), "missing suite {want}");
        }
    }

    #[test]
    fn suite_ids_unique() {
        let mut ids: Vec<&str> = SUITES.iter().map(|s| s.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn safe_dir_rejects_traversal() {
        assert!(safe_dir("../etc").is_none());
        assert!(safe_dir("/../../x").is_none());
    }

    #[test]
    fn suites_mpya_zipo() {
        for id in ["kotlin", "swift", "csharp", "cpp-cmake"] {
            assert!(suite_by_id(id).is_some(), "suite {} haipo", id);
        }
    }

    #[test]
    fn detect_inaunga_csproj_kwa_wildcard() {
        let dir = std::env::temp_dir().join(format!("tk_wild_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mradi.csproj"), "<Project />").unwrap();
        let found = detect(&dir);
        assert!(found.iter().any(|f| f["id"] == "csharp"), "wildcard *.csproj haikugundua");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
