use fundi_core::traits::AgentOverrides;
use fundi_core::{Message, StreamChunk};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BusMessage {
    AgentRequest(AgentRequest),
    AgentStreamChunk(AgentStreamChunk),
    AgentResponse(AgentResponse),
    PlatformIncoming(PlatformIncoming),
    PlatformOutgoing(PlatformOutgoing),
    SessionQuery(SessionQuery),
    SessionResponse(SessionResponse),
    CronTrigger(CronTrigger),
    StatusQuery(StatusQuery),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRequest {
    pub request_id: String,
    pub session_id: String,
    pub text: String,
    pub overrides: AgentOverrides,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentStreamChunk {
    pub request_id: String,
    pub session_id: String,
    pub chunk: StreamChunk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentResponse {
    pub request_id: String,
    pub session_id: String,
    pub text: String,
    pub message_count: usize,
    pub error: Option<String>,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformIncoming {
    pub platform: String,
    pub chat_id: String,
    pub user_id: String,
    pub text: String,
    pub message_id: Option<String>,
    pub is_dm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformOutgoing {
    pub platform: String,
    pub chat_id: String,
    pub text: String,
    pub parse_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionQuery {
    pub request_id: String,
    pub session_id: String,
    pub action: SessionQueryAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionQueryAction {
    GetMessages,
    ResetSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionResponse {
    pub request_id: String,
    #[serde(default)]
    pub sessions: Vec<SessionSummary>,
    pub total: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSummary {
    pub id: String,
    pub model: Option<String>,
    pub platform: Option<String>,
    pub title: Option<String>,
    pub message_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CronTrigger {
    pub job_id: String,
    pub prompt: String,
    pub scheduled_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatusQuery {
    pub include_platforms: bool,
    pub include_sessions: bool,
}
