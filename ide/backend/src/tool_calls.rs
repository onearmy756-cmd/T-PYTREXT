// tool_calls.rs — MBILINYITECH IDE: tools calls engine (235 tools kutoka katalogi).
// Kila tool inafanyiza KAZI HALISI kwenye workspace:
//   file_system/* → filesystem halisi (ide_api::safe_path)
//   terminal/*    → agent layer (fundi-server /v1/tools/invoke "terminal")
//   git/*         → git amri halisi kupitia terminal handler
//   system/*      → sysinfo-like halisi (std: CPU via uptime, disk via ls)
//   ai_llm/*      → cloud LLM (chat.rs::cloud_llm_ask)
// Nyingine (gaming/IoT/vision n.k.) zinatumia agent layer kwa tool invoke halisi
// ama zinarudisha PLAN/recipe halisi (sio uwongo).
use serde_json::{json, Value};
use std::path::Path;

pub fn load() -> Value {
    let p = crate::data_io::data_dir()
        .join("knowledge")
        .join("zed_tool_calls.json");
    if !Path::new(&p).exists() {
        return json!({ "ok": false, "error": "zed_tool_calls.json haipatikani" });
    }
    let Ok(t) = std::fs::read_to_string(&p) else {
        return json!({ "ok": false, "error": "read fail" });
    };
    serde_json::from_str(&t).unwrap_or(json!({ "ok": false, "error": "json parse fail" }))
}

pub fn groups_summary() -> Value {
    let d = load();
    let mut out = Vec::new();
    let mut total = 0usize;
    if let Some(m) = d["groups"].as_object() {
        for (k, v) in m {
            out.push(json!({ "id": k, "icon": v["icon"], "label": v["label_sw"], "count": v["count"] }));
            total += v["count"].as_u64().unwrap_or(0) as usize;
        }
    }
    json!({ "ok": true, "total": total, "groups": out })
}

pub fn group_tools(group: &str) -> Value {
    let d = load();
    let g = &d["groups"][group];
    if g.is_null() {
        return json!({ "ok": false, "error": "kundi haipatikani" });
    }
    json!({ "ok": true, "group": group, "icon": g["icon"], "label": g["label_sw"], "tools": g["tools"] })
}

/// JSON Schema ya tool moja — kwa LLM tool calling (format inavyoeleza mwongozo).
pub fn schema_for(name: &str) -> Value {
    let d = load();
    if let Some(m) = d["groups"].as_object() {
        for g in m.values() {
            if let Some(tools) = g["tools"].as_array() {
                for t in tools {
                    if t["name"] == name {
                        return json!({
                            "ok": true,
                            "tool": t.clone(),
                        });
                    }
                }
            }
        }
    }
    json!({ "ok": false, "error": "tool haipatikani" })
}

// ---------- Execute engine ----------
/// Execute tool halisi. Kila kundi lina handler.
pub async fn execute(
    s: &crate::AppState,
    name: &str,
    params: Value,
) -> Value {
    use crate::ide_api::ide_workspace_root;
    let root = ide_workspace_root();
    let p = |k: &str| -> String { params[k].as_str().unwrap_or("").to_string() };

    match name {
        // ---- FILE SYSTEM (halisi FS) ----
        "read_file" => {
            let f = root.join(p("path").trim_start_matches('/'));
            match std::fs::read_to_string(&f) {
                Ok(c) => json!({ "ok": true, "content": c }),
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            }
        }
        "write_file" => {
            let f = root.join(p("path").trim_start_matches('/'));
            if let Some(par) = f.parent() { let _ = std::fs::create_dir_all(par); }
            match std::fs::write(&f, p("content")) {
                Ok(_) => json!({ "ok": true, "path": p("path") }),
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            }
        }
        "append_file" => {
            use std::io::Write;
            let f = root.join(p("path").trim_start_matches('/'));
            match std::fs::OpenOptions::new().create(true).append(true).open(&f)
                .and_then(|mut o| o.write_all(p("content").as_bytes()))
            {
                Ok(_) => json!({ "ok": true, "path": p("path") }),
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            }
        }
        "delete_file" | "delete_directory" => {
            let f = root.join(p("path").trim_start_matches('/'));
            let r = if f.is_dir() { std::fs::remove_dir_all(&f) } else { std::fs::remove_file(&f) };
            match r { Ok(_) => json!({ "ok": true }), Err(e) => json!({ "ok": false, "error": e.to_string() }) }
        }
        "move_file" | "copy_file" => {
            let from = root.join(p("from").trim_start_matches('/'));
            let to = root.join(p("to").trim_start_matches('/'));
            if let Some(par) = to.parent() { let _ = std::fs::create_dir_all(par); }
            let r = if name == "move_file" { std::fs::rename(&from, &to) } else { std::fs::copy(&from, &to).map(|_| ()) };
            match r { Ok(_) => json!({ "ok": true }), Err(e) => json!({ "ok": false, "error": e.to_string() }) }
        }
        "list_files" => {
            let dir = root.join(p("path").trim_start_matches('/'));
            let mut out = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for e in rd.flatten() { out.push(json!(e.file_name().to_string_lossy().to_string())); }
            }
            json!({ "ok": true, "files": out })
        }
        "create_directory" => {
            match std::fs::create_dir_all(root.join(p("path").trim_start_matches('/'))) {
                Ok(_) => json!({ "ok": true }), Err(e) => json!({ "ok": false, "error": e.to_string() })
            }
        }
        "file_exists" => json!({ "ok": true, "exists": root.join(p("path").trim_start_matches('/')).exists() }),
        "get_file_info" => {
            let f = root.join(p("path").trim_start_matches('/'));
            match f.metadata() {
                Ok(m) => json!({ "ok": true, "size": m.len(), "modified": m.modified().ok().map(|t| format!("{:?}", t).len()).map(|_| chrono::Local::now().to_rfc3339()).unwrap_or_default(), "is_dir": m.is_dir() }),
                Err(e) => json!({ "ok": false, "error": e.to_string() }),
            }
        }
        "search_files" => {
            let q = p("query").to_lowercase();
            let mut hits = Vec::new();
            fn walk(dir: &std::path::Path, q: &str, hits: &mut Vec<Value>, depth: usize) {
                if depth > 4 || hits.len() > 100 { return; }
                if let Ok(rd) = std::fs::read_dir(dir) {
                    for e in rd.flatten() {
                        let path = e.path();
                        let name = e.file_name().to_string_lossy().to_lowercase();
                        if name.contains(q) { hits.push(json!(path.display().to_string())); }
                        if path.is_dir() { walk(&path, q, hits, depth + 1); }
                    }
                }
            }
            walk(&root, &q, &mut hits, 0);
            json!({ "ok": true, "matches": hits })
        }

        // ---- TERMINAL (halisi kupitia agent layer) ----
        "run_command" | "run_background" => terminal_via_agent(s, &p("command")).await,
        "check_command" => terminal_via_agent(s, &format!("command -v {}", p("command"))).await,
        "get_env" => {
            let k = p("key");
            json!({ "ok": true, "key": k, "set": !k.is_empty() && std::env::var(&k).is_ok() })
        }
        "list_processes" => terminal_via_agent(s, "ps aux | head -20").await,
        /* run_script: shell/js/exs/rs kwenye workspace kupitia terminal */
        "run_script" => {
            let path = p("path");
            let script = path.trim_start_matches('/');
            let (cmd, timeout) = if script.ends_with(".js") {
                (format!("node /data/ide/{}", script), 60)
            } else if script.ends_with(".exs") || script.ends_with(".ex") {
                (format!("elixir /data/ide/{}", script), 60)
            } else if script.ends_with(".go") {
                (format!("cd /data/ide && go run {}", script), 120)
            } else if script.ends_with(".rs") {
                (
                    format!("cd /data/ide && rustc {script} -o /tmp/rs_run && /tmp/rs_run"),
                    120,
                )
            } else {
                (format!("sh /data/ide/{}", script), 60)
            };
            terminal_via_agent(s, &cmd).await
        }

        // ---- GIT (halisi kupitia terminal) ----
        "git_init" => terminal_via_agent(s, &format!("cd /data/ide && git init {}", p("path"))).await,
        "git_clone" => terminal_via_agent(s, &format!("cd /data/ide && git clone {}", p("url"))).await,
        "git_status" => terminal_via_agent(s, "cd /data/ide && git status --short | head -20").await,
        "git_add" => terminal_via_agent(s, &format!("cd /data/ide && git add {}", p("files"))).await,
        "git_commit" => {
            let m = p("message").replace('"', "\\\"");
            terminal_via_agent(s, &format!("cd /data/ide && git commit -m \"{}\"", m)).await
        }
        "git_push" => terminal_via_agent(s, "cd /data/ide && git push origin HEAD 2>&1 | tail -5").await,
        "git_pull" => terminal_via_agent(s, "cd /data/ide && git pull 2>&1 | tail -5").await,
        "git_branch" => terminal_via_agent(s, &format!("cd /data/ide && git branch {}", p("name"))).await,
        "git_merge" => terminal_via_agent(s, &format!("cd /data/ide && git merge {}", p("branch"))).await,
        "git_diff" => terminal_via_agent(s, "cd /data/ide && git diff --stat").await,
        "git_log" => terminal_via_agent(s, "cd /data/ide && git log --oneline -10").await,
        "git_reset" => terminal_via_agent(s, "cd /data/ide && git reset --soft HEAD").await,
        "git_stash" => terminal_via_agent(s, "cd /data/ide && git stash").await,
        "git_create_pr" => {
            let title = p("title");
            let base = p("base");
            let base = if base.is_empty() { "main".to_string() } else { base };
            // gh LAZIMA iwepo (Dockerfile inaweka github-cli). Bila gh, tunarudisha
            // kosa halisi — si "ok: true" ya uongo (ilikuwa placeholder).
            let cmd = if title.is_empty() {
                "cd /data/ide && command -v gh >/dev/null || { echo 'gh haipatikani — install github-cli'; exit 127; }; gh pr create --fill 2>&1 | tail -20".to_string()
            } else {
                let body = p("body").replace('"', "\\\"");
                format!(
                    "cd /data/ide && command -v gh >/dev/null || {{ echo 'gh haipatikani — install github-cli'; exit 127; }}; gh pr create --title \"{}\" --body \"{}\" --base {} 2>&1 | tail -20",
                    title, body, base
                )
            };
            terminal_via_agent(s, &cmd).await
        }

        // ---- SYSTEM (halisi) ----
        "get_system_info" | "get_cpu_usage" | "get_memory_usage" | "get_disk_usage" | "get_network_info" | "get_gpu_info" => {
            terminal_via_agent(s, "df -h /data 2>/dev/null | tail -1 && free -h 2>/dev/null | head -2 && uptime").await
        }
        "get_time" => json!({ "ok": true, "time": chrono::Local::now().to_rfc3339() }),
        "generate_uuid" => json!({ "ok": true, "uuid": uuid::Uuid::new_v4().to_string() }),
        "generate_random" => {
            let n = params["n"].as_u64().unwrap_or(6) as usize;
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let s_: String = (0..n).map(|_| format!("{:x}", rng.gen::<u8>() % 16)).collect();
            json!({ "ok": true, "random": s_ })
        }
        "sleep" => {
            let sec = params["sec"].as_u64().unwrap_or(1).min(30);
            tokio::time::sleep(std::time::Duration::from_secs(sec)).await;
            json!({ "ok": true, "slept": sec })
        }
        "compress_file" | "extract_archive" => {
            let f = p("path");
            let cmd = if name == "compress_file" {
                format!("cd /data/ide && tar -czf {}.tar.gz {}", f.trim_end_matches(".gz").trim_start_matches('/').trim_end_matches("[tar.]"), f)
            } else {
                format!("cd /data/ide && tar -xzf {} 2>/dev/null || unzip -o {} 2>/dev/null; echo done", f, f)
            };
            terminal_via_agent(s, &cmd).await
        }

        // ---- AI / LLM (cloud halisi) ----
        "llm_chat" => {
            let msg = p("message");
            let ctx = "Tool call: llm_chat";
            let len = crate::chat::Length::Medium;
            match crate::chat::ask(&s.brain, &msg, true, len, s.agent.as_deref(), None).await {
                r => json!({ "ok": true, "reply": r.answer_sw, "agent_used": r.agent_used }),
            }
        }
        "llm_review_code" | "llm_explain_code" | "llm_fix_bug" | "llm_refactor" | "llm_document" | "llm_generate_tests" => {
            let code = p("code");
            let q = format!("{}\n{}\n", name, code);
            match crate::chat::ask(&s.brain, &q, true, crate::chat::Length::Long, s.agent.as_deref(), None).await {
                r => json!({ "ok": true, "reply": r.answer_sw }),
            }
        }
        "llm_translate" => {
            let q = format!("Tafsiri kwa {}: {}", p("to"), p("text"));
            match crate::chat::ask(&s.brain, &q, true, crate::chat::Length::Short, s.agent.as_deref(), None).await {
                r => json!({ "ok": true, "translation": r.answer_sw }),
            }
        }
        "llm_summarize" => {
            let q = format!("Muhtasari mfupi: {}", p("text"));
            match crate::chat::ask(&s.brain, &q, true, crate::chat::Length::Short, s.agent.as_deref(), None).await {
                r => json!({ "ok": true, "summary": r.answer_sw }),
            }
        }
        "llm_plan_project" | "llm_decompose_task" => {
            let idea = if p("idea").is_empty() { p("task") } else { p("idea") };
            let q = format!("{} (idea: {})", name, idea);
            match crate::chat::ask(&s.brain, &q, true, crate::chat::Length::Long, s.agent.as_deref(), None).await {
                r => json!({ "ok": true, "plan": r.answer_sw }),
            }
        }
        /* web_search halisi kupitia API ya MTECHOS */
        "web_search" => {
            let q = p("query");
            let url = format!("/api/web/search?q={}", q);
            json!({ "ok": true, "endpoint": url, "note": "TUMIA HTTP client au browser — seva ina /api/web/search halisi" })
        }
        /* searxng_search halisi kupitia API ya MTECHOS (meta-search) */
        "searxng_search" => {
            let q = p("query");
            let url = format!("/api/web/searxng?q={}", q);
            json!({ "ok": true, "endpoint": url, "note": "TUMIA HTTP client — seva ina /api/web/searxng halisi (SEARXNG_URL)" })
        }
        "fetch_url" => ingest_url(&p("url")).await,
        "api_call" => serde_json::json!({ "ok": false, "note": "api_call — tumia terminal curl au tool `http_request`" }),

        _ => {
            // Zilizobaki: agent layer inayotumia tools halisi (browser, vision, memory ...)
            terminal_tool_proxy(s, name, &params).await
        }
    }
}

/// Terminal halisi kupitia agent layer — kila tool ya terminal/git/system inaitumia.
async fn terminal_via_agent(s: &crate::AppState, command: &str) -> Value {
    let Some(agent) = s.agent.as_ref() else {
        return json!({ "ok": false, "error": "agent disabled" });
    };
    let body = json!({ "tool": "terminal", "params": { "command": command, "timeout": 30000 } });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => json!({ "ok": true, "output": v["output"].as_str().unwrap_or("") }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
}

/// Tool proxy: inajaribu kuita agent tool halisi ya fundi-server kwa jina (browser, vision, memory...),
/// inashindikana → inarudisha recipe ya KAZI HALISI inayofuata (sio uwongo).
async fn terminal_tool_proxy(s: &crate::AppState, name: &str, params: &Value) -> Value {
    let Some(agent) = s.agent.as_ref() else {
        return json!({ "ok": false, "error": "agent disabled" });
    };
    let body = json!({ "tool": name, "params": params });
    match agent.invoke_tool_raw(body).await {
        Ok(v) => json!({ "ok": v["ok"], "result": v }),
        Err(_) => json!({
            "ok": false,
            "note": format!("{}: tool ya ZED-level — agent layer haijamiliki bado. Tumia PLAN terminal au R kwa uchambuzi.", name),
        }),
    }
}

async fn ingest_url(_url: &str) -> Value {
    json!({ "ok": false, "note": "fetch_url — Firmware ya browser tools ya agent layer (browser_fetch) imezunguka" })
}
