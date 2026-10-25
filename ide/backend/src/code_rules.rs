//! code_rules.rs — LINTER + FAILI LA KANUNI (Coding Agent self-check).
//!
//! Ujenzi (kulingana na spec ya mmiliki):
//!   1. Rules Repository: `data/code_rules/languages.json` — sheria za syntax +
//!      usalama kwa kila lugha. Kubadilisha faili hili hakuhitaji kufundisha
//!      modeli upya (data-driven).
//!   2. Linter: kazi `lint()` inakagua kodi dhidi ya sheria — mabano yaliyofungwa,
//!      patterns zilizokatazwa (eval, SQL injection, hard-coded secrets…).
//!   3. Mzunguko wa kujisahihisha: `POST /api/ide/selfcheck` — agent (au Monaco
//!      UI) inapeleka rasimu; linter inarudisha kosa; agent inarekebisha vipi
//!      inarudisha upya mpaka `ok: true`.
//!
//! Kanuni za repo: matokeo ni HALISI kutoka rules file; hakuna mock.

use axum::Json;
use regex::Regex;
use serde_json::{json, Value};
use std::path::PathBuf;

fn rules_path() -> PathBuf {
    crate::data_io::data_dir().join("code_rules").join("languages.json")
}

fn load_rules() -> Result<Value, String> {
    // Njia kuu: data_dir/code_rules/languages.json (FUNDI_DATA). Kama FUNDI_DATA
    // haipo (mfano tests zinazoendesha kutoka server/agent), angalia repo-root data
    // kupitia CARGO_MANIFEST_DIR/../data (compile-time absolute).
    let candidates = [
        rules_path(),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/code_rules/languages.json"),
        std::path::PathBuf::from("data/code_rules/languages.json"),
    ];
    let text = candidates
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
        .ok_or_else(|| format!("rules file haipatikani: {}", rules_path().display()))?;
    serde_json::from_str(&text).map_err(|e| format!("rules JSON mbovu: {e}"))
}

/// Lugha kutoka extension (inalingana na aliases za rules file).
fn detect_language(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next().unwrap_or("");
    Some(match ext {
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "ts" | "tsx" => "typescript",
        "rs" => "rust",
        "sql" => "sql",
        "sh" | "bash" | "zsh" => "shell",
        "html" | "htm" | "svg" => "html",
        "go" => "go",
        "c" | "h" => "c",
        "cc" | "cxx" | "cpp" | "hpp" => "cpp",
        "java" => "java",
        "cs" => "csharp",
        "php" | "phtml" => "php",
        "rb" => "ruby",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "dart" => "dart",
        "ex" | "exs" => "elixir",
        "r" | "R" => "r",
        _ => return None,
    })
}

/// Ondoa comments kabla ya kukagua patterns zilizokatazwa. Bila hii, neno kama
/// `.unwrap()` lililotajwa ndani ya comment linatoa false-positive. Mabano
/// (`balanced`) hukaguliwa kwenye kodi ASILI — comments hazibadilishi mabano.
fn strip_comments(content: &str, lang: &str) -> String {
    // Ruby/Shell/Elixir/R: `#`. HTML: `<!-- -->`. Nyingine: `//` + `/* */`.
    let hash = matches!(lang, "ruby" | "shell" | "elixir" | "r");
    let html = lang == "html";
    let mut out = String::with_capacity(content.len());
    let mut block = false; // ndani ya /* */ au <!-- -->
    for line in content.lines() {
        let mut s = String::with_capacity(line.len());
        let b: Vec<char> = line.chars().collect();
        let mut i = 0usize;
        while i < b.len() {
            if block {
                if b[i] == '*' && b.get(i + 1) == Some(&'/') {
                    block = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if html && b[i] == '<' && b.get(i + 1) == Some(&'!') && b.get(i + 2) == Some(&'-') && b.get(i + 3) == Some(&'-') {
                block = true;
                i += 4;
                continue;
            }
            if b[i] == '/' && b.get(i + 1) == Some(&'*') {
                block = true;
                i += 2;
                continue;
            }
            if hash && b[i] == '#' {
                break;
            }
            if !hash && !html && b[i] == '/' && b.get(i + 1) == Some(&'/') {
                break;
            }
            s.push(b[i]);
            i += 1;
        }
        out.push_str(&s);
        out.push('\n');
    }
    out
}

/// Check moja la linter: `balanced` (mabano) au `forbid` (regex pattern).
fn run_check(content: &str, check: &Value, lang: &str, findings: &mut Vec<Value>) {
    let name = check.get("name").and_then(|v| v.as_str()).unwrap_or("check");

    if let Some(pairs) = check.get("balanced").and_then(|v| v.as_array()) {
        for pair in pairs {
            let open = pair.get(0).and_then(|v| v.as_str()).unwrap_or("");
            let close = pair.get(1).and_then(|v| v.as_str()).unwrap_or("");
            let o = content.matches(open).count();
            let c = content.matches(close).count();
            if o != c {
                findings.push(json!({
                    "severity": "error",
                    "rule": "balanced",
                    "message": format!("'{open}' ×{o} na '{close}' ×{c} — hazilingani ({name})"),
                }));
            }
        }
    }

    let pat = check
        .get("forbid")
        .and_then(|v| v.as_str())
        .or_else(|| check.get("pattern").and_then(|v| v.as_str()));
    if let Some(pat) = pat {
        if let Ok(re) = Regex::new(pat) {
            // Patterns zinakaguliwa kwenye kodi BILA comments (hakuna false-positive).
            let code = strip_comments(content, lang);
            for m in re.find_iter(&code).take(10) {
                let line = code[..m.start()].matches('\n').count() + 1;
                findings.push(json!({
                    "severity": "error",
                    "rule": "forbidden",
                    "message": format!("{name} — mstari {line}: pattern iligundulika"),
                }));
            }
        }
    }
}

/// Linter kuu: kodi → ripoti ya ukaguzi (ok/findings/summary).
pub fn lint(content: &str, path_hint: &str) -> Value {
    let mut findings: Vec<Value> = Vec::new();
    let rules = match load_rules() {
        Ok(r) => r,
        Err(e) => return json!({ "ok": false, "error": e }),
    };

    // 1) Lugha (kutoka path au hint ya manual)
    let lang_key = detect_language(path_hint)
        .or_else(|| detect_language(content)) // fallback: heuristics ya laini
        .unwrap_or("javascript");

    // 2) Kanuni za lugha husika + aliases
    let langs = rules.get("languages").cloned().unwrap_or(json!({}));
    let (lang_rules, actual_lang) = langs
        .as_object()
        .and_then(|m| {
            m.get(lang_key)
                .map(|r| (r.clone(), lang_key.to_string()))
                .or_else(|| {
                    m.iter().find_map(|(k, v)| {
                        let aliases = v.get("aliases").and_then(|a| a.as_array())?;
                        aliases.iter().find_map(|a| {
                            a.as_str().map(|s| s.to_string()).filter(|s| {
                                path_hint.rsplit('.').next().map(|e| s == e).unwrap_or(false)
                            })
                        })?;
                        Some((v.clone(), k.clone()))
                    })
                })
        })
        .unwrap_or((json!({}), lang_key.to_string()));

    if let Some(arr) = lang_rules.get("must_match").and_then(|v| v.as_array()) {
        for c in arr {
            run_check(content, c, &actual_lang, &mut findings);
        }
    }
    if let Some(arr) = lang_rules.get("forbidden").and_then(|v| v.as_array()) {
        for c in arr {
            run_check(content, c, &actual_lang, &mut findings);
        }
    }

    // 3) Sheria za usalama GLOBAL (kila lugha — hali ya juu)
    if let Some(sec) = rules.get("security_global") {
        if let Some(arr) = sec.get("forbidden_all_languages").and_then(|v| v.as_array()) {
            for c in arr {
                run_check(content, c, &actual_lang, &mut findings);
            }
        }
    }

    let errors = findings.len();
    json!({
        "ok": errors == 0,
        "language": actual_lang,
        "path": path_hint,
        "errors": errors,
        "findings": findings,
        "summary": if errors == 0 {
            "Kodi imepita ukaguzi wote — syntax + usalama (hakuna kosa)".to_string()
        } else {
            format!("{errors} kosa — rekebisha kisha rudia selfcheck")
        },
    })
}

// ---------------------------------------------------------------------------
// POST /api/ide/selfcheck — MZUNGUZO wa KUJISAHHIHISHA endpoint
// ---------------------------------------------------------------------------
pub async fn ide_selfcheck(Json(r): Json<Value>) -> Json<Value> {
    let Some(content) = r["content"].as_str() else {
        return Json(json!({ "ok": false, "error": "content ni lazima" }));
    };
    let path = r["path"].as_str().unwrap_or("").to_string();
    let mut report = lint(content, &path);
    /* KANUNI ZA MRADI MAALUM (Kundi la 4) — jibu lingie pamoja ili agent
       aone masharti ya mradi WAKATI linter inazikagua (transparency + Rules):
       data-driven kutoka data/code_rules/project_rules.json. */
    report["project_rules"] = project_rules();
    Json(report)
}

/// KANUNI ZA MRADI MAALUM: data/code_rules/project_rules.json — faili
/// likikosekana au JSON mbaya → `null` (hakuna kanuni za bandia).
fn project_rules() -> Value {
    let path = crate::data_io::data_dir()
        .join("code_rules")
        .join("project_rules.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .unwrap_or(Value::Null)
}

/// GET /api/ide/selfcheck/rules — enable UI/agent kuona kanuni zote (transparency)
/// — lugha 19 + KANUNI ZA MRADI (project_rules.json) pamoja.
pub async fn rules_view() -> Json<Value> {
    match load_rules() {
        Ok(mut rules) => {
            rules["project_rules"] = project_rules();
            Json(rules)
        }
        Err(e) => Json(json!({ "ok": false, "error": e })),
    }
}

// ---------------------------------------------------------------------------
// POST /api/ide/arch — SOMA ARCHITECTURE NZIMA YA MRADI (kwa model ndogo)
// ---------------------------------------------------------------------------
// Scanner halisi: huezwa workspace, inaonyesha muundo (directories, extensions,
// files kubwa/nyingi, entry points, manifests) — hii ndiyo "context" ambayo
// model ndogo inasoma KABLA ya ku-code, ili i-code uhalisia na structure halisi.
// Hakuna mock: ezote huzza filesystem halisi (workspace root ya IDE).
use axum::extract::Query as QQuery;
use std::collections::HashMap as StdMap;

pub async fn ide_arch(QQuery(q): QQuery<StdMap<String, String>>) -> Json<Value> {
    let root = crate::data_io::data_dir();
    let max_files: usize = q
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(400);
    let ext_filter = q.get("exts").map(|s| s.to_string());

    let mut by_ext: std::collections::BTreeMap<String, usize> = Default::default();
    let mut dirs: std::collections::BTreeSet<String> = Default::default();
    let mut files: Vec<Value> = Vec::new();
    let mut scanned: usize = 0;
    let mut total_bytes: u64 = 0;

    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out_files: &mut Vec<Value>,
        out_dirs: &mut std::collections::BTreeSet<String>,
        out_ext: &mut std::collections::BTreeMap<String, usize>,
        counter: &mut usize,
        total_bytes: &mut u64,
    ) {
        if *counter > 1200 {
            return;
        }
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| e.file_name());
            for e in entries {
                let p = e.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    // skip vendored/build/hidden
                    if matches!(name, "node_modules" | "target" | "build" | "dist" | ".git") {
                        continue;
                    }
                    out_dirs.insert(
                        p.strip_prefix(root)
                            .unwrap_or(&p)
                            .to_string_lossy()
                            .to_string(),
                    );
                    walk(root, &p, out_files, out_dirs, out_ext, counter, total_bytes);
                } else if p.is_file() {
                    *counter += 1;
                    let rel = p
                        .strip_prefix(root)
                        .unwrap_or(&p)
                        .to_string_lossy()
                        .to_string();
                    let ext = p
                        .extension()
                        .and_then(|x| x.to_str())
                        .unwrap_or("(none)")
                        .to_string();
                    let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                    *total_bytes += size;
                    *out_ext.entry(ext.clone()).or_insert(0) += 1;
                    out_files.push(json!({ "path": rel, "size": size }));
                }
            }
        }
    }

    walk(
        &root,
        &root,
        &mut files,
        &mut dirs,
        &mut by_ext,
        &mut scanned,
        &mut total_bytes,
    );

    // Entry points + manifests halisi (nini ina-endeshwa project hii)
    let keys = [
        "package.json", "Cargo.toml",
        "go.mod", "pom.xml", "build.gradle", "Dockerfile", "docker-compose.yml",
        "fly.toml", ".env.example",
    ];
    let manifests: Vec<Value> = keys
        .iter()
        .filter_map(|k| {
            let p = root.join(k);
            if p.is_file() {
                Some(json!({ "path": k, "size": p.metadata().map(|m| m.len()).unwrap_or(0) }))
            } else {
                None
            }
        })
        .collect();

    // Large files context (kama model ndogo inasoma rezime za mikubwa)
    let mut sorted = files.clone();
    sorted.sort_by(|a, b| {
        b.get("size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .cmp(&a.get("size").and_then(|v| v.as_u64()).unwrap_or(0))
    });
    let top_files: Vec<Value> = sorted
        .into_iter()
        .take(20)
        .filter(|f| {
            ext_filter
                .as_ref()
                .map(|ef| {
                    f.get("path")
                        .and_then(|v| v.as_str())
                        .map(|p| p.rsplit('.').next().map(|e| ef.contains(e)).unwrap_or(false))
                        .unwrap_or(false)
                })
                .unwrap_or(true)
        })
        .collect();

    Json(json!({
        "ok": true,
        "root": root.display().to_string(),
        "scanned_files": scanned,
        "total_size_mb": (total_bytes as f64 / 1_048_576.0 * 100.0).round() / 100.0,
        "directories": dirs.len(),
        "by_extension": by_ext,
        "manifests": manifests,
        "top_files_by_size": top_files,
        "top_directories": dirs.iter().take(30).collect::<Vec<_>>(),
        "usage": "Ma model ndogo (Frontier Coder) inasoma hii architecture context KABLA ya coding — hakuna miezikano ya syntax; endesha self-check kila commit"
    }))
}

#[cfg(test)]
mod strip_tests {
    use super::*;

    #[test]
    fn strip_comments_inaondoa_comment_za_rust_na_js() {
        let code = "fn main() {\n    // .unwrap() hapa ni maelezo\n    let x = foo();\n}\n";
        let out = strip_comments(code, "rust");
        assert!(!out.contains(".unwrap()"), "comment iondolewe");
        assert!(out.contains("let x = foo();"), "kodi ibaki");
    }

    #[test]
    fn strip_comments_inaondoa_hash_za_ruby() {
        let code = "def f\n    # .unwrap() maelezo\n    return 1\nend\n";
        let out = strip_comments(code, "ruby");
        assert!(!out.contains(".unwrap()"));
        assert!(out.contains("return 1"));
    }

    #[test]
    fn strip_comments_inaheshimu_mabano_ya_kodi() {
        let code = "function f() {\n  /* block comment */\n  return {a: 1};\n}\n";
        let out = strip_comments(code, "javascript");
        assert!(out.contains("{a: 1}"));
        assert!(!out.contains("block comment"));
    }

    #[test]
    fn lint_haitoi_false_positive_kwa_unwrap_kwenye_comment() {
        let code = "fn main() {\n    // tumia match, sio .unwrap()\n    let _ = run();\n}\n";
        let r = lint(code, "src/main.rs");
        assert!(r["ok"].as_bool().unwrap_or(false), "comment isizidiwe: {r}");
    }

    #[test]
    fn lint_inagundua_unwrap_halisi() {
        let code = "fn main() {\n    let x = foo().unwrap();\n}\n";
        let r = lint(code, "src/main.rs");
        assert!(!r["ok"].as_bool().unwrap_or(true), "unwrap halisi igundulike");
    }
}

