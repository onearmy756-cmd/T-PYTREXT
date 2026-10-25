//! conversations.rs — conversation lifecycle, events, sockets (WS), bash,
//! file, git, workspaces, skills, plugins, tools, na njia nyingine za
//! protocol ya agent-server (Rust-native, bila Python).

use axum::{
    body::Bytes,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::events as ev;
use super::state::{self, jstr, lock_store, persist};
use crate::AppState;

fn ok(v: Value) -> Json<Value> {
    Json(v)
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "detail": msg })))
}

/// Workspace ya msingi ya agent (dir ya kazi ya conversation).
fn workspace_root() -> std::path::PathBuf {
    if let Ok(v) = std::env::var("FUNDI_WORKSPACE") {
        if !v.trim().is_empty() {
            return std::path::PathBuf::from(v);
        }
    }
    // LAZIMA ipatane na `ide_api::ide_workspace_root()` — la sivyo UI file panel
    // na write_file huonyesha/kuandika kwenye roots tofauti.
    let ide = crate::ide_api::ide_workspace_root();
    if ide.is_dir() {
        return ide;
    }
    let cand = std::path::PathBuf::from("/home/openhands/mbilinyi-ide");
    if cand.is_dir() {
        return cand;
    }
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Tafuta conversation kwa id.
fn find_convo(s: &state::NativeStore, id: &str) -> Option<Value> {
    s.convos.iter().find(|c| jstr(c, "id") == id).cloned()
}

/// Ongeza event kwenye log ya conversation + tangaza `durable` frame kwa WS.
fn push_event(s: &mut state::NativeStore, convo_id: &str, event: Value) -> u64 {
    let log = s.events.entry(convo_id.to_string()).or_default();
    log.push(event.clone());
    let seq = (log.len() - 1) as u64;
    state::publish_frame(convo_id, ev::frame_durable(seq, event));
    seq
}

/// Weka SystemPromptEvent mwanzoni (Canvas huionyesha kama history halisi).
fn seed_system_prompt(convo_id: &str) {
    const SYSTEM_PROMPT: &str = "Wewe ni MTECHOS Agent — msaidizi wa AI wa kujenga programu kwenye kompyuta. \
Unatekeleza amri, hubadilisha faili, na hutatua matatizo ya kiufundi. \
Injini ni ya Rust (fundi-engine); hakuna Python kwenye runtime.";
    let mut s = lock_store();
    if s.events.get(convo_id).map(|l| l.is_empty()).unwrap_or(true) {
        let e = ev::system_prompt_event(SYSTEM_PROMPT);
        push_event(&mut s, convo_id, e);
        persist(&s);
    }
}

/// Parent id ya event ya mwisho (mti wa events).
fn last_parent(s: &state::NativeStore, convo_id: &str) -> Option<String> {
    s.events
        .get(convo_id)
        .and_then(|v| v.last())
        .and_then(|e| e.get("id"))
        .and_then(|x| x.as_str())
        .map(String::from)
}

// ── Probes + metadata ────────────────────────────────────────────────────

pub async fn alive() -> Json<Value> {
    ok(json!({ "status": "ok" }))
}

pub async fn ready() -> Json<Value> {
    ok(json!({ "status": "ok" }))
}

/// GET /server_info — metadata ya server (Rust-native runtime).
pub async fn server_info() -> Json<Value> {
    ok(json!({
        "uptime": 0.0,
        "idle_time": 0.0,
        "title": "MTECHOS Agent Runtime",
        "version": "1.54.0",
        "sdk_version": "1.55.1",
        "tools_version": "1.55.1",
        "workspace_version": "unknown",
        "build_git_sha": null,
        "build_git_ref": null,
        "python_version": null,
        "runtime": state::runtime_name(),
        "native_protocol_version": super::NATIVE_PROTOCOL_VERSION,
        "usable_tools": crate::agent_catalogs::USABLE_TOOLS,
        "runtime_idle_timeout_seconds": 1200.0,
        "conversation_runtime": "local",
        "capabilities": crate::agent_catalogs::CAPABILITIES,
        "app_backend_ingress_url": null,
        "max_foreground_terminal_timeout_seconds": 1080.0,
        "docs": null,
        "redoc": null,
    }))
}

/// GET /api/init — server haiko kwenye deferred_init.
pub async fn init_get() -> Json<Value> {
    ok(json!({ "detail": "server haianzishwi na deferred_init; /api/init haipatikani" }))
}

/// POST /api/init.
pub async fn init_post() -> Json<Value> {
    ok(json!({ "success": true }))
}

// ── Conversations ────────────────────────────────────────────────────────

/// GET /api/conversations?ids=a,b — batch get.
///
/// HURUDISHA BARE ARRAY (si `{items}`): `ConversationClient.getConversations`
/// hupitisha jibu moja kwa moja kwa `requireDirectConversationBatch`, ambayo
/// huhitaji `Array.isArray(items)`. `{items: [...]}` husababisha
/// "invalid conversation response" na UI kurudi kwenye list.
pub async fn batch_get_conversations(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let ids: Vec<String> = q
        .get("ids")
        .map(|s| s.split(',').map(|x| x.trim().to_string()).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        return Json(Value::Array(vec![]));
    }
    let s = lock_store();
    let items: Vec<Value> = ids
        .iter()
        .map(|id| find_convo(&s, id).unwrap_or(Value::Null))
        .collect();
    Json(Value::Array(items))
}

/// GET /api/conversations/search — orodha ya conversations.
pub async fn search_conversations(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let limit: usize = q
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(20);
    let s = lock_store();
    let mut items: Vec<Value> = s.convos.clone();
    // Mpangilio wa hivi karibuni kwanza.
    items.sort_by(|a, b| jstr(b, "updated_at").cmp(&jstr(a, "updated_at")));
    items.truncate(limit);
    ok(json!({ "items": items, "next_page_id": null }))
}

/// GET /api/conversations/count.
pub async fn count_conversations() -> Json<Value> {
    let s = lock_store();
    ok(json!(s.convos.len()))
}

/// POST /api/conversations — anzisha conversation.
pub async fn start_conversation(
    State(app): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let convo = build_conversation(&body);
    let id = jstr(&convo, "id");
    {
        let mut s = lock_store();
        s.convos.push(convo.clone());
        persist(&s);
    }
    // Ujumbe wa kwanza (kama upo) → endesha agent mara moja.
    if let Some(prompt) = initial_prompt(&body) {
        send_and_run(app, id.clone(), prompt);
    }
    (StatusCode::CREATED, Json(convo)).into_response()
}

/// Hutatua `working_dir` ya conversation mpya kwa uthibitisho.
///
/// UI hupeana path inayotokana na `DEFAULT_WORKING_DIR` (mf.
/// `/home/openhands/workspace/project/<hex>`) ambayo HAIKUI kwenye diski. Wakati
/// huo `write_file` huandika kila mara kwenye `ide_workspace_root()` (`/data/ide`).
/// Matokeo: Files panel ilionyesha mti mtupu ingawa kazi ilifanyika.
///
/// Kwa hiyo: kama path iliyotolewa ipo, itumie; la sivyo anguka kwenye root
/// halisi ya workspace. Path kamili ipo tayari inaruhusiwa (UI hupeana kamili).
fn resolve_working_dir(body: &Value) -> String {
    let root = workspace_root();
    let requested = body
        .pointer("/workspace/working_dir")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(p) = requested {
        let path = std::path::Path::new(p);
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(p)
        };
        if abs.is_dir() {
            return abs.display().to_string();
        }
        // Haipo: tengeneza ikiwa iko ndani ya root, la sivyo tumia root.
        if abs.starts_with(&root) {
            if std::fs::create_dir_all(&abs).is_ok() {
                return abs.display().to_string();
            }
        }
    }
    root.display().to_string()
}

/// Huunda ConversationInfo kutoka ombi (schema ya agent-server).
fn build_conversation(body: &Value) -> Value {
    let id = body
        .get("conversation_id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(state::new_id);
    let working_dir = resolve_working_dir(body);
    let max_iterations = body
        .get("max_iterations")
        .and_then(|v| v.as_i64())
        .unwrap_or(500);
    let agent_settings = body
        .get("agent_settings")
        .cloned()
        .unwrap_or_else(|| state::default_settings()["agent_settings"].clone());
    let now = state::now_rfc3339();
    json!({
        "id": id,
        "workspace": { "working_dir": working_dir, "kind": "LocalWorkspace" },
        "persistence_dir": null,
        "conversation_url": null,
        "session_api_key": null,
        "max_iterations": max_iterations,
        "stuck_detection": body.get("stuck_detection").and_then(|v| v.as_bool()).unwrap_or(true),
        "execution_status": "idle",
        "confirmation_policy": { "kind": "NeverConfirm" },
        "security_analyzer": null,
        "activated_knowledge_skills": [],
        "invoked_skills": [],
        "blocked_actions": {},
        "blocked_messages": {},
        "last_user_message_id": null,
        "leaf_event_id": null,
        "stats": { "usage_to_metrics": {} },
        "secret_registry": { "secret_sources": {} },
        "agent_state": {},
        "hook_config": null,
        "title": body.get("title").cloned().unwrap_or(Value::Null),
        "metrics": null,
        "created_at": now,
        "updated_at": now,
        "forked_from_conversation_id": body.get("parent_conversation_id").cloned().unwrap_or(Value::Null),
        "forked_from_event_id": null,
        "parent_conversation_id": body.get("parent_conversation_id").cloned().unwrap_or(Value::Null),
        "sub_conversation_ids": [],
        "tags": body.get("tags").cloned().unwrap_or(json!({})),
        "current_model_id": agent_settings.pointer("/llm/model").cloned().unwrap_or(Value::Null),
        "available_models": [],
        "supports_runtime_model_switch": false,
        "launched_agent_profile": null,
        "agent": agent_settings,
        "client_tools": body.get("client_tools").cloned().unwrap_or(json!([])),
        "tool_module_qualnames": {
            "terminal": "fundi.tools.terminal",
            "file_editor": "fundi.tools.file_editor",
            "task_tracker": "fundi.tools.task_tracker",
            "browser_tool_set": "fundi.tools.browser",
        },
        "runtime_info": null,
    })
}

/// Ujumbe wa kwanza kutoka ombi (`initial_message`).
fn initial_prompt(body: &Value) -> Option<String> {
    let im = body.get("initial_message")?;
    text_of_content(im.get("content"))
}

/// Toa maandishi kutoka `content` (string au array ya TextContent).
fn text_of_content(content: Option<&Value>) -> Option<String> {
    let c = content?;
    if let Some(s) = c.as_str() {
        return Some(s.to_string());
    }
    let arr = c.as_array()?;
    let mut out = String::new();
    for part in arr {
        if let Some(t) = part.get("text").and_then(|x| x.as_str()) {
            out.push_str(t);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// GET /api/conversations/:id.
pub async fn get_conversation(Path(id): Path<String>) -> Response {
    let s = lock_store();
    match find_convo(&s, &id) {
        Some(c) => Json(c).into_response(),
        None => err(StatusCode::NOT_FOUND, "conversation haipo").into_response(),
    }
}

/// PATCH /api/conversations/:id — badilisha title/tags.
pub async fn update_conversation(Path(id): Path<String>, Json(body): Json<Value>) -> Response {
    let mut s = lock_store();
    let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) else {
        return err(StatusCode::NOT_FOUND, "conversation haipo").into_response();
    };
    if let Some(o) = c.as_object_mut() {
        if let Some(t) = body.get("title") {
            o.insert("title".into(), t.clone());
        }
        if let Some(t) = body.get("tags") {
            o.insert("tags".into(), t.clone());
        }
        o.insert("updated_at".into(), json!(state::now_rfc3339()));
    }
    let out = c.clone();
    persist(&s);
    Json(out).into_response()
}

/// DELETE /api/conversations/:id.
pub async fn delete_conversation(Path(id): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.convos.retain(|c| jstr(c, "id") != id);
        s.events.remove(&id);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/fork.
pub async fn fork_conversation(Path(id): Path<String>, Json(body): Json<Value>) -> Response {
    let mut s = lock_store();
    let Some(src) = find_convo(&s, &id) else {
        return err(StatusCode::NOT_FOUND, "conversation haipo").into_response();
    };
    let new_id = state::new_id();
    let mut forked = src.clone();
    if let Some(o) = forked.as_object_mut() {
        o.insert("id".into(), json!(new_id));
        o.insert("forked_from_conversation_id".into(), json!(id));
        o.insert("created_at".into(), json!(state::now_rfc3339()));
        o.insert("updated_at".into(), json!(state::now_rfc3339()));
        o.insert("execution_status".into(), json!("idle"));
    }
    let _ = body;
    s.convos.push(forked.clone());
    // Nakili events za chanzo.
    if let Some(log) = s.events.get(&id).cloned() {
        s.events.insert(new_id, log);
    }
    persist(&s);
    (StatusCode::CREATED, Json(forked)).into_response()
}

/// POST /api/conversations/:id/confirmation_policy.
pub async fn set_confirmation_policy(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            o.insert("confirmation_policy".into(), body);
        }
    }
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/security_analyzer.
pub async fn set_security_analyzer(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            o.insert("security_analyzer".into(), body);
        }
    }
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/switch_llm.
pub async fn switch_llm(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let model = jstr(&body, "model");
    let mut s = lock_store();
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            if let Some(llm) = o.get_mut("agent").and_then(|a| a.get_mut("llm")) {
                if let Some(llo) = llm.as_object_mut() {
                    if !model.is_empty() {
                        llo.insert("model".into(), json!(model));
                    }
                }
            }
            o.insert("current_model_id".into(), json!(model));
        }
    }
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/switch_profile.
pub async fn switch_profile(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let name = body
        .get("profile")
        .or_else(|| body.get("profile_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut s = lock_store();
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            o.insert("launched_agent_profile".into(), json!(name));
        }
    }
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/switch_acp_model.
pub async fn switch_acp_model(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    switch_llm(Path(id), Json(body)).await
}

/// POST /api/conversations/:id/secrets.
pub async fn add_secrets(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    if let Some(list) = body.as_array() {
        for item in list {
            let name = jstr(item, "name");
            let val = jstr(item, "value");
            if !name.is_empty() {
                s.secrets.insert(name.clone(), val);
            }
        }
    } else {
        let name = jstr(&body, "name");
        if !name.is_empty() {
            s.secrets.insert(name, jstr(&body, "value"));
        }
    }
    let _ = id;
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/load_plugin.
pub async fn load_plugin(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "plugin_name");
    let mut s = lock_store();
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            let invoked = o
                .entry("invoked_skills".to_string())
                .or_insert_with(|| json!([]));
            if let Some(arr) = invoked.as_array_mut() {
                arr.push(json!(name));
            }
        }
    }
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/navigate.
pub async fn navigate(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    let parent = last_parent(&s, &id);
    let e = ev::message_event("user", &format!("[navigate] {}", jstr(&body, "url")), "user", parent.as_deref());
    push_event(&mut s, &id, e);
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/condense.
pub async fn condense(Path(id): Path<String>) -> Json<Value> {
    let mut s = lock_store();
    let parent = last_parent(&s, &id);
    let e = ev::state_update("condensation_requested", json!(true), parent.as_deref());
    push_event(&mut s, &id, e);
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/pause.
pub async fn pause_conversation(Path(id): Path<String>) -> Json<Value> {
    set_status(&id, "paused");
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/interrupt.
pub async fn interrupt_conversation(Path(id): Path<String>) -> Json<Value> {
    set_status(&id, "idle");
    let mut s = lock_store();
    let parent = last_parent(&s, &id);
    let e = ev::state_update("interrupted", json!(true), parent.as_deref());
    push_event(&mut s, &id, e);
    persist(&s);
    ok(json!({ "success": true }))
}

fn set_status(id: &str, status: &str) {
    let mut s = lock_store();
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            o.insert("execution_status".into(), json!(status));
            o.insert("updated_at".into(), json!(state::now_rfc3339()));
        }
    }
    persist(&s);
}

/// GET /api/conversations/:id/runtime.
pub async fn get_runtime(Path(id): Path<String>) -> Json<Value> {
    let s = lock_store();
    let wd = find_convo(&s, &id)
        .and_then(|c| c.pointer("/workspace/working_dir").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| workspace_root().display().to_string());
    ok(json!({
        "id": id,
        "runtime": state::runtime_name(),
        "kind": "LocalWorkspace",
        "working_dir": wd,
        "status": "ready",
    }))
}

/// POST /api/conversations/:id/runtime/reprovision.
pub async fn reprovision_runtime(Path(id): Path<String>) -> Json<Value> {
    let s = lock_store();
    let wd = find_convo(&s, &id)
        .and_then(|c| c.pointer("/workspace/working_dir").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| workspace_root().display().to_string());
    ok(json!({ "success": true, "id": id, "working_dir": wd }))
}

/// GET /api/conversations/:id/workspace — orodha ya faili.
pub async fn list_workspace_files(Path(id): Path<String>) -> Json<Value> {
    let s = lock_store();
    let wd = find_convo(&s, &id)
        .and_then(|c| c.pointer("/workspace/working_dir").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| workspace_root().display().to_string());
    drop(s);
    ok(json!({ "files": list_dir_names(std::path::Path::new(&wd)), "working_dir": wd }))
}

fn list_dir_names(dir: &std::path::Path) -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            out.push(json!(e.file_name().to_string_lossy()));
        }
    }
    out.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    out
}

/// GET /api/conversations/:id/workspace/*path — faili moja.
pub async fn get_workspace_file(Path((id, path)): Path<(String, String)>) -> Response {
    let s = lock_store();
    let wd = find_convo(&s, &id)
        .and_then(|c| c.pointer("/workspace/working_dir").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| workspace_root().display().to_string());
    drop(s);
    let full = std::path::Path::new(&wd).join(&path);
    match std::fs::read_to_string(&full) {
        Ok(txt) => Json(json!({ "path": path, "content": txt })).into_response(),
        Err(e) => err(StatusCode::NOT_FOUND, &format!("faili haipatikani: {e}")).into_response(),
    }
}

/// POST /api/conversations/:id/goal.
pub async fn set_goal(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    let parent = last_parent(&s, &id);
    let e = ev::state_update("goal", body.clone(), parent.as_deref());
    push_event(&mut s, &id, e);
    persist(&s);
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/goal/resume.
pub async fn goal_resume(Path(id): Path<String>) -> Json<Value> {
    set_status(&id, "running");
    ok(json!({ "success": true }))
}

/// POST /api/conversations/:id/goal/stop.
pub async fn goal_stop(Path(id): Path<String>) -> Json<Value> {
    set_status(&id, "idle");
    ok(json!({ "success": true }))
}

/// GET /api/conversations/:id/agent_final_response.
pub async fn agent_final_response(Path(id): Path<String>) -> Json<Value> {
    let s = lock_store();
    let text = s
        .events
        .get(&id)
        .and_then(|log| {
            log.iter().rev().find_map(|e| {
                if jstr(e, "kind") == "MessageEvent" && jstr(e, "source") == "agent" {
                    text_of_content(e.pointer("/llm_message/content"))
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();
    ok(json!({ "response": text }))
}

/// POST /api/conversations/:id/ask_agent.
pub async fn ask_agent(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let q = jstr(&body, "question");
    ok(json!({ "id": id, "response": format!("Swali limepokelewa: {q}") }))
}

// ── Events ───────────────────────────────────────────────────────────────

/// GET /api/conversations/:id/events — events zote.
pub async fn get_events(Path(id): Path<String>, Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let limit: usize = q
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let s = lock_store();
    let log = s.events.get(&id).cloned().unwrap_or_default();
    let items: Vec<Value> = log.into_iter().take(limit).collect();
    ok(json!({ "items": items }))
}

/// GET /api/conversations/:id/events/search — pagination + sort halisi.
///
/// Husaidi `sort_order` (`TIMESTAMP` asc / `TIMESTAMP_DESC` desc), `limit`,
/// `page_id` (offset) na `kind` — sawa na RemoteEventsList ya Canvas.
pub async fn search_events(Path(id): Path<String>, Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let limit: usize = q
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);
    let offset: usize = q
        .get("page_id")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let kind = q.get("kind").cloned().unwrap_or_default();
    let desc = q
        .get("sort_order")
        .map(|v| v.to_uppercase().contains("DESC"))
        .unwrap_or(false);
    let s = lock_store();
    let mut log = s.events.get(&id).cloned().unwrap_or_default();
    if !kind.is_empty() {
        log.retain(|e| jstr(e, "kind") == kind);
    }
    if desc {
        log.reverse();
    }
    let total = log.len();
    let page: Vec<Value> = log.into_iter().skip(offset).take(limit).collect();
    let next = if offset + page.len() < total {
        Some((offset + page.len()).to_string())
    } else {
        None
    };
    ok(json!({ "items": page, "next_page_id": next }))
}

/// GET /api/conversations/:id/events/count.
pub async fn events_count(Path(id): Path<String>) -> Json<Value> {
    let s = lock_store();
    ok(json!(s.events.get(&id).map(|v| v.len()).unwrap_or(0)))
}

/// GET /api/conversations/:id/events/:event_id.
pub async fn get_event(Path((id, event_id)): Path<(String, String)>) -> Response {
    let s = lock_store();
    let found = s
        .events
        .get(&id)
        .and_then(|log| log.iter().find(|e| jstr(e, "id") == event_id).cloned());
    match found {
        Some(e) => Json(e).into_response(),
        None => err(StatusCode::NOT_FOUND, "event haipo").into_response(),
    }
}

/// POST /api/conversations/:id/events — tuma ujumbe (run=true → endesha).
pub async fn send_message(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    let role = state::role_of(&body);
    let text = text_of_content(body.get("content")).unwrap_or_default();
    if !convo_exists(&id) {
        return err(StatusCode::NOT_FOUND, "conversation haipo").into_response();
    }
    let run = body.get("run").and_then(|v| v.as_bool()).unwrap_or(false);
    append_message(&id, &role, &text, if role == "user" { "user" } else { "agent" });
    if run && role == "user" {
        send_and_run(app, id, text);
    }
    Json(json!({ "success": true })).into_response()
}

fn convo_exists(id: &str) -> bool {
    let s = lock_store();
    s.convos.iter().any(|c| jstr(c, "id") == id)
}

fn append_message(id: &str, role: &str, text: &str, source: &str) {
    let mut s = lock_store();
    let parent = last_parent(&s, id);
    let e = ev::message_event(role, text, source, parent.as_deref());
    let eid = jstr(&e, "id");
    push_event(&mut s, id, e);
    if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == id) {
        if let Some(o) = c.as_object_mut() {
            o.insert("updated_at".into(), json!(state::now_rfc3339()));
            if source == "user" {
                o.insert("last_user_message_id".into(), json!(eid));
            }
            o.insert("leaf_event_id".into(), json!(eid));
        }
    }
    persist(&s);
}

/// POST /api/conversations/:id/events/respond_to_confirmation.
pub async fn respond_to_confirmation(Path(id): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    let parent = last_parent(&s, &id);
    let e = ev::state_update("confirmation_response", body, parent.as_deref());
    push_event(&mut s, &id, e);
    persist(&s);
    ok(json!({ "success": true }))
}

// ── Mzunguko wa agent (HALISI — Rust, bila Python) ───────────────────────

/// POST /api/conversations/:id/run — endesha agent.
pub async fn run_conversation(State(app): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let prompt = last_user_text(&id).unwrap_or_else(|| "Endelea na kazi.".to_string());
    send_and_run(app, id, prompt);
    ok(json!({ "success": true }))
}

fn last_user_text(id: &str) -> Option<String> {
    let s = lock_store();
    s.events.get(id).and_then(|log| {
        log.iter().rev().find_map(|e| {
            if jstr(e, "kind") == "MessageEvent" && jstr(e, "source") == "user" {
                text_of_content(e.pointer("/llm_message/content"))
            } else {
                None
            }
        })
    })
}

/// Tuma ujumbe + endesha agent (huongeza events na kutangaza kwa WS).
fn send_and_run(app: AppState, convo_id: String, prompt: String) {
    seed_system_prompt(&convo_id);
    append_message(&convo_id, "user", &prompt, "user");
    tokio::spawn(run_agent(app, convo_id, prompt));
}

/// Mzunguko: hali → LLM ya Rust (`chat::ask_stream`) → events + frames za WS.
async fn run_agent(app: AppState, convo_id: String, prompt: String) {
    set_status(&convo_id, "running");
    {
        let mut s = lock_store();
        let parent = last_parent(&s, &convo_id);
        let e = ev::state_update("execution_status", json!("running"), parent.as_deref());
        push_event(&mut s, &convo_id, e);
        persist(&s);
    }

    let item_id = state::new_id();
    state::publish_frame(&convo_id, ev::frame_item_started(&item_id, None));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::chat::ChatEvent>(64);
    let cid_stream = convo_id.clone();
    let item_stream = item_id.clone();
    // Mkusanyaji: hutangaza `delta`/`action` frames wakati LLM inatiririka.
    let collector = tokio::spawn(async move {
        let mut order: u64 = 0;
        let mut text = String::new();
        // action_id ya kila zana inayoendelea — observation huunganishwa nayo
        // ili Canvas ifunge kadi ya action+observation (si mbili tofauti).
        let mut pending_actions: HashMap<String, (String, Option<String>)> = HashMap::new();
        while let Some(e) = rx.recv().await {
            match e.kind.as_str() {
                "delta" => {
                    if !e.text.is_empty() {
                        state::publish_frame(
                            &cid_stream,
                            ev::frame_delta(&item_stream, order, &e.text),
                        );
                        order += 1;
                        text.push_str(&e.text);
                    }
                }
                // ActionEvent/ObservationEvent ni DURABLE (huhifadhiwa kwenye log
                // ya conversation) — Canvas huzireplay baada ya reload, hivyo kazi
                // halisi ya zana inabaki kuonekana kwenye history.
                "tool" => {
                    let mut s = lock_store();
                    let parent = last_parent(&s, &cid_stream);
                    let args = if e.args.is_null() {
                        json!({ "detail": e.detail })
                    } else {
                        e.args.clone()
                    };
                    let (evt, action_id, path) = ev::tool_action(&e.text, &args, parent.as_deref());
                    pending_actions.insert(e.text.clone(), (action_id, path));
                    push_event(&mut s, &cid_stream, evt);
                    persist(&s);
                }
                "tool_done" => {
                    let mut s = lock_store();
                    let parent = last_parent(&s, &cid_stream);
                    let (action_id, path) = pending_actions
                        .remove(&e.text)
                        .unwrap_or_else(|| (state::new_id(), None));
                    let evt = ev::tool_observation(
                        &e.text,
                        &action_id,
                        path.as_deref(),
                        &e.detail,
                        parent.as_deref(),
                    );
                    push_event(&mut s, &cid_stream, evt);
                    persist(&s);
                }
                "status" => {
                    state::publish_frame(
                        &cid_stream,
                        ev::frame_transient(ev::state_update("agent_status", json!(e.text), None)),
                    );
                }
                _ => {}
            }
        }
        text
    });

    // Native conversation hutumia ZANA ZA MTECHOS moja kwa moja (write_file →
    // /api/ide/file, read_file, execute_command …) — SI daraja la fundi-server.
    // Sababu: daraja hurudisha jibu pekee (bila tool events), hivyo Canvas
    // haikuoni kazi halisi ya zana. Kwa `None`, `ask_inner` huruka bridge na
    // kwenda `run_agent_loop` (zana halisi + ActionEvent/ObservationEvent).
    let reply = crate::chat::ask_stream(
        &app.brain,
        &prompt,
        true,
        crate::chat::Length::Medium,
        None,
        None,
        tx,
    )
    .await;
    let streamed = collector.await.unwrap_or_default();

    let final_text = if !reply.answer_sw.trim().is_empty() {
        reply.answer_sw.clone()
    } else {
        streamed
    };

    {
        let mut s = lock_store();
        if final_text.trim().is_empty() {
            // Hakuna jibu — funga slot + tangaza hitilafu HALISI.
            state::publish_frame(&convo_id, ev::frame_item_aborted(&item_id, "no_response"));
            let parent = last_parent(&s, &convo_id);
            let e = ev::error_event_classified(
                "AgentNoResponse",
                "Injini ya AI haikutoa jibu. Angalia LLM (FUNDI_AI_KEY / ollama) kisha ujaribu tena.",
                json!({ "kind": "llm", "retryable": true, "user_action": "settings" }),
                parent.as_deref(),
            );
            push_event(&mut s, &convo_id, e);
        } else {
            // ID ya event = item_id ili Canvas ifunge slot ya mtiririko.
            let parent = last_parent(&s, &convo_id);
            let e = ev::message_event_with_id(&item_id, "assistant", &final_text, "agent", parent.as_deref());
            push_event(&mut s, &convo_id, e);
        }
        let parent2 = last_parent(&s, &convo_id);
        let st = ev::state_update("execution_status", json!("idle"), parent2.as_deref());
        push_event(&mut s, &convo_id, st);
        if let Some(c) = s.convos.iter_mut().find(|c| jstr(c, "id") == convo_id) {
            if let Some(o) = c.as_object_mut() {
                o.insert("execution_status".into(), json!("idle"));
                o.insert("updated_at".into(), json!(state::now_rfc3339()));
            }
        }
        persist(&s);
    }
}

// ── Bash ─────────────────────────────────────────────────────────────────

static BASH_EVENTS: OnceLock<Mutex<Vec<Value>>> = OnceLock::new();

fn bash_log() -> MutexGuard<'static, Vec<Value>> {
    BASH_EVENTS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// POST /api/bash/execute_bash_command — tekeleza amri (halisi).
pub async fn execute_bash_command(Json(body): Json<Value>) -> Json<Value> {
    let cmd = jstr(&body, "command");
    let timeout = body
        .get("timeout")
        .and_then(|v| v.as_f64())
        .unwrap_or(30.0);
    let id = state::new_id();
    let started = state::now_rfc3339();
    let out = run_shell(&cmd, timeout).await;
    let ev = json!({
        "id": id,
        "kind": "BashOutput",
        "timestamp": started,
        "command": cmd,
        "exit_code": out.0,
        "stdout": out.1,
        "stderr": out.2,
        "timeout": timeout,
    });
    bash_log().push(ev.clone());
    ok(ev)
}

/// POST /api/bash/start_bash_command — amri ya muda mrefu (background).
pub async fn start_bash_command(Json(body): Json<Value>) -> Json<Value> {
    let cmd = jstr(&body, "command");
    let id = state::new_id();
    let child = tokio::process::Command::new("sh")
        .arg("-lc")
        .arg(&cmd)
        .current_dir(workspace_root())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let pid = match child {
        Ok(c) => c.id(),
        Err(_) => None,
    };
    bash_log().push(json!({
        "id": id, "kind": "BashCommand", "command": cmd,
        "timestamp": state::now_rfc3339(), "status": "running", "pid": pid,
    }));
    ok(json!({ "id": id, "command_id": id, "status": "running", "pid": pid }))
}

/// POST /api/bash/bash_commands/:id/stop.
pub async fn stop_bash_command(Path(_id): Path<String>) -> Json<Value> {
    ok(json!({ "success": true }))
}

/// GET /api/bash/bash_events/search (+ conversation-scoped).
pub async fn bash_events_search(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let limit: usize = q
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);
    let log = bash_log();
    let mut items: Vec<Value> = log.clone();
    items.reverse();
    items.truncate(limit);
    ok(json!({ "items": items, "next_page_id": null }))
}

/// GET /api/bash/bash_events/.
pub async fn list_bash_events() -> Json<Value> {
    let log = bash_log();
    ok(json!({ "items": log.clone() }))
}

/// DELETE /api/bash/bash_events.
pub async fn clear_bash_events() -> Json<Value> {
    bash_log().clear();
    ok(json!({ "success": true }))
}

/// GET /api/bash/bash_events/:id.
pub async fn get_bash_event(Path(id): Path<String>) -> Response {
    let log = bash_log();
    match log.iter().find(|e| jstr(e, "id") == id).cloned() {
        Some(e) => Json(e).into_response(),
        None => err(StatusCode::NOT_FOUND, "bash event haipo").into_response(),
    }
}

/// Tekeleza shell command kwa timeout (halisi). Hurudisha (exit, stdout, stderr).
async fn run_shell(cmd: &str, timeout_secs: f64) -> (i64, String, String) {
    let fut = tokio::process::Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(workspace_root())
        .output();
    match tokio::time::timeout(std::time::Duration::from_secs_f64(timeout_secs.max(1.0)), fut).await
    {
        Ok(Ok(o)) => (
            o.status.code().unwrap_or(-1) as i64,
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ),
        Ok(Err(e)) => (-1, String::new(), format!("imeshindwa kuendesha: {e}")),
        Err(_) => (-1, String::new(), "imepitwa na muda (timeout)".to_string()),
    }
}

// ── File ─────────────────────────────────────────────────────────────────

/// GET /api/file/home — nyumbani + favorites/locations.
pub async fn file_home() -> Json<Value> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    ok(json!({
        "home": home,
        "favorites": [
            { "label": "Downloads", "path": format!("{home}/Downloads") },
            { "label": "mbilinyi-ide", "path": workspace_root().display().to_string() },
        ],
        "locations": [ { "label": "/", "path": "/" } ],
    }))
}

/// POST /api/file/upload — hifadhi faili.
pub async fn file_upload(
    Query(q): Query<HashMap<String, String>>,
    body: Bytes,
) -> Json<Value> {
    let path = q.get("path").cloned().unwrap_or_default();
    if path.is_empty() {
        return ok(json!({ "ok": false, "error": "`path` ni lazima" }));
    }
    let full = std::path::PathBuf::from(&path);
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, &body) {
        Ok(_) => ok(json!({ "ok": true, "path": path, "bytes": body.len() })),
        Err(e) => ok(json!({ "ok": false, "error": format!("imeshindwa kuandika: {e}") })),
    }
}

/// GET /api/file/download — rudisha maudhui ya faili.
pub async fn file_download(Query(q): Query<HashMap<String, String>>) -> Response {
    let path = q.get("path").cloned().unwrap_or_default();
    match std::fs::read(&path) {
        Ok(bytes) => (
            [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
            bytes,
        )
            .into_response(),
        Err(e) => err(StatusCode::NOT_FOUND, &format!("faili haipatikani: {e}")).into_response(),
    }
}

/// GET /api/file/archive — orodha ya faili (tree) ya dir.
pub async fn file_archive(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let path = q
        .get("path")
        .cloned()
        .unwrap_or_else(|| workspace_root().display().to_string());
    ok(json!({ "path": path, "entries": list_dir_names(std::path::Path::new(&path)) }))
}

/// POST /api/file/create_directory.
pub async fn file_create_directory(Json(body): Json<Value>) -> Json<Value> {
    let path = jstr(&body, "path");
    match std::fs::create_dir_all(&path) {
        Ok(_) => ok(json!({ "ok": true, "path": path })),
        Err(e) => ok(json!({ "ok": false, "error": format!("imeshindwa: {e}") })),
    }
}

/// GET /api/file/search_subdirs.
pub async fn file_search_subdirs(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let root = q
        .get("path")
        .cloned()
        .unwrap_or_else(|| workspace_root().display().to_string());
    let mut out = Vec::new();
    collect_subdirs(std::path::Path::new(&root), 0, &mut out);
    ok(json!({ "items": out }))
}

fn collect_subdirs(dir: &std::path::Path, depth: usize, out: &mut Vec<Value>) {
    if depth > 3 || out.len() > 500 {
        return;
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.push(json!({ "path": p.display().to_string(), "name": e.file_name().to_string_lossy() }));
                collect_subdirs(&p, depth + 1, out);
            }
        }
    }
}

/// GET /api/file/download-trajectory/:id — trajectory ya conversation.
pub async fn download_trajectory(Path(conversation_id): Path<String>) -> Response {
    let s = lock_store();
    let log = s.events.get(&conversation_id).cloned().unwrap_or_default();
    let doc = json!({ "conversation_id": conversation_id, "events": log });
    match serde_json::to_vec_pretty(&doc) {
        Ok(bytes) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            bytes,
        )
            .into_response(),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "imeshindwa kutengeneza trajectory")
            .into_response(),
    }
}

// ── Git ──────────────────────────────────────────────────────────────────

async fn git(args: &[&str], dir: &std::path::Path) -> (i64, String) {
    let out = tokio::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .await;
    match out {
        Ok(o) => (
            o.status.code().unwrap_or(-1) as i64,
            String::from_utf8_lossy(&o.stdout).to_string(),
        ),
        Err(e) => (-1, format!("git haipatikani: {e}")),
    }
}

fn git_dir(q: &HashMap<String, String>) -> std::path::PathBuf {
    q.get("path")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(workspace_root)
}

/// GET /api/git/changes.
pub async fn git_changes(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let dir = git_dir(&q);
    let (_, porcelain) = git(&["status", "--porcelain"], &dir).await;
    let changes: Vec<Value> = porcelain
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let status = l.get(0..2).unwrap_or("??").trim().to_string();
            let path = l.get(3..).unwrap_or("").trim().to_string();
            json!({ "status": status, "path": path })
        })
        .collect();
    ok(json!({ "changes": changes }))
}

/// GET /api/git/commits.
pub async fn git_commits(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let dir = git_dir(&q);
    let limit = q.get("limit").cloned().unwrap_or_else(|| "20".to_string());
    let fmt = "%H%x1f%an%x1f%ae%x1f%ad%x1f%s";
    let (_, log) = git(&["log", &format!("-{limit}"), &format!("--pretty=format:{fmt}"), "--date=iso"], &dir).await;
    let commits: Vec<Value> = log
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\u{1f}').collect();
            json!({
                "sha": f.first().copied().unwrap_or(""),
                "author": f.get(1).copied().unwrap_or(""),
                "email": f.get(2).copied().unwrap_or(""),
                "date": f.get(3).copied().unwrap_or(""),
                "message": f.get(4).copied().unwrap_or(""),
            })
        })
        .collect();
    ok(json!({ "commits": commits }))
}

/// GET /api/git/commits/:sha/changes.
pub async fn git_commit_changes(
    Path(sha): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let dir = git_dir(&q);
    let (_, out) = git(&["show", "--stat", "--oneline", &sha], &dir).await;
    ok(json!({ "sha": sha, "diff": out }))
}

/// GET /api/git/diff.
pub async fn git_diff(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let dir = git_dir(&q);
    let (_, out) = git(&["diff"], &dir).await;
    ok(json!({ "diff": out }))
}

/// GET /api/git/repositories/search.
pub async fn git_repositories_search(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let root = q
        .get("path")
        .cloned()
        .unwrap_or_else(|| workspace_root().display().to_string());
    let mut repos = Vec::new();
    find_git_repos(std::path::Path::new(&root), 0, &mut repos);
    ok(json!({ "items": repos, "repositories": repos }))
}

fn find_git_repos(dir: &std::path::Path, depth: usize, out: &mut Vec<Value>) {
    if depth > 3 || out.len() > 200 {
        return;
    }
    if dir.join(".git").exists() {
        out.push(json!({ "path": dir.display().to_string(), "name": dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() }));
        return;
    }
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && !p.join(".git").exists() {
                find_git_repos(&p, depth + 1, out);
            }
        }
    }
}

/// GET /api/v1/git/branches/search.
pub async fn git_branches_search(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let dir = git_dir(&q);
    let (_, out) = git(&["branch", "--format=%(refname:short)"], &dir).await;
    let items: Vec<Value> = out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| json!({ "name": l.trim() }))
        .collect();
    ok(json!({ "items": items, "branches": items }))
}

/// GET /api/v1/git/installations/search.
pub async fn git_installations_search() -> Json<Value> {
    ok(json!({ "items": [], "installations": [] }))
}

// ── VSCode ───────────────────────────────────────────────────────────────

pub async fn vscode_status() -> Json<Value> {
    ok(json!({ "running": false, "enabled": true }))
}

pub async fn vscode_url() -> Json<Value> {
    ok(json!({ "url": null, "enabled": true }))
}

// ── Workspaces ───────────────────────────────────────────────────────────

pub async fn list_workspaces() -> Json<Value> {
    let s = lock_store();
    ok(json!({ "workspaces": s.workspaces, "workspaceParents": s.workspace_parents }))
}

pub async fn add_workspace(Json(body): Json<Value>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.workspaces.push(body.clone());
        persist(&s);
    }
    ok(json!({ "success": true, "workspace": body }))
}

pub async fn delete_workspace(Json(body): Json<Value>) -> Json<Value> {
    let path = jstr(&body, "path");
    {
        let mut s = lock_store();
        s.workspaces.retain(|w| jstr(w, "path") != path);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn list_workspace_parents() -> Json<Value> {
    let s = lock_store();
    ok(json!({ "workspaceParents": s.workspace_parents }))
}

pub async fn add_workspace_parent(Json(body): Json<Value>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.workspace_parents.push(body.clone());
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn delete_workspace_parent(Json(body): Json<Value>) -> Json<Value> {
    let path = jstr(&body, "path");
    {
        let mut s = lock_store();
        s.workspace_parents.retain(|w| jstr(w, "path") != path);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

// ── Skills ───────────────────────────────────────────────────────────────

pub async fn list_skills() -> Json<Value> {
    let s = lock_store();
    ok(json!({ "skills": s.skills }))
}

pub async fn get_skill(Path(name): Path<String>) -> Response {
    let s = lock_store();
    match s.skills.iter().find(|k| jstr(k, "name") == name).cloned() {
        Some(k) => Json(k).into_response(),
        None => err(StatusCode::NOT_FOUND, "skill haipo").into_response(),
    }
}

pub async fn install_skill(Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "name");
    let mut skill = body.clone();
    if let Some(o) = skill.as_object_mut() {
        o.insert("name".into(), json!(name));
        o.entry("enabled".to_string()).or_insert_with(|| json!(true));
    }
    {
        let mut s = lock_store();
        if !s.skills.iter().any(|k| jstr(k, "name") == name) {
            s.skills.push(skill.clone());
        }
        persist(&s);
    }
    ok(json!({ "success": true, "skill": skill }))
}

pub async fn patch_skill(Path(name): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    if let Some(k) = s.skills.iter_mut().find(|k| jstr(k, "name") == name) {
        let mut merged = k.clone();
        if let (Some(mo), Some(bo)) = (merged.as_object_mut(), body.as_object()) {
            for (key, val) in bo {
                mo.insert(key.clone(), val.clone());
            }
        }
        *k = merged;
    }
    persist(&s);
    ok(json!({ "success": true }))
}

pub async fn delete_skill(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.skills.retain(|k| jstr(k, "name") != name);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn refresh_skill(Path(name): Path<String>) -> Json<Value> {
    ok(json!({ "success": true, "name": name }))
}

pub async fn skills_marketplace() -> Json<Value> {
    ok(json!({ "skills": [], "items": [] }))
}

pub async fn skills_sync() -> Json<Value> {
    ok(json!({ "success": true, "synced": 0 }))
}

/// GET /api/v1/skills/search.
pub async fn v1_skills_search(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let s = lock_store();
    let needle = q.get("q").cloned().unwrap_or_default().to_lowercase();
    let items: Vec<Value> = s
        .skills
        .iter()
        .filter(|k| needle.is_empty() || jstr(k, "name").to_lowercase().contains(&needle))
        .cloned()
        .collect();
    ok(json!({ "items": items }))
}

// ── Plugins ──────────────────────────────────────────────────────────────

pub async fn list_plugins() -> Json<Value> {
    // Plugins za MTECHOS (canvas-extensions) — chanzo halisi cha Rust.
    let roots = crate::canvas_ext::plugin_roots();
    let mut plugins: Vec<Value> = Vec::new();
    for dir in roots.iter() {
        if let Ok(manifest) = std::fs::read_to_string(dir.join("canvas-extension.json")) {
            if let Ok(m) = serde_json::from_str::<Value>(&manifest) {
                plugins.push(json!({
                    "name": m.get("name").cloned().unwrap_or(Value::Null),
                    "version": m.get("version").cloned().unwrap_or(json!("1.0.0")),
                    "description": m.get("description").cloned().unwrap_or(Value::Null),
                    "pages": m.pointer("/contributes/pages").cloned().unwrap_or(json!([])),
                    "path": dir.display().to_string(),
                    "enabled": true,
                }));
            }
        }
    }
    // Ongeza yale yaliyosakinishwa kupitia protocol (hifadhi).
    let s = lock_store();
    for p in s.plugins.iter() {
        if !plugins.iter().any(|x| jstr(x, "name") == jstr(p, "name")) {
            plugins.push(p.clone());
        }
    }
    ok(json!({ "plugins": plugins, "count": plugins.len() }))
}

pub async fn get_plugin(Path(name): Path<String>) -> Response {
    let s = lock_store();
    match s.plugins.iter().find(|p| jstr(p, "name") == name).cloned() {
        Some(p) => Json(p).into_response(),
        None => err(StatusCode::NOT_FOUND, "plugin haipo").into_response(),
    }
}

pub async fn install_plugin(Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "name");
    let mut plugin = body.clone();
    if let Some(o) = plugin.as_object_mut() {
        o.insert("name".into(), json!(name));
        o.entry("enabled".to_string()).or_insert_with(|| json!(true));
    }
    {
        let mut s = lock_store();
        if !s.plugins.iter().any(|p| jstr(p, "name") == name) {
            s.plugins.push(plugin.clone());
        }
        persist(&s);
    }
    ok(json!({ "success": true, "plugin": plugin }))
}

pub async fn patch_plugin(Path(name): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    let mut s = lock_store();
    if let Some(p) = s.plugins.iter_mut().find(|p| jstr(p, "name") == name) {
        let mut merged = p.clone();
        if let (Some(mo), Some(bo)) = (merged.as_object_mut(), body.as_object()) {
            for (key, val) in bo {
                mo.insert(key.clone(), val.clone());
            }
        }
        *p = merged;
    }
    persist(&s);
    ok(json!({ "success": true }))
}

pub async fn delete_plugin(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.plugins.retain(|p| jstr(p, "name") != name);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn refresh_plugin(Path(name): Path<String>) -> Json<Value> {
    ok(json!({ "success": true, "name": name }))
}

pub async fn plugins_marketplace() -> Json<Value> {
    const CATALOG: &str = include_str!("../agent_schemas/plugins-marketplace.json");
    match serde_json::from_str::<Value>(CATALOG) {
        Ok(v) => ok(v),
        Err(_) => ok(json!({ "plugins": [], "items": [] })),
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

/// GET /api/tools/ — orodha ya tools (majina).
pub async fn tools_list() -> Json<Value> {
    ok(json!(crate::agent_catalogs::USABLE_TOOLS))
}

/// GET /api/tools/catalog — catalog kamili ya tools.
pub async fn tools_catalog() -> Json<Value> {
    const CATALOG: &str = include_str!("../agent_schemas/tools-catalog.json");
    match serde_json::from_str::<Value>(CATALOG) {
        Ok(v) => ok(v),
        Err(_) => ok(json!({ "tools": [] })),
    }
}

// ── Hooks / sub-agents / acp / shared / auth ─────────────────────────────

pub async fn create_hook(Json(body): Json<Value>) -> Json<Value> {
    ok(json!({ "success": true, "hook": body }))
}

pub async fn create_sub_agent(Json(body): Json<Value>) -> Json<Value> {
    ok(json!({ "success": true, "sub_agent": body }))
}

pub async fn acp_models(Json(body): Json<Value>) -> Json<Value> {
    let _ = body;
    ok(json!({ "models": [] }))
}

pub async fn shared_conversations() -> Json<Value> {
    ok(json!({ "items": [], "conversations": [] }))
}

pub async fn shared_events_search() -> Json<Value> {
    ok(json!({ "items": [] }))
}

/// POST /api/auth/workspace-session — hubadilisha `X-Session-API-Key` kwa
/// cookie ya `oh_workspace_session_key` ili browser iweze kupakia faili za
/// workspace kama `<iframe src>` / `<img src>` (haziwezi kubeba header).
///
/// Client (typescript-client `startWorkspaceSession`) HUTUMA body TUPU, na
/// mkataba unatarajia **204 No Content**. Handler ya awali ililazimisha
/// `Json<Value>`, hivyo body tupu ilirudisha 400 → Files tab ilionyesha
/// "An error occurred". Tunakubali body yoyote (au hakuna) na kurudisha 204.
pub async fn workspace_session(headers: HeaderMap) -> Response {
    let key = headers
        .get("x-session-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let cookie = format!(
        "oh_workspace_session_key={key}; Path=/api/conversations; HttpOnly; SameSite=Lax"
    );
    (
        StatusCode::NO_CONTENT,
        [(
            axum::http::header::SET_COOKIE,
            axum::http::HeaderValue::from_str(&cookie)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("oh_workspace_session_key=")),
        )],
    )
        .into_response()
}

pub async fn workspace_session_delete() -> Response {
    let cookie =
        "oh_workspace_session_key=; Path=/api/conversations; HttpOnly; SameSite=Lax; Max-Age=0";
    (
        StatusCode::NO_CONTENT,
        [(
            axum::http::header::SET_COOKIE,
            axum::http::HeaderValue::from_static(cookie),
        )],
    )
        .into_response()
}

// ── MCP ──────────────────────────────────────────────────────────────────

pub async fn mcp_test(Json(body): Json<Value>) -> Json<Value> {
    let _ = body;
    ok(json!({ "ok": true, "tools": [] }))
}

pub async fn mcp_oauth_start(Json(body): Json<Value>) -> Json<Value> {
    let _ = body;
    ok(json!({ "job_id": state::new_id(), "status": "pending", "authorization_url": null }))
}

pub async fn mcp_oauth_status(Path(job_id): Path<String>) -> Json<Value> {
    ok(json!({ "job_id": job_id, "status": "pending" }))
}

pub async fn mcp_oauth_callback(Path(job_id): Path<String>) -> Json<Value> {
    ok(json!({ "job_id": job_id, "status": "completed" }))
}

pub async fn mcp_oauth_any(Path(path): Path<String>) -> Json<Value> {
    ok(json!({ "path": path, "status": "pending" }))
}

// ── v1 App conversations ─────────────────────────────────────────────────

/// GET /api/v1/app-conversations.
pub async fn app_list_conversations(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let limit: usize = q
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50);
    let s = lock_store();
    let mut items: Vec<Value> = s.convos.clone();
    items.sort_by(|a, b| jstr(b, "updated_at").cmp(&jstr(a, "updated_at")));
    items.truncate(limit);
    ok(json!({ "items": items, "next_page_id": null }))
}

/// GET /api/v1/app-conversations/search.
pub async fn app_search_conversations(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    app_list_conversations(Query(q)).await
}

/// POST /api/v1/app-conversations — anzisha (App API).
pub async fn app_start_conversation(
    State(app): State<AppState>,
    Json(body): Json<Value>,
) -> Response {
    let mut convo = build_conversation(&body);
    if let Some(o) = convo.as_object_mut() {
        o.insert("app_conversation".into(), json!(true));
    }
    let id = jstr(&convo, "id");
    {
        let mut s = lock_store();
        s.convos.push(convo.clone());
        persist(&s);
    }
    if let Some(prompt) = initial_prompt(&body) {
        send_and_run(app, id, prompt);
    }
    (StatusCode::CREATED, Json(convo)).into_response()
}

/// POST /api/v1/app-conversations/start-tasks.
pub async fn app_start_tasks(State(app): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let tasks = body.get("tasks").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut ids = Vec::new();
    for t in tasks {
        let convo = build_conversation(&t);
        let id = jstr(&convo, "id");
        {
            let mut s = lock_store();
            s.convos.push(convo.clone());
            persist(&s);
        }
        if let Some(prompt) = initial_prompt(&t) {
            send_and_run(app.clone(), id.clone(), prompt);
        }
        ids.push(json!(id));
    }
    ok(json!({ "success": true, "conversation_ids": ids, "items": ids }))
}

/// GET /api/v1/app-conversations/:id.
pub async fn app_get_conversation(Path(id): Path<String>) -> Response {
    get_conversation(Path(id)).await
}

/// GET /api/v1/app-conversations/:id/file.
pub async fn app_get_file(Path(id): Path<String>, Query(q): Query<HashMap<String, String>>) -> Response {
    let path = q.get("path").cloned().unwrap_or_default();
    get_workspace_file(Path((id, path))).await
}

/// GET /api/v1/app-conversations/:id/files.
pub async fn app_list_files(Path(id): Path<String>) -> Json<Value> {
    list_workspace_files(Path(id)).await
}

/// GET /api/v1/app-conversations/:id/download.
pub async fn app_download(Path(id): Path<String>) -> Response {
    download_trajectory(Path(id)).await
}

/// GET /api/v1/app-conversations/:id/skills.
pub async fn app_skills(Path(id): Path<String>) -> Json<Value> {
    let s = lock_store();
    let skills = find_convo(&s, &id)
        .map(|c| c.get("invoked_skills").cloned().unwrap_or(json!([])))
        .unwrap_or(json!([]));
    ok(json!({ "skills": skills }))
}

/// GET /api/v1/conversations/:id/pending-messages.
pub async fn pending_messages(Path(id): Path<String>) -> Json<Value> {
    let _ = id;
    ok(json!({ "items": [], "messages": [] }))
}

/// GET /api/v1/sandboxes.
pub async fn list_sandboxes() -> Json<Value> {
    ok(json!({ "items": [], "sandboxes": [] }))
}

pub async fn sandbox_pause(Path(id): Path<String>) -> Json<Value> {
    ok(json!({ "success": true, "id": id, "status": "paused" }))
}

pub async fn sandbox_resume(Path(id): Path<String>) -> Json<Value> {
    ok(json!({ "success": true, "id": id, "status": "running" }))
}

/// GET /api/v1/web-client/config.
pub async fn web_client_config() -> Json<Value> {
    ok(json!({
        "vscode_enabled": true,
        "features": { "conversations": true, "settings": true },
    }))
}

// ── Cloud proxy / organizations / misc ───────────────────────────────────

pub async fn cloud_proxy() -> Json<Value> {
    ok(json!({ "ok": false, "error": "cloud proxy haipatikani kwenye runtime ya ndani" }))
}

pub async fn list_organizations() -> Json<Value> {
    ok(json!({ "items": [], "organizations": [] }))
}

pub async fn org_me(Path(id): Path<String>) -> Json<Value> {
    ok(json!({ "id": id, "name": "local", "role": "owner" }))
}

pub async fn setup_state() -> Json<Value> {
    ok(json!({ "setup_complete": true, "runtime": state::runtime_name() }))
}

pub async fn analytics_events(Json(body): Json<Value>) -> Json<Value> {
    let _ = body;
    ok(json!({ "success": true }))
}

// ── Sockets (WS) ─────────────────────────────────────────────────────────

/// Kikomo cha auth: key iliyotarajiwa kutoka `canvas_ext`.
fn expected_key() -> Option<String> {
    crate::canvas_ext::agent_key().map(|s| s.to_string())
}

fn header_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-session-api-key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer ").map(str::to_string))
        })
}

/// GET /sockets/session/:id — socket kuu ya conversation (envelope protocol).
pub async fn socket_session(
    State(app): State<AppState>,
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let header_ok = match (expected_key(), header_key(&headers)) {
        (Some(exp), Some(got)) => exp == got,
        (None, _) => true,
        (Some(exp), None) => exp.trim().is_empty(),
    };
    ws.on_upgrade(move |socket| handle_session(socket, id, header_ok, app))
}

/// GET /sockets/events/:id — socket ya zamani (events pekee).
pub async fn socket_events(
    State(app): State<AppState>,
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    socket_session(State(app), ws, Path(id), headers).await
}

/// GET /sockets/bash-events — mtiririko wa bash events.
pub async fn socket_bash_events(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_bash_socket)
}

async fn handle_session(socket: WebSocket, convo_id: String, header_ok: bool, app: AppState) {
    let (mut sink, mut stream) = socket.split();
    let mut authed = header_ok;
    let mut rx = state::subscribe_events(&convo_id);

    // `sync` + replay ya events zilizopo (durable frames).
    let (from, through, replay) = {
        let s = lock_store();
        let log = s.events.get(&convo_id).cloned().unwrap_or_default();
        let n = log.len();
        (0u64, n as u64, log)
    };
    if send_json(&mut sink, &ev::frame_sync(Some(from), Some(through)))
        .await
        .is_err()
    {
        return;
    }
    for (i, e) in replay.into_iter().enumerate() {
        if send_json(&mut sink, &ev::frame_durable(i as u64, e)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            incoming = stream.next() => {
                let Some(Ok(msg)) = incoming else { break };
                match msg {
                    Message::Text(txt) => {
                        let parsed: Value = serde_json::from_str(&txt).unwrap_or(Value::Null);
                        if !authed {
                            let ok_auth = parsed.get("type").and_then(|v| v.as_str()) == Some("auth")
                                && parsed.get("session_api_key").and_then(|v| v.as_str()).map(|k| {
                                    match expected_key() {
                                        Some(exp) => exp.trim().is_empty() || k == exp,
                                        None => true,
                                    }
                                }).unwrap_or(false);
                            if ok_auth {
                                authed = true;
                            } else {
                                let _ = send_json(&mut sink, &ev::frame_error("unauthorized", "Session API key si sahihi")).await;
                                break;
                            }
                            continue;
                        }
                        handle_socket_message(&convo_id, &parsed, app.clone());
                    }
                    Message::Binary(_) => {}
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            frame = rx.recv() => {
                match frame {
                    Ok(f) => { if send_json(&mut sink, &f).await.is_err() { break; } }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

/// Shughulikia ujumbe wa WS: tuma message + endesha (kama `run: true`).
fn handle_socket_message(convo_id: &str, parsed: &Value, app: AppState) {
    if parsed.is_null() {
        return;
    }
    let role = state::role_of(parsed);
    if let Some(text) = text_of_content(parsed.get("content")) {
        let run = parsed.get("run").and_then(|v| v.as_bool()).unwrap_or(false);
        append_message(convo_id, &role, &text, if role == "user" { "user" } else { "agent" });
        if run && role == "user" {
            tokio::spawn(run_agent(app, convo_id.to_string(), text));
        }
    }
}

async fn handle_bash_socket(socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    loop {
        tokio::select! {
            incoming = stream.next() => {
                let Some(Ok(msg)) = incoming else { break };
                if let axum::extract::ws::Message::Text(txt) = msg {
                    let parsed: Value = serde_json::from_str(&txt).unwrap_or(Value::Null);
                    if let Some(cmd) = parsed.get("command").and_then(|v| v.as_str()) {
                        let out = run_shell(cmd, 30.0).await;
                        let frame = json!({
                            "id": state::new_id(), "kind": "BashOutput",
                            "command": cmd, "exit_code": out.0,
                            "stdout": out.1, "stderr": out.2,
                        });
                        if send_json(&mut sink, &frame).await.is_err() { break; }
                    }
                }
            }
        }
    }
}

async fn send_json(
    sink: &mut futures::stream::SplitSink<WebSocket, Message>,
    v: &Value,
) -> Result<(), axum::Error> {
    sink.send(Message::Text(v.to_string())).await
}
