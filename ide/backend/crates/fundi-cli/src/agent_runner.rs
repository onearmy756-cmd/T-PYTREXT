//! Shared helper that runs one agent turn (LLM provider + builtin tools).
//!
//! Used by both the ACP adapter and the OpenAI-compatible API server so every
//! inbound surface executes the real Fundi agent loop instead of an echo stub.

use std::sync::Arc;

use fundi_core::{Message, MessageRole};

use crate::app;

/// Run the agent loop for `messages` and return the final assistant text.
///
/// `platform` tags the run (e.g. `"acp"`, `"api-server"`).
pub async fn run_agent_turn(
    config: &fundi_config::GatewayConfig,
    model: &str,
    platform: &str,
    session_id: &str,
    messages: Vec<Message>,
) -> Result<String, String> {
    use fundi_environments::LocalBackend;
    use fundi_skills::{FileSkillStore, SkillManager};

    let tool_registry = Arc::new(fundi_tools::ToolRegistry::new());
    let terminal_backend: Arc<dyn fundi_core::TerminalBackend> = Arc::new(LocalBackend::default());
    let skill_store = Arc::new(FileSkillStore::new(FileSkillStore::default_dir()));
    let skill_provider: Arc<dyn fundi_core::SkillProvider> =
        Arc::new(SkillManager::new(skill_store));
    fundi_tools::register_builtin_tools(&tool_registry, terminal_backend, skill_provider);
    let _ = crate::live_messaging::enable_live_messaging_tool(config, &tool_registry).await;
    let agent_tool_registry = Arc::new(app::bridge_tool_registry(&tool_registry));

    let mut agent_config = app::build_agent_config(config, model, Some(platform));
    agent_config.session_id = Some(session_id.to_string());
    let provider = app::build_provider(config, model);

    let agent = fundi_agent::AgentLoop::new(agent_config, agent_tool_registry, provider);
    let result = agent
        .run(messages, None)
        .await
        .map_err(|e| format!("agent loop: {e}"))?;

    Ok(result
        .messages
        .iter()
        .rev()
        .find_map(|m| {
            if m.role == MessageRole::Assistant {
                m.content.clone()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "(no assistant reply)".to_string()))
}

/// Map an OpenAI-style role string to a [`MessageRole`].
pub fn role_from_str(role: &str) -> MessageRole {
    match role {
        "system" => MessageRole::System,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        _ => MessageRole::User,
    }
}

/// [`ApiAgentRunner`](fundi_gateway::platforms::api_server::ApiAgentRunner) impl
/// backing the OpenAI-compatible `api_server` adapter with the real agent loop.
pub struct GatewayApiAgentRunner {
    config: fundi_config::GatewayConfig,
    model: String,
}

impl GatewayApiAgentRunner {
    pub fn new(config: fundi_config::GatewayConfig, model: String) -> Self {
        Self { config, model }
    }
}

#[async_trait::async_trait]
impl fundi_gateway::platforms::api_server::ApiAgentRunner for GatewayApiAgentRunner {
    async fn run(&self, messages: Vec<(String, String)>) -> Result<String, String> {
        let msgs: Vec<Message> = messages
            .iter()
            .map(|(role, content)| Message {
                role: role_from_str(role),
                content: Some(content.clone()),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                reasoning_content: None,
                cache_control: None,
            })
            .collect();
        run_agent_turn(
            &self.config,
            &self.model,
            "api-server",
            "api-server",
            msgs,
        )
        .await
    }
}
