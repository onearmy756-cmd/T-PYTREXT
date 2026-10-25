//! [`TaskRollout`](crate::runner::TaskRollout) implementation backed by [`fundi_agent::AgentLoop`].
//!
//! Enable with `--features agent-loop`. Callers construct [`AgentLoop`](fundi_agent::AgentLoop)
//! the same way as `fundi-cli` (config, tool registry, provider), wrap in [`std::sync::Arc`],
//! then pass [`AgentLoopRollout`] to [`Runner::run`](crate::runner::Runner::run).
//!
//! ```ignore
//! use fundi_eval::runner::{Runner, RunnerConfig};
//! use fundi_eval::agent_rollout::AgentLoopRollout;
//! use std::sync::Arc;
//!
//! let agent: Arc<fundi_agent::AgentLoop> = /* from CLI-style setup */;
//! let rollout = Arc::new(AgentLoopRollout::new(agent));
//! let record = Runner::new(RunnerConfig::default())
//!     .run(Arc::new(adapter), rollout)
//!     .await?;
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use fundi_agent::AgentLoop;
use fundi_core::Message;
use serde_json::{json, Value};

use crate::adapter::TaskSpec;
use crate::error::{EvalError, EvalResult};
use crate::runner::TaskRollout;

/// Drives [`AgentLoop::run`](fundi_agent::AgentLoop::run) once per eval task (user message = task instruction).
#[derive(Clone)]
pub struct AgentLoopRollout {
    agent: Arc<AgentLoop>,
}

impl AgentLoopRollout {
    pub fn new(agent: Arc<AgentLoop>) -> Self {
        Self { agent }
    }

    /// Build the JSON blob passed to the benchmark [`Verifier`](crate::verifier::Verifier).
    ///
    /// Default: compact summary to keep artifacts small; set `full` to embed the whole
    /// [`fundi_core::AgentResult`] (can be very large).
    pub fn summarize_result(
        result: &fundi_core::AgentResult,
        task_id: &str,
        full: bool,
    ) -> EvalResult<Value> {
        if full {
            return serde_json::to_value(result).map_err(EvalError::Serde);
        }
        let last_assistant = result
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, fundi_core::MessageRole::Assistant))
            .and_then(|m| m.content.clone())
            .unwrap_or_default();
        let preview: String = last_assistant.chars().take(2000).collect();
        Ok(json!({
            "task_id": task_id,
            "finished_naturally": result.finished_naturally,
            "total_turns": result.total_turns,
            "message_count": result.messages.len(),
            "tool_error_count": result.tool_errors.len(),
            "usage": result.usage,
            "last_assistant_preview": preview,
        }))
    }
}

#[async_trait]
impl TaskRollout for AgentLoopRollout {
    async fn execute(&self, task: &TaskSpec) -> EvalResult<Value> {
        let messages = vec![Message::user(task.instruction.clone())];
        let result = self
            .agent
            .run(messages, None)
            .await
            .map_err(|e| EvalError::TaskExecution(e.to_string()))?;
        Self::summarize_result(&result, &task.task_id, false)
    }
}
