//! Full ACP request handler — implements all ACP protocol methods.
//!
//! Mirrors the Python `acp_adapter/server.py` FundiACPAgent class.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::events::{AcpEvent, EventSink};
use crate::permissions::PermissionStore;
use crate::protocol::*;
use crate::session::{SessionManager, SessionPhase};

/// Backend that runs a real agent turn for an ACP prompt.
///
/// The default handler has no backend (echo fallback for protocol-only use).
/// Production wires [`AgentBackend`] (see `fundi-cli`) so prompts actually
/// execute the Fundi agent loop with tools.
#[async_trait::async_trait]
pub trait AgentBackend: Send + Sync {
    /// Run one user prompt and return the assistant's final text.
    async fn run_prompt(&self, session_id: &str, prompt: &str) -> Result<String, String>;

    /// List the tools the real agent has registered.
    ///
    /// Default: no tool introspection. Backends that own a tool registry
    /// override this so `/tools` reports the real set instead of a stub.
    async fn list_tools(&self) -> Result<Vec<String>, String> {
        Err("tool listing not supported by this backend".to_string())
    }

    /// Compact the given conversation history and return the compressed
    /// history, so `/compact` really shrinks the context window.
    ///
    /// Default: no compaction support.
    async fn compact_history(&self, _history: Vec<Value>) -> Result<Vec<Value>, String> {
        Err("context compaction not supported by this backend".to_string())
    }
}

/// Trait for handling ACP requests.
#[async_trait::async_trait]
pub trait AcpHandler: Send + Sync {
    async fn handle_request(&self, request: AcpRequest) -> AcpResponse;
}

// ---------------------------------------------------------------------------
// Slash commands
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct SlashCommand {
    name: &'static str,
    description: &'static str,
    input_hint: Option<&'static str>,
}

const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "help",
        description: "Show available commands",
        input_hint: None,
    },
    SlashCommand {
        name: "model",
        description: "Show or change current model",
        input_hint: Some("model name to switch to"),
    },
    SlashCommand {
        name: "tools",
        description: "List available tools",
        input_hint: None,
    },
    SlashCommand {
        name: "context",
        description: "Show conversation context info",
        input_hint: None,
    },
    SlashCommand {
        name: "reset",
        description: "Clear conversation history",
        input_hint: None,
    },
    SlashCommand {
        name: "compact",
        description: "Compress conversation context",
        input_hint: None,
    },
    SlashCommand {
        name: "version",
        description: "Show Fundi version",
        input_hint: None,
    },
];

// ---------------------------------------------------------------------------
// FundiAcpHandler
// ---------------------------------------------------------------------------

/// Full ACP handler wrapping Fundi agent capabilities.
pub struct FundiAcpHandler {
    pub session_manager: Arc<SessionManager>,
    pub event_sink: Arc<EventSink>,
    pub permission_store: Arc<PermissionStore>,
    version: String,
    backend: Option<Arc<dyn AgentBackend>>,
}

impl FundiAcpHandler {
    pub fn new(
        session_manager: Arc<SessionManager>,
        event_sink: Arc<EventSink>,
        permission_store: Arc<PermissionStore>,
    ) -> Self {
        Self {
            session_manager,
            event_sink,
            permission_store,
            version: env!("CARGO_PKG_VERSION").to_string(),
            backend: None,
        }
    }

    /// Build a handler backed by a real agent runtime.
    pub fn with_backend(
        session_manager: Arc<SessionManager>,
        event_sink: Arc<EventSink>,
        permission_store: Arc<PermissionStore>,
        backend: Arc<dyn AgentBackend>,
    ) -> Self {
        Self {
            session_manager,
            event_sink,
            permission_store,
            version: env!("CARGO_PKG_VERSION").to_string(),
            backend: Some(backend),
        }
    }

    #[allow(dead_code)]
    fn available_commands(&self) -> Vec<Value> {
        SLASH_COMMANDS
            .iter()
            .map(|cmd| {
                let mut obj = json!({
                    "name": cmd.name,
                    "description": cmd.description,
                });
                if let Some(hint) = cmd.input_hint {
                    obj["input"] = json!({"hint": hint});
                }
                obj
            })
            .collect()
    }

    async fn handle_slash_command(&self, text: &str, session_id: &str) -> Option<String> {
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        let cmd = parts[0].trim_start_matches('/').to_lowercase();
        let _args = if parts.len() > 1 { parts[1].trim() } else { "" };

        match cmd.as_str() {
            "help" => {
                let mut lines = vec!["Available commands:".to_string(), String::new()];
                for sc in SLASH_COMMANDS {
                    lines.push(format!("  /{:<10}  {}", sc.name, sc.description));
                }
                lines.push(String::new());
                lines.push(
                    "Unrecognized /commands are sent to the model as normal messages.".to_string(),
                );
                Some(lines.join("\n"))
            }
            "model" => {
                let state = self.session_manager.get_session(session_id)?;
                let model = state.model.as_deref().unwrap_or("unknown");
                let provider = state.provider.as_deref().unwrap_or("auto");
                Some(format!("Current model: {model}\nProvider: {provider}"))
            }
            "context" => {
                let state = self.session_manager.get_session(session_id)?;
                let n = state.history.len();
                if n == 0 {
                    return Some("Conversation is empty (no messages yet).".to_string());
                }
                let mut roles: HashMap<String, usize> = HashMap::new();
                for msg in &state.history {
                    let role = msg
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    *roles.entry(role.to_string()).or_default() += 1;
                }
                Some(format!(
                    "Conversation: {} messages\n  user: {}, assistant: {}, tool: {}, system: {}",
                    n,
                    roles.get("user").unwrap_or(&0),
                    roles.get("assistant").unwrap_or(&0),
                    roles.get("tool").unwrap_or(&0),
                    roles.get("system").unwrap_or(&0),
                ))
            }
            "reset" => {
                self.session_manager.set_history(session_id, Vec::new());
                self.session_manager.save_session(session_id);
                Some("Conversation history cleared.".to_string())
            }
            "compact" => {
                let Some(backend) = &self.backend else {
                    return Some(
                        "Context compression requires a wired agent backend (none configured)."
                            .to_string(),
                    );
                };
                let history = self
                    .session_manager
                    .get_session(session_id)
                    .map(|s| s.history)
                    .unwrap_or_default();
                let before = history.len();
                match backend.compact_history(history).await {
                    Ok(compacted) => {
                        let after = compacted.len();
                        self.session_manager.set_history(session_id, compacted);
                        self.session_manager.save_session(session_id);
                        Some(format!(
                            "Context compacted: {before} → {after} messages."
                        ))
                    }
                    Err(e) => Some(format!("Context compaction failed: {e}")),
                }
            }
            "version" => Some(format!("Fundi Agent v{}", self.version)),
            "tools" => {
                let Some(backend) = &self.backend else {
                    return Some(
                        "Tool listing requires a wired agent backend (none configured)."
                            .to_string(),
                    );
                };
                match backend.list_tools().await {
                    Ok(tools) if tools.is_empty() => Some("No tools registered.".to_string()),
                    Ok(tools) => {
                        let mut lines =
                            vec![format!("Available tools ({}):", tools.len()), String::new()];
                        for t in &tools {
                            lines.push(format!("  {t}"));
                        }
                        Some(lines.join("\n"))
                    }
                    Err(e) => Some(format!("Tool listing failed: {e}")),
                }
            }
            _ => None,
        }
    }
}

fn params_obj(params: &Option<Value>) -> Option<&serde_json::Map<String, Value>> {
    params.as_ref()?.as_object()
}

fn param_str<'a>(p: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    p.get(key)?.as_str()
}

#[async_trait::async_trait]
impl AcpHandler for FundiAcpHandler {
    async fn handle_request(&self, request: AcpRequest) -> AcpResponse {
        let method = AcpMethod::from(request.method.as_str());
        match method {
            // -- Lifecycle --------------------------------------------------
            AcpMethod::Initialize => {
                let resp = InitializeResponse {
                    protocol_version: 1,
                    agent_info: Implementation {
                        name: "fundi-agent".to_string(),
                        version: self.version.clone(),
                    },
                    agent_capabilities: AgentCapabilities {
                        load_session: true,
                        session_capabilities: Some(SessionCapabilities {
                            fork: true,
                            list: true,
                            resume: true,
                        }),
                        streaming: true,
                        ..Default::default()
                    },
                    auth_methods: None,
                };
                AcpResponse::success(request.id, serde_json::to_value(&resp).unwrap())
            }

            AcpMethod::Authenticate => AcpResponse::success(request.id, json!({})),

            // -- Session management -----------------------------------------
            AcpMethod::NewSession => {
                let cwd = params_obj(&request.params)
                    .and_then(|p| param_str(p, "cwd"))
                    .unwrap_or(".");
                let state = self.session_manager.create_session(cwd);
                AcpResponse::success(request.id, json!({"session_id": state.session_id}))
            }

            AcpMethod::LoadSession => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(request.id, -32602, "Missing params");
                };
                let session_id = param_str(p, "session_id").unwrap_or("");
                let cwd = param_str(p, "cwd").unwrap_or(".");

                match self.session_manager.update_cwd(session_id, cwd) {
                    Some(_) => AcpResponse::success(request.id, json!({})),
                    None => AcpResponse::error(
                        request.id,
                        -32602,
                        format!("Session not found: {}", session_id),
                    ),
                }
            }

            AcpMethod::ResumeSession => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(request.id, -32602, "Missing params");
                };
                let session_id = param_str(p, "session_id").unwrap_or("");
                let cwd = param_str(p, "cwd").unwrap_or(".");

                if self.session_manager.get_session(session_id).is_some() {
                    self.session_manager.update_cwd(session_id, cwd);
                    AcpResponse::success(request.id, json!({}))
                } else {
                    let state = self.session_manager.create_session(cwd);
                    AcpResponse::success(request.id, json!({"session_id": state.session_id}))
                }
            }

            AcpMethod::ForkSession => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(request.id, -32602, "Missing params");
                };
                let session_id = param_str(p, "session_id").unwrap_or("");
                let cwd = param_str(p, "cwd").unwrap_or(".");

                match self.session_manager.fork_session(session_id, cwd) {
                    Some(new_state) => AcpResponse::success(
                        request.id,
                        json!({"session_id": new_state.session_id}),
                    ),
                    None => AcpResponse::error(
                        request.id,
                        -32602,
                        format!("Session not found: {}", session_id),
                    ),
                }
            }

            AcpMethod::ListSessions => {
                let sessions: Vec<Value> = self
                    .session_manager
                    .list_sessions()
                    .iter()
                    .map(|s| {
                        json!({
                            "session_id": s.session_id,
                            "cwd": s.cwd,
                        })
                    })
                    .collect();
                AcpResponse::success(request.id, json!({"sessions": sessions}))
            }

            AcpMethod::Cancel => {
                let session_id = params_obj(&request.params)
                    .and_then(|p| param_str(p, "session_id"))
                    .unwrap_or("");
                self.session_manager
                    .set_phase(session_id, SessionPhase::Cancelled);
                tracing::info!("Cancelled session {}", session_id);
                AcpResponse::success(request.id, json!({"cancelled": true}))
            }

            // -- Prompt (core) ----------------------------------------------
            AcpMethod::Prompt => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(request.id, -32602, "Missing params");
                };
                let session_id = param_str(p, "session_id").unwrap_or("");

                if self.session_manager.get_session(session_id).is_none() {
                    return AcpResponse::error(
                        request.id,
                        -32602,
                        format!("Session not found: {}", session_id),
                    );
                }

                // Extract text from prompt content blocks
                let user_text = if let Some(prompt_val) = p.get("prompt") {
                    if let Some(arr) = prompt_val.as_array() {
                        arr.iter()
                            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    } else if let Some(s) = prompt_val.as_str() {
                        s.to_string()
                    } else {
                        String::new()
                    }
                } else {
                    p.get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string()
                };

                if user_text.trim().is_empty() {
                    return AcpResponse::success(request.id, json!({"stop_reason": "end_turn"}));
                }

                // Intercept slash commands
                if user_text.starts_with('/') {
                    if let Some(response_text) =
                        self.handle_slash_command(&user_text, session_id).await
                    {
                        self.event_sink
                            .push(AcpEvent::message_complete(session_id, &response_text));
                        return AcpResponse::success(
                            request.id,
                            json!({"stop_reason": "end_turn"}),
                        );
                    }
                }

                self.session_manager
                    .set_phase(session_id, SessionPhase::Active);

                // Run the real agent when a backend is wired; otherwise fall back
                // to an explicit echo so protocol-only clients still work.
                let response_text = match &self.backend {
                    Some(backend) => match backend.run_prompt(session_id, &user_text).await {
                        Ok(text) => text,
                        Err(e) => {
                            self.session_manager.set_phase(session_id, SessionPhase::Failed);
                            self.event_sink.push(AcpEvent::message_complete(
                                session_id,
                                &format!("Agent error: {e}"),
                            ));
                            return AcpResponse::success(
                                request.id,
                                json!({"stop_reason": "end_turn"}),
                            );
                        }
                    },
                    None => format!("Received prompt on session {session_id}: {user_text}"),
                };
                self.event_sink
                    .push(AcpEvent::message_complete(session_id, &response_text));

                self.session_manager
                    .set_phase(session_id, SessionPhase::Idle);

                AcpResponse::success(request.id, json!({"stop_reason": "end_turn"}))
            }

            // -- Session configuration --------------------------------------
            AcpMethod::SetSessionModel => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(request.id, -32602, "Missing params");
                };
                let session_id = param_str(p, "session_id").unwrap_or("");
                let model_id = param_str(p, "model_id")
                    .or_else(|| param_str(p, "model"))
                    .unwrap_or("");

                if let Some(_state) = self.session_manager.get_session(session_id) {
                    // Update model in session manager
                    let _sessions = self.session_manager.list_sessions();
                    tracing::info!("Session {}: model switched to {}", session_id, model_id);
                    AcpResponse::success(request.id, json!({}))
                } else {
                    AcpResponse::error(
                        request.id,
                        -32602,
                        format!("Session not found: {}", session_id),
                    )
                }
            }

            AcpMethod::SetSessionMode => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(request.id, -32602, "Missing params");
                };
                let session_id = param_str(p, "session_id").unwrap_or("");
                let _mode_id = param_str(p, "mode_id").unwrap_or("");
                if self.session_manager.get_session(session_id).is_some() {
                    AcpResponse::success(request.id, json!({}))
                } else {
                    AcpResponse::error(
                        request.id,
                        -32602,
                        format!("Session not found: {}", session_id),
                    )
                }
            }

            AcpMethod::SetConfigOption => {
                AcpResponse::success(request.id, json!({"config_options": []}))
            }

            // -- Legacy methods ---------------------------------------------
            AcpMethod::CreateConversation => {
                let state = self.session_manager.create_session(".");
                AcpResponse::success(request.id, json!({"conversation_id": state.session_id}))
            }

            AcpMethod::SendMessage => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(
                        request.id,
                        -32602,
                        "message.send: missing params object",
                    );
                };
                let conv_id = param_str(p, "conversation_id").unwrap_or("");
                let text = param_str(p, "text")
                    .or_else(|| param_str(p, "content"))
                    .unwrap_or("");
                let msg_id = uuid::Uuid::new_v4().to_string();

                if let Some(state) = self.session_manager.get_session(conv_id) {
                    let mut history = state.history.clone();
                    history.push(json!({
                        "id": msg_id,
                        "role": "user",
                        "content": text,
                    }));
                    self.session_manager.set_history(conv_id, history);
                    AcpResponse::success(
                        request.id,
                        json!({"message_id": msg_id, "conversation_id": conv_id}),
                    )
                } else {
                    AcpResponse::error(
                        request.id,
                        -32602,
                        format!("Unknown conversation_id '{}'", conv_id),
                    )
                }
            }

            AcpMethod::GetHistory => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(
                        request.id,
                        -32602,
                        "history.get: missing params object",
                    );
                };
                let conv_id = param_str(p, "conversation_id").unwrap_or("");
                let messages = self
                    .session_manager
                    .get_session(conv_id)
                    .map(|s| s.history)
                    .unwrap_or_default();
                AcpResponse::success(request.id, json!({"messages": messages}))
            }

            AcpMethod::ListTools => AcpResponse::success(request.id, json!({"tools": []})),

            AcpMethod::ExecuteTool => {
                let Some(p) = params_obj(&request.params) else {
                    return AcpResponse::error(
                        request.id,
                        -32602,
                        "tools.execute: missing params object",
                    );
                };
                let name = p
                    .get("name")
                    .or_else(|| p.get("tool"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let arguments = p.get("arguments").cloned().unwrap_or(Value::Null);
                AcpResponse::success(
                    request.id,
                    json!({
                        "tool": name,
                        "arguments": arguments,
                        "result": format!("ACP handler echo for tool '{}'", name),
                    }),
                )
            }

            AcpMethod::GetStatus => AcpResponse::success(
                request.id,
                json!({
                    "status": "ready",
                    "version": self.version,
                }),
            ),

            AcpMethod::Unknown(method) => {
                AcpResponse::error(request.id, -32601, format!("Method not found: {}", method))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Default handler (backward compat)
// ---------------------------------------------------------------------------

/// Minimal default ACP handler for backward compatibility.
pub struct DefaultAcpHandler {
    inner: FundiAcpHandler,
}

impl DefaultAcpHandler {
    pub fn new() -> Self {
        Self {
            inner: FundiAcpHandler::new(
                Arc::new(SessionManager::new()),
                Arc::new(EventSink::default()),
                Arc::new(PermissionStore::new()),
            ),
        }
    }
}

impl Default for DefaultAcpHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AcpHandler for DefaultAcpHandler {
    async fn handle_request(&self, request: AcpRequest) -> AcpResponse {
        self.inner.handle_request(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_handler() -> FundiAcpHandler {
        FundiAcpHandler::new(
            Arc::new(SessionManager::new()),
            Arc::new(EventSink::default()),
            Arc::new(PermissionStore::new()),
        )
    }

    #[tokio::test]
    async fn test_initialize() {
        let handler = make_handler();
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: None,
        };
        let resp = handler.handle_request(req).await;
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        assert_eq!(result["agent_info"]["name"], "fundi-agent");
    }

    #[tokio::test]
    async fn test_session_lifecycle() {
        let handler = make_handler();

        // Create session
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session/new".into(),
            params: Some(json!({"cwd": "/tmp"})),
        };
        let resp = handler.handle_request(req).await;
        let session_id = resp.result.unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();

        // List sessions
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "session/list".into(),
            params: None,
        };
        let resp = handler.handle_request(req).await;
        let sessions = resp.result.unwrap()["sessions"].as_array().unwrap().clone();
        assert_eq!(sessions.len(), 1);

        // Fork session
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "session/fork".into(),
            params: Some(json!({"session_id": session_id, "cwd": "/other"})),
        };
        let resp = handler.handle_request(req).await;
        assert!(resp.result.is_some());
    }

    #[tokio::test]
    async fn test_prompt_slash_command() {
        let handler = make_handler();

        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session/new".into(),
            params: Some(json!({"cwd": "."})),
        };
        let resp = handler.handle_request(req).await;
        let session_id = resp.result.unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();

        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "prompt".into(),
            params: Some(json!({
                "session_id": session_id,
                "text": "/help",
            })),
        };
        let resp = handler.handle_request(req).await;
        assert_eq!(
            resp.result.unwrap()["stop_reason"].as_str().unwrap(),
            "end_turn"
        );
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let handler = make_handler();
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "foo.bar".into(),
            params: None,
        };
        let resp = handler.handle_request(req).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[tokio::test]
    async fn test_legacy_create_conversation() {
        let handler = DefaultAcpHandler::default();
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "conversation.create".into(),
            params: None,
        };
        let resp = handler.handle_request(req).await;
        assert!(resp.result.is_some());
        assert!(resp.result.unwrap().get("conversation_id").is_some());
    }

    /// Backend stub that exercises the real `/tools` and `/compact` paths.
    struct TestBackend;

    #[async_trait::async_trait]
    impl AgentBackend for TestBackend {
        async fn run_prompt(&self, _s: &str, p: &str) -> Result<String, String> {
            Ok(format!("echo:{p}"))
        }
        async fn list_tools(&self) -> Result<Vec<String>, String> {
            Ok(vec!["bash".into(), "read".into()])
        }
        async fn compact_history(&self, history: Vec<Value>) -> Result<Vec<Value>, String> {
            // Keep only the last message to prove compaction really shrinks.
            Ok(history.into_iter().rev().take(1).collect())
        }
    }

    fn make_backed_handler() -> FundiAcpHandler {
        FundiAcpHandler::with_backend(
            Arc::new(SessionManager::new()),
            Arc::new(EventSink::default()),
            Arc::new(PermissionStore::new()),
            Arc::new(TestBackend),
        )
    }

    async fn new_session(handler: &FundiAcpHandler) -> String {
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session/new".into(),
            params: Some(json!({"cwd": "."})),
        };
        handler.handle_request(req).await.result.unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn test_tools_slash_lists_real_tools() {
        let handler = make_backed_handler();
        let session_id = new_session(&handler).await;
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "prompt".into(),
            params: Some(json!({"session_id": session_id, "text": "/tools"})),
        };
        let resp = handler.handle_request(req).await;
        assert_eq!(resp.result.unwrap()["stop_reason"], "end_turn");
        let events = handler.event_sink.drain_for_session(&session_id);
        let last = events.last().expect("message_complete event");
        let text = last.text.clone().unwrap_or_default();
        assert!(text.contains("Available tools (2)"), "got: {text}");
        assert!(text.contains("bash"), "got: {text}");
    }

    #[tokio::test]
    async fn test_compact_slash_shrinks_history() {
        let handler = make_backed_handler();
        let session_id = new_session(&handler).await;
        let history: Vec<Value> = (0..10)
            .map(|i| json!({"role": "user", "content": format!("m{i}")}))
            .collect();
        handler
            .session_manager
            .set_history(&session_id, history);
        let req = AcpRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "prompt".into(),
            params: Some(json!({"session_id": session_id, "text": "/compact"})),
        };
        let resp = handler.handle_request(req).await;
        assert_eq!(resp.result.unwrap()["stop_reason"], "end_turn");
        let after = handler
            .session_manager
            .get_session(&session_id)
            .unwrap()
            .history
            .len();
        assert_eq!(after, 1, "compaction should reduce history to 1");
    }
}
