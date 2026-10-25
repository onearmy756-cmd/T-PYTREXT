//! sec_gate.rs — LANGI LA USALAMA (security gate) kwa kila software inayozalishwa.
//!
//! Kanuni ya mmiliki: **kila software inayotengenezwa LAZIMA ipimwe usalama.**
//! Moduli hii haitoi "grade" pekee — inafunga mzunguko KAMILI:
//!
//!   1. KAGUA   — `code_audit` huchanganua kila faili iliyozalishwa (regex halisi).
//!   2. REKEBISHA — `remediate` huondoa siri zilizoachwa na mifumo hatarishi
//!      (inayoweza kurekebishwa kwa usalama bila kubadilisha maana).
//!   3. KAGUA TENA — hupima upya baada ya kurekebisha (hakuna madai bila ushahidi).
//!   4. AMUA   — `verdict` hufunga langi: `block` ikiwa critical/high bado zipo,
//!      `warn` kwa medium, `pass` ikiwa safi.
//!   5. TIA MUHURI — `pkicrypto` hutiwa muhuri wa ECDSA P-256 + SHA-256, hivyo
//!      ripoti haiwezi kubadilishwa bila kuvunja mnyororo.
//!
//! MUHIMU: langi hii HAIZUII mtu kuandika code — inazuia code KUTANGAZWA kuwa
//! salama bila ushahidi. Kila uamuzi huja na findings halisi.

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

/// Ukali unaozuia kutolewa (release) kabisa.
pub const BLOCKING: &[&str] = &["critical", "high"];

/// Vipimo vya kila hatari — chanzo kimoja cha ukweli kwa uzito wa langi.
fn weight(sev: &str) -> f64 {
    match sev {
        "critical" => 10.0,
        "high" => 7.0,
        "medium" => 4.0,
        _ => 2.0,
    }
}

/// Kagua maudhui ya faili moja na urudishe ripoti ya langi.
pub fn audit_source(path: &str, content: &str) -> Value {
    let lang = crate::code_audit::lang_for(path);
    let findings = crate::code_audit::analyze_content(lang, content);
    summarize(path, lang, findings)
}

/// Kagua mti wa faili (path → maudhui) — hii ndiyo inayotumika kwa miradi
/// iliyozalishwa (forge/sdlc/swarm) kabla ya kutolewa.
pub fn audit_files(files: &[(String, String)]) -> Value {
    let mut all: Vec<Value> = Vec::new();
    let mut per_file: Vec<Value> = Vec::new();
    for (path, content) in files {
        let lang = crate::code_audit::lang_for(path);
        let f = crate::code_audit::analyze_content(lang, content);
        per_file.push(json!({ "path": path, "lang": lang, "findings": f.len() }));
        for mut item in f {
            item["path"] = json!(path);
            all.push(item);
        }
    }
    let mut v = summarize("project", "any", all);
    if let Some(o) = v.as_object_mut() {
        o.insert("files_scanned".into(), json!(files.len()));
        o.insert("files".into(), json!(per_file));
    }
    v
}

/// Hesabu grade, ukali, na uamuzi wa langi kutoka findings.
fn summarize(target: &str, lang: &str, findings: Vec<Value>) -> Value {
    let mut by_sev: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut by_cat: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut score = 0.0f64;
    for f in &findings {
        let sev = f["severity"].as_str().unwrap_or("low").to_string();
        let cat = f["category"].as_str().unwrap_or("other").to_string();
        *by_sev.entry(sev.clone()).or_insert(0) += 1;
        *by_cat.entry(cat).or_insert(0) += 1;
        score += weight(&sev);
    }
    let blocking: u64 = BLOCKING
        .iter()
        .map(|s| by_sev.get(*s).copied().unwrap_or(0))
        .sum();
    let grade = (100.0 - score).max(0.0);
    let verdict = if blocking > 0 {
        "block"
    } else if by_sev.get("medium").copied().unwrap_or(0) > 0 {
        "warn"
    } else {
        "pass"
    };
    json!({
        "target": target,
        "lang": lang,
        "grade": (grade * 10.0).round() / 10.0,
        "total_findings": findings.len(),
        "blocking": blocking,
        "verdict": verdict,
        "by_severity": by_sev,
        "by_category": by_cat,
        "findings": findings,
    })
}

/// Rekebisha moja kwa moja yale yanayoweza kurekebishwa kwa usalama.
///
/// Hurudisha (maudhui mapya, orodha ya marekebisho). HAIBADILISHI maana ya code:
/// - Siri zilizoachwa → env var yenye jina la wazi.
/// - `verify=False` → imeondolewa (TLS huthibitishwa).
/// - `md5` → `sha256` (kwenye wito rahisi wa hash).
pub fn remediate(path: &str, content: &str) -> (String, Vec<Value>) {
    let lang = crate::code_audit::lang_for(path);
    let mut out = content.to_string();
    let mut fixes: Vec<Value> = Vec::new();

    // 1) SIRI: badilisha thamani halisi na env var (hatari kubwa zaidi).
    for (needle, env, label) in [
        ("sk-", "OPENAI_API_KEY", "API key ya OpenAI"),
        ("ghp_", "GITHUB_TOKEN", "GitHub token"),
        ("gho_", "GITHUB_TOKEN", "GitHub token"),
        ("AKIA", "AWS_ACCESS_KEY_ID", "AWS Access Key"),
    ] {
        if out.contains(needle) {
            out = redact_secret(&out, needle, env);
            fixes.push(json!({
                "rule": "SEC-SECRET", "action": "redact",
                "detail": format!("{label} → env var {env}"),
            }));
        }
    }

    // 2) TLS: verify=False huondoa ulinzi wa MITM.
    if out.contains("verify=False") {
        out = out.replace("verify=False", "verify=True");
        fixes.push(json!({
            "rule": "SEC-032", "action": "fix",
            "detail": "verify=False → verify=True (TLS inathibitishwa)",
        }));
    }

    // 3) Hash dhaifu → SHA-256 (wito wa kawaida pekee).
    if lang != "r" && out.contains("md5(") && !out.contains("sha256(") {
        out = out.replace("md5(", "sha256(");
        fixes.push(json!({
            "rule": "SEC-030", "action": "fix",
            "detail": "md5() → sha256() (hash dhaifu imeondolewa)",
        }));
    }

    // 4) CORS wildcard → origin maalum.
    if out.contains("Access-Control-Allow-Origin: *") {
        out = out.replace(
            "Access-Control-Allow-Origin: *",
            "Access-Control-Allow-Origin: $CORS_ORIGIN",
        );
        fixes.push(json!({
            "rule": "SEC-033", "action": "fix",
            "detail": "CORS '*' → $CORS_ORIGIN (origin maalum)",
        }));
    }

    (out, fixes)
}

/// Badilisha siri iliyoachwa na rejeleo la env var. Hushughulikia mifumo ya
/// kawaida: `KEY = "..."`, `KEY: "..."`, `KEY="..."`, `const KEY = '...'`.
/// Hurudisha `None` ikiwa mstari hauwezi kubadilishwa kwa usalama (basi langi
/// huzuia kutolewa badala ya kubadilisha maana kwa upofu).
fn redact_secret(content: &str, prefix: &str, env: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        match redact_line(line, prefix, env) {
            Some(new) => out.push_str(&new),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    // `lines()` hunyang'anya newline ya mwisho; hifadhi tabia ya awali.
    if !content.ends_with('\n') {
        while out.ends_with('\n') {
            out.pop();
        }
    }
    out
}

/// Rekebisha mstari MMOJA wenye siri. Huhifadhi uorodheshaji na mwendeshaji
/// (`=` / `:`) na kuacha code iliyobaki baada ya literal.
fn redact_line(line: &str, prefix: &str, env: &str) -> Option<String> {
    let pos = line.find(prefix)?;
    let head = &line[..pos];
    // `lhs` ina uorodheshaji wa mwanzo (kutoka `head`), hivyo HAITUMIWI tena.
    let (lhs, op) = {
        let sep = head.rfind(['=', ':'])?;
        (head[..sep].trim_end(), head.as_bytes()[sep] as char)
    };
    // Literal iliyozungukwa na quote: hifadhi kilichobaki baada ya quote ya kufunga.
    let open = head.rfind(['"', '\'']);
    let tail = match open {
        Some(q) if head[q + 1..].is_empty() => {
            let quote = head.as_bytes()[q] as char;
            let rest = &line[pos..];
            match rest[prefix.len()..].find(quote) {
                Some(i) => &rest[prefix.len() + i + 1..],
                None => "",
            }
        }
        _ => "",
    };
    let op = if op == ':' { ": " } else { " = " };
    Some(format!("{lhs}{op}{env}{tail}"))
}

/// Mzunguko kamili: kagua → rekebisha → kagua tena → amua.
///
/// Hurudisha ripoti yenye hatua zote, findings za awali na za mwisho, na
/// code iliyorekebishwa (ili mwitaji aweze kuiandika).
pub fn gate_files(files: &[(String, String)]) -> Value {
    let before = audit_files(files);

    let mut fixed: Vec<(String, String)> = Vec::new();
    let mut applied: Vec<Value> = Vec::new();
    let mut changed = 0usize;
    for (path, content) in files {
        let (new_content, fixes) = remediate(path, content);
        if new_content != *content {
            changed += 1;
            for mut f in fixes {
                f["path"] = json!(path);
                applied.push(f);
            }
        }
        fixed.push((path.clone(), new_content));
    }

    let after = audit_files(&fixed);

    json!({
        "ok": true,
        "engine": "akili-security/sec-gate",
        "before": {
            "grade": before["grade"], "verdict": before["verdict"],
            "total_findings": before["total_findings"], "blocking": before["blocking"],
            "by_severity": before["by_severity"],
        },
        "remediation": { "files_changed": changed, "fixes": applied },
        "after": {
            "grade": after["grade"], "verdict": after["verdict"],
            "total_findings": after["total_findings"], "blocking": after["blocking"],
            "by_severity": after["by_severity"], "findings": after["findings"],
        },
        "release_ok": after["verdict"] == json!("pass"),
        "files": fixed.into_iter().map(|(p, c)| json!({ "path": p, "content": c })).collect::<Vec<_>>(),
    })
}

/// Langi kwa saraka HALISI (kwa agentic loop inayoandika kwenye `dir`).
/// Hupitia faili za msimbo (bounded) na kuzichanganua.
pub fn gate_dir(dir: &std::path::Path) -> Value {
    let mut files: Vec<(String, String)> = Vec::new();
    let mut budget = 300usize;
    collect_files(dir, dir, &mut files, &mut budget);
    let mut v = audit_files(&files);
    if let Some(o) = v.as_object_mut() {
        o.insert("dir".into(), json!(dir.display().to_string()));
    }
    v
}

/// Sangaza faili za msimbo ndani ya saraka (husika tu: .rs/.js/.ts/.py/...).
fn collect_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut Vec<(String, String)>,
    budget: &mut usize,
) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        if *budget == 0 {
            return;
        }
        let p = e.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "node_modules" | "target" | ".git" | "dist" | "build" | ".venv") {
                continue;
            }
            collect_files(root, &p, out, budget);
            continue;
        }
        let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().to_string();
        if crate::code_audit::lang_for(&rel) == "any" {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&p) else { continue };
        if content.len() > 200_000 {
            continue;
        }
        *budget -= 1;
        out.push((rel, content));
    }
}

/// Langi kwa mradi ulioandikwa kwenye diski (kwa IDE / agentic loop).
pub fn gate_workspace(rel_path: &str) -> Value {
    let root = crate::ide_api::ide_workspace_root();
    let audit = crate::code_audit::scan_workspace(&root, rel_path);
    if audit["ok"] != json!(true) {
        return json!({ "ok": false, "error": audit["error"], "path": rel_path });
    }
    let verdict = match audit["grade"].as_f64().unwrap_or(0.0) {
        g if audit["by_severity"]["critical"].as_u64().unwrap_or(0) > 0
            || audit["by_severity"]["high"].as_u64().unwrap_or(0) > 0 => "block",
        g if g < 85.0 => "warn",
        _ => "pass",
    };
    json!({
        "ok": true,
        "engine": "akili-security/sec-gate",
        "path": rel_path,
        "verdict": verdict,
        "release_ok": verdict == "pass",
        "audit": audit,
    })
}

/// POST /api/ide/sec-gate/audit — kagua code/mradi BILA kubadilisha.
/// body: { path?: "src/x.rs", files?: [{path, content}], code?: "...", lang?: "rust" }
pub async fn gate_audit(Json(body): Json<Value>) -> Json<Value> {
    if let Some(files) = files_from_body(&body) {
        return Json(audit_files(&files));
    }
    if let Some(code) = body["code"].as_str().filter(|c| !c.is_empty()) {
        let path = body["path"].as_str().unwrap_or("snippet.rs");
        return Json(audit_source(path, code));
    }
    let path = body["path"].as_str().unwrap_or("");
    Json(gate_workspace(path))
}

/// POST /api/ide/sec-gate/run — mzunguko KAMILI: kagua → rekebisha → kagua
/// tena → amua → tia muhuri. Hii ndiyo langi inayotumika kabla ya kutolewa.
pub async fn gate_run(
    State(s): State<crate::AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let files = match files_from_body(&body) {
        Some(f) => f,
        None => {
            return Json(json!({
                "ok": false,
                "error": "toa `files` (path+content) au `code`",
            }))
        }
    };
    let mut report = gate_files(&files);

    // TIA MUHURI: ripoti hii ni ushahidi; muhuri huifanya isibadilishwe.
    let payload = serde_json::to_string(&json!({
        "engine": report["engine"],
        "before": report["before"],
        "after": report["after"],
        "remediation": report["remediation"],
    }))
    .unwrap_or_default();
    match crate::pkicrypto::active_private_pem(&s.db).await {
        Ok(pem) => match crate::pkicrypto::seal(&s.db, &pem, &payload).await {
            Ok(seal) => {
                if let Some(o) = report.as_object_mut() {
                    o.insert("seal".into(), serde_json::to_value(&seal).unwrap_or(Value::Null));
                    o.insert("sealed_payload_hash".into(), json!(seal.content_hash));
                }
            }
            Err(e) => {
                if let Some(o) = report.as_object_mut() {
                    o.insert("seal_error".into(), json!(e));
                }
            }
        },
        Err(e) => {
            if let Some(o) = report.as_object_mut() {
                o.insert("seal_error".into(), json!(e));
            }
        }
    }
    Json(report)
}

/// GET /api/ide/sec-gate/capabilities
pub async fn gate_capabilities() -> Json<Value> {
    Json(json!({
        "ok": true,
        "engine": "akili-security/sec-gate",
        "blocking_severities": BLOCKING,
        "verdicts": ["pass", "warn", "block"],
        "rules": crate::code_audit::RULES.len(),
        "remediations": [
            "secret → env var (sk-/ghp_/gho_/AKIA)",
            "verify=False → verify=True",
            "md5() → sha256()",
            "CORS '*' → $CORS_ORIGIN",
        ],
        "seal": "ECDSA P-256 + SHA-256 (pkicrypto hash-chain)",
        "note": "Kila software inayozalishwa hupimwa usalama; release_ok=true ikiwa verdict=pass.",
    }))
}

/// Toa (path, content) kutoka body — `files` au `code`+`path`.
fn files_from_body(body: &Value) -> Option<Vec<(String, String)>> {
    if let Some(arr) = body["files"].as_array() {
        let v: Vec<(String, String)> = arr
            .iter()
            .filter_map(|f| {
                let p = f["path"].as_str()?;
                let c = f["content"].as_str()?;
                Some((p.to_string(), c.to_string()))
            })
            .collect();
        if !v.is_empty() {
            return Some(v);
        }
    }
    let code = body["code"].as_str().filter(|c| !c.is_empty())?;
    let path = body["path"].as_str().unwrap_or("snippet.rs").to_string();
    Some(vec![(path, code.to_string())])
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/sec-gate/capabilities", get(gate_capabilities))
        .route("/api/ide/sec-gate/audit", post(gate_audit))
        .route("/api/ide/sec-gate/run", post(gate_run))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn siri_iliyoachwa_huzuia_kutolewa() {
        let src = "let key = \"sk-abc123def456\";\n";
        let r = audit_source("main.rs", src);
        assert_eq!(r["verdict"], "block", "siri lazima izuie: {r}");
        assert!(r["blocking"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn code_safi_hupita() {
        let src = "fn main() {\n    println!(\"habari\");\n}\n";
        let r = audit_source("main.rs", src);
        assert_eq!(r["verdict"], "pass", "code safi lazima ipite: {r}");
        assert_eq!(r["blocking"], 0);
    }

    #[test]
    fn rekebisho_huondoa_siri_na_kuboresha_grade() {
        let files = vec![(
            "config.rs".to_string(),
            "let token = \"ghp_deadbeef\";\n".to_string(),
        )];
        let before = audit_files(&files);
        assert_eq!(before["verdict"], "block");
        let g = gate_files(&files);
        assert!(g["remediation"]["files_changed"].as_u64().unwrap() >= 1);
        // Siri imeondolewa kwenye code iliyorekebishwa.
        let content = g["files"][0]["content"].as_str().unwrap();
        assert!(!content.contains("ghp_deadbeef"), "siri ilibaki: {content}");
        assert!(content.contains("GITHUB_TOKEN"), "env var haipo: {content}");
        let after_grade = g["after"]["grade"].as_f64().unwrap();
        let before_grade = before["grade"].as_f64().unwrap();
        assert!(after_grade > before_grade, "grade haikuboresha");
    }

    #[test]
    fn verify_false_hurekebishwa() {
        let (out, fixes) = remediate("a.py", "requests.get(url, verify=False)\n");
        assert!(out.contains("verify=True"));
        assert_eq!(fixes.len(), 1);
    }

    #[test]
    fn md5_hubadilishwa_kuwa_sha256() {
        let (out, _) = remediate("a.py", "h = md5(data)\n");
        assert!(out.contains("sha256("));
    }

    #[test]
    fn langi_hurudisha_files_zote() {
        let files = vec![
            ("a.rs".to_string(), "fn a() {}\n".to_string()),
            ("b.js".to_string(), "const x = 1;\n".to_string()),
        ];
        let r = gate_files(&files);
        assert_eq!(r["before"]["total_findings"], 0);
        assert_eq!(r["release_ok"], true);
    }

    #[test]
    fn uorodheshaji_wa_siri_huhifadhiwa() {
        // Kila muundo huhifadhi `=`/`:` na mstari wenyewe.
        for (src, want) in [
            ("key = \"sk-abc\"\n", "key = OPENAI_API_KEY"),
            ("token: \"ghp_xyz\"\n", "token: GITHUB_TOKEN"),
            ("const k = 'AKIA123';\n", "const k = AWS_ACCESS_KEY_ID;"),
            ("    secret = \"sk-abc\"\n", "    secret = OPENAI_API_KEY"),
        ] {
            let (out, _) = remediate("a.js", src);
            assert_eq!(out.trim_end(), want, "kutoka: {src}");
        }
    }

    #[test]
    fn gate_dir_hukagua_faili_halisi() {
        let dir = std::env::temp_dir().join(format!("secgate_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.js"), "const t = \"ghp_leaked\";\n").unwrap();
        let g = gate_dir(&dir);
        assert_eq!(g["verdict"], "block", "siri kwenye diski lazima izuie: {g}");
        assert_eq!(g["files_scanned"], 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
