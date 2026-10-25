//! Real agent backend for the ACP adapter.
//!
//! Wires the ACP `prompt` method to the actual Fundi agent loop (LLM provider +
//! registered tools) so external orchestrators drive production behaviour
//! instead of the echo placeholder in `FundiAcpHandler`'s default mode.

use std::sync::Arc;

use fundi_acp::AgentBackend;
use fundi_core::{AgentError, Message};

/// Runs ACP prompts through a fresh [`fundi_agent::AgentLoop`] per session.
pub struct FundiAgentBackend {
    config: fundi_config::GatewayConfig,
    model: String,
}

impl FundiAgentBackend {
    pub fn new(config: fundi_config::GatewayConfig, model: String) -> Self {
        Self { config, model }
    }
}

/// Tools registered for an agent turn (builtin registry).
fn build_tool_registry() -> Arc<fundi_tools::ToolRegistry> {
    use fundi_environments::LocalBackend;
    use fundi_skills::{FileSkillStore, SkillManager};

    let registry = Arc::new(fundi_tools::ToolRegistry::new());
    let terminal_backend: Arc<dyn fundi_core::TerminalBackend> = Arc::new(LocalBackend::default());
    let skill_store = Arc::new(FileSkillStore::new(FileSkillStore::default_dir()));
    let skill_provider: Arc<dyn fundi_core::SkillProvider> =
        Arc::new(SkillManager::new(skill_store));
    fundi_tools::register_builtin_tools(&registry, terminal_backend, skill_provider);
    registry
}

/// Number of trailing messages preserved verbatim by `/compact`.
const COMPACT_KEEP_RECENT: usize = 6;
/// Max chars of a single compacted message body.
const COMPACT_MAX_CHARS: usize = 400;

#[async_trait::async_trait]
impl AgentBackend for FundiAgentBackend {
    async fn run_prompt(&self, session_id: &str, prompt: &str) -> Result<String, String> {
        crate::agent_runner::run_agent_turn(
            &self.config,
            &self.model,
            "acp",
            session_id,
            vec![Message::user(prompt)],
        )
        .await
    }

    async fn list_tools(&self) -> Result<Vec<String>, String> {
        let registry = build_tool_registry();
        let mut names: Vec<String> = registry
            .list_tools()
            .into_iter()
            .map(|t| t.name)
            .collect();
        names.sort();
        Ok(names)
    }

    async fn compact_history(&self, history: Vec<serde_json::Value>) -> Result<Vec<serde_json::Value>, String> {
        Ok(compact_values(&history))
    }
}

/// Compact a conversation history deterministically.
///
/// Keeps any leading system message, folds the middle into a single summary
/// system message, and preserves the most recent [`COMPACT_KEEP_RECENT`]
/// messages verbatim. Token-free (no LLM call) and idempotent enough for a
/// context-window trim.
pub fn compact_values(history: &[serde_json::Value]) -> Vec<serde_json::Value> {
    if history.len() <= COMPACT_KEEP_RECENT + 1 {
        return history.to_vec();
    }
    let mut out: Vec<serde_json::Value> = Vec::new();
    let mut start = 0;
    if history
        .first()
        .and_then(|m| m.get("role"))
        .and_then(|r| r.as_str())
        == Some("system")
    {
        out.push(history[0].clone());
        start = 1;
    }
    let keep_from = history.len().saturating_sub(COMPACT_KEEP_RECENT);
    let folded = &history[start..keep_from];
    let mut roles: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut excerpt = String::new();
    for m in folded {
        let role = m
            .get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown")
            .to_string();
        *roles.entry(role).or_default() += 1;
        if let Some(c) = m.get("content").and_then(|c| c.as_str()) {
            if excerpt.len() < COMPACT_MAX_CHARS {
                let remaining = COMPACT_MAX_CHARS - excerpt.len();
                let take: String = c.chars().take(remaining).collect();
                excerpt.push_str(&take);
                excerpt.push(' ');
            }
        }
    }
    let summary = serde_json::json!({
        "role": "system",
        "content": format!(
            "[compacted {} earlier messages: {}] {}{}",
            folded.len(),
            roles
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join(", "),
            excerpt.trim(),
            if excerpt.len() >= COMPACT_MAX_CHARS { "…" } else { "" }
        ),
    });
    out.push(summary);
    out.extend_from_slice(&history[keep_from..]);
    out
}

/// Build an ACP backend from the loaded gateway config.
pub fn build_agent_backend(
    config: &fundi_config::GatewayConfig,
    model: &str,
) -> Result<Arc<dyn AgentBackend>, AgentError> {
    Ok(Arc::new(FundiAgentBackend::new(config.clone(), model.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compact_is_noop_for_short_history() {
        let history: Vec<serde_json::Value> = (0..3)
            .map(|i| json!({"role": "user", "content": format!("m{i}")}))
            .collect();
        let out = compact_values(&history);
        assert_eq!(out.len(), 3);
        assert_eq!(out, history);
    }

    #[test]
    fn compact_preserves_system_and_recent_messages() {
        let mut history = vec![json!({"role": "system", "content": "SYS"})];
        for i in 0..12 {
            history.push(json!({"role": "user", "content": format!("msg-{i}")}));
        }
        let out = compact_values(&history);
        // leading system + summary + last COMPACT_KEEP_RECENT
        assert_eq!(out.len(), 1 + 1 + COMPACT_KEEP_RECENT);
        assert_eq!(out[0]["content"], "SYS");
        assert_eq!(out[1]["role"], "system");
        assert!(out[1]["content"].as_str().unwrap().contains("compacted"));
        assert_eq!(
            out.last().unwrap()["content"],
            format!("msg-{}", 11)
        );
        assert!(out.len() < history.len());
    }

    #[test]
    fn compact_without_system_still_shrinks() {
        let history: Vec<serde_json::Value> = (0..20)
            .map(|i| json!({"role": "assistant", "content": format!("a{i}")}))
            .collect();
        let out = compact_values(&history);
        assert_eq!(out.len(), 1 + COMPACT_KEEP_RECENT);
        assert!(out.len() < history.len());
    }
}
