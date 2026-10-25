//! ide_swarm.rs — MTECHOS IDE AGENT SWARM: agents 50 wanaofanya kazi tofauti.
//!
//! Huunganisha: GENERATION (code halisi), SECURITY AUDIT (code_audit + ThreatSwarm),
//! CODE REVIEW (bugs/quality), TEST (lab_test), DOCS/PLAN/DEPLOY (templates halisi),
//! ANALYTICS (R engine). Kila agent hurudisha ripoti HALISI (artifacts/findings).
//!
//! Kanuni: hakuna mock. Audit/security agents hutumia `code_audit` (regex halisi),
//! test agent hutumia `ide_lab::lab_test` (build/test halisi), analytics hutumia
//! `r_engine` (Rscript halisi), generation hutumia LLM halisi (chat::ask) au
//! templates deterministic zinazozalisha file halisi.

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cap {
    Gen,
    Audit,
    Test,
    Docs,
    Plan,
    Analyze,
    Deploy,
    Research,
}

pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    pub domain: &'static str,
    pub cap: Cap,
    /// Audit: category ya kuchuja findings. Gen: template key. Analyze: R task.
    pub focus: &'static str,
}

/// AGENTS 50 — kila mmoja na kazi yake maalum.
pub const AGENTS: &[Agent] = &[
    // --- PRODUCT (3) ---
    Agent { id: "product-manager", name: "Meneja Bidhaa", domain: "product", cap: Cap::Plan, focus: "product" },
    Agent { id: "business-analyst", name: "Mchanganuzi Biashara", domain: "product", cap: Cap::Plan, focus: "business" },
    Agent { id: "ux-designer", name: "Mbunifu UX", domain: "product", cap: Cap::Docs, focus: "ux" },
    // --- ARCHITECTURE (4) ---
    Agent { id: "architect", name: "Mbunifu Mkuu", domain: "architecture", cap: Cap::Plan, focus: "architecture" },
    Agent { id: "api-designer", name: "Mbunifu API", domain: "architecture", cap: Cap::Plan, focus: "api" },
    Agent { id: "db-designer", name: "Mbunifu DB", domain: "architecture", cap: Cap::Deploy, focus: "db" },
    Agent { id: "systems-designer", name: "Mbunifu Mifumo", domain: "architecture", cap: Cap::Plan, focus: "systems" },
    // --- GENERATION (8) ---
    Agent { id: "fullstack-generator", name: "Jenereta Fullstack", domain: "generation", cap: Cap::Gen, focus: "fullstack" },
    Agent { id: "frontend-generator", name: "Jenereta Frontend", domain: "generation", cap: Cap::Gen, focus: "frontend" },
    Agent { id: "backend-generator", name: "Jenereta Backend", domain: "generation", cap: Cap::Gen, focus: "backend" },
    Agent { id: "mobile-generator", name: "Jenereta Mobile", domain: "generation", cap: Cap::Gen, focus: "mobile" },
    Agent { id: "iot-generator", name: "Jenereta IoT", domain: "generation", cap: Cap::Gen, focus: "iot" },
    Agent { id: "gaming-generator", name: "Jenereta Gaming", domain: "generation", cap: Cap::Gen, focus: "gaming" },
    Agent { id: "data-generator", name: "Jenereta Data", domain: "generation", cap: Cap::Gen, focus: "data" },
    Agent { id: "api-generator", name: "Jenereta API", domain: "generation", cap: Cap::Gen, focus: "api" },
    // --- FRONTEND (3) ---
    Agent { id: "ui-engineer", name: "Mhandisi UI", domain: "frontend", cap: Cap::Gen, focus: "ui" },
    Agent { id: "css-engineer", name: "Mhandisi CSS", domain: "frontend", cap: Cap::Gen, focus: "css" },
    Agent { id: "a11y-auditor", name: "Mkaguzi Ufikivu", domain: "frontend", cap: Cap::Audit, focus: "a11y" },
    // --- BACKEND (3) ---
    Agent { id: "api-engineer", name: "Mhandisi API", domain: "backend", cap: Cap::Gen, focus: "api" },
    Agent { id: "service-engineer", name: "Mhandisi Huduma", domain: "backend", cap: Cap::Gen, focus: "service" },
    Agent { id: "auth-engineer", name: "Mhandisi Uthibitisho", domain: "backend", cap: Cap::Audit, focus: "auth" },
    // --- DATA / AI (4) ---
    Agent { id: "data-scientist", name: "Mwanasayansi Data", domain: "data", cap: Cap::Analyze, focus: "iot_stats" },
    Agent { id: "ml-engineer", name: "Mhandisi ML", domain: "data", cap: Cap::Analyze, focus: "forecast" },
    Agent { id: "r-analyst", name: "Mchanganuzi R", domain: "data", cap: Cap::Analyze, focus: "summary" },
    Agent { id: "statistician", name: "Mwanastatistiki", domain: "data", cap: Cap::Analyze, focus: "anomaly" },
    // --- SECURITY (10) ---
    Agent { id: "security-lead", name: "Kiongozi Usalama", domain: "security", cap: Cap::Audit, focus: "" },
    Agent { id: "secrets-scanner", name: "Skana Siri", domain: "security", cap: Cap::Audit, focus: "secret" },
    Agent { id: "injection-auditor", name: "Mkaguzi Injection", domain: "security", cap: Cap::Audit, focus: "sqli" },
    Agent { id: "auth-auditor", name: "Mkaguzi Uthibitisho", domain: "security", cap: Cap::Audit, focus: "auth" },
    Agent { id: "crypto-auditor", name: "Mkaguzi Crypto", domain: "security", cap: Cap::Audit, focus: "crypto" },
    Agent { id: "xss-auditor", name: "Mkaguzi XSS", domain: "security", cap: Cap::Audit, focus: "xss" },
    Agent { id: "path-traversal-auditor", name: "Mkaguzi Path", domain: "security", cap: Cap::Audit, focus: "traversal" },
    Agent { id: "rce-auditor", name: "Mkaguzi RCE", domain: "security", cap: Cap::Audit, focus: "rce" },
    Agent { id: "threat-modeler", name: "Mwanamitindo Vitisho", domain: "security", cap: Cap::Plan, focus: "threat" },
    Agent { id: "pentest-coordinator", name: "Mratibu Pentest", domain: "security", cap: Cap::Research, focus: "pentest" },
    // --- QUALITY (5) ---
    Agent { id: "code-reviewer", name: "Mkaguzi Code", domain: "quality", cap: Cap::Audit, focus: "" },
    Agent { id: "bug-hunter", name: "Mwinda Bugs", domain: "quality", cap: Cap::Audit, focus: "panic" },
    Agent { id: "test-engineer", name: "Mhandisi Test", domain: "quality", cap: Cap::Test, focus: "test" },
    Agent { id: "rust-reviewer", name: "Mkaguzi Rust", domain: "quality", cap: Cap::Audit, focus: "rust" },
    Agent { id: "perf-engineer", name: "Mhandisi Utendaji", domain: "quality", cap: Cap::Audit, focus: "quality" },
    // --- INFRA (4) ---
    Agent { id: "devops", name: "DevOps", domain: "infra", cap: Cap::Deploy, focus: "ci" },
    Agent { id: "docker-engineer", name: "Mhandisi Docker", domain: "infra", cap: Cap::Deploy, focus: "docker" },
    Agent { id: "ci-engineer", name: "Mhandisi CI", domain: "infra", cap: Cap::Deploy, focus: "ci" },
    Agent { id: "cloud-engineer", name: "Mhandisi Cloud", domain: "infra", cap: Cap::Deploy, focus: "cloud" },
    // --- DOCS (3) ---
    Agent { id: "tech-writer", name: "Mwandishi Teknolojia", domain: "docs", cap: Cap::Docs, focus: "tech" },
    Agent { id: "readme-writer", name: "Mwandishi README", domain: "docs", cap: Cap::Docs, focus: "readme" },
    Agent { id: "api-doc-writer", name: "Mwandishi API Docs", domain: "docs", cap: Cap::Docs, focus: "api" },
    // --- OPS (3) ---
    Agent { id: "release-manager", name: "Meneja Release", domain: "ops", cap: Cap::Deploy, focus: "release" },
    Agent { id: "observability-engineer", name: "Mhandisi Ufuatiliaji", domain: "ops", cap: Cap::Deploy, focus: "observability" },
    Agent { id: "compliance-officer", name: "Afisa Uzingatiaji", domain: "ops", cap: Cap::Audit, focus: "" },
];

pub fn catalog() -> Value {
    let agents: Vec<Value> = AGENTS
        .iter()
        .map(|a| {
            json!({
                "id": a.id, "name": a.name, "domain": a.domain,
                "capability": format!("{:?}", a.cap), "focus": a.focus,
            })
        })
        .collect();
    let mut by_domain: std::collections::BTreeMap<String, usize> = Default::default();
    for a in AGENTS {
        *by_domain.entry(a.domain.to_string()).or_insert(0) += 1;
    }
    json!({
        "ok": true,
        "total_agents": AGENTS.len(),
        "by_domain": by_domain,
        "agents": agents,
        "note": "Swarm ya agents 50 — generation, security audit, review, test, docs, analytics (Rust + R).",
    })
}

fn preview(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' || c == '_' {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    let t = out.trim_matches('-').to_string();
    if t.is_empty() { "project".into() } else { t.chars().take(40).collect() }
}

/// Templates za generation — huzalisha file HALISI zenye kufanya kazi.
fn generate_artifact(agent_id: &str, focus: &str, idea: &str, kind: &str) -> Value {
    let name = slug(idea);
    let (path, content, stack) = match focus {
        "frontend" | "ui" | "css" => (
            "web/index.html",
            format!(
                "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"/>\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/>\n<title>{idea}</title>\n<style>body{{font-family:system-ui;margin:0;background:#0d1117;color:#e6edf3}}header{{padding:32px}}.btn{{background:#58a6ff;color:#fff;border:0;border-radius:8px;padding:10px 18px}}</style>\n</head><body>\n<header><h1>{idea}</h1><p>Aina: {kind}</p><button class=\"btn\" onclick=\"document.getElementById('o').textContent='Hello '+new Date().toISOString()\">Bofya</button><p id=\"o\"></p></header>\n</body></html>\n"
            ),
            "html/css/js",
        ),
        "backend" | "api" | "service" => (
            "server/main.rs",
            format!(
                "// {idea} — backend (Rust, axum)\nuse axum::{{routing::get, Router, Json}};\nuse serde_json::json;\n\nasync fn health() -> Json<serde_json::Value> {{ Json(json!({{ \"ok\": true, \"app\": \"{name}\" }})) }}\n\n#[tokio::main]\nasync fn main() {{\n    let app = Router::new().route(\"/health\", get(health));\n    let listener = tokio::net::TcpListener::bind(\"0.0.0.0:8080\").await.unwrap();\n    axum::serve(listener, app).await.unwrap();\n}}\n"
            ),
            "rust/axum",
        ),
        "mobile" => (
            "mobile/index.html",
            format!(
                "<!DOCTYPE html><html><head><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/><title>{idea}</title></head>\n<body style=\"font-family:-apple-system;margin:0;padding:24px\"><h2>{idea}</h2><p>Mobile app ({kind})</p><button style=\"padding:14px 24px;border-radius:14px;border:0;background:#007AFF;color:#fff\">Anza</button></body></html>\n"
            ),
            "html/mobile",
        ),
        "iot" => (
            "iot/telemetry.R",
            format!(
                "# {idea} — IoT analytics (R)\nset.seed(42)\nn <- 48\nt <- seq_len(n)\ntemp <- 22 + 3*sin(t/6) + rnorm(n, 0, 0.4)\ncat(sprintf(\"n=%d mean=%.2f sd=%.2f\\n\", n, mean(temp), sd(temp)))\n"
            ),
            "r",
        ),
        "gaming" => (
            "game/index.html",
            format!(
                "<!DOCTYPE html><html><head><meta charset=\"utf-8\"/><title>{idea}</title></head>\n<body style=\"margin:0;background:#05070d;color:#8b949e;font:14px monospace;padding:24px\">{idea} — 3D game ({kind}). Tumia Three.js.</body></html>\n"
            ),
            "three.js",
        ),
        "data" => (
            "data/analysis.R",
            format!(
                "# {idea} — data analysis (R)\nx <- c(1,2,3,4,5,6,7,8,9,10)\ncat(sprintf(\"mean=%.2f sd=%.2f\\n\", mean(x), sd(x)))\n"
            ),
            "r",
        ),
        _ => (
            "README.md",
            format!("# {idea}\n\nAina: {kind}\n\n## Kuanza\n\n```bash\n# angalia stack\nls\n```\n\nImeundwa na agent `{agent_id}` (MTECHOS swarm).\n"),
            "docs",
        ),
    };
    json!({
        "artifact": path,
        "stack": stack,
        "content_preview": preview(&content, 400),
        "content_len": content.len(),
        "_content": content,
    })
}

fn doc_outline(focus: &str, idea: &str) -> Value {
    let sections = match focus {
        "readme" => vec!["Maelezo", "Usanidi", "Matumizi", "Mchango", "Leseni"],
        "api" => vec!["Uthibitisho", "Endpoints", "Mifano", "Makosa", "Kiwango"],
        "ux" => vec!["Watumiaji", "Mtiririko", "Wireframes", "Ufikivu", "Vipimo"],
        _ => vec!["Muhtasari", "Usanifu", "Utekelezaji", "Upimaji", "Uendeshaji"],
    };
    json!({ "doc": focus, "idea": idea, "sections": sections })
}

fn plan_outline(focus: &str, idea: &str, kind: &str) -> Value {
    json!({
        "plan": focus,
        "idea": idea,
        "kind": kind,
        "steps": [
            "Kusanya mahitaji + acceptance criteria",
            "Chagua stack + usanifu",
            "Tengeneza skeleton ya mradi",
            "Tekeleza vipengele kwa TDD",
            "Security review (swarm) + fix",
            "Andaa docs + deploy",
        ],
    })
}

fn deploy_snippet(focus: &str, idea: &str) -> Value {
    match focus {
        "docker" => json!({
            "file": "Dockerfile",
            "content": "# syntax=docker/dockerfile:1\nFROM alpine:3.20\nWORKDIR /app\nCOPY . .\n# TODO: weka build/run kwa stack yako\nCMD [\"sh\",\"-c\",\"echo run\"]\n",
        }),
        "cloud" => json!({
            "file": "fly.toml",
            "content": "app = \"PROJECT\"\nprimary_region = \"jnb\"\n\n[http_service]\n  internal_port = 8080\n  force_https = true\n",
        }),
        "ci" => json!({
            "file": ".github/workflows/ci.yml",
            "content": "name: ci\non: [push, pull_request]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: echo \"weka build/test ya stack\"\n",
        }),
        "observability" => json!({
            "note": "Ongeza /health, structured logs (tracing), na metrics (Prometheus).",
        }),
        "db" => json!({
            "note": "Schema: tumia migrations; index kwenye FK; parameterized queries pekee.",
        }),
        _ => json!({ "note": "Release checklist: tests zote, security grade, version bump, changelog." }),
    }
}

fn research(focus: &str) -> Value {
    let refs = match focus {
        "pentest" => vec![
            "OWASP Web Security Testing Guide",
            "MITRE ATT&CK",
            "PTES (Penetration Testing Execution Standard)",
        ],
        _ => vec!["OWASP Top 10", "CWE Top 25", "NIST SSDF"],
    };
    json!({ "research": focus, "references": refs })
}

/// ENDOSHA SWARM: agents 50, kila mmoja hurudisha ripoti HALISI.
pub async fn run_all(idea: &str, kind: &str, path: &str, include_llm: bool, brain: &crate::brain::Brain) -> Value {
    let root = crate::ide_api::ide_workspace_root();
    // 1) SCAN MARA MOJA — findings zinagawanywa kwa agents (efficient + halisi).
    let scan = crate::code_audit::scan_workspace(&root, path);
    let findings = scan["findings"].as_array().cloned().unwrap_or_default();

    // 2) TEST mara moja — build/test halisi.
    let test_summary = {
        let t = crate::ide_lab::lab_test(Json(json!({ "path": path, "target": "test" }))).await;
        t.0
    };

    // 3) GENERATION: zalisha scaffold halisi kwenye workspace (kama idea imetolewa).
    let gen_dir = if idea.trim().is_empty() {
        None
    } else {
        let dir = root.join(slug(idea));
        let _ = std::fs::create_dir_all(&dir);
        Some(dir)
    };

    let mut reports: Vec<Value> = Vec::new();
    for agent in AGENTS {
        let report = match agent.cap {
            Cap::Audit => {
                let matched: Vec<Value> = if agent.focus.is_empty() {
                    findings.clone()
                } else {
                    findings
                        .iter()
                        .filter(|f| {
                            f.get("category").and_then(|v| v.as_str()) == Some(agent.focus)
                                || f.get("lang").and_then(|v| v.as_str()) == Some(agent.focus)
                        })
                        .cloned()
                        .collect()
                };
                json!({
                    "agent": agent.id, "domain": agent.domain, "capability": "audit",
                    "focus": agent.focus, "findings_count": matched.len(),
                    "findings": matched.into_iter().take(50).collect::<Vec<_>>(),
                })
            }
            Cap::Test => json!({
                "agent": agent.id, "domain": agent.domain, "capability": "test",
                "result": test_summary,
            }),
            Cap::Gen => {
                let mut art = generate_artifact(agent.id, agent.focus, idea, kind);
                // andika file HALISI kama idea imetolewa
                let mut wrote = Value::Null;
                if let Some(dir) = &gen_dir {
                    if let (Some(rel), Some(content)) = (art["artifact"].as_str(), art["_content"].as_str()) {
                        let full = dir.join(rel);
                        if let Some(p) = full.parent() {
                            let _ = std::fs::create_dir_all(p);
                        }
                        let ok = std::fs::write(&full, content).is_ok();
                        wrote = json!({ "path": full.display().to_string(), "ok": ok });
                    }
                }
                if let Some(o) = art.as_object_mut() {
                    o.remove("_content");
                }
                art["agent"] = json!(agent.id);
                art["written"] = wrote;
                art
            }
            Cap::Docs => {
                let mut d = doc_outline(agent.focus, idea);
                d["agent"] = json!(agent.id);
                d["domain"] = json!(agent.domain);
                d
            }
            Cap::Plan => {
                let mut p = plan_outline(agent.focus, idea, kind);
                p["agent"] = json!(agent.id);
                p["domain"] = json!(agent.domain);
                p
            }
            Cap::Deploy => {
                let mut d = deploy_snippet(agent.focus, idea);
                d["agent"] = json!(agent.id);
                d["domain"] = json!(agent.domain);
                d
            }
            Cap::Research => {
                let mut r = research(agent.focus);
                r["agent"] = json!(agent.id);
                r["domain"] = json!(agent.domain);
                r
            }
            Cap::Analyze => {
                // R engine halisi kwenye data ya mfano (au findings).
                let data: Vec<f64> = if findings.is_empty() {
                    vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
                } else {
                    findings
                        .iter()
                        .map(|f| match f.get("severity").and_then(|v| v.as_str()).unwrap_or("low") {
                            "critical" => 9.5,
                            "high" => 7.5,
                            "medium" => 5.0,
                            _ => 2.0,
                        })
                        .collect()
                };
                let r = crate::r_engine::run_req(agent.focus, &json!({ "data": data })).await;
                json!({ "agent": agent.id, "domain": agent.domain, "capability": "analyze", "task": agent.focus, "r": r })
            }
        };
        reports.push(report);
    }

    // LLM enrichment (optional) — generate agent inaandika code halisi kwa LLM.
    let llm_artifact = if include_llm {
        let prompt = format!(
            "Andika code HALISI (bila maelezo) kwa mradi: '{idea}' (aina {kind}). \
             Rejesha faili MOJA muhimu zaidi tu, kwenye code block moja."
        );
        let reply = crate::chat::ask(brain, &prompt, true, crate::chat::Length::Medium, None, None).await;
        json!({ "agent": "llm-generator", "answer": reply.answer_translated, "source": reply.source })
    } else {
        Value::Null
    };

    // Aggregate risk score (R engine) kutoka severity weights.
    let weights: Vec<f64> = findings
        .iter()
        .map(|f| match f.get("severity").and_then(|v| v.as_str()).unwrap_or("low") {
            "critical" => 10.0,
            "high" => 7.0,
            "medium" => 4.0,
            _ => 2.0,
        })
        .collect();
    let risk = if weights.is_empty() {
        json!({ "ok": true, "note": "hakuna findings" })
    } else {
        crate::r_engine::run_req("risk_score", &json!({ "data": weights })).await
    };

    json!({
        "ok": true,
        "idea": idea,
        "kind": kind,
        "agents_run": reports.len(),
        "scan": {
            "files_scanned": scan["files_scanned"],
            "total_findings": scan["total_findings"],
            "by_severity": scan["by_severity"],
            "by_category": scan["by_category"],
            "grade": scan["grade"],
        },
        "test": test_summary,
        "risk_analytics": risk,
        "generated_dir": gen_dir.map(|d| d.display().to_string()),
        "llm_artifact": llm_artifact,
        "reports": reports,
    })
}

/// ENDOSHA AGENT MMOJA (kutoka swarm ya 50). Ripoti HALISI kwa aina ya uwezo.
pub async fn run_one(agent_id: &str, idea: &str, kind: &str, path: &str) -> Value {
    let Some(agent) = AGENTS.iter().find(|a| a.id == agent_id) else {
        return json!({ "ok": false, "error": format!("agent '{agent_id}' haipo kwenye swarm") });
    };
    let root = crate::ide_api::ide_workspace_root();
    let report = match agent.cap {
        Cap::Audit => {
            let scan = crate::code_audit::scan_workspace(&root, path);
            let findings = scan["findings"].as_array().cloned().unwrap_or_default();
            let matched: Vec<Value> = if agent.focus.is_empty() {
                findings
            } else {
                findings
                    .iter()
                    .filter(|f| {
                        f.get("category").and_then(|v| v.as_str()) == Some(agent.focus)
                            || f.get("lang").and_then(|v| v.as_str()) == Some(agent.focus)
                    })
                    .cloned()
                    .collect()
            };
            json!({
                "agent": agent.id, "domain": agent.domain, "capability": "audit",
                "focus": agent.focus, "findings_count": matched.len(),
                "findings": matched.into_iter().take(50).collect::<Vec<_>>(),
            })
        }
        Cap::Test => {
            let t = crate::ide_lab::lab_test(Json(json!({ "path": path, "target": "test" }))).await;
            json!({ "agent": agent.id, "domain": agent.domain, "capability": "test", "result": t.0 })
        }
        Cap::Gen => {
            let mut art = generate_artifact(agent.id, agent.focus, idea, kind);
            let mut wrote = Value::Null;
            if !idea.trim().is_empty() {
                if let (Some(rel), Some(content)) =
                    (art["artifact"].as_str(), art["_content"].as_str())
                {
                    let dir = root.join(slug(idea));
                    let _ = std::fs::create_dir_all(&dir);
                    let full = dir.join(rel);
                    if let Some(p) = full.parent() {
                        let _ = std::fs::create_dir_all(p);
                    }
                    let ok = std::fs::write(&full, content).is_ok();
                    wrote = json!({ "path": full.display().to_string(), "ok": ok });
                }
            }
            if let Some(o) = art.as_object_mut() {
                o.remove("_content");
            }
            art["agent"] = json!(agent.id);
            art["written"] = wrote;
            art
        }
        Cap::Docs => {
            let mut d = doc_outline(agent.focus, idea);
            d["agent"] = json!(agent.id);
            d["domain"] = json!(agent.domain);
            d
        }
        Cap::Plan => {
            let mut p = plan_outline(agent.focus, idea, kind);
            p["agent"] = json!(agent.id);
            p["domain"] = json!(agent.domain);
            p
        }
        Cap::Deploy => {
            let mut d = deploy_snippet(agent.focus, idea);
            d["agent"] = json!(agent.id);
            d["domain"] = json!(agent.domain);
            d
        }
        Cap::Research => {
            let mut r = research(agent.focus);
            r["agent"] = json!(agent.id);
            r["domain"] = json!(agent.domain);
            r
        }
        Cap::Analyze => {
            let scan = crate::code_audit::scan_workspace(&root, path);
            let findings = scan["findings"].as_array().cloned().unwrap_or_default();
            let data: Vec<f64> = if findings.is_empty() {
                vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
            } else {
                findings
                    .iter()
                    .map(|f| match f.get("severity").and_then(|v| v.as_str()).unwrap_or("low") {
                        "critical" => 9.5,
                        "high" => 7.5,
                        "medium" => 5.0,
                        _ => 2.0,
                    })
                    .collect()
            };
            let r = crate::r_engine::run_req(agent.focus, &json!({ "data": data })).await;
            json!({ "agent": agent.id, "domain": agent.domain, "capability": "analyze", "task": agent.focus, "r": r })
        }
    };
    json!({ "ok": true, "agent": agent_id, "report": report })
}

/// POST /api/ide/swarm/generate — LLM inaandika code halisi + audit papo hapo.
pub async fn swarm_generate(
    State(s): State<crate::AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let idea = body["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = body["kind"].as_str().unwrap_or("website").trim().to_string();
    let lang = body["lang"].as_str().unwrap_or("rust").trim().to_string();
    if idea.is_empty() {
        return Json(json!({ "ok": false, "error": "idea ni lazima" }));
    }
    let prompt = format!(
        "Wewe ni senior software engineer. Andika code HALISI na KAMILI (bila maelezo, bila TODO) \
         kwa: '{idea}' (aina {kind}, lugha {lang}). Rejesha code block moja tu yenye kufanya kazi."
    );
    let reply = crate::chat::ask(&s.brain, &prompt, true, crate::chat::Length::Long, None, None).await;
    let code = reply.answer_translated;
    // Ondoa code fences
    let cleaned: String = code
        .lines()
        .filter(|l| !l.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n");
    let audit_lang = if lang.is_empty() { "any" } else { lang.as_str() };
    let audit = crate::code_audit::analyze_content(audit_lang, &cleaned);
    let grade: f64 = {
        let s: f64 = audit
            .iter()
            .map(|f| match f["severity"].as_str().unwrap_or("low") {
                "critical" => 10.0,
                "high" => 7.0,
                "medium" => 4.0,
                _ => 2.0,
            })
            .sum();
        (100.0 - s).max(0.0)
    };
    Json(json!({
        "ok": true,
        "source": reply.source,
        "code": cleaned,
        "audit": { "total_findings": audit.len(), "grade": grade, "findings": audit },
    }))
}

pub fn router() -> axum::Router<crate::AppState> {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/ide/swarm/agents", get(swarm_agents))
        .route("/api/ide/swarm/run", post(swarm_run))
        .route("/api/ide/swarm/generate", post(swarm_generate))
}

pub async fn swarm_agents() -> Json<Value> {
    Json(catalog())
}

pub async fn swarm_run(State(s): State<crate::AppState>, Json(body): Json<Value>) -> Json<Value> {
    let idea = body["idea"].as_str().unwrap_or("").trim().to_string();
    let kind = body["kind"].as_str().unwrap_or("website").trim().to_string();
    let path = body["path"].as_str().unwrap_or("").trim().to_string();
    let include_llm = body["use_llm"].as_bool().unwrap_or(false);
    Json(run_all(&idea, &kind, &path, include_llm, &s.brain).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_are_fifty() {
        assert_eq!(AGENTS.len(), 50, "swarm lazima iwe na agents 50");
    }

    #[test]
    fn agent_ids_unique() {
        let mut ids: Vec<&str> = AGENTS.iter().map(|a| a.id).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "agent ids ziwe za kipekee");
    }

    #[test]
    fn security_domain_has_ten() {
        assert_eq!(AGENTS.iter().filter(|a| a.domain == "security").count(), 10);
    }

    #[test]
    fn slug_works() {
        assert_eq!(slug("My Cool App!"), "my-cool-app");
        assert_eq!(slug(""), "project");
    }
}
