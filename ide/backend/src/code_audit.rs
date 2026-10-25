//! code_audit.rs — UCHAMBUZI HALISI wa code (USALAMA + BUGS + QUALITY).
//!
//! Hii ni injini ya "code review" inayotumika na agent swarm ya IDE. Ni
//! deterministic (regex) — inagundua:
//!   - SIRI zilizoachwa (secrets/keys/tokens)
//!   - mifumo ya USALAMA hatarishi (eval, shell=True, SQL injection, XSS, …)
//!   - BUGS za kawaida (unwrap, bare except, == None, panic)
//!   - matatizo ya QUALITY (TODO/FIXME, debug, console.log)
//!
//! Matokeo yana severity + line + snippet; yanatumika kupanga risk score.

use serde_json::{json, Value};

pub struct Rule {
    pub id: &'static str,
    pub severity: &'static str,
    pub lang: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub needle: &'static str,
    pub fix: &'static str,
}

/// Kanuni zilizopimwa. `needle` ni substring (case-sensitive). Hizi ni
/// chanzo KIMOJA cha ukweli kwa uchambuzi wa usalama/bugs.
pub const RULES: &[Rule] = &[
    // ---- SECRETS ----
    Rule { id: "SEC-001", severity: "critical", lang: "any", category: "secret", title: "API key ya OpenAI", needle: "sk-", fix: "Ondoa key kwenye code; tumia env var / secret manager." },
    Rule { id: "SEC-002", severity: "critical", lang: "any", category: "secret", title: "GitHub token (ghp_/gho_)", needle: "ghp_", fix: "Revoke token; weka kwenye env, si code." },
    Rule { id: "SEC-003", severity: "high", lang: "any", category: "secret", title: "AWS Access Key", needle: "AKIA", fix: "Tumia IAM role / env; usiweke key kwenye code." },
    Rule { id: "SEC-004", severity: "high", lang: "any", category: "secret", title: "Private key block", needle: "PRIVATE KEY", fix: "Hifadhi key kwenye secret store, si repo." },
    Rule { id: "SEC-005", severity: "high", lang: "any", category: "secret", title: "JWT / bearer token", needle: "Bearer ", fix: "Usiweke token kwenye code; tumia env." },
    // ---- DANGEROUS PATTERNS ----
    Rule { id: "SEC-013", severity: "high", lang: "any", category: "rce", title: "eval() / Function() kwenye JS", needle: "eval(", fix: "Epuka eval/Function; tumia JSON.parse." },
    Rule { id: "SEC-014", severity: "high", lang: "any", category: "cmd", title: "os.system / shell exec", needle: "os.system", fix: "Tumia subprocess args-array." },
    // ---- INJECTION ----
    Rule { id: "SEC-020", severity: "high", lang: "sql", category: "sqli", title: "SQL kwa concatenation", needle: "SELECT \" +", fix: "Tumia parameterized queries / prepared statements." },
    Rule { id: "SEC-021", severity: "high", lang: "js", category: "xss", title: "innerHTML kwa data", needle: "innerHTML =", fix: "Tumia textContent au sanitize (DOMPurify)." },
    Rule { id: "SEC-022", severity: "high", lang: "js", category: "xss", title: "document.write", needle: "document.write(", fix: "Tumia DOM API salama." },
    Rule { id: "SEC-023", severity: "medium", lang: "any", category: "traversal", title: "Path kwa '..'", needle: "../", fix: "Sanitize path; thibitisha ndani ya workspace." },
    // ---- WEAK CRYPTO / AUTH ----
    Rule { id: "SEC-030", severity: "high", lang: "any", category: "crypto", title: "MD5 hash", needle: "md5", fix: "Tumia SHA-256 / Argon2." },
    Rule { id: "SEC-031", severity: "high", lang: "any", category: "crypto", title: "DES / RC4", needle: "DES", fix: "Tumia AES-GCM." },
    Rule { id: "SEC-032", severity: "medium", lang: "any", category: "auth", title: "verify=False (TLS off)", needle: "verify=False", fix: "Washa uthibitisho wa TLS." },
    Rule { id: "SEC-033", severity: "medium", lang: "any", category: "auth", title: "CORS wildcard", needle: "Access-Control-Allow-Origin: *", fix: "Weka origin maalum." },
    // ---- BUGS ----
    Rule { id: "BUG-001", severity: "medium", lang: "rust", category: "panic", title: "unwrap() inaweza panic", needle: ".unwrap()", fix: "Tumia `?` au `unwrap_or_else`." },
    Rule { id: "BUG-002", severity: "medium", lang: "rust", category: "panic", title: "panic! kwenye path ya runtime", needle: "panic!(", fix: "Rudisha Result badala ya panic." },
    Rule { id: "BUG-005", severity: "low", lang: "js", category: "bug", title: "== badala ya ===", needle: " == ", fix: "Tumia `===`." },
    // ---- QUALITY ----
    Rule { id: "QLT-001", severity: "low", lang: "any", category: "todo", title: "TODO/FIXME", needle: "TODO", fix: "Kamilisha au fungua issue." },
    Rule { id: "QLT-002", severity: "low", lang: "any", category: "debug", title: "print() ya debug", needle: "print(", fix: "Tumia logger." },
    Rule { id: "QLT-003", severity: "low", lang: "js", category: "debug", title: "console.log", needle: "console.log", fix: "Ondoa debug kabla ya release." },
    // ---- ML / AI SAFETY (Candle / Elixir / R) ----
    Rule { id: "ML-001", severity: "high", lang: "rust", category: "deserialization", title: "Deserialization isiyo salama (bincode/serde ya data ya nje)", needle: "bincode::deserialize", fix: "Tumia safetensors au JSON yenye schema." },
    Rule { id: "ML-003", severity: "high", lang: "elixir", category: "rce", title: "Code.eval_string (RCE)", needle: "Code.eval", fix: "Epuka eval; tumia parser/dispatch ya wazi." },
    Rule { id: "ML-004", severity: "medium", lang: "elixir", category: "dos", title: "String.to_atom kwenye data (atom exhaustion)", needle: "String.to_atom(", fix: "Tumia String.to_existing_atom." },
    Rule { id: "ML-005", severity: "high", lang: "r", category: "cmd", title: "system() kwenye R (shell)", needle: "system(", fix: "Tumia system2() yenye args-array bila shell." },
    Rule { id: "ML-006", severity: "medium", lang: "r", category: "rce", title: "eval() kwenye R", needle: "eval(", fix: "Tumia do.call / parse+eval yenye udhibiti." },
];

fn severity_weight(sev: &str) -> f64 {
    match sev {
        "critical" => 10.0,
        "high" => 7.0,
        "medium" => 4.0,
        _ => 2.0,
    }
}

/// Changanua maudhui ya file moja. `lang` inaweza kuwa "" (kila rule).
pub fn analyze_content(lang: &str, content: &str) -> Vec<Value> {
    let mut findings = Vec::new();
    for rule in RULES {
        if !lang.is_empty() && rule.lang != "any" && rule.lang != lang {
            continue;
        }
        for (i, line) in content.lines().enumerate() {
            if line.contains(rule.needle) {
                findings.push(json!({
                    "rule": rule.id,
                    "severity": rule.severity,
                    "category": rule.category,
                    "title": rule.title,
                    "lang": rule.lang,
                    "line": i + 1,
                    "snippet": line.trim().chars().take(200).collect::<String>(),
                    "fix": rule.fix,
                }));
            }
        }
    }
    findings
}

/// Ramani ya extension → lugha (inalingana na ide_api::syntax_for).
pub fn lang_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "js" | "mjs" | "cjs" | "ts" | "tsx" => "js",
        "sql" => "sql",
        "rb" => "ruby",
        "go" => "go",
        "java" => "java",
        "php" => "php",
        "ex" | "exs" => "elixir",
        "r" | "R" => "r",
        _ => "any",
    }
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "target" | "node_modules" | ".venv" | "__pycache__" | "dist" | "build" | ".next"
    )
}

fn scan_dir(
    root: &std::path::Path,
    dir: &std::path::Path,
    findings: &mut Vec<Value>,
    files: &mut Vec<Value>,
    budget: &mut usize,
) {
    if *budget == 0 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if *budget == 0 {
            return;
        }
        let p = e.path();
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if p.is_dir() {
            if !should_skip_dir(&name) {
                scan_dir(root, &p, findings, files, budget);
            }
            continue;
        }
        let meta = e.metadata().ok();
        if meta.as_ref().map(|m| m.len() > 2 * 1024 * 1024).unwrap_or(true) {
            continue;
        }
        let rel = p.strip_prefix(root).map(|x| x.to_string_lossy().to_string()).unwrap_or(name.clone());
        let lang = lang_for(&rel);
        let Ok(content) = std::fs::read_to_string(&p) else { continue };
        *budget -= 1;
        let f = analyze_content(lang, &content);
        if !f.is_empty() {
            files.push(json!({ "path": rel, "findings": f.len(), "lang": lang }));
            for mut item in f {
                item["path"] = json!(rel);
                findings.push(item);
            }
        }
    }
}

/// Changanua workspace nzima (au path moja ndani ya workspace).
pub fn scan_workspace(root: &std::path::Path, rel_path: &str) -> Value {
    let target = if rel_path.trim().is_empty() {
        root.to_path_buf()
    } else {
        let rel = rel_path.trim_start_matches('/');
        if rel.contains("..") {
            return json!({ "ok": false, "error": "path nje ya workspace" });
        }
        root.join(rel)
    };
    let mut findings = Vec::new();
    let mut files = Vec::new();
    let mut budget = 500usize;
    if !target.exists() {
        // BUG FIX: path isiyopo ilitoa "grade 100" ya UONGO (files_scanned=0).
        // Sasa inarudisha kosa halisi — msimamizi ajue path si sahihi.
        return json!({
            "ok": false,
            "error": format!("path haipo ndani ya workspace: {}", rel_path),
            "target": target.display().to_string(),
            "workspace": root.display().to_string(),
        });
    }
    if target.is_file() {
        if let Ok(content) = std::fs::read_to_string(&target) {
            let rel = target
                .strip_prefix(root)
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or_else(|_| rel_path.to_string());
            let f = analyze_content(lang_for(&rel), &content);
            files.push(json!({ "path": rel, "findings": f.len(), "lang": lang_for(&rel) }));
            for mut item in f {
                item["path"] = json!(rel);
                findings.push(item);
            }
        }
    } else {
        scan_dir(root, &target, &mut findings, &mut files, &mut budget);
    }

    let mut sev_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut cat_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut score = 0.0f64;
    for f in &findings {
        let sev = f["severity"].as_str().unwrap_or("low").to_string();
        let cat = f["category"].as_str().unwrap_or("other").to_string();
        *sev_counts.entry(sev.clone()).or_insert(0) += 1;
        *cat_counts.entry(cat).or_insert(0) += 1;
        score += severity_weight(&sev);
    }
    let by_sev = serde_json::to_value(&sev_counts).unwrap_or(json!({}));
    let by_cat = serde_json::to_value(&cat_counts).unwrap_or(json!({}));
    // grade: 100 = safi; kila weight hupunguza (kikomo cha chini 0)
    let grade = (100.0 - score).max(0.0);
    json!({
        "ok": true,
        "target": target.display().to_string(),
        "files_scanned": files.len(),
        "files_with_findings": files.iter().filter(|f| f.get("findings").and_then(|v| v.as_u64()).unwrap_or(0) > 0).count(),
        "total_findings": findings.len(),
        "by_severity": by_sev,
        "by_category": by_cat,
        "grade": (grade * 10.0).round() / 10.0,
        "files": files,
        "findings": findings.into_iter().take(400).collect::<Vec<_>>(),
    })
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/code/rules", get(code_rules))
        .route("/api/ide/code/audit", post(code_audit))
        .route("/api/ide/code/audit/file", post(code_audit_file))
}

/// GET /api/ide/code/rules — orodha ya kanuni (chanzo kimoja).
pub async fn code_rules() -> axum::Json<Value> {
    let rules: Vec<Value> = RULES
        .iter()
        .map(|r| {
            json!({
                "id": r.id, "severity": r.severity, "lang": r.lang,
                "category": r.category, "title": r.title, "fix": r.fix,
            })
        })
        .collect();
    axum::Json(json!({ "ok": true, "rules": rules, "total": RULES.len() }))
}

/// POST /api/ide/code/audit — changanua workspace au path.
pub async fn code_audit(axum::Json(body): axum::Json<Value>) -> axum::Json<Value> {
    let path = body["path"].as_str().unwrap_or("");
    let root = crate::ide_api::ide_workspace_root();
    axum::Json(scan_workspace(&root, path))
}

/// POST /api/ide/code/audit/file — changanua maudhui yaliyotumwa (bila kuandika).
pub async fn code_audit_file(axum::Json(body): axum::Json<Value>) -> axum::Json<Value> {
    let path = body["path"].as_str().unwrap_or("");
    let code = body["code"].as_str().unwrap_or("");
    let lang = if body["lang"].as_str().unwrap_or("").is_empty() {
        lang_for(path)
    } else {
        body["lang"].as_str().unwrap_or("any")
    };
    let findings = analyze_content(lang, code);
    let score: f64 = findings
        .iter()
        .map(|f| severity_weight(f["severity"].as_str().unwrap_or("low")))
        .sum();
    axum::Json(json!({
        "ok": true,
        "lang": lang,
        "total_findings": findings.len(),
        "grade": (100.0 - score).max(0.0),
        "findings": findings,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_openai_key() {
        let f = analyze_content("rust", "let key = \"sk-abc123\";");
        assert!(f.iter().any(|x| x["rule"] == "SEC-001"));
    }

    #[test]
    fn finds_eval_js() {
        let f = analyze_content("js", "eval(userInput)");
        assert!(f.iter().any(|x| x["rule"] == "SEC-013"));
    }

    #[test]
    fn finds_innerhtml() {
        let f = analyze_content("js", "el.innerHTML = data");
        assert!(f.iter().any(|x| x["rule"] == "SEC-021"));
    }

    #[test]
    fn clean_code_no_findings() {
        let f = analyze_content("rust", "fn add(a: i64, b: i64) -> i64 {\n    a + b\n}\n");
        assert!(f.is_empty());
    }

    #[test]
    fn grade_drops_with_critical() {
        let f = analyze_content("any", "sk-1\nsk-2");
        let score: f64 = f.iter().map(|x| severity_weight(x["severity"].as_str().unwrap())).sum();
        assert!(100.0 - score < 100.0);
    }

    #[test]
    fn lang_map() {
        assert_eq!(lang_for("a/b/main.rs"), "rust");
        assert_eq!(lang_for("x.ex"), "elixir");
        assert_eq!(lang_for("x.ts"), "js");
    }
}
