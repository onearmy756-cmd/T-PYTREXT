//! settings.rs — settings, profiles, agent-profiles, meta-profiles, secrets,
//! na LLM (models/providers/provider-connections).
//!
//! Maumbo yanafuata schema HALISI ya agent-server (yamenaswa kutoka protocol
//! ya Python kabla ya kuiondoa) ili Canvas SPA isibadilike kwa upande wake.

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::state::{self, jstr, lock_store, mask_secret, persist};

fn ok(v: Value) -> Json<Value> {
    Json(v)
}

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "detail": msg })))
}

// ── Settings ─────────────────────────────────────────────────────────────

/// GET /api/settings — agent_settings + conversation_settings + flags.
pub async fn get_settings() -> Json<Value> {
    let s = lock_store();
    let mut v = s.settings.clone();
    if !v.is_object() {
        v = state::default_settings();
    }
    // `llm_api_key_is_set` HUTOKA env/DB halisi (sio iliyohifadhiwa).
    if let Some(o) = v.as_object_mut() {
        o.insert(
            "llm_api_key_is_set".into(),
            json!(crate::ai_config::ai_key().is_some()),
        );
        o.insert(
            "active_profile".into(),
            json!(s.active_profile.clone().unwrap_or_else(|| "default".into())),
        );
        o.insert(
            "active_agent_profile_id".into(),
            json!(s
                .active_agent_profile_id
                .clone()
                .unwrap_or_else(|| "default".into())),
        );
        o.insert(
            "active_meta_profile".into(),
            json!(s.active_meta_profile.clone()),
        );
    }
    ok(v)
}

/// Gawa `*_diff` moja ya settings ndani ya `settings`.
///
/// Hukubali `agent_settings_diff`, `conversation_settings_diff`, na
/// `misc_settings_diff` (kanuni ya agent-server 1.27+; pia `*_settings` bila
/// kiambishi kwa round-trip). `misc_settings` hubeba mapendeleo ya frontend
/// (`app_preferences`: language, `user_consents_to_analytics`, git identity …).
///
/// Hurudisha `Err` kama hakuna diff yoyote inayotambulika — hii inazuia UI
/// kuandika settings kwa makosa ya muundo.
fn apply_settings_patch(settings: &mut Value, body: &Value) -> Result<(), &'static str> {
    let agent = body
        .get("agent_settings_diff")
        .or_else(|| body.get("agent_settings"));
    let conversation = body
        .get("conversation_settings_diff")
        .or_else(|| body.get("conversation_settings"));
    let misc = body
        .get("misc_settings_diff")
        .or_else(|| body.get("misc_settings"));

    if agent.is_none() && conversation.is_none() && misc.is_none() {
        return Err(
            "moja ya `agent_settings_diff`, `conversation_settings_diff`, au \
             `misc_settings_diff` ni lazima (POST haukubaliwi; tumia PATCH)",
        );
    }

    if let Some(d) = agent {
        merge_into(settings, &json!({ "agent_settings": d }));
    }
    if let Some(d) = conversation {
        merge_into(settings, &json!({ "conversation_settings": d }));
    }
    if let Some(d) = misc {
        merge_into(settings, &json!({ "misc_settings": d }));
    }
    Ok(())
}

/// PATCH /api/settings — hukubali diffs za agent/conversation/misc.
pub async fn patch_settings(Json(body): Json<Value>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    {
        let mut s = lock_store();
        if !s.settings.is_object() {
            s.settings = state::default_settings();
        }
        if let Err(msg) = apply_settings_patch(&mut s.settings, &body) {
            return Err(err(StatusCode::BAD_REQUEST, msg));
        }
        persist(&s);
    }
    Ok(get_settings().await)
}

/// GET /api/settings/agent-schema — schema HALISI ya AgentSettings.
pub async fn agent_schema() -> Json<Value> {
    const SCHEMA: &str = include_str!("../agent_schemas/agent-schema.json");
    match serde_json::from_str::<Value>(SCHEMA) {
        Ok(v) => ok(v),
        Err(_) => ok(json!({ "model_name": "AgentSettings", "sections": [] })),
    }
}

/// GET /api/settings/conversation-schema.
pub async fn conversation_schema() -> Json<Value> {
    const SCHEMA: &str = include_str!("../agent_schemas/conversation-schema.json");
    match serde_json::from_str::<Value>(SCHEMA) {
        Ok(v) => ok(v),
        Err(_) => ok(json!({ "model_name": "ConversationSettings", "sections": [] })),
    }
}

/// Unganisha JSON kwa kina (objects zinaunganishwa, mengine hubadilishwa).
fn merge_into(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    b.insert(k.clone(), Value::Null);
                } else {
                    merge_into(b.entry(k.clone()).or_insert(Value::Null), v);
                }
            }
        }
        (b, p) => *b = p.clone(),
    }
}

// ── Secrets ──────────────────────────────────────────────────────────────

/// GET /api/settings/secrets — majina + thamani zilizofichwa (masked).
pub async fn list_secrets() -> Json<Value> {
    let s = lock_store();
    let items: Vec<Value> = s
        .secrets
        .iter()
        .map(|(name, val)| {
            json!({
                "name": name,
                "description": null,
                "value": mask_secret(val),
                "scope": "global",
            })
        })
        .collect();
    ok(json!({ "secrets": items }))
}

/// PUT /api/settings/secrets — hifadhi secret (jina + thamani).
pub async fn put_secret(Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "name");
    let val = jstr(&body, "value");
    if name.is_empty() {
        return ok(json!({ "ok": false, "error": "jina la secret ni lazima" }));
    }
    {
        let mut s = lock_store();
        s.secrets.insert(name.clone(), val);
        persist(&s);
    }
    ok(json!({ "ok": true, "name": name }))
}

/// GET /api/settings/secrets/:name.
pub async fn get_secret(Path(name): Path<String>) -> Json<Value> {
    let s = lock_store();
    match s.secrets.get(&name) {
        Some(v) => ok(json!({ "name": name, "value": mask_secret(v), "scope": "global" })),
        None => ok(json!({ "name": name, "value": null, "scope": "global" })),
    }
}

/// DELETE /api/settings/secrets/:name.
pub async fn delete_secret(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.secrets.remove(&name);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

/// POST/PATCH/DELETE /api/settings/mcp/:key — mcp_config ya settings.
pub async fn mcp_set(Path(key): Path<String>, Json(body): Json<Value>) -> Json<Value> {
    {
        let mut s = lock_store();
        if !s.settings.is_object() {
            s.settings = state::default_settings();
        }
        let mcp = s
            .settings
            .pointer_mut("/agent_settings/mcp_config")
            .map(|v| {
                if !v.is_object() {
                    *v = json!({});
                }
                v
            });
        if let Some(m) = mcp {
            if let Some(o) = m.as_object_mut() {
                o.insert(key.clone(), body);
            }
        }
        persist(&s);
    }
    ok(json!({ "success": true }))
}

/// DELETE /api/settings/mcp/:key.
pub async fn mcp_delete(Path(key): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        if let Some(m) = s.settings.pointer_mut("/agent_settings/mcp_config") {
            if let Some(o) = m.as_object_mut() {
                o.remove(&key);
            }
        }
        persist(&s);
    }
    ok(json!({ "success": true }))
}

// ── Profiles (LLM profiles) ──────────────────────────────────────────────

/// Badilisha profile iliyohifadhiwa → umbo la `ProfileInfo` la SDK
/// (`{name, model, base_url, api_key_set}`). SDK/UI hutegemea `api_key_set`
/// ili kuamua LLM iko tayari; bila hiyo Canvas huonyesha "LLM isn't set up"
/// na huzuia Autopilot/chat.
fn profile_info(p: &Value) -> Value {
    let llm = p.get("llm").cloned().unwrap_or(Value::Null);
    let model = llm.get("model").cloned().unwrap_or(Value::Null);
    let base_url = llm
        .get("base_url")
        .or_else(|| llm.get("ollama_base_url"))
        .cloned()
        .unwrap_or(Value::Null);
    // Key imewekwa kama `api_key` si tupu, au LLM ya LAN (ollama) isiyohitaji key.
    let has_key = llm
        .get("api_key")
        .and_then(|k| k.as_str())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let url_is_local = base_url
        .as_str()
        .map(|u| u.contains("127.0.0.1") || u.contains("localhost"))
        .unwrap_or(false);
    json!({
        "name": jstr(p, "name"),
        "model": model,
        "base_url": base_url,
        "api_key_set": has_key || url_is_local,
    })
}

pub async fn list_profiles() -> Json<Value> {
    let s = lock_store();
    let profiles: Vec<Value> = s.profiles.iter().map(profile_info).collect();
    ok(json!({
        "profiles": profiles,
        "active_profile": s.active_profile,
    }))
}

pub async fn get_profile(Path(name): Path<String>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let s = lock_store();
    match s.profiles.iter().find(|p| jstr(p, "name") == name) {
        Some(p) => {
            let info = profile_info(p);
            let config = p.get("llm").cloned().unwrap_or_else(|| json!({}));
            Ok(ok(json!({
                "name": jstr(p, "name"),
                "config": config,
                "api_key_set": info["api_key_set"],
            })))
        }
        None => Err(err(StatusCode::NOT_FOUND, "profile haipo")),
    }
}

pub async fn upsert_profile(Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "name");
    upsert_profile_impl(name, body).await
}

pub async fn upsert_profile_named(
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    upsert_profile_impl(name, body).await
}

async fn upsert_profile_impl(name: String, mut body: Value) -> Json<Value> {
    if name.is_empty() {
        return ok(json!({ "ok": false, "error": "jina la profile ni lazima" }));
    }
    if let Some(o) = body.as_object_mut() {
        o.insert("name".into(), json!(name));
        o.entry("llm".to_string()).or_insert_with(state::default_llm);
    }
    {
        let mut s = lock_store();
        match s.profiles.iter_mut().find(|p| jstr(p, "name") == name) {
            Some(p) => *p = body.clone(),
            None => s.profiles.push(body.clone()),
        }
        persist(&s);
    }
    ok(body)
}

pub async fn delete_profile(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.profiles.retain(|p| jstr(p, "name") != name);
        if s.active_profile.as_deref() == Some(name.as_str()) {
            s.active_profile = None;
        }
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn activate_profile(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        if s.profiles.iter().any(|p| jstr(p, "name") == name) {
            s.active_profile = Some(name.clone());
        } else {
            s.profiles.push(json!({ "name": name, "llm": state::default_llm() }));
            s.active_profile = Some(name.clone());
        }
        persist(&s);
    }
    ok(json!({ "success": true, "active_profile": name }))
}

pub async fn rename_profile(
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let new = jstr(&body, "new_name");
    if new.is_empty() {
        return ok(json!({ "ok": false, "error": "new_name ni lazima" }));
    }
    {
        let mut s = lock_store();
        if let Some(p) = s.profiles.iter_mut().find(|p| jstr(p, "name") == name) {
            if let Some(o) = p.as_object_mut() {
                o.insert("name".into(), json!(new));
            }
        }
        if s.active_profile.as_deref() == Some(name.as_str()) {
            s.active_profile = Some(new.clone());
        }
        persist(&s);
    }
    ok(json!({ "success": true, "name": new }))
}

/// POST /api/profiles/:name/validate — huthibitisha profile (schema ndogo).
pub async fn validate_profile(Json(body): Json<Value>) -> Json<Value> {
    let has_llm = body.get("llm").is_some();
    ok(json!({ "valid": has_llm, "errors": if has_llm { json!([]) } else { json!(["`llm` haipo"]) } }))
}

// ── Agent profiles ───────────────────────────────────────────────────────

pub async fn list_agent_profiles() -> Json<Value> {
    let s = lock_store();
    ok(json!({
        "profiles": s.agent_profiles,
        "active_agent_profile_id": s.active_agent_profile_id,
    }))
}

pub async fn get_agent_profile(Path(name): Path<String>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let s = lock_store();
    match s.agent_profiles.iter().find(|p| jstr(p, "name") == name || jstr(p, "id") == name) {
        Some(p) => Ok(ok(p.clone())),
        None => Err(err(StatusCode::NOT_FOUND, "agent profile haipo")),
    }
}

pub async fn upsert_agent_profile(Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "name");
    upsert_agent_impl(if name.is_empty() { state::new_id() } else { name }, body).await
}

pub async fn upsert_agent_profile_named(
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    upsert_agent_impl(name, body).await
}

async fn upsert_agent_impl(name: String, mut body: Value) -> Json<Value> {
    if let Some(o) = body.as_object_mut() {
        o.insert("name".into(), json!(name));
        o.entry("id".to_string()).or_insert_with(|| json!(name));
        o.entry("agent_kind".to_string()).or_insert_with(|| json!("openhands"));
        o.entry("revision".to_string()).or_insert_with(|| json!(0));
        o.entry("llm_profile_ref".to_string())
            .or_insert_with(|| json!("default"));
        o.entry("mcp_server_refs".to_string()).or_insert_with(|| Value::Null);
    }
    {
        let mut s = lock_store();
        let id = jstr(&body, "id");
        match s
            .agent_profiles
            .iter_mut()
            .find(|p| jstr(p, "name") == name || jstr(p, "id") == id)
        {
            Some(p) => *p = body.clone(),
            None => s.agent_profiles.push(body.clone()),
        }
        persist(&s);
    }
    ok(body)
}

pub async fn delete_agent_profile(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.agent_profiles
            .retain(|p| jstr(p, "name") != name && jstr(p, "id") != name);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn activate_agent_profile(Path(id): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        if !s.agent_profiles.iter().any(|p| jstr(p, "id") == id || jstr(p, "name") == id) {
            s.agent_profiles.push(json!({
                "id": id, "name": id, "agent_kind": "openhands",
                "revision": 0, "llm_profile_ref": "default", "mcp_server_refs": null,
            }));
        }
        s.active_agent_profile_id = Some(id.clone());
        if let Some(o) = s.settings.pointer_mut("/agent_settings") {
            if let Some(oo) = o.as_object_mut() {
                oo.insert("active_agent_profile_id".into(), json!(id));
            }
        }
        persist(&s);
    }
    ok(json!({ "success": true, "active_agent_profile_id": id }))
}

pub async fn rename_agent_profile(
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let new = jstr(&body, "new_name");
    {
        let mut s = lock_store();
        if let Some(p) = s.agent_profiles.iter_mut().find(|p| jstr(p, "name") == name) {
            if let Some(o) = p.as_object_mut() {
                o.insert("name".into(), json!(new));
            }
        }
        persist(&s);
    }
    ok(json!({ "success": true, "name": new }))
}

/// POST /api/agent-profiles/:name/materialize — huthibitisha draft.
pub async fn materialize_agent_profile(
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let mut profile = body
        .get("profile")
        .cloned()
        .unwrap_or_else(|| json!({ "name": name, "agent_kind": "openhands" }));
    if let Some(o) = profile.as_object_mut() {
        o.insert("name".into(), json!(name));
    }
    ok(profile)
}

// ── Meta profiles ────────────────────────────────────────────────────────

pub async fn list_meta_profiles() -> Json<Value> {
    let s = lock_store();
    ok(json!({
        "meta_profiles": s.meta_profiles,
        "active_meta_profile": s.active_meta_profile,
    }))
}

pub async fn get_meta_profile(Path(name): Path<String>) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let s = lock_store();
    match s.meta_profiles.iter().find(|p| jstr(p, "name") == name) {
        Some(p) => Ok(ok(p.clone())),
        None => Err(err(StatusCode::NOT_FOUND, "meta profile haipo")),
    }
}

pub async fn upsert_meta_profile(Json(body): Json<Value>) -> Json<Value> {
    let name = jstr(&body, "name");
    upsert_meta_impl(if name.is_empty() { state::new_id() } else { name }, body).await
}

pub async fn upsert_meta_profile_named(
    Path(name): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    upsert_meta_impl(name, body).await
}

async fn upsert_meta_impl(name: String, mut body: Value) -> Json<Value> {
    if let Some(o) = body.as_object_mut() {
        o.insert("name".into(), json!(name));
    }
    {
        let mut s = lock_store();
        match s.meta_profiles.iter_mut().find(|p| jstr(p, "name") == name) {
            Some(p) => *p = body.clone(),
            None => s.meta_profiles.push(body.clone()),
        }
        persist(&s);
    }
    ok(body)
}

pub async fn delete_meta_profile(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.meta_profiles.retain(|p| jstr(p, "name") != name);
        if s.active_meta_profile.as_deref() == Some(name.as_str()) {
            s.active_meta_profile = None;
        }
        persist(&s);
    }
    ok(json!({ "success": true }))
}

pub async fn activate_meta_profile(Path(name): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.active_meta_profile = Some(name.clone());
        persist(&s);
    }
    ok(json!({ "success": true, "active_meta_profile": name }))
}

// ── LLM ──────────────────────────────────────────────────────────────────

/// GET /api/llm/models/verified — model zilizothibitishwa kwa kila provider.
pub async fn llm_models_verified() -> Json<Value> {
    let mut map = serde_json::Map::new();
    map.insert("openhands".into(), str_arr(crate::agent_catalogs::MODELS_OPENHANDS));
    map.insert("anthropic".into(), str_arr(crate::agent_catalogs::MODELS_ANTHROPIC));
    map.insert("openai".into(), str_arr(crate::agent_catalogs::MODELS_OPENAI));
    map.insert("mistral".into(), str_arr(crate::agent_catalogs::MODELS_MISTRAL));
    map.insert("gemini".into(), str_arr(crate::agent_catalogs::MODELS_GEMINI));
    map.insert("deepseek".into(), str_arr(crate::agent_catalogs::MODELS_DEEPSEEK));
    map.insert("moonshot".into(), str_arr(crate::agent_catalogs::MODELS_MOONSHOT));
    map.insert("minimax".into(), str_arr(crate::agent_catalogs::MODELS_MINIMAX));
    map.insert("glm".into(), str_arr(crate::agent_catalogs::MODELS_GLM));
    map.insert("nvidia".into(), str_arr(crate::agent_catalogs::MODELS_NVIDIA));
    map.insert("qwen".into(), str_arr(crate::agent_catalogs::MODELS_QWEN));
    map.insert("openrouter".into(), str_arr(crate::agent_catalogs::MODELS_OPENROUTER));
    map.insert(
        "digitalocean".into(),
        str_arr(crate::agent_catalogs::MODELS_DIGITALOCEAN),
    );
    // Injini ya ndani (GGUF/ollama) — model halisi zilizopakiwa upande wetu.
    let local: Vec<Value> = crate::llm_setup::models_list()
        .iter()
        .filter_map(|m| m.get("id").and_then(|x| x.as_str()))
        .map(|id| json!(format!("ollama/{id}")))
        .collect();
    if !local.is_empty() {
        map.insert("ollama".into(), Value::Array(local));
    }
    ok(json!({ "models": Value::Object(map) }))
}

fn str_arr(items: &[&str]) -> Value {
    Value::Array(items.iter().map(|s| json!(s)).collect())
}

/// GET /api/llm/providers — orodha kamili ya provider za LiteLLM.
pub async fn llm_providers() -> Json<Value> {
    ok(json!({ "providers": str_arr(crate::agent_catalogs::VERIFIED_PROVIDERS) }))
}

/// GET /api/llm/provider-connections.
pub async fn list_provider_connections() -> Json<Value> {
    let s = lock_store();
    ok(Value::Array(s.provider_connections.clone()))
}

/// POST /api/llm/provider-connections.
pub async fn create_provider_connection(Json(body): Json<Value>) -> Json<Value> {
    let mut conn = body.clone();
    if let Some(o) = conn.as_object_mut() {
        o.entry("id".to_string()).or_insert_with(|| json!(state::new_id()));
    }
    {
        let mut s = lock_store();
        s.provider_connections.push(conn.clone());
        persist(&s);
    }
    ok(conn)
}

/// PATCH /api/llm/provider-connections/:id.
pub async fn update_provider_connection(
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    {
        let mut s = lock_store();
        if let Some(c) = s
            .provider_connections
            .iter_mut()
            .find(|c| jstr(c, "id") == id)
        {
            let mut merged = c.clone();
            merge_into(&mut merged, &body);
            *c = merged;
        }
        persist(&s);
    }
    ok(json!({ "success": true, "id": id }))
}

/// DELETE /api/llm/provider-connections/:id.
pub async fn delete_provider_connection(Path(id): Path<String>) -> Json<Value> {
    {
        let mut s = lock_store();
        s.provider_connections.retain(|c| jstr(c, "id") != id);
        persist(&s);
    }
    ok(json!({ "success": true }))
}

/// GET /api/keys/current — hali ya key (haikurudishi key yenyewe).
pub async fn keys_current() -> Json<Value> {
    let set = crate::ai_config::ai_key().is_some();
    ok(json!({
        "api_key_is_set": set,
        "source": if set { "env" } else { "none" },
    }))
}

/// GET /api/llm/subscription/openai/status.
pub async fn sub_status() -> Json<Value> {
    ok(json!({ "connected": false, "status": "not_configured" }))
}

/// GET /api/llm/subscription/openai/models.
pub async fn sub_models() -> Json<Value> {
    ok(json!({ "models": [] }))
}

/// POST /api/llm/subscription/openai/device/start.
pub async fn sub_device_start() -> Json<Value> {
    ok(json!({
        "ok": false,
        "error": "Subscription ya OpenAI haipatikani kwenye MTECHOS — tumia API key (FUNDI_AI_KEY)."
    }))
}

/// POST /api/llm/subscription/openai/device/poll.
pub async fn sub_device_poll() -> Json<Value> {
    ok(json!({ "status": "not_configured" }))
}

/// POST /api/llm/subscription/openai/logout.
pub async fn sub_logout() -> Json<Value> {
    ok(json!({ "success": true }))
}

// ── v1 config search ─────────────────────────────────────────────────────

/// GET /api/v1/config/models/search.
pub async fn v1_models_search(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let needle = q
        .get("q")
        .or_else(|| q.get("name"))
        .cloned()
        .unwrap_or_default()
        .to_lowercase();
    let mut out: Vec<Value> = Vec::new();
    for (provider, models) in [
        ("openhands", crate::agent_catalogs::MODELS_OPENHANDS),
        ("anthropic", crate::agent_catalogs::MODELS_ANTHROPIC),
        ("openai", crate::agent_catalogs::MODELS_OPENAI),
        ("gemini", crate::agent_catalogs::MODELS_GEMINI),
        ("deepseek", crate::agent_catalogs::MODELS_DEEPSEEK),
        ("qwen", crate::agent_catalogs::MODELS_QWEN),
        ("openrouter", crate::agent_catalogs::MODELS_OPENROUTER),
    ] {
        for m in models {
            if needle.is_empty() || m.to_lowercase().contains(&needle) {
                out.push(json!({ "provider": provider, "model": m, "name": m }));
            }
        }
    }
    ok(json!({ "items": out, "results": out }))
}

/// GET /api/v1/config/providers/search.
pub async fn v1_providers_search(Query(q): Query<HashMap<String, String>>) -> Json<Value> {
    let needle = q
        .get("q")
        .or_else(|| q.get("name"))
        .cloned()
        .unwrap_or_default()
        .to_lowercase();
    let items: Vec<Value> = crate::agent_catalogs::VERIFIED_PROVIDERS
        .iter()
        .filter(|p| needle.is_empty() || p.to_lowercase().contains(&needle))
        .map(|p| json!({ "name": p, "provider": p }))
        .collect();
    ok(json!({ "items": items, "results": items }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `misc_settings_diff.app_preferences` LAZIMA ikubaliwe — hivi ndivyo
    /// Canvas SPA (agent-server 1.27+) huwasilisha mapendeleo kama
    /// `user_consents_to_analytics`. Kukataa huku kulifanya banner ya
    /// telemetry isifungwe (PATCH → 400).
    #[test]
    fn misc_settings_diff_inakubaliwa() {
        let mut settings = state::default_settings();
        let body = json!({
            "misc_settings_diff": {
                "app_preferences": { "user_consents_to_analytics": true }
            }
        });
        assert!(apply_settings_patch(&mut settings, &body).is_ok());
        assert_eq!(
            settings["misc_settings"]["app_preferences"]["user_consents_to_analytics"],
            json!(true)
        );
    }

    /// `agent_settings_diff` bado inafanya kazi, na diffs hurundikana.
    #[test]
    fn agent_settings_diff_inarundikana() {
        let mut settings = state::default_settings();
        apply_settings_patch(
            &mut settings,
            &json!({ "agent_settings_diff": { "llm": { "model": "m1" } } }),
        )
        .unwrap();
        apply_settings_patch(
            &mut settings,
            &json!({ "agent_settings_diff": { "llm": { "base_url": "http://x" } } }),
        )
        .unwrap();
        assert_eq!(settings["agent_settings"]["llm"]["model"], json!("m1"));
        assert_eq!(
            settings["agent_settings"]["llm"]["base_url"],
            json!("http://x")
        );
    }

    /// Ombi bila diff yoyote hukataliwa (hulinda UI dhidi ya maumbo mabovu).
    #[test]
    fn ombi_bila_diff_hukataliwa() {
        let mut settings = state::default_settings();
        assert!(apply_settings_patch(&mut settings, &json!({})).is_err());
    }

    /// `profile_info` LAZIMA itoe umbo la SDK (`api_key_set`), si `{llm:{…}}`.
    /// Bila `api_key_set` Canvas huonyesha "LLM isn't set up" na huzuia chat.
    #[test]
    fn profile_info_ina_api_key_set() {
        // Profile ya LAN (ollama) — haiitaji key, hivyo api_key_set = true.
        let p = json!({
            "name": "default",
            "llm": { "model": "qwen2.5vl:3b", "base_url": "http://127.0.0.1:11434", "api_key": null }
        });
        let info = profile_info(&p);
        assert_eq!(info["name"], json!("default"));
        assert_eq!(info["model"], json!("qwen2.5vl:3b"));
        assert_eq!(info["base_url"], json!("http://127.0.0.1:11434"));
        assert_eq!(info["api_key_set"], json!(true));

        // Cloud yenye key → true; cloud bila key → false.
        let cloud_key = json!({
            "name": "cloud",
            "llm": { "model": "gemma4:31b", "base_url": "https://ollama.com/v1", "api_key": "sk-123" }
        });
        assert_eq!(profile_info(&cloud_key)["api_key_set"], json!(true));
        let cloud_nokey = json!({
            "name": "cloud2",
            "llm": { "model": "gemma4:31b", "base_url": "https://ollama.com/v1", "api_key": null }
        });
        assert_eq!(profile_info(&cloud_nokey)["api_key_set"], json!(false));
    }
}
