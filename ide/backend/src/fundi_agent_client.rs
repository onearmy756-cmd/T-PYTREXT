//! fundi_agent_client.rs — REST client ya Fundi Server (binary fundi serve, port 8787).
//!
//! Inatumia routes halisi za `crates/fundi-server/src/lib.rs`:
//!   - GET  /health
//!   - GET  /v1/sessions                  (ListSessionsResponse)
//!   - POST /v1/sessions                  (CreateSessionRequest -> SessionSummary)
//!   - POST /v1/sessions/{id}/messages    (SendMessageRequest -> SendMessageResponse)
//!   - GET  /v1/sessions/{id}/messages    (SessionMessagesResponse)
//!   - DELETE /v1/sessions/{id}
//!   - POST /v1/commands
//!
//! Environment:
//!   - FUNDI_AGENT_ADDR: base URL (default http://127.0.0.1:8787)
//!   - FUNDI_AGENT_TOKEN: optional bearer token
//!   - FUNDI_AGENT_ENABLED: 1/0 (default 1) — ukiweka 0, bridge haiwezi kufanya kazi.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundiAgentConfig {
    pub base_url: String,
    pub token: Option<String>,
    pub enabled: bool,
    pub timeout_secs: u64,
}

impl Default for FundiAgentConfig {
    fn default() -> Self {
        let base_url = std::env::var("FUNDI_AGENT_ADDR")
            .or_else(|_| std::env::var("FUNDI_SERVER_ADDR"))
            .or_else(|_| std::env::var("FUNDI_DASHBOARD_ADDR"))
            .unwrap_or_else(|_| "http://127.0.0.1:8787".to_string());
        let token = std::env::var("FUNDI_AGENT_TOKEN")
            .or_else(|_| std::env::var("FUNDI_SERVER_TOKEN"))
            .ok()
            .filter(|s| !s.trim().is_empty());
        let enabled = std::env::var("FUNDI_AGENT_ENABLED")
            .map(|v| v != "0" && v.to_ascii_lowercase() != "false")
            .unwrap_or(true);
        let timeout_secs = std::env::var("FUNDI_AGENT_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(180);
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            enabled,
            timeout_secs,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FundiAgentError {
    #[error("fundi-agent disabled")]
    Disabled,
    #[error("http: {0}")]
    Http(String),
    #[error("decode: {0}")]
    Decode(String),
    #[error("timeout")]
    Timeout,
}

impl From<reqwest::Error> for FundiAgentError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            FundiAgentError::Timeout
        } else {
            FundiAgentError::Http(e.to_string())
        }
    }
}

// -----------------------------------------------------------------
// Wire types (matching fundi_transport schema)
// -----------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    #[serde(default)]
    pub total: usize,
    #[serde(default)]
    pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateSessionRequest<'a> {
    #[serde(default)]
    pub title: &'a str,
    #[serde(default)]
    pub project: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest<'a> {
    pub text: &'a str,
    #[serde(default)]
    pub model: Option<&'a str>,
    #[serde(default)]
    pub provider: Option<&'a str>,
    #[serde(default)]
    pub personality: Option<&'a str>,
    #[serde(default)]
    pub user_id: Option<&'a str>,
    #[serde(default)]
    pub request_id: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub session_id: String,
    pub reply: String,
    #[serde(default)]
    pub message_count: usize,
    #[serde(default)]
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub id: String,
    #[serde(rename = "role")]
    pub role_raw: serde_json::Value,
    pub content: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessagesResponse {
    pub session_id: String,
    #[serde(default)]
    pub messages: Vec<ProtocolMessage>,
}

#[derive(Clone)]
pub struct FundiAgentClient {
    cfg: FundiAgentConfig,
    http: reqwest::Client,
}

impl FundiAgentClient {
    pub fn new(cfg: FundiAgentConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { cfg, http }
    }

    pub fn from_env() -> Self {
        Self::new(FundiAgentConfig::default())
    }

    pub fn config(&self) -> &FundiAgentConfig {
        &self.cfg
    }

    pub fn enabled(&self) -> bool {
        self.cfg.enabled
    }

    fn apply_auth(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(t) = &self.cfg.token {
            req = req.bearer_auth(t);
        }
        req
    }

    async fn post_json<B: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, FundiAgentError> {
        if !self.cfg.enabled {
            return Err(FundiAgentError::Disabled);
        }
        let url = format!("{}{}", self.cfg.base_url, path);
        let req = self.apply_auth(
            self.http
                .post(&url)
                .header("Content-Type", "application/json")
                .json(body),
        );
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(FundiAgentError::Http(format!("HTTP {}: {}", status, text)));
        }
        let bytes = resp.bytes().await?;
        serde_json::from_slice(&bytes).map_err(|e| FundiAgentError::Decode(e.to_string()))
    }

    async fn get_json<R: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<R, FundiAgentError> {
        if !self.cfg.enabled {
            return Err(FundiAgentError::Disabled);
        }
        let url = format!("{}{}", self.cfg.base_url, path);
        let resp = self.apply_auth(self.http.get(&url)).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(FundiAgentError::Http(format!("HTTP {}: {}", status, text)));
        }
        let bytes = resp.bytes().await?;
        serde_json::from_slice(&bytes).map_err(|e| FundiAgentError::Decode(e.to_string()))
    }

    pub async fn health(&self) -> Result<serde_json::Value, FundiAgentError> {
        self.get_json::<serde_json::Value>("/health").await
    }

    /// ITA ZANA MOJA HALISI ya fundi-server (`/v1/tools/invoke`) — image_gen,
    /// vision, tts, terminal, browser n.k. Hii ni lango la agent kwa uwezo wote
    /// wa registry (tools 60) bila kuingia kwenye chat loop.
    pub async fn invoke_tool_raw(
        &self,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, FundiAgentError> {
        self.post_json::<serde_json::Value, serde_json::Value>("/v1/tools/invoke", &body)
            .await
    }

    pub async fn list_sessions(&self) -> Result<ListSessionsResponse, FundiAgentError> {
        self.get_json::<ListSessionsResponse>("/v1/sessions").await
    }

    pub async fn create_session(&self, title: &str) -> Result<SessionSummary, FundiAgentError> {
        let req = CreateSessionRequest {
            title: if title.is_empty() {
                "MTECH OS Fundi Deploy chat"
            } else {
                title
            },
            project: None,
        };
        self.post_json::<CreateSessionRequest, SessionSummary>("/v1/sessions", &req)
            .await
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), FundiAgentError> {
        if !self.cfg.enabled {
            return Err(FundiAgentError::Disabled);
        }
        let url = format!("{}/v1/sessions/{}", self.cfg.base_url, id);
        let resp = self.apply_auth(self.http.delete(&url)).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(FundiAgentError::Http(format!("HTTP {}: {}", status, text)));
        }
        Ok(())
    }

    pub async fn list_messages(
        &self,
        session_id: &str,
    ) -> Result<SessionMessagesResponse, FundiAgentError> {
        self.get_json::<SessionMessagesResponse>(&format!("/v1/sessions/{}/messages", session_id))
            .await
    }

    pub async fn send_message(
        &self,
        session_id: &str,
        text: &str,
        opts: SendMessageOptions<'_>,
    ) -> Result<SendMessageResponse, FundiAgentError> {
        let req = SendMessageRequest {
            text,
            model: opts.model,
            provider: opts.provider,
            personality: opts.personality,
            user_id: opts.user_id,
            request_id: opts.request_id,
        };
        self.post_json::<SendMessageRequest, SendMessageResponse>(
            &format!("/v1/sessions/{}/messages", session_id),
            &req,
        )
        .await
    }

    /// High-level: create session lazily (au reuse ipo), tuma ujumbe, rudi reply.
    pub async fn chat(
        &self,
        existing_session_id: Option<&str>,
        title: &str,
        text: &str,
        opts: SendMessageOptions<'_>,
    ) -> Result<ChatResult, FundiAgentError> {
        let session_id = match existing_session_id {
            Some(id) if !id.is_empty() => {
                // Jaribu ku-list na ku-check ipo; ikiwa haipo, create.
                let mut reuse = None;
                if let Ok(list) = self.list_sessions().await {
                    for s in list.sessions {
                        if s.id == id {
                            reuse = Some(s.id);
                            break;
                        }
                    }
                }
                if let Some(rid) = reuse {
                    rid
                } else {
                    self.create_session(title).await?.id
                }
            }
            _ => self.create_session(title).await?.id,
        };
        let r = self.send_message(&session_id, text, opts).await?;
        Ok(ChatResult {
            session_id: r.session_id.clone(),
            reply: r.reply,
            message_count: r.message_count,
            trace_id: r.trace_id,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct SendMessageOptions<'a> {
    pub model: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub personality: Option<&'a str>,
    pub user_id: Option<&'a str>,
    pub request_id: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResult {
    pub session_id: String,
    pub reply: String,
    #[serde(default)]
    pub message_count: usize,
    #[serde(default)]
    pub trace_id: Option<String>,
}
