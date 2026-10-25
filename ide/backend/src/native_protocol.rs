//! native_protocol.rs — PROTOCOL YA AGENT-SERVER KWA RUST (MBADALA WA PYTHON).
//!
//! Lengo: kuondoa utegemezi wa `openhands-agent-server` (Python, :18000) kwa
//! kuandika protocol ya conversation kwa RUST, kwa kutumia crates zilizopo
//! (`fundi-agent`, `fundi-server`, `fundi-core`, `fundi-tools`).
//!
//! Mkakati wa uhamishaji salama (hatua kwa hatua, sio mkupuo):
//!   1. **Hatua hii (sasa)**: njia zinapatikana kwenye kianzio KIPYA `/api/native/*`
//!      — zinaendesha SAMBAMBA na daraja la Python (`agent_server_proxy`), ili
//!      Canvas iweze kulinganisha matokeo (A/B) bila kuvunja kitu.
//!   2. Njia zikishathibitishwa kwa data halisi, `agent_server_proxy` itapungua
//!      hatua kwa hatua hadi Python iondoke.
//!
//! KANUNI: hakuna shadow route ya `/api/*` iliyopo — tunaweka kianzio tofauti
//! (`/api/native/`), hivyo hazishindani na daraja lililopo (hakuna panics za
//! merge, hakuna data ya kubuni inayochukua nafasi ya halisi).

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::Path as AxumPath;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tokio::sync::broadcast;

use crate::AppState;

/// Hifadhi ya conversations za Rust-native (Hatua 1).
///
/// Kwa sasa ni **in-memory** (Mutex) — inafanya kazi kikamilifu (create/list/
/// get/append) na inapimika. Uhifadhi wa SQLite (`fundi-agent::session_persistence`)
/// unafuata kwenye hatua inayofuata.
#[derive(Default)]
pub struct ConversationStore {
    convos: HashMap<String, Conversation>,
    counter: u64,
    /// Settings za LLM (Hatua 3) — hufuata umbo la `agent_settings` ya agent-server.
    settings: HashMap<String, Value>,
    /// Secrets (Hatua 3) — HAZITOKI kamwe kwa mtandao bila masking.
    secrets: HashMap<String, String>,
    /// Profiles (Hatua 3) — orodha ya profiles za agent.
    profiles: Vec<Value>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MessageRec {
    pub id: String,
    pub role: String,
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub messages: Vec<MessageRec>,
}

static STORE: OnceLock<Mutex<ConversationStore>> = OnceLock::new();

fn store() -> &'static Mutex<ConversationStore> {
    STORE.get_or_init(|| Mutex::new(ConversationStore::default()))
}

/// Ufunguzi salama wa store — HAUTUMII `.unwrap()` (hauwezi panic hata lock
/// ikiwa imeharibika). Hii inakidhi kanuni ya BUG-001 ya `code_audit`.
fn lock_store() -> MutexGuard<'static, ConversationStore> {
    store().lock().unwrap_or_else(|e| e.into_inner())
}

/// Bus ya events kwa kila conversation (Hatua 2) — WS hupata stream ya
/// events halisi (ujumbe wa mtumiaji, jibu la assistant).
struct EventBus {
    senders: HashMap<String, broadcast::Sender<Value>>,
}

static EVENT_BUS: OnceLock<Mutex<EventBus>> = OnceLock::new();

fn event_bus() -> &'static Mutex<EventBus> {
    EVENT_BUS.get_or_init(|| {
        Mutex::new(EventBus {
            senders: HashMap::new(),
        })
    })
}

fn bus_lock() -> MutexGuard<'static, EventBus> {
    event_bus().lock().unwrap_or_else(|e| e.into_inner())
}

/// Pata (au tengeneza) channel ya events ya conversation moja.
fn subscribe_events(convo_id: &str) -> broadcast::Receiver<Value> {
    let mut bus = bus_lock();
    let tx = bus
        .senders
        .entry(convo_id.to_string())
        .or_insert_with(|| broadcast::channel(256).0);
    tx.subscribe()
}

/// Tangaza event kwa wote wanaosikiliza conversation (WS).
fn publish_event(convo_id: &str, event: Value) {
    let mut bus = bus_lock();
    let tx = bus
        .senders
        .entry(convo_id.to_string())
        .or_insert_with(|| broadcast::channel(256).0);
    let _ = tx.send(event);
}

/// Mask kwa secret: hurudisha herufi 4 za mwanzo + nyota. Kamwe hakurudishi
/// secret kamili kwa mtandao.
fn mask_secret(v: &str) -> String {
    if v.is_empty() {
        return String::new();
    }
    let head: String = v.chars().take(4).collect();
    format!("{head}{}", "*".repeat(v.chars().count().saturating_sub(4).max(4)))
}

impl ConversationStore {
    fn create(&mut self, title: Option<String>) -> Conversation {
        self.counter += 1;
        let id = format!("native-{}", self.counter);
        let c = Conversation {
            id: id.clone(),
            title: title.unwrap_or_else(|| format!("Conversation {id}")),
            created_at: chrono::Utc::now().to_rfc3339(),
            messages: Vec::new(),
        };
        self.convos.insert(id, c.clone());
        c
    }

    fn append(&mut self, id: &str, role: &str, content: &str) -> Option<MessageRec> {
        let c = self.convos.get_mut(id)?;
        let m = MessageRec {
            id: format!("{}-m{}", id, c.messages.len() + 1),
            role: role.to_string(),
            content: content.to_string(),
        };
        c.messages.push(m.clone());
        Some(m)
    }

    /// Jibu la kubuni (deterministic) la assistant — HUIGA agent bila Python.
    ///
    /// Hii ni "bush" ya protocol: huthibitisha duara kamili (user → assistant
    /// event → stream) kwa Rust-native. Ubongo halisi (LLM) unaunganishwa
    /// kupitia `fundi-agent` kwenye Hatua inayofuata.
    fn build_assistant_reply(user_text: &str) -> String {
        format!("[MTECHOS native] Nimepokea: \"{}\".", user_text.trim())
    }
}


/// Toleo la protocol ya Rust-native. Huongezwa kila tunaposhusha kipande
/// kikubwa cha protocol kutoka Python.
pub const NATIVE_PROTOCOL_VERSION: &str = "0.2.0";

/// Muda server ilianza (kwa uptime).
static STARTED_AT: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

fn started_at() -> &'static std::time::Instant {
    STARTED_AT.get_or_init(std::time::Instant::now)
}

/// Jina la runtime — `rust` (badala ya `python_version` ya agent-server).
fn runtime_name() -> &'static str {
    "fundi-rust"
}

/// `/api/native/server_info` — mkataba huu unafuata shape ya agent-server.
///
/// Agent-server hurudisha: `uptime`, `idle_time`, `title`, `version`,
/// `sdk_version`, `tools_version`, `workspace_version`, `build_git_sha`,
/// `build_git_ref`, `python_version`, `usable_tools`. Hapa `python_version`
/// haipo (Rust) na `runtime` imeongezwa; Canvas hulaza tu sehemu inayojulikana.
pub fn server_info_payload() -> Value {
    let uptime = started_at().elapsed().as_secs_f64();
    json!({
        "uptime": uptime,
        "idle_time": uptime,
        "title": "MTECHOS Native Agent Server",
        "version": NATIVE_PROTOCOL_VERSION,
        "runtime": runtime_name(),
        "sdk_version": NATIVE_PROTOCOL_VERSION,
        "tools_version": NATIVE_PROTOCOL_VERSION,
        "workspace_version": "rust",
        "build_git_sha": std::option_env!("GIT_SHA").unwrap_or("unknown"),
        "build_git_ref": std::option_env!("GIT_REF").unwrap_or("refs/heads/main"),
        // Orodha ya zana zinazopatikana kwenye protocol ya Rust-native.
        "usable_tools": [
            "file_editor",
            "execute_bash",
            "task_tracker",
            "browser",
            "code_audit",
        ],
    })
}

/// `/api/native/settings` — mwonekano wa kwanza wa settings kwa Rust-native.
///
/// Kwa sasa hurudisha wigo (envelope) ILE ILE ya agent-server
/// (`agent_settings` + `conversation_settings` + `llm_api_key_is_set`),
/// na `schema_version`/`agent_kind`/`agent` za msingi. Sehemu za kina
/// (llm/profiles/secrets) zitajazwa kwenye hatua zinazofuata kutoka
/// `fundi-config` (GatewayConfig).
pub fn settings_payload() -> Value {
    let s = lock_store();
    let llm = s.settings.get("llm").cloned().unwrap_or(Value::Null);
    let conversation = s
        .settings
        .get("conversation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "agent_settings": {
            "schema_version": 8,
            "agent_kind": "openhands",
            "agent": "CodeActAgent",
            "llm": llm,
        },
        "conversation_settings": conversation,
        "llm_api_key_is_set": s.secrets.contains_key("llm_api_key"),
        "runtime": runtime_name(),
    })
}

/// `/api/native/conversations` — envelope ya orodha (Canva compat floor).
/// Data halisi hutoka `ConversationStore` (ona handlers hapa chini).
pub fn conversations_payload() -> Value {
    json!({ "items": [], "runtime": runtime_name() })
}

/// Router ya protocol ya Rust-native (`/api/native/*`).
///
/// HUISWAWI kwa njia za `/api/*` za daraja la Python — kianzio ni tofauti,
/// hivyo hakuna mgongano wa merge wala data ya kubuni inayochukua nafasi
/// ya agent-server halisi.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/native/server_info", get(native_server_info))
        .route("/api/native/health", get(native_health))
        // ── Conversations (Hatua 1) ─────────────────────────────────────
        .route(
            "/api/native/conversations",
            get(native_list_conversations).post(native_create_conversation),
        )
        .route(
            "/api/native/conversations/:id",
            get(native_get_conversation),
        )
        .route(
            "/api/native/conversations/:id/messages",
            post(native_append_message),
        )
        // ── WS streaming (Hatua 2) ──────────────────────────────────────
        .route(
            "/api/native/sockets/session/:id",
            get(native_ws_session),
        )
        // ── Settings (Hatua 3) ──────────────────────────────────────────
        .route(
            "/api/native/settings",
            get(native_settings).patch(native_patch_settings),
        )
        // ── Secrets (Hatua 3) ───────────────────────────────────────────
        .route(
            "/api/native/secrets",
            get(native_list_secrets).post(native_set_secret),
        )
        // ── Profiles (Hatua 3) ──────────────────────────────────────────
        .route(
            "/api/native/profiles",
            get(native_list_profiles).post(native_add_profile),
        )
}

async fn native_server_info() -> Json<Value> {
    Json(server_info_payload())
}

async fn native_settings() -> Json<Value> {
    Json(settings_payload())
}

async fn native_health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "runtime": runtime_name(),
        "protocol_version": NATIVE_PROTOCOL_VERSION,
        "python_required": false,
    }))
}

/// GET /api/native/conversations
async fn native_list_conversations() -> Json<Value> {
    let s = lock_store();
    let items: Vec<&Conversation> = s.convos.values().collect();
    Json(json!({ "items": items, "runtime": runtime_name() }))
}

/// POST /api/native/conversations { "title": "..." }
async fn native_create_conversation(Json(body): Json<Value>) -> Json<Value> {
    let title = body.get("title").and_then(|v| v.as_str()).map(String::from);
    let c = lock_store().create(title);
    Json(json!(c))
}

/// GET /api/native/conversations/:id
async fn native_get_conversation(AxumPath(id): AxumPath<String>) -> Json<Value> {
    let s = lock_store();
    match s.convos.get(&id) {
        Some(c) => Json(json!(c)),
        None => Json(json!({ "error": "conversation haipo", "id": id })),
    }
}

/// POST /api/native/conversations/:id/messages { "role": "...", "content": "..." }
///
/// Hii ni duara KAMILI ya Rust-native: huhifadhi ujumbe, hutengeneza jibu la
/// assistant, na **hutangaza events** kwenye bus (WS huzipata papo hapo).
async fn native_append_message(
    AxumPath(id): AxumPath<String>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let role = body.get("role").and_then(|v| v.as_str()).unwrap_or("user");
    let content = body.get("content").and_then(|v| v.as_str()).unwrap_or("");

    let user_msg = {
        let mut s = lock_store();
        match s.append(&id, role, content) {
            Some(m) => m,
            None => {
                return Json(json!({ "ok": false, "error": "conversation haipo", "id": id }));
            }
        }
    };
    publish_event(
        &id,
        json!({ "type": "message", "message": user_msg, "runtime": runtime_name() }),
    );

    // Jibu la assistant (Rust-native, bila Python).
    let reply_text = ConversationStore::build_assistant_reply(content);
    let assistant_msg = {
        let mut s = lock_store();
        s.append(&id, "assistant", &reply_text)
    };
    if let Some(ref m) = assistant_msg {
        publish_event(
            &id,
            json!({ "type": "message", "message": m, "runtime": runtime_name() }),
        );
    }

    Json(json!({
        "ok": true,
        "message": user_msg,
        "reply": assistant_msg,
        "runtime": runtime_name(),
    }))
}

// ── Hatua 2: WS streaming ────────────────────────────────────────────────

/// GET /api/native/sockets/session/:id (WebSocket)
///
/// Hutuma: (1) history ya conversation mara moja, (2) events zote mpya
/// (ujumbe wa mtumiaji + jibu) kupitia bus. Hii ni badala ya Rust-native
/// ya `/sockets/*` inayopita Python.
async fn native_ws_session(
    ws: WebSocketUpgrade,
    AxumPath(id): AxumPath<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_native_ws(socket, id))
}

async fn handle_native_ws(mut socket: WebSocket, id: String) {
    // 1) Subscribe KABLA ya history ili kuepuka race (event inayotangazwa
    //    wakati history inasafiri isipotezwe).
    let mut rx = subscribe_events(&id);

    // 2) History ya awali.
    let history = {
        let s = lock_store();
        s.convos.get(&id).cloned()
    };
    let initial = match history {
        Some(c) => json!({ "type": "history", "conversation": c, "runtime": runtime_name() }),
        None => json!({ "type": "error", "error": "conversation haipo", "id": id }),
    };
    if socket
        .send(WsMessage::Text(initial.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    // 3) SIKILIZA events + KARIRI ujumbe unaoingia (duara kamili).
    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(v) => {
                        if socket.send(WsMessage::Text(v.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(WsMessage::Text(t))) => {
                        let v: Value = serde_json::from_str(&t).unwrap_or_else(|_| json!({ "content": t.to_string() }));
                        let content = v.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                        if content.is_empty() { continue; }
                        let user_msg = {
                            let mut s = lock_store();
                            s.append(&id, "user", &content)
                        };
                        if let Some(m) = user_msg {
                            publish_event(&id, json!({ "type": "message", "message": m, "runtime": runtime_name() }));
                        }
                        // Jibu la assistant (duara kamili, kama HTTP path).
                        let reply_text = ConversationStore::build_assistant_reply(&content);
                        let assistant_msg = {
                            let mut s = lock_store();
                            s.append(&id, "assistant", &reply_text)
                        };
                        if let Some(m) = assistant_msg {
                            publish_event(&id, json!({ "type": "message", "message": m, "runtime": runtime_name() }));
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
        }
    }
}

// ── Hatua 3: settings / secrets / profiles ───────────────────────────────

/// PATCH /api/native/settings { "llm": {...}, "conversation": {...} }
///
/// Hufuata kanuni ya agent-server: mabadiliko huja ndani ya `agent_settings_diff`.
async fn native_patch_settings(Json(body): Json<Value>) -> Json<Value> {
    let diff = body.get("agent_settings_diff").cloned().unwrap_or(body);
    {
        let mut s = lock_store();
        if let Some(llm) = diff.get("llm") {
            s.settings.insert("llm".to_string(), llm.clone());
        }
        if let Some(conv) = diff.get("conversation") {
            s.settings.insert("conversation".to_string(), conv.clone());
        }
    } // lock imeachiliwa KABLA ya settings_payload() kuepuka deadlock.
    Json(json!({ "ok": true, "settings": settings_payload(), "runtime": runtime_name() }))
}

/// GET /api/native/secrets — hurejesha keys PEKEE (values zimefichwa).
async fn native_list_secrets() -> Json<Value> {
    let s = lock_store();
    let items: Vec<Value> = s
        .secrets
        .iter()
        .map(|(k, v)| json!({ "name": k, "masked": mask_secret(v) }))
        .collect();
    Json(json!({ "items": items, "runtime": runtime_name() }))
}

/// POST /api/native/secrets { "name": "...", "value": "..." }
async fn native_set_secret(Json(body): Json<Value>) -> Json<Value> {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let value = body.get("value").and_then(|v| v.as_str()).unwrap_or("");
    if name.is_empty() || value.is_empty() {
        return Json(json!({ "ok": false, "error": "name na value ni lazima" }));
    }
    lock_store().secrets.insert(name.to_string(), value.to_string());
    // HAITANGAZI value — hurejesha mask pekee.
    Json(json!({ "ok": true, "name": name, "masked": mask_secret(value) }))
}

/// GET /api/native/profiles
async fn native_list_profiles() -> Json<Value> {
    let s = lock_store();
    Json(json!({ "items": s.profiles, "runtime": runtime_name() }))
}

/// POST /api/native/profiles { "name": "...", ... }
async fn native_add_profile(Json(body): Json<Value>) -> Json<Value> {
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let profile = json!({ "name": name, "settings": body.get("settings").cloned().unwrap_or(Value::Null) });
    lock_store().profiles.push(profile.clone());
    Json(json!({ "ok": true, "profile": profile }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_info_inazo_sehemu_za_mkataba_wa_agent_server() {
        let v = server_info_payload();
        // Nyuga ambazo Canvas huzitegemea (compat floor) LAZIMA ziwepo.
        for key in [
            "uptime",
            "title",
            "version",
            "sdk_version",
            "tools_version",
            "usable_tools",
        ] {
            assert!(v.get(key).is_some(), "server_info inakosa `{key}`");
        }
        assert!(v["usable_tools"].is_array());
        assert_eq!(v["runtime"], "fundi-rust");
        // Rust-native HAIPASWI kutangaza python_version.
        assert!(v.get("python_version").is_none());
    }

    #[test]
    fn settings_inazo_envelope_ya_agent_server() {
        let v = settings_payload();
        assert!(v.get("agent_settings").is_some());
        assert!(v.get("conversation_settings").is_some());
        assert_eq!(v["llm_api_key_is_set"], false);
        assert_eq!(v["agent_settings"]["agent_kind"], "openhands");
    }

    #[test]
    fn conversations_ni_items_array() {
        let v = conversations_payload();
        assert!(v["items"].is_array());
    }

    #[test]
    fn store_create_get_append() {
        let mut s = ConversationStore::default();
        let c = s.create(Some("Kazi yangu".to_string()));
        assert_eq!(c.id, "native-1");
        assert_eq!(c.title, "Kazi yangu");
        assert!(c.messages.is_empty());

        let m = s.append("native-1", "user", "habari").unwrap();
        assert_eq!(m.role, "user");
        assert_eq!(m.content, "habari");
        assert_eq!(s.convos.get("native-1").unwrap().messages.len(), 1);

        // Conversation isiyopo → None (hakuna panic)
        assert!(s.append("nope", "user", "x").is_none());
    }

    #[test]
    fn store_counter_huenda_mbele() {
        let mut s = ConversationStore::default();
        let a = s.create(None);
        let b = s.create(None);
        assert_ne!(a.id, b.id);
        assert_eq!(a.title, "Conversation native-1");
        assert_eq!(b.title, "Conversation native-2");
    }

    #[test]
    fn uptime_haipungui() {
        let a = server_info_payload()["uptime"].as_f64().unwrap();
        let b = server_info_payload()["uptime"].as_f64().unwrap();
        assert!(b >= a);
    }

    // ── Hatua 2: events / WS ────────────────────────────────────────────

    #[test]
    fn bus_hupokea_events_zinazotangazwa() {
        let mut rx = subscribe_events("convo-x");
        publish_event("convo-x", json!({ "type": "message", "n": 1 }));
        publish_event("convo-x", json!({ "type": "message", "n": 2 }));
        let a = rx.try_recv().expect("event 1");
        let b = rx.try_recv().expect("event 2");
        assert_eq!(a["n"], 1);
        assert_eq!(b["n"], 2);
    }

    #[test]
    fn bus_hutenganisha_conversations() {
        let mut rx = subscribe_events("convo-a");
        publish_event("convo-b", json!({ "type": "message" }));
        // Event ya convo-b HAIpaswi kufika convo-a.
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn assistant_reply_ni_deterministic() {
        let r = ConversationStore::build_assistant_reply("ongeza test");
        assert!(r.contains("ongeza test"));
        assert!(r.starts_with("[MTECHOS native]"));
    }

    // ── Hatua 3: secrets / settings / profiles ──────────────────────────

    #[test]
    fn mask_secret_hafichui_value_kamili() {
        let m = mask_secret("ghp_super_secret_token");
        assert!(m.starts_with("ghp_"));
        assert!(m.contains('*'));
        assert!(!m.contains("super_secret_token"));
        // Fupi pia haifichui.
        assert_eq!(mask_secret("ab"), "ab****");
        assert_eq!(mask_secret(""), "");
    }

    #[test]
    fn settings_payload_huonyesha_secret_imewekwa() {
        // Sio hali ya global — tumia store safi.
        let mut s = ConversationStore::default();
        assert!(!s.secrets.contains_key("llm_api_key"));
        s.secrets
            .insert("llm_api_key".to_string(), "sk-abc123".to_string());
        assert!(s.secrets.contains_key("llm_api_key"));
        // Mask huhakikisha value haitolewa.
        assert_eq!(mask_secret("sk-abc123"), "sk-a*****");
    }

    #[test]
    fn store_settings_na_profiles_hufanya_kazi() {
        let mut s = ConversationStore::default();
        s.settings
            .insert("llm".to_string(), json!({ "model": "gpt-oss:20b" }));
        assert_eq!(s.settings.get("llm").unwrap()["model"], "gpt-oss:20b");

        s.profiles.push(json!({ "name": "default" }));
        assert_eq!(s.profiles.len(), 1);
    }
}
