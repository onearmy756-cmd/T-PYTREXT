//! state.rs — hali ya protocol ya agent-server kwa Rust-native.
//!
//! Hifadhi moja (OnceLock + Mutex) inayoshirikiwa na handlers zote za
//! `/api/*` + `/sockets/*`. Haitegemei Python. Huhifadhi kwenye diski
//! (`<FUNDI_DATA>/agent-native/state.json`) ili conversations zidumu.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tokio::sync::broadcast;

/// Jina la runtime — `fundi-rust` (badala ya `python_version`).
pub fn runtime_name() -> &'static str {
    "fundi-rust"
}

/// Hifadhi kuu. Values ni JSON (schema ya agent-server) ili kuwa rahisi
/// kuongeza nyuga bila kuandika structs nyingi.
#[derive(Default, Clone)]
pub struct NativeStore {
    pub convos: Vec<Value>,
    pub events: HashMap<String, Vec<Value>>,
    pub counter: u64,
    pub settings: Value,
    pub secrets: HashMap<String, String>,
    pub profiles: Vec<Value>,
    pub agent_profiles: Vec<Value>,
    pub meta_profiles: Vec<Value>,
    pub workspaces: Vec<Value>,
    pub workspace_parents: Vec<Value>,
    pub skills: Vec<Value>,
    pub plugins: Vec<Value>,
    pub provider_connections: Vec<Value>,
    pub active_profile: Option<String>,
    pub active_agent_profile_id: Option<String>,
    pub active_meta_profile: Option<String>,
}

static STORE: OnceLock<Mutex<NativeStore>> = OnceLock::new();

fn store_cell() -> &'static Mutex<NativeStore> {
    STORE.get_or_init(|| Mutex::new(load_or_default()))
}

/// Ufunguzi salama — HAUTUMII `.unwrap()` (hauwezi panic hata lock ikiwa
/// imeharibika). Kanuni ya BUG-001 ya `code_audit`.
pub fn lock_store() -> MutexGuard<'static, NativeStore> {
    store_cell().lock().unwrap_or_else(|e| e.into_inner())
}

pub fn jstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

/// Sarufi ya role; hukubali maadha ya agent-server (`user` default).
pub fn role_of(v: &Value) -> String {
    let r = jstr(v, "role");
    if r.is_empty() {
        "user".to_string()
    } else {
        r
    }
}

/// Dir ya hifadhi: env `FUNDI_AGENT_NATIVE_DIR` → `<FUNDI_DATA>/agent-native`
/// → `/data/agent-native` → `./agent-native`.
pub fn data_dir() -> std::path::PathBuf {
    if let Ok(v) = std::env::var("FUNDI_AGENT_NATIVE_DIR") {
        if !v.trim().is_empty() {
            return std::path::PathBuf::from(v);
        }
    }
    let base = std::env::var("FUNDI_DATA").unwrap_or_else(|_| {
        if std::path::Path::new("/data").is_dir() {
            "/data".to_string()
        } else {
            ".".to_string()
        }
    });
    std::path::PathBuf::from(format!("{base}/agent-native"))
}

fn state_file() -> std::path::PathBuf {
    data_dir().join("state.json")
}

fn load_or_default() -> NativeStore {
    let mut s = match std::fs::read_to_string(state_file()) {
        Ok(txt) => serde_json::from_str::<Value>(&txt)
            .map(store_from_json)
            .unwrap_or_default(),
        Err(_) => NativeStore::default(),
    };
    if s.settings.is_null() {
        s.settings = default_settings();
    }
    if s.profiles.is_empty() {
        s.profiles = vec![json!({ "name": "default", "llm": default_llm() })];
    }
    if s.agent_profiles.is_empty() {
        s.agent_profiles = vec![json!({
            "id": "default",
            "name": "default",
            "agent_kind": "openhands",
            "revision": 0,
            "llm_profile_ref": "default",
            "mcp_server_refs": null,
        })];
        if s.active_agent_profile_id.is_none() {
            s.active_agent_profile_id = Some("default".to_string());
        }
    }
    s
}

fn store_from_json(v: Value) -> NativeStore {
    let mut s = NativeStore::default();
    s.convos = v.get("convos").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.counter = v.get("counter").and_then(|x| x.as_u64()).unwrap_or(0);
    s.settings = v.get("settings").cloned().unwrap_or(Value::Null);
    s.profiles = v.get("profiles").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.agent_profiles = v.get("agent_profiles").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.meta_profiles = v.get("meta_profiles").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.workspaces = v.get("workspaces").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.workspace_parents = v.get("workspace_parents").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.skills = v.get("skills").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.plugins = v.get("plugins").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    s.active_profile = v.get("active_profile").and_then(|x| x.as_str()).map(String::from);
    s.active_agent_profile_id = v.get("active_agent_profile_id").and_then(|x| x.as_str()).map(String::from);
    s.active_meta_profile = v.get("active_meta_profile").and_then(|x| x.as_str()).map(String::from);
    if let Some(map) = v.get("events").and_then(|x| x.as_object()) {
        for (k, arr) in map.iter() {
            if let Some(items) = arr.as_array() {
                s.events.insert(k.clone(), items.clone());
            }
        }
    }
    s
}

/// Hifadhi hali kwenye diski (best-effort — hitilafu hazizuii ombi).
pub fn persist(s: &NativeStore) {
    let dir = data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let events: serde_json::Map<String, Value> = s
        .events
        .iter()
        .map(|(k, v)| (k.clone(), Value::Array(v.clone())))
        .collect();
    let doc = json!({
        "convos": s.convos,
        "counter": s.counter,
        "settings": s.settings,
        "profiles": s.profiles,
        "agent_profiles": s.agent_profiles,
        "meta_profiles": s.meta_profiles,
        "workspaces": s.workspaces,
        "workspace_parents": s.workspace_parents,
        "skills": s.skills,
        "plugins": s.plugins,
        "active_profile": s.active_profile,
        "active_agent_profile_id": s.active_agent_profile_id,
        "active_meta_profile": s.active_meta_profile,
        "events": Value::Object(events),
    });
    if let Ok(txt) = serde_json::to_string(&doc) {
        let _ = std::fs::write(state_file(), txt);
    }
}

/// LLM ya msingi (inafuata umbo la agent-server `agent_settings.llm`).
///
/// Hupata model/URL kutoka `ai_config` (LLM ya LAN/cloud ya MTECHOS) ili
/// protocol itumie injini HALISI ya Rust, sio ya kubuni.
pub fn default_llm() -> Value {
    let url = crate::ai_config::llm_url();
    let model = crate::ai_config::default_config().model;
    json!({
        "model": model,
        "api_key": null,
        "provider_connection_id": null,
        "auth_type": "api_key",
        "base_url": url,
        "ollama_base_url": url,
        "api_version": null,
        "num_retries": 5,
        "retry_multiplier": 8.0,
        "retry_min_wait": 8,
        "retry_max_wait": 64,
        "timeout": 300,
        "stream": false,
        "drop_params": true,
        "caching_prompt": true,
        "log_completions": false,
        "native_tool_calling": false,
        "reasoning_effort": "high",
        "seed": null,
        "usage_id": "default",
        "litellm_extra_body": {},
        "is_subscription": false,
    })
}

/// LLM ya profile INAYOTUMIKA sasa (`active_profile`) — model/URL/API key
/// halisi zilizochaguliwa kwenye UI. Bila hii, uchaguzi wa mtumiaji
/// (mf. `deepseekcoder6b`) haufiki injini: mazungumzo yote hujibiwa na
/// `ai_settings`/env pekee, hivyo model aliyochagua haitumiki kamwe.
///
/// Hurudisha `(model, base_url, api_key)`; model tupu = tumia default.
pub fn active_profile_llm() -> (String, String, Option<String>) {
    let s = lock_store();
    let Some(name) = s.active_profile.clone() else {
        return (String::new(), String::new(), None);
    };
    if name == "default" {
        return (String::new(), String::new(), None);
    }
    let Some(p) = s.profiles.iter().find(|p| jstr(p, "name") == name) else {
        return (String::new(), String::new(), None);
    };
    let llm = p.get("llm").cloned().unwrap_or(Value::Null);
    let raw = llm
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    // Ollama Cloud hutumia `ollama/<model>` kwenye profiles; endpoint inataka
    // jina pekee la model.
    let model = raw.strip_prefix("ollama/").unwrap_or(raw).to_string();
    let base = llm
        .get("base_url")
        .or_else(|| llm.get("ollama_base_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let key = llm
        .get("api_key")
        .and_then(|v| v.as_str())
        .filter(|k| !k.trim().is_empty())
        .map(String::from);
    (model, base, key)
}

pub fn default_settings() -> Value {
    json!({
        "agent_settings": {
            "schema_version": 8,
            "agent_kind": "openhands",
            "agent": "CodeActAgent",
            "llm": default_llm(),
            "tools": null,
            "enable_classify_and_switch_llm_tool": false,
            "active_meta_profile": null,
            "meta_profile": null,
            "meta_profile_llms": {},
            "tool_concurrency_limit": 1,
            "mcp_config": {},
            "persona": null,
            "agent_context": {
                "skills": [],
                "system_message_suffix": null,
                "user_message_suffix": null,
                "load_user_skills": false,
                "load_public_skills": false,
                "marketplace_path": "marketplaces/default.json",
                "registered_marketplaces": [],
                "load_project_skills": false,
            },
            "condenser": {},
            "verification": {},
        },
        "conversation_settings": { "max_iterations": 500, "stuck_detection": true },
        "llm_api_key_is_set": crate::ai_config::ai_key().is_some(),
        "active_profile": "default",
        "active_agent_profile_id": "default",
        "active_meta_profile": null,
        "misc_settings": {},
    })
}

// ── Bus ya events (WS) ───────────────────────────────────────────────────

struct EventBus {
    senders: HashMap<String, broadcast::Sender<Value>>,
}

static EVENT_BUS: OnceLock<Mutex<EventBus>> = OnceLock::new();

fn event_bus() -> &'static Mutex<EventBus> {
    EVENT_BUS.get_or_init(|| Mutex::new(EventBus { senders: HashMap::new() }))
}

fn bus_lock() -> MutexGuard<'static, EventBus> {
    event_bus().lock().unwrap_or_else(|e| e.into_inner())
}

/// Jiandikishe kwa events za conversation moja (WS).
pub fn subscribe_events(convo_id: &str) -> broadcast::Receiver<Value> {
    let mut bus = bus_lock();
    let tx = bus
        .senders
        .entry(convo_id.to_string())
        .or_insert_with(|| broadcast::channel(256).0);
    tx.subscribe()
}

/// Tangaza frame kwa wote wanaosikiliza conversation (WS).
pub fn publish_frame(convo_id: &str, frame: Value) {
    let mut bus = bus_lock();
    let tx = bus
        .senders
        .entry(convo_id.to_string())
        .or_insert_with(|| broadcast::channel(256).0);
    let _ = tx.send(frame);
}

/// Mask kwa secret: herufi 4 za mwanzo + nyota. Kamwe hakurudishi value kamili.
pub fn mask_secret(v: &str) -> String {
    if v.is_empty() {
        return String::new();
    }
    let head: String = v.chars().take(4).collect();
    format!("{head}{}", "*".repeat(v.chars().count().saturating_sub(4).max(4)))
}

/// ID mpya (UUID v4) — hufuata umbo la agent-server.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Muda wa sasa (RFC3339) kwa events.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}
