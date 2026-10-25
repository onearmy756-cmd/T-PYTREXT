// agent_engine.rs — MBILINYITECH CODING AGENT ENGINE (halisi, sio talk):
//
// WORKFLOW (kama Devin/Trae/Claude Code):
//   0. AKILI V6  — tabaka 200 (ethics/HITL/ushauri) via akili_bridge
//   1. PLAN      — LLM inatengeneza project plan JSON (features + files + commands)
//   2. SCAFFOLD  — agent inaunda file structure HALISI kwenye /data/ide/<project>/
//   3. CODE      — kila file inaandikwa HALISI (LLM-generated code, real writes)
//   4. TEST      — terminal halisi kupitia fundi-server "terminal" tool
//   5. IMAGE/3D  — image_gen (visual/design halisi) au vision (analysis ya image)
//   6. REPORT    — summary ya files za mwisho + how-to-run
//
// KANUNI: hakuna uwongo — kila step inategemea tools HALISI (file/terminal/
// image/vision zitoke /v1/tools/invoke ya fundi-server au direct fs handlers).
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::akili_bridge;
/// Autopilot run ni moja kwa wakati mmoja (safety)
static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn is_running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

/// Sink ya matukio ya agentic loop (kwa SSE/UI live). `None` = hakuna streaming.
pub type EventSink = Option<std::sync::Arc<dyn Fn(Value) + Send + Sync>>;

/// Tuma tukio kwa sink (kama ipo). Haizuii loop — ni fire-and-forget.
pub fn emit(sink: &EventSink, ev: Value) {
    if let Some(f) = sink {
        f(ev);
    }
}

/// Matokeo ya mwisho ya agentic run (kwa UI inayopoll — proxy inaweza kuvunja SSE).
static LAST_RESULT: std::sync::OnceLock<std::sync::Mutex<Option<Value>>> =
    std::sync::OnceLock::new();

fn last_result_slot() -> &'static std::sync::Mutex<Option<Value>> {
    LAST_RESULT.get_or_init(|| std::sync::Mutex::new(None))
}

/// Hifadhi matokeo ya mwisho (huitwa na loop mwishoni).
fn store_last_result(v: &Value) {
    if let Ok(mut g) = last_result_slot().lock() {
        *g = Some(v.clone());
    }
}

/// Matokeo ya mwisho ya agentic (kwa endpoint ya kupoll).
pub fn last_result() -> Option<Value> {
    last_result_slot().lock().ok().and_then(|g| g.clone())
}

pub struct AgentRunState {
    pub project: String,
    pub kind: String,
    pub steps: Vec<Value>,
}

pub async fn build_plan(
    idea: &str,
    kind: &str,
    workspace: &std::path::Path,
) -> Result<Value, String> {
    let sys = format!(
        "Wewe ni senior software engineer wa MBILINYITECH IDE (coding agent engine, \
         mchanganyiko wa Devin, Trae, Claude Code na ZCode AI). \
         Mtumiaji ametoa WAZO la project aina '{kind}'. \
         Rejesha JSON MOJA (bila markdown) yenye: \
         \"project_name\" (kebab-case, inbox), \
         \"summary\" (Kiswahili kifupi), \
         \"stack\" (orodha ya tech), \
         \"features\" (orodha 4-6), \
         \"files\": orodha ya objects {{\"path\",\"content\"}} — content HALISI inayoweza \
         kufanya kazi (README.md au Cargo.toml au package.json, \
         index/main/ui file, .gitignore, docker-compose.md coverage). \n\
         KANUNI: content ya file ziwe code KWELI yenye kufanya kazi, sio TODO. \
         Project iwe ndogo ynayoweza kuanza kwa npm install && npm run dev ama \
         cargo run — hii ni AUTOPILOT (Devil)."
    );
    let url = std::env::var("FUNDI_CLOUD_URL").unwrap_or_else(|_| "https://ollama.com/v1".into());
    let model = std::env::var("FUNDI_CLOUD_MODEL").unwrap_or_else(|_| "gpt-oss:120b".into());
    let key = std::env::var("FUNDI_AI_KEY").unwrap_or_default();
    let lan_url = std::env::var("FUNDI_LLM_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let lan_model = std::env::var("FUNDI_LLM_MODEL").unwrap_or_else(|_| "qwen2.5vl:3b".into());
    let has_cloud = !key.trim().is_empty();
    if !has_cloud {
        // FALLBACK 1: Ollama LAN (model ndogo inatosha kwa plan ikitumie template)
        match try_plan_via_ollama_lan(&lan_url, &lan_model, idea, kind, &workspace).await {
            Ok(p) => return Ok(p),
            Err(e) => tracing::warn!("plan LAN imeshindikana: {}", e),
        }
        // FALLBACK 2: deterministic plan (hakuna LLM — hustle halisi, haifui)
        let proj_name = slugify(&format!("{} {}", kind, idea));
        let plan = deterministic_plan(idea, kind, &proj_name, &workspace);
        return Ok(plan);
    }
    let body = json!({
        "model": model,
        "messages": [
            {"role":"system","content": sys},
            {"role":"user","content": idea}
        ],
        "stream": false,
        "temperature": 0.3
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(crate::ai_config::llm_timeout_secs()))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/chat/completions", url.trim_end_matches('/')))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("LLM cloud imeshindikana: HTTP {}", resp.status()));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("LLM haikurudisha content")?
        .to_string();
    let clean = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let plan: Value = serde_json::from_str(clean).map_err(|e| {
        format!(
            "JSON ya plan haikusomwa: {} — content: {}",
            e,
            &clean[..clean.len().min(120)]
        )
    })?;
    let mut p = plan.clone();
    let name = p["project_name"]
        .as_str()
        .map(|s| s.to_lowercase().replace(' ', "-").replace('_', "-"))
        .unwrap_or_else(|| "new-project".into());
    p["project_name"] = json!(name);
    p["workspace"] = json!(workspace.join(&name).display().to_string());
    // Akili V6 enrichment
    p["akili"] = akili_bridge::enrich_plan(idea, kind)["akili"].clone();
    Ok(p)
}

pub fn scaffold_project(
    workspace_root: &std::path::Path,
    project: &str,
    files: &[Value],
) -> Value {
    let dir = workspace_root.join(project);
    let mut written: Vec<Value> = Vec::new();
    for f in files {
        let Some(path) = f["path"].as_str() else {
            continue;
        };
        if path.contains("..") || path.starts_with('/') {
            continue;
        }
        let full = dir.join(path);
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = f["content"].as_str().unwrap_or("");
        if std::fs::write(&full, content).is_ok() {
            written.push(json!({ "path": path, "size": content.len() }));
        }
    }
    json!({ "ok": !written.is_empty(), "project_dir": dir.display().to_string(), "files": written })
}

/// RUN (autopilot): AKILI → PLAN → SCAFFOLD → TEST → IMAGE → REPORT
pub async fn run_autopilot(
    idea: &str,
    kind: &str,
    agent: Option<&crate::fundi_agent_client::FundiAgentClient>,
) -> Value {
    if RUNNING.swap(true, Ordering::Relaxed) {
        return json!({ "ok": false, "error": "autopilot inaendesha tayari — subiri au angalia status" });
    }
    let _guard = finally(|| RUNNING.store(false, Ordering::Relaxed));
    let workspace = crate::ide_api::ide_workspace_root();

    // STEP 0: AKILI V6 (tabaka 200)
    let akili = akili_bridge::think(&format!("{kind}: {idea}"));
    let mut steps = vec![json!({
        "step": "akili_v6",
        "status": "ok",
        "action": akili["decision"]["action"],
        "confidence": akili["decision"]["confidence"],
        "requires_hitl": akili["decision"]["requires_hitl"],
        "ethical_allowed": akili["decision"]["ethical_allowed"],
    })];
    if akili["decision"]["ethical_allowed"] == json!(false) {
        steps[0]["status"] = json!("blocked");
        return json!({
            "ok": false,
            "error": "Akili V6 imezuia — ethics/dhamiri",
            "akili": akili["decision"],
            "steps": steps
        });
    }

    // STEP 1: PLAN
    steps.push(json!({"step":"plan","status":"run","note":"LLM inaandika plan halisi…"}));
    let plan = match build_plan(idea, kind, &workspace).await {
        Ok(p) => {
            let i = steps.len() - 1;
            steps[i]["status"] = json!("ok");
            steps[i]["project"] = p["project_name"].clone();
            p
        }
        Err(e) => {
            let i = steps.len() - 1;
            steps[i]["status"] = json!("error");
            return json!({ "ok": false, "error": format!("plan haikufanikiwa: {}", e), "steps": steps });
        }
    };

    // STEP 2: SCAFFOLD
    steps.push(json!({"step":"scaffold","status":"run","note":"File structure + code…"}));
    let files = plan["files"].as_array().cloned().unwrap_or_default();
    let scaffolded = scaffold_project(
        &workspace,
        plan["project_name"].as_str().unwrap_or("project"),
        &files,
    );
    {
        let i = steps.len() - 1;
        steps[i]["status"] = if scaffolded["ok"] == json!(true) {
            json!("ok")
        } else {
            json!("error")
        };
        steps[i]["files_count"] =
            json!(scaffolded["files"].as_array().map(|a| a.len()).unwrap_or(0));
    }

    // STEP 3: TEST
    if let Some(agent) = agent {
        let dir_in = workspace.join(plan["project_name"].as_str().unwrap_or("project"));
        let mut test_cmd = "ls -la .".to_string();
        let desc = json!({
            "package.json": "npm install --no-audit --no-fund 2>/dev/null | tail -5",
            "Cargo.toml": "cargo test --quiet 2>&1 | tail -5",
        });
        for (k, v) in desc.as_object().unwrap() {
            let p = dir_in.join(k);
            if p.exists() {
                test_cmd = v.as_str().unwrap().to_string();
                break;
            }
        }
        steps.push(json!({"step":"test","status":"run","command": test_cmd}));
        let body = json!({ "tool": "terminal", "params": json!({ "command": test_cmd, "workdir": dir_in.display().to_string(), "timeout": 30000 }) });
        match agent.invoke_tool_raw(body).await {
            Ok(v) => {
                let i = steps.len() - 1;
                steps[i]["status"] = json!("ok");
                steps[i]["output"] = json!(v["output"].as_str().unwrap_or("").chars().take(1200).collect::<String>());
            }
            Err(e) => {
                let i = steps.len() - 1;
                steps[i]["status"] = json!("error");
                steps[i]["error"] = json!(e.to_string());
            }
        }
    }

    // STEP 3.5: CODE REVIEW (USALAMA + BUGS) — kila code iliyozalishwa inakaguliwa HALISI
    {
        let dir_in = workspace.join(plan["project_name"].as_str().unwrap_or("project"));
        let audit = crate::code_audit::scan_workspace(&dir_in, "");
        let grade = audit["grade"].as_f64().unwrap_or(100.0);
        let sev = audit["by_severity"].clone();
        let has_critical = sev["critical"].as_u64().unwrap_or(0) > 0;
        steps.push(json!({
            "step": "code_review",
            "status": if has_critical { "warning" } else { "ok" },
            "grade": grade,
            "total_findings": audit["total_findings"],
            "by_severity": sev,
            "by_category": audit["by_category"],
            "findings": audit["findings"].as_array().map(|a| a.iter().take(20).cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "note": if has_critical { "⚠️ CRITICAL findings — code iliyozalishwa ina hatari za usalama" } else { "Hakuna critical findings" },
        }));
    }

    // STEP 4: IMAGE
    let k = kind.to_lowercase();
    if k.contains("image") || k.contains("vision") || k.contains("3d") || k.contains("game") {
        if let Some(agent) = agent {
            steps.push(json!({"step":"image","status":"run","note":"Image concept…"}));
            let prompt = format!("{kind} kiwango cha juu, professional, concept art — {idea}");
            let body = json!({ "tool": "image_gen", "params": json!({ "prompt": prompt, "size": "1024x1024" }) });
            match agent.invoke_tool_raw(body).await {
                Ok(v) => {
                    let i = steps.len() - 1;
                    steps[i]["status"] = json!("ok");
                    steps[i]["result"] = v;
                }
                Err(e) => {
                    let i = steps.len() - 1;
                    steps[i]["status"] = json!("skipped");
                    steps[i]["error"] = json!(format!("image_gen: {e}"));
                }
            }
        }
    }

    // STEP 5: REPORT
    let readme = workspace
        .join(plan["project_name"].as_str().unwrap_or("project"))
        .join("README.md");
    let summary = std::fs::read_to_string(&readme).unwrap_or_default();
    json!({
        "ok": true,
        "project": plan["project_name"],
        "workspace": plan["workspace"],
        "akili": plan.get("akili").cloned().unwrap_or(akili["decision"].clone()),
        "plan": { "summary": plan["summary"], "stack": plan["stack"], "features": plan["features"] },
        "steps": steps,
        "readme_head": summary.chars().take(900).collect::<String>(),
        "note": "Autopilot + Akili V6 — files HALISI kwenye workspace"
    })
}

// FALLBACK 1: Ollama LAN plan — model ndogo + prompt fupi (haraka, bila key)
async fn try_plan_via_ollama_lan(
    lan_url: &str,
    lan_model: &str,
    idea: &str,
    kind: &str,
    workspace: &std::path::Path,
) -> Result<Value, String> {
    let sys = "Wewe ni project planner. Jibu na JSON MOJA tu (hakuna markdown) yenye: \
project_name (kebab-case), summary (Kiswahili fupi), stack (_ID_ROBOT), features (5), \
files: orodha ya {path, content} — content ni code KWELI (README.md, Cargo.toml au package.json, index.html au main file). \
Project ifanye kazi mara moja (npm install && npm start, cargo run, au fungua index.html).";
    let prompt = format!(
        "{sys}\n\nWAZO ({kind}): {idea}\n\nJSON:" 
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(crate::ai_config::llm_timeout_secs()))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{}/api/generate", lan_url.trim_end_matches('/')))
        .json(&json!({
            "model": lan_model,
            "prompt": prompt,
            "stream": false,
            "options": { "num_predict": crate::ai_config::llm_max_tokens(), "temperature": 0.4 }
        }))
        .send()
        .await
        .map_err(|e| format!("LAN: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("LAN HTTP {}", resp.status()));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["response"].as_str().unwrap_or("").trim().to_string();
    if content.is_empty() {
        return Err("LAN jibu tupu".into());
    }
    // Ondoa fensi + parse JSON (kata kutoka { ya kwanza hadi } ya mwisho)
    let s = content.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let st = s.find('{').ok_or("hakuna JSON kwenye jibu la LAN")?;
    let en = s.rfind('}').ok_or("JSON haijakamilika")?;
    let plan: Value = serde_json::from_str(&s[st..=en])
        .map_err(|e| format!("LAN JSON parse: {e}"))?;
    let name = plan["project_name"]
        .as_str()
        .map(|x| slugify(x))
        .filter(|x| !x.is_empty())
        .unwrap_or_else(|| slugify(&format!("{} {}", kind, idea)));
    let mut p = plan;
    p["project_name"] = json!(name);
    p["workspace"] = json!(workspace.join(&name).display().to_string());
    p["akili"] = akili_bridge::enrich_plan(idea, kind)["akili"].clone();
    Ok(p)
}

// FALLBACK 2: Deterministic plan (bila LLM) — templates halisi kwa kila kind.
// Hii inahakikisha AUTOPILOT inafanya kazi hata BILA LLM (offline-first, KANUNI ya mradi).
fn deterministic_plan(
    idea: &str,
    kind: &str,
    name: &str,
    workspace: &std::path::Path,
) -> Value {
    let k = kind.to_lowercase();
    let title = idea.chars().take(60).collect::<String>();
    let (stack, files): (Vec<&str>, Vec<Value>) = if k.contains("mobile") {
        (
            vec!["HTML5", "CSS", "JS (PWA)", "Service Worker"],
            vec![
                json!({"path": "index.html", "content": format!("<!DOCTYPE html>\n<html lang=\"sw\"><head><meta charset=\"utf-8\"><title>{title}</title>\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<style>body{{font-family:system-ui;margin:0;background:#0f172a;color:#e2e8f0}}main{{padding:16px;max-width:480px;margin:auto}}h1{{font-size:1.2rem}}button{{background:#2563eb;color:#fff;border:0;border-radius:8px;padding:10px 16px;font-size:1rem}}</style></head>\n<body><main><h1>{title}</h1>\n<p id=\"status\">Nawahisha app…</p>\n<button onclick=\"act()\">Anza</button>\n<script>\nfunction act(){{document.getElementById('status').textContent='Kazi imefanyika @ '+new Date().toLocaleTimeString();}}\nif('serviceWorker' in navigator)navigator.serviceWorker.register('sw.js');\n</script></main></body></html>")}),
                json!({"path": "sw.js", "content": "self.addEventListener('install',e=>self.skipWaiting());\nself.addEventListener('fetch',e=>e.respondWith(fetch(e.request).catch(()=>caches.match(e.request))));"}),
            ],
        )
    } else if k.contains("rust") || k.contains("rust-cli") || k.contains("cli") {
        (
            vec!["Rust", "Cargo"],
            vec![
                json!({"path": "Cargo.toml", "content": format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"", name)}),
                json!({"path": "src/main.rs", "content": format!("fn main() {{\n    let args: Vec<String> = std::env::args().collect();\n    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or(\"help\");\n    match cmd {{\n        \"run\" | \"start\" => println!(\"{title} — inaendelea…\"),\n        _ => println!(\"Matumizi: {{bin}} run|help\"),\n    }}\n}}")}),
            ],
        )
    } else if k.contains("api") || k.contains("backend") {
        (
            vec!["Rust", "std::net (bila deps)"],
            vec![
                json!({"path": "Cargo.toml", "content": format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n", name)}),
                json!({"path": "src/main.rs", "content": format!("//! {title} — API ndogo (stdlib tu)\nuse std::io::{{Read, Write}};\nuse std::net::TcpListener;\n\nfn main() {{\n    let listener = TcpListener::bind(\"0.0.0.0:8080\").unwrap();\n    println!(\"API inaendesha :8080\");\n    for stream in listener.incoming() {{\n        if let Ok(mut s) = stream {{\n            let mut buf = [0u8; 1024];\n            let _ = s.read(&mut buf);\n            let body = \"{{\\\"ok\\\":true,\\\"service\\\":\\\"{title}\\\"}}\";\n            let resp = format!(\"HTTP/1.1 200 OK\\r\\nContent-Type: application/json\\r\\nContent-Length: {{}}\\r\\n\\r\\n{{}}\", body.len(), body);\n            let _ = s.write_all(resp.as_bytes());\n        }}\n    }}\n}}")}),
            ],
        )
    } else {
        // website / iot / gaming default — PWA halisi
        (
            vec!["HTML5", "CSS", "JS"],
            vec![
                json!({"path": "index.html", "content": format!("<!DOCTYPE html>\n<html lang=\"sw\"><head><meta charset=\"utf-8\"><title>{title}</title>\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<style>:root{{--b:#2563eb}}body{{font-family:system-ui;margin:0;background:#f8fafc;color:#0f172a}}header{{background:var(--b);color:#fff;padding:24px 16px}}main{{padding:16px;max-width:720px;margin:auto}}input{{width:100%%;padding:10px;border:1px solid #cbd5e1;border-radius:8px;margin:6px 0}}button{{background:var(--b);color:#fff;border:0;border-radius:8px;padding:10px 18px}}</style></head>\n<body><header><h1>{title}</h1></header>\n<main id=\"app\"><input id=\"q\" placeholder=\"Andika kitu…\"><button onclick=\"go()\">Tuma</button>\n<ul id=\"out\"></ul>\n<script>\nfunction go(){{const q=document.getElementById('q').value;if(!q)return;\nconst li=document.createElement('li');li.textContent=q+' — '+new Date().toLocaleTimeString();\ndocument.getElementById('out').appendChild(li);document.getElementById('q').value='';}}\n</script></main></body></html>")}),
                json!({"path": "README.md", "content": format!("# {title}\n\n{idea}\n\n## Jinsi ya kuanza\n- Fungua `index.html` kwenye kivinjari — HAKUNA install."),}),
            ],
        )
    };
    // ONGEZA README + .gitignore kwa kila template (kama hazipo)
    let mut all_files = files;
    if !all_files.iter().any(|f| f["path"].as_str() == Some("README.md")) {
        all_files.push(json!({"path": "README.md", "content": format!("# {title}\n\n{idea}\n\n## Jinsi ya kuanza\nSoma michoro ya files hapa chini."),}));
    }
    all_files.push(json!({"path": ".gitignore", "content": "node_modules/\ntarget/\n__pycache__/\n.env\n"}));
    json!({
        "project_name": name,
        "summary": format!("{title} (template ya {kind} — deterministic)"),
        "stack": stack,
        "features": ["msingi ufanye kazi", "offline-first", "responsive", "Kiswahili", "autopilot-ready"],
        "files": all_files,
        "workspace": workspace.join(name).display().to_string(),
        "plan_source": "deterministic",
    })
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join("-")
}

fn finally<F: FnOnce()>(f: F) -> impl Drop {
    struct D<F: FnOnce()>(Option<F>);
    impl<F: FnOnce()> Drop for D<F> {
        fn drop(&mut self) {
            self.0.take().map(|f| f());
        }
    }
    D(Some(f))
}

// ═══════════════════════════════════════════════════════════════════════════
// AGENTIC LOOP (ReAct) — LLM inaandika file MOJA kwa wakati, inaendesha
// terminal, inajirekebisha, na inakamilisha mradi. Hii ni njia ya kweli ya
// "AI agentic": SI one-shot JSON, bali hatua kwa hatua hadi DONE.
//
// Itifaki (JSON text — hufanya kazi kwa model YOYOTE, hata bila function-calling):
//   {"action":"write_file","path":"...","content":"..."}
//   {"action":"run","command":"..."}
//   {"action":"read_file","path":"..."}
//   {"action":"list_files"}
//   {"action":"done","summary":"..."}
// Kila hatua: matokeo (stdout/error) hurudishwa kwa LLM → inaamua hatua ifuatayo.
// ═══════════════════════════════════════════════════════════════════════════

fn truncate_chars(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{t}\n…[imekatwa]")
    }
}

fn agentic_system_prompt(project: &str, kind: &str, max_steps: usize) -> String {
    let lessons = crate::nyi::lessons_for(&format!("{project} {kind}"), 4);
    let lessons_block = if lessons.is_empty() {
        String::new()
    } else {
        format!("{lessons}\n")
    };
    format!(
        "Wewe ni 2NYI — MBILINYI CODING AGENT, SUPER-INTELLIGENCE software engineer wa MTECHOS, \
ukifanya kazi kwa mwendo wa RADI (kasi ya umeme: hatua nyingi huru kwa mzunguko mmoja, pima mara moja). \
Ni staff/principal engineer. \
Unaweza lugha ZOTE (Rust, Node/TS, Go, Java/Kotlin, C#, C/C++, Ruby, PHP, Swift, \
Dart/Flutter, Elixir, Scala, Zig…) na nyanja zote (OS/systems, IoT/embedded, mobile, desktop, \
web, backend, data, devops). \
Unajenga mradi KAMILI, unaofanya kazi, hatua kwa hatua kwenye workspace.\n\
{}\n\
{lessons_block}\
MRADI: {project}\nAINA: {kind}\n\
KUFIKIRI (kabla ya kuandika): tambua lugha/framework, tenga muundo (modules), \
chagua algoriti sahihi, na panga hatua. Kisha TEKELEZA.\n\
Kanuni:\n\
1) Andika faili HALISI zenye kufanya kazi (sio TODO/mock isipokuwa kuombwa).\n\
1b) KASI (RADI): kila jibu liwe na faili 1-3 (kila moja mistari <150). Faili ndogo \
zinazohusiana zinaweza kuandikwa pamoja (JSON kadhaa kwa jibu moja); vitendo huru viwe \
kwenye jibu MOJA, na pima mara MOJA kwa command KAMILI.\n\
2) Baada ya faili muhimu, ENDESHA (run) ili kuthibitisha (cargo/go test, npm test, dart test, ctest…).\n\
3) Kama command inafeli, SOMA kosa HALISI na REKEBISHA (write_file content KAMILI) — \
usikate tamaa; unaweza kujirekebisha hadi mizunguko 5.\n\
4) Jibu kwa JSON MOJA tu kwa kila hatua — HAKUNA maandishi mengine.\n\
5) Kwa Node: tenga logic (module) na CLI. Usiweke `process.exit`/console top-level bila guard \
ya `if (import.meta.url === pathToFileURL(process.argv[1]).href)` — vinginevyo `node --test` hufeli.\n\
5b) ULINGANIFU WA AINA (uhalisia): hakikisha types/API zinalingana kati ya modules/tests \
(mfano Rust: `fibonacci(n: u64)` → mwito uwe `u64`, sio `u32`). Code LAZIMA icompile.\n\
6) Ukipata >={max_steps} hatua, maliza na done — LAKINI uthibitisho lazima UPITE kwanza.\n\
Vitendo vinavyopatikana (action):\n\
  {{\"action\":\"write_file\",\"path\":\"rel/path\",\"content\":\"...\"}}  — andika faili kamili\n\
  {{\"action\":\"read_file\",\"path\":\"rel/path\"}}                     — soma faili\n\
  {{\"action\":\"list_files\"}}                                        — orodha ya faili\n\
  {{\"action\":\"run\",\"command\":\"...\"}}                            — endesha shell (cwd=mradi)\n\
  {{\"action\":\"see_image\",\"image\":\"<base64>\",\"prompt\":\"...\"}} — eleza picha (Magma, kama ipo)\n\
  {{\"action\":\"mkdir\",\"path\":\"rel/dir\"}}                         — tengeneza saraka\n\
  {{\"action\":\"done\",\"summary\":\"...\"}}                           — kamilisha mradi\n\
MUHIMU (uhalisia — LAZIMA):\n\
7) USIendesha `cargo init`/`npm init`/`go mod init` — andika manifests (Cargo.toml, \
package.json, go.mod, pom.xml, CMakeLists.txt…) MWENYEWE kwa write_file.\n\
8) LAZIMA uandike tests HALISI zinazopita kwa lugha husika:\n\
   Rust `#[test]` · Node `node:test`/jest/vitest · Go `_test.go` · \
   Java JUnit · C# xUnit · C/C++ ctest · Dart `dart test`. Mradi bila tests HAUhesabiwi.\n\
9) USIRUDIE faili uliyoandika. Kila write_file iwe faili MPYA au marekebisho ya kosa halisi.\n\
10) Lugha isiyo na toolchain kwenye mashine → andika code NZURI bado, na weka tests; \
uthibitisho utaonyesha wazi kama toolchain haipo.\n\
GROUNDING (2NYI — LAZIMA): usibuni API, kifurushi, kitenzi, wala tokeo usiloliona. \
Kama hujathibitisha, sema wazi; usikisie ili kuonekana unajua.\n\
USALAMA (LAZIMA — si hiari): kila code unayoandika hupimwa na LANGI LA USALAMA \
(sec_gate) kabla ya kutolewa. Hivyo:\n\
  11) USIWEKE siri kwenye code (sk-, ghp_, AKIA, PRIVATE KEY) — tumia env var \
      (process.env.X / std::env::var / os.environ). Siri iliyoachwa huzuia kutolewa.\n\
  12) Tumia queries ZENYE PARAMETA (prepared statements) — usiunganishe SQL kwa string.\n\
  13) Usitumie eval()/exec()/os.system kwa data ya nje; tumia API salama.\n\
  14) Thibitisha TLS (usiweke verify=False); tumia SHA-256/Argon2, SI md5/DES.\n\
  15) Weka CORS origin MAALUM, si '*'; sanitize path (kataa '..').\n\
  16) Eleza kwa `summary` uamuzi wa usalama: ni hatari gani uliepuka na kwa nini.\n\
Amri hatari (rm -rf /, mkfs, n.k.) huzuiwa na GOVERNANCE — usizijaribu.\n\
Jibu JSON pekee (bila ``` na bila maelezo).\n\
MFANO KAMILI (fuata muundo HUU kwa mradi wako; vitendo kadhaa kwa jibu moja):\n\
{{\"action\":\"write_file\",\"path\":\"package.json\",\"content\":\"{{\\n  \\\"name\\\": \\\"app\\\",\\n  \\\"scripts\\\": {{\\\"test\\\": \\\"node --test\\\"}}\\n}}\\n\"}}\n\
{{\"action\":\"write_file\",\"path\":\"logic.js\",\"content\":\"export function add(a, b) {{ return a + b; }}\\n\"}}\n\
{{\"action\":\"write_file\",\"path\":\"logic.test.js\",\"content\":\"import test from 'node:test';\\nimport assert from 'node:assert';\\nimport {{ add }} from './logic.js';\\ntest('add', () => {{ assert.equal(add(1, 2), 3); }});\\n\"}}\n\
{{\"action\":\"run\",\"command\":\"npm test\"}}\n\
{{\"action\":\"done\",\"summary\":\"mradi umekamilika\"}}\n\
SASA tekeleza mradi wa MTUMIAJI kwa muundo huo — anza na faili ya kwanza HALISI.",
        if crate::ai_config::llm_compact() {
            crate::nyi::directive().to_string()
        } else {
            crate::nyi::block()
        }
    )
}

/// Chambua hatua ya agent kutoka jibu la LLM (JSON, au heuristics ikiwa imefeli).
fn parse_agent_action(raw: &str) -> Value {
    let s = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    // 1) JSON safi.
    if let Ok(v) = serde_json::from_str::<Value>(s) {
        if v.get("action").and_then(|a| a.as_str()).is_some() {
            return v;
        }
    }
    // 2) JSON iliyozungukwa na maandishi — kata { ya kwanza hadi } ya mwisho.
    if let (Some(st), Some(en)) = (s.find('{'), s.rfind('}')) {
        if st < en {
            if let Ok(v) = serde_json::from_str::<Value>(&s[st..=en]) {
                if v.get("action").and_then(|a| a.as_str()).is_some() {
                    return v;
                }
            }
        }
    }
    // 3) Heuristic: fence ya code yenye lugha → write_file (path ya kubahatisha).
    if let Some(pos) = raw.find("```") {
        let after = &raw[pos + 3..];
        let lang = after.split(['\n', ' ']).next().unwrap_or("").trim().to_lowercase();
        let body_start = after.find('\n').map(|i| i + 1).unwrap_or(0);
        let body = &after[body_start..];
        let body = body.trim_end_matches("```").trim_end();
        let path = match lang.as_str() {
            "rust" => "src/main.rs",
            "javascript" | "js" => "index.js",
            "typescript" | "ts" => "index.ts",
            "html" => "index.html",
            "css" => "style.css",
            "json" => "data.json",
            "yaml" | "yml" => "config.yaml",
            "toml" => "Cargo.toml",
            "elixir" | "ex" => "lib/main.ex",
            _ => "main.txt",
        };
        return json!({ "action": "write_file", "path": path, "content": body });
    }
    json!({ "action": "done", "summary": format!("Jibu lisiloeleweka: {}", truncate_chars(raw, 160)) })
}

/// Chambua VITENDO VINGI kutoka jibu moja la LLM. Modeli nyingi huandika JSON
/// kadhaa mfululizo (`{"action":...}\n{"action":...}`) — tukichukua ya kwanza tu,
/// kazi nyingine hupotea na modeli hurudia hatua ya kwanza (kitanzi). Hapa
/// tunachambua kila JSON yenye `action` (brace-matching, inaheshimu strings).
fn parse_agent_actions(raw: &str) -> Vec<Value> {
    let bytes: Vec<char> = raw.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == '{' {
            // Brace-match kutoka i, tukizingatia strings/escapes.
            let mut depth = 0i32;
            let mut in_str = false;
            let mut esc = false;
            let mut j = i;
            while j < bytes.len() {
                let c = bytes[j];
                if in_str {
                    if esc {
                        esc = false;
                    } else if c == '\\' {
                        esc = true;
                    } else if c == '"' {
                        in_str = false;
                    }
                } else if c == '"' {
                    in_str = true;
                } else if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if j >= bytes.len() {
                break; // haijafungwa — truncation; achilia salvage_truncated_write
            }
            let seg: String = bytes[i..=j].iter().collect();
            if let Ok(v) = serde_json::from_str::<Value>(&seg) {
                if v.get("action").and_then(|a| a.as_str()).is_some() {
                    out.push(v);
                }
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    if out.is_empty() {
        out.push(parse_agent_action(raw));
    }
    out
}

/// Jibu lilikatwa katikati ya `content`: tunajaribu kuokoa path + content
/// iliyopo (JSON escapes hutafsiriwa; content haijakamilika inaandikwa kama
/// ilivyo) — bora kuandika faili ya sehemu kuliko kupoteza kazi yote.
fn salvage_truncated_write(raw: &str) -> Value {
    let path = raw
        .find("\"path\"")
        .and_then(|i| raw[i..].find(':').map(|j| &raw[i + j + 1..]))
        .and_then(|rest| rest.find('"').map(|q| &rest[q + 1..]))
        .and_then(|rest| rest.find('"').map(|q| rest[..q].to_string()));
    let content = raw
        .find("\"content\"")
        .and_then(|i| raw[i..].find(':').map(|j| &raw[i + j + 1..]))
        .and_then(|rest| rest.find('"').map(|q| rest[q + 1..].to_string()))
        .map(|s| {
            // Tafsiri escapes za JSON zilizopo (bila `"` ya kufunga).
            let mut out = String::new();
            let mut chars = s.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some('r') => out.push('\r'),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some(o) => {
                            out.push('\\');
                            out.push(o);
                        }
                        None => break,
                    }
                } else if c == '"' {
                    break;
                } else {
                    out.push(c);
                }
            }
            out
        });
    match (path, content) {
        (Some(p), Some(c)) if !p.is_empty() => json!({ "action": "write_file", "path": p, "content": c }),
        _ => json!({ "action": "done", "summary": "jibu lilikatwa — hakuna hatua salama" }),
    }
}

/// Endesha shell command (blocking) kwenye workdir, na timeout.
fn run_shell(workdir: &std::path::Path, command: &str, timeout_secs: u64) -> (String, bool) {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(workdir)
        .env("NPM_CONFIG_YES", "true")
        .env("CI", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return (format!("spawn error: {e}"), false),
    };
    let start = std::time::Instant::now();
    let limit = std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child.wait_with_output().ok();
                let (mut so, mut se) = (String::new(), String::new());
                if let Some(o) = out {
                    so = String::from_utf8_lossy(&o.stdout).to_string();
                    se = String::from_utf8_lossy(&o.stderr).to_string();
                }
                let combined = format!("{}\n{}", so, se);
                return (truncate_chars(&combined, 2500), status.success());
            }
            Ok(None) => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    return (format!("timeout baada ya {timeout_secs}s"), false);
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
            Err(e) => return (format!("wait error: {e}"), false),
        }
    }
}

/// Amri ya kuthibitisha mradi. Hutumia rejista ya lugha ZOTE (`lang_verify`),
/// hivyo mfumo unashughulikia lugha 20+ (Rust, Node, Dart, Go, Java,
/// C#, Ruby, PHP, Swift, Kotlin, C/C++, Elixir, Haskell, Zig, Scala…).
fn verify_command(dir: &std::path::Path) -> String {
    let (prefix, _lang, cmd) = crate::lang_verify::verify_for(dir);
    format!("{prefix}{cmd}")
}

/// Kama `run_shell` lakini inapatikana nje ya module (kwa endpoint ya "Run").
pub fn run_shell_public(workdir: &std::path::Path, command: &str, timeout_secs: u64) -> (String, bool) {
    run_shell(workdir, command, timeout_secs)
}

/// Kitanzi cha AGENTIC: LLM ↔ tools hadi done / max_steps.
pub async fn run_agentic(
    idea: &str,
    kind: &str,
    max_steps: usize,
    cfg: &crate::ai_config::LlmConfig,
) -> Value {
    run_agentic_stream(idea, kind, max_steps, cfg, None).await
}

/// Kama `run_agentic` lakini hutuma matukio ya kila hatua kwa `sink` (SSE/UI).
pub async fn run_agentic_stream(
    idea: &str,
    kind: &str,
    max_steps: usize,
    cfg: &crate::ai_config::LlmConfig,
    sink: EventSink,
) -> Value {
    if RUNNING.swap(true, Ordering::Relaxed) {
        emit(&sink, json!({"type":"error","error":"agent inaendesha tayari — subiri"}));
        return json!({ "ok": false, "error": "agent inaendesha tayari — subiri" });
    }
    let _guard = finally(|| RUNNING.store(false, Ordering::Relaxed));
    let workspace = crate::ide_api::ide_workspace_root();
    let project = slugify(&format!("{} {}", kind, idea));
    let dir = workspace.join(&project);
    let _ = std::fs::create_dir_all(&dir);
    emit(&sink, json!({
        "type": "start", "project": project, "workspace": dir.display().to_string(),
        "model": cfg.model, "max_steps": max_steps
    }));

    let key = crate::ai_config::ai_key().unwrap_or_default();
    let style = crate::ai_config::style_for_url(&cfg.url);
    let url = if style == "openai" {
        format!(
            "{}/chat/completions",
            crate::ai_config::openai_base(&cfg.url)
        )
    } else {
        format!("{}/api/generate", cfg.url.trim_end_matches('/'))
    };

    let sys = agentic_system_prompt(&project, kind, max_steps);
    // NGUYU MOJA: ingiza skills ZOTE (engineering + VisualClaw SKILL.md + kanuni
    // za Swahili) pamoja na masomo ya awali (memory). Hii inahakikisha agent
    // haikosi kitu kilichopo kwenye repo.
    // COMPACT (modeli ndogo za local/CPU): block kubwa husababisha modeli 3B
    // kurudia schema badala ya kuandika code. Tunapunguza kwa prompt fupi +
    // mfano (few-shot) — code halisi inaandikwa.
    let sys = if crate::ai_config::llm_compact() {
        format!("{sys}\n\n{}\n(Skills/memory zimefupishwa kwa modeli ya local — fuata MFANO juu.)",
                crate::radi::directive())
    } else {
        let skills = crate::unified_engines::unified_skills_block(kind, idea);
        let memory = crate::unified_engines::memory_block(idea);
        let mut s = sys;
        if !skills.trim().is_empty() {
            s.push_str("\n\n=== SENIOR ENGINEERING SKILLS (fuata KWA KAMILI) ===\n");
            s.push_str(&skills);
        }
        if !memory.trim().is_empty() {
            s.push('\n');
            s.push_str(&memory);
        }
        s
    };
    let mut transcript = format!(
        "WAZO LA MTUMIAJI: {idea}\n\nAnza. Toa vitendo kama JSON (kimoja kwa kila mstari).\n"
    );
    let mut steps: Vec<Value> = Vec::new();
    let mut files_written: Vec<String> = Vec::new();
    let mut done = false;
    let mut written_sigs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(crate::ai_config::llm_timeout_secs()))
        .build()
        .unwrap_or_default();

    let max_tokens = crate::ai_config::llm_max_tokens();
    let mut verify_round = 0u32;
    let mut step = 1usize;
    let started = std::time::Instant::now();
    let mut run_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    while step <= max_steps {
        // ── Uliza LLM hatua ifuatayo ──
        let body = if style == "openai" {
            json!({
                "model": cfg.model,
                "messages": [
                    {"role":"system","content": sys},
                    {"role":"user","content": transcript}
                ],
                "stream": false,
                "temperature": 0.2,
                "max_tokens": max_tokens
            })
        } else {
            json!({
                "model": cfg.model,
                "prompt": format!("{sys}\n\n{transcript}\n\nJSON:"),
                "stream": false,
                "options": {
                    "temperature": 0.2,
                    "num_predict": max_tokens,
                    "num_ctx": crate::ai_config::llm_num_ctx()
                }
            })
        };
        let mut req = client.post(&url).json(&body);
        if !key.is_empty() {
            req = req.bearer_auth(&key);
        }
        let (reply, truncated) = match req.send().await {
            Ok(r) if r.status().is_success() => {
                let v: Value = r.json().await.unwrap_or(Value::Null);
                let trunc = v["choices"][0]["finish_reason"].as_str() == Some("length")
                    || v["done_reason"].as_str() == Some("length");
                let txt = v["choices"][0]["message"]["content"]
                    .as_str()
                    .or_else(|| v["response"].as_str())
                    .unwrap_or("")
                    .to_string();
                (txt, trunc)
            }
            Ok(r) => {
                steps.push(json!({"step":step,"action":"llm","status":"error","error":format!("HTTP {}", r.status())}));
                (String::new(), false)
            }
            Err(e) => {
                steps.push(json!({"step":step,"action":"llm","status":"error","error":e.to_string()}));
                (String::new(), false)
            }
        };
        if reply.is_empty() {
            break;
        }
        // Chambua VITENDO VINGI (modeli nyingi huandika JSON kadhaa kwa jibu moja).
        // Jibu lilikatwa → okoa write_file ya sehemu + mwambie LLM iandike NDOGO.
        let actions = if truncated {
            transcript.push_str(
                "\nONYO: jibu lilikatwa (token limit). Andika faili MOJA ndogo kwa kila hatua \
                 (chini ya mistari ~150); gawanya kurasa kubwa kwenye partials. JSON:\n",
            );
            vec![salvage_truncated_write(&reply)]
        } else {
            parse_agent_actions(&reply)
        };
        if actions.is_empty() {
            break;
        }

        for act in &actions {
            if done {
                break;
            }
            let action = act["action"].as_str().unwrap_or("done").to_string();
            match action.as_str() {
                "write_file" => {
                    let path = act["path"].as_str().unwrap_or("").trim().to_string();
                    let content = act["content"].as_str().unwrap_or("").to_string();
                    let sig = format!("{path}:{}", content.len());
                    let dup = !written_sigs.insert(sig);
                    let valid = !path.is_empty() && !path.contains("..") && !path.starts_with('/');
                    let mut ok = false;
                    if valid && !dup {
                        let full = dir.join(&path);
                        if let Some(parent) = full.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        ok = std::fs::write(&full, &content).is_ok();
                    }
                    if ok {
                        files_written.push(path.clone());
                    }
                    let res = if dup {
                        format!("UMESHAANDIKA {path} — USIRUDIE; endelea na faili inayofuata au run.")
                    } else if ok {
                        format!("IMEANDIKWA: {path} ({} bytes)", content.len())
                    } else {
                        format!("KOSA: path si sahihi: {path}")
                    };
                    steps.push(json!({"step":step,"action":"write_file","path":path,"status":if ok {"ok"} else {"error"},"bytes":content.len()}));
                    emit(&sink, json!({"type":"write","step":step,"path":path,"status":if ok {"ok"} else {"error"},"bytes":content.len(),"dup":dup}));
                    transcript.push_str(&format!("\nHATUA {step}: write_file {path}.\nMATOKEO: {res}\n"));
                }
                "read_file" => {
                    let path = act["path"].as_str().unwrap_or("").trim().to_string();
                    let content = if path.is_empty() || path.contains("..") {
                        "KOSA: path si sahihi".to_string()
                    } else {
                        std::fs::read_to_string(dir.join(&path))
                            .map(|c| truncate_chars(&c, 2000))
                            .unwrap_or_else(|_| "KOSA: faili haipo".into())
                    };
                    steps.push(json!({"step":step,"action":"read_file","path":path,"status":"ok"}));
                    transcript.push_str(&format!("\nHATUA {step}: read_file {path}.\nMAUDHUI:\n{content}\n"));
                }
                "list_files" => {
                    let mut names: Vec<String> = Vec::new();
                    if let Ok(rd) = std::fs::read_dir(&dir) {
                        for e in rd.flatten() {
                            names.push(e.file_name().to_string_lossy().to_string());
                        }
                    }
                    names.sort();
                    steps.push(json!({"step":step,"action":"list_files","status":"ok","count":names.len()}));
                    transcript.push_str(&format!("\nHATUA {step}: list_files → {:?}\n", names));
                }
                "mkdir" => {
                    let path = act["path"].as_str().unwrap_or("").trim().to_string();
                    let ok = !path.is_empty()
                        && !path.contains("..")
                        && !path.starts_with('/')
                        && std::fs::create_dir_all(dir.join(&path)).is_ok();
                    steps.push(json!({"step":step,"action":"mkdir","path":path,"status":if ok {"ok"} else {"error"}}));
                    emit(&sink, json!({"type":"mkdir","step":step,"path":path,"status":if ok {"ok"} else {"error"}}));
                    transcript.push_str(&format!("\nHATUA {step}: mkdir {path} → ok={ok}\n"));
                }
                "run" => {
                    let command = act["command"].as_str().unwrap_or("").trim().to_string();
                    // KASI: zuia amri ile ile kurudiwa (waste ya muda).
                    if !run_seen.insert(command.clone()) {
                        steps.push(json!({"step":step,"action":"run","command":command,"status":"skipped-dup"}));
                        transcript.push_str(&format!("\nHATUA {step}: run `{command}` IMERUKWA (ilishakimbia). Endelea.\n"));
                        continue;
                    }
                    // GOVERNANCE (VisualClaw constitution): zuia amri hatari.
                    let (allowed, reason) = crate::unified_engines::governance_check(&command);
                    if !allowed {
                        steps.push(json!({"step":step,"action":"run","command":command,"status":"blocked","output":reason}));
                        transcript.push_str(&format!("\nHATUA {step}: run `{command}` ILIZUIWA.\nMATOKEO: {reason}\n"));
                        continue;
                    }
                    let (out, ok) = run_shell(&dir, &command, 120);
                    steps.push(json!({"step":step,"action":"run","command":command,"status":if ok {"ok"} else {"error"},"output":out}));
                    emit(&sink, json!({"type":"run","step":step,"command":command,"status":if ok {"ok"} else {"error"},"output":out}));
                    transcript.push_str(&format!("\nHATUA {step}: run `{command}`.\nMATOKEO (exit ok={ok}):\n{out}\n"));
                }
                "see_image" => {
                    // VISION (Magma): eleza picha — kama server ipo.
                    let img = act["image"].as_str().unwrap_or("").to_string();
                    let q = act["prompt"].as_str().unwrap_or("Eleza picha hii kwa ufupi.").to_string();
                    let (out, ok) = crate::unified_engines::see_image(&img, &q).await;
                    steps.push(json!({"step":step,"action":"see_image","status":if ok {"ok"} else {"unavailable"},"output":out}));
                    transcript.push_str(&format!("\nHATUA {step}: see_image.\nMATOKEO (ok={ok}):\n{out}\n"));
                }
                _ => {
                    // "done" si mwisho wa moja kwa moja: tunathibitisha kwanza.
                    let verify_cmd = verify_command(&dir);
                    let (vout, vok) = run_shell(&dir, &verify_cmd, 300);
                    steps.push(json!({
                        "step": step, "action": "verify", "status": if vok {"ok"} else {"error"},
                        "command": verify_cmd, "output": vout
                    }));
                    emit(&sink, json!({"type":"verify","step":step,"ok":vok,"command":verify_cmd,"output":vout,"round":verify_round}));
                    if vok {
                        let summary = act["summary"].as_str().unwrap_or("mradi umekamilika").to_string();
                        steps.push(json!({"step":step,"action":"done","status":"ok","summary":summary}));
                        emit(&sink, json!({"type":"done","step":step,"summary":summary,"verify_ok":true}));
                        done = true;
                    } else if verify_round < 5 {
                        // Kosa la uthibitisho → rudisha kwa LLM ili IREKEBISHE (si kuacha).
                        verify_round += 1;
                        emit(&sink, json!({"type":"fix_round","step":step,"round":verify_round,"max":5}));
                        transcript.push_str(&format!(
                            "\nUTHIBITISHO (`{verify_cmd}`) UMESHINDWA (mzunguko wa kujirekebisha {verify_round}/5).\n\
                             ==== KOSA HALISI (soma kwa makini) ====\n{vout}\n==== MWISHO WA KOSA ====\n\
                             REKEBISHA kwa hatua MADOGO: 1) tafuta mstari wenye `error`/`FAILED`; \
                             2) andika faili ILE yenye kosa kwa `write_file` (content KAMILI, si kipande); \
                             3) rudia `run` ikiwa ni lazima. Maliza (`done`) tu baada ya uthibitisho KUPITA. JSON:\n"
                        ));
                    } else {
                        let summary = act["summary"].as_str().unwrap_or("mradi umekamilika (uthibitisho haukupita)").to_string();
                        steps.push(json!({"step":step,"action":"done","status":"warning","summary":summary}));
                        emit(&sink, json!({"type":"done","step":step,"summary":summary,"verify_ok":false}));
                        done = true;
                    }
                }
            }
        }

        if done {
            break;
        }
        // Kama jibu halikuwa na `done`, mwambie LLM iendelee na hatua inayofuata.
        if step == max_steps {
            emit(&sink, json!({"type":"max_steps","step":step,"max":max_steps}));
        }
        transcript.push_str("\nEndelea na hatua INAYOFUATA (faili tofauti au run). JSON:\n");
        step += 1;
    }

    // ── Uthibitisho wa mwisho (independent) ──
    let verify_cmd = verify_command(&dir);
    emit(&sink, json!({"type":"verify","step":step,"ok":null,"command":verify_cmd,"round":verify_round,"final":true}));
    let (verify_out, verify_ok) = run_shell(&dir, &verify_cmd, 300);
    emit(&sink, json!({"type":"verify","step":step,"ok":verify_ok,"command":verify_cmd,"output":verify_out,"final":true}));

    // ── LANGI LA USALAMA (sec_gate) — LAZIMA kwa kila mradi uliozalishwa ──
    // Mzunguko: kagua kila faili → rekebisha yanayowezekana → kagua tena → amua.
    // Ripoti hii ni ushahidi kwamba code iliyotengenezwa imepimwa usalama.
    let gate = crate::sec_gate::gate_dir(&dir);
    emit(&sink, json!({
        "type": "security_gate", "step": step,
        "verdict": gate["verdict"], "grade": gate["grade"],
        "findings": gate["total_findings"], "blocking": gate["blocking"],
        "by_severity": gate["by_severity"], "files_scanned": gate["files_scanned"],
    }));
    let security_ok = gate["verdict"] == json!("pass");

    // MEMORY (VisualClaw-style): hifadhi somo la mradi kwa loop ijayo.
    let summary = format!(
        "{}: faili {} {}, uthibitisho {}",
        kind,
        files_written.len(),
        if verify_ok { "zimekamilika" } else { "zimeandikwa (verify ilishindwa)" },
        if verify_ok { "umepita" } else { "haukupita" }
    );
    crate::unified_engines::remember(&project, kind, verify_ok, &summary);
    // KUJIFUNZA KUTOKA UHALISIA: hifadhi somo kwenye kumbukumbu ya vector —
    // linaakisi matokeo HALISI ya uthibitisho, na litarejeshwa kwenye kazi zijazo.
    let _ = crate::nyi::record_lesson(
        "2nyi-agentic",
        &kind,
        &format!("{kind}: {project}"),
        &summary,
        if verify_ok { 0.9 } else { 0.4 },
    );

    let result = json!({
        "ok": true,
        "project": project,
        "workspace": dir.display().to_string(),
        "engine": "agentic_react",
        "model": cfg.model,
        "elapsed_ms": started.elapsed().as_millis() as u64,
        "steps_count": steps.len(),
        "files_written": files_written,
        "verify": { "ok": verify_ok, "command": verify_cmd, "output": verify_out },
        "security": {
            "verdict": gate["verdict"],
            "release_ok": security_ok,
            "grade": gate["grade"],
            "total_findings": gate["total_findings"],
            "blocking": gate["blocking"],
            "by_severity": gate["by_severity"],
            "findings": gate["findings"],
        },
        "steps": steps,
        "note": "Agentic loop (ReAct) — LLM iliandika faili, iliendesha, ikajirekebisha"
    });
    emit(&sink, json!({"type":"result","result":result.clone()}));
    store_last_result(&result);
    result
}

#[cfg(test)]
mod agentic_tests {
    use super::*;

    #[test]
    fn parse_write_file_clean_json() {
        let a = parse_agent_action(r#"{"action":"write_file","path":"a.js","content":"x"}"#);
        assert_eq!(a["action"], "write_file");
        assert_eq!(a["path"], "a.js");
    }

    #[test]
    fn parse_multiple_actions_from_one_reply() {
        let raw = "{\"action\":\"write_file\",\"path\":\"a.js\",\"content\":\"x\"}\n\
                   {\"action\":\"write_file\",\"path\":\"b.js\",\"content\":\"y\"}\n\
                   {\"action\":\"run\",\"command\":\"npm test\"}";
        let acts = parse_agent_actions(raw);
        assert_eq!(acts.len(), 3);
        assert_eq!(acts[1]["path"], "b.js");
        assert_eq!(acts[2]["action"], "run");
    }

    #[test]
    fn parse_actions_ignores_braces_in_content() {
        let raw = r#"{"action":"write_file","path":"a.js","content":"function f(){ return {}; }"}"#;
        let acts = parse_agent_actions(raw);
        assert_eq!(acts.len(), 1);
        assert!(acts[0]["content"].as_str().unwrap().contains("return {}"));
    }

    #[test]
    fn salvage_recovers_truncated_write() {
        let raw = "{\"action\":\"write_file\",\"path\":\"src/main.rs\",\"content\":\"fn main() {}\\nfn";
        let a = salvage_truncated_write(raw);
        assert_eq!(a["action"], "write_file");
        assert_eq!(a["path"], "src/main.rs");
        assert!(a["content"].as_str().unwrap().contains("fn main()"));
    }

    #[test]
    fn verify_picks_cargo_for_nested_rust() {
        let base = std::env::temp_dir().join(format!("vc_{}", std::process::id()));
        let proj = base.join("myapp");
        std::fs::create_dir_all(proj.join("src")).unwrap();
        std::fs::write(proj.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let cmd = verify_command(&base);
        assert!(cmd.contains("cargo test"), "cmd={cmd}");
        assert!(cmd.contains("cd myapp"), "cmd={cmd}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn verify_cargo_requires_real_tests() {
        let base = std::env::temp_dir().join(format!("vcr_{}", std::process::id()));
        std::fs::create_dir_all(base.join("src")).unwrap();
        std::fs::write(base.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let cmd = verify_command(&base);
        assert!(cmd.contains("test result: ok"), "lazima ithibitishe tests zimepita: {cmd}");
        assert!(cmd.contains("[1-9][0-9]* passed"), "lazima idai tests halisi: {cmd}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn verify_node_requires_test_script() {
        let base = std::env::temp_dir().join(format!("vcn_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("package.json"), "{}").unwrap();
        let cmd = verify_command(&base);
        assert!(cmd.contains("hakuna script ya test"), "cmd={cmd}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn verify_unknown_lang_requires_explicit_failure() {
        let base = std::env::temp_dir().join(format!("vcp_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let cmd = verify_command(&base);
        assert!(cmd.contains("exit 1"), "cmd={cmd}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn parse_json_wrapped_in_text() {
        let a = parse_agent_action("Hapa ni hatua:\n```json\n{\"action\":\"run\",\"command\":\"ls\"}\n```\n");
        assert_eq!(a["action"], "run");
        assert_eq!(a["command"], "ls");
    }

    #[test]
    fn parse_fenced_code_falls_back_to_write_file() {
        let a = parse_agent_action("```rust\nfn main() {}\n```");
        assert_eq!(a["action"], "write_file");
        assert_eq!(a["path"], "src/main.rs");
        assert!(a["content"].as_str().unwrap().contains("fn main"));
    }

    #[test]
    fn parse_garbage_becomes_done() {
        let a = parse_agent_action("sijui nini nifanye");
        assert_eq!(a["action"], "done");
    }

    #[test]
    fn slugify_is_kebab_and_short() {
        let s = slugify("Website ya Duka la Nguo Kubwa Sana");
        assert!(s.len() <= 40);
        assert!(!s.contains(' '));
        assert!(!s.starts_with('-'));
    }
}
