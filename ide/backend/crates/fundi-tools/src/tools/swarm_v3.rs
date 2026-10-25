//! Swarm V3 tools: REST client for the MTECHOS core (:8080) so Fundi Agent can
//! drive the second vision layer — Warp-Cortex, Orthogonal insight, Remind cycle,
//! Agent descent, 6-tier memory, and MegaFlow.
//!
//! Chanzo kimoja: zana hizi hazina mantiki ya maono; hupiga tu `/api/swarm/v3/*`
//! kwenye Fundi Deploy core (swarm_v3.rs). Hivyo AGENT (Devin-like) na UI
//! hutumia injini ile ile (hakuna mfumo wa pili).
//!
//! Environment:
//!   - FUNDI_DEPLOY_URL (default http://127.0.0.1:8080)
//!   - FUNDI_DEPLOY_TOKEN (optional bearer token)

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::time::Duration;

use fundi_core::{tool_schema, JsonSchema, ToolError, ToolHandler, ToolSchema};

fn base_url() -> String {
    std::env::var("FUNDI_DEPLOY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn token() -> Option<String> {
    std::env::var("FUNDI_DEPLOY_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn parse(b: &[u8]) -> Result<String, ToolError> {
    match serde_json::from_slice::<Value>(b) {
        Ok(v) => serde_json::to_string_pretty(&v)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string())),
        Err(_) => String::from_utf8(b.to_vec())
            .map_err(|e| ToolError::ExecutionFailed(e.to_string())),
    }
}

async fn get_json(path: &str) -> Result<String, ToolError> {
    let url = format!("{}{}", base_url(), path);
    let mut req = client().get(&url);
    if let Some(t) = token() {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    if !status.is_success() {
        return Err(ToolError::ExecutionFailed(format!(
            "HTTP {}: {}",
            status,
            String::from_utf8_lossy(&bytes)
        )));
    }
    parse(&bytes)
}

async fn post_json(path: &str, body: &Value) -> Result<String, ToolError> {
    let url = format!("{}{}", base_url(), path);
    let mut req = client()
        .post(&url)
        .header("Content-Type", "application/json")
        .json(body);
    if let Some(t) = token() {
        req = req.bearer_auth(t);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    if !status.is_success() {
        return Err(ToolError::ExecutionFailed(format!(
            "HTTP {}: {}",
            status,
            String::from_utf8_lossy(&bytes)
        )));
    }
    parse(&bytes)
}

fn obj(props: Vec<(&str, Value)>, required: Vec<String>) -> JsonSchema {
    let mut m = IndexMap::new();
    for (k, v) in props {
        m.insert(k.to_string(), v);
    }
    JsonSchema::object(m, required)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub struct SwarmV3StatusHandler;
#[async_trait]
impl ToolHandler for SwarmV3StatusHandler {
    async fn execute(&self, _p: Value) -> Result<String, ToolError> {
        get_json("/api/swarm/v3/status").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "swarm_v3_status",
            "Show Swarm V3 engine status (Warp-Cortex, Orthogonal, AgentDescent, 6-tier memory, MegaFlow) — halisi kutoka core.",
            obj(vec![], vec![]),
        )
    }
}

pub struct SwarmV3CortexHandler;
#[async_trait]
impl ToolHandler for SwarmV3CortexHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        post_json("/api/swarm/v3/cortex", &p).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "warp_cortex_plan",
            "Warp-Cortex: plan massively parallel agent work (thousands of agents). Returns an execution plan.",
            obj(
                vec![
                    ("agents", json!({"type":"integer","description":"Idadi ya agents (default 100; mf. 1000)"})),
                    ("sweep", json!({"type":"array","items":{"type":"integer"},"description":"Orodha ya idadi ya kuhesabu VRAM kwa pamoja (mf. [100,1000,10000])"})),
                ],
                vec![],
            ),
        )
    }
}

pub struct SwarmV3OrthogonalHandler;
#[async_trait]
impl ToolHandler for SwarmV3OrthogonalHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        post_json("/api/swarm/v3/orthogonal", &p).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "orthogonal_insight",
            "Orthogonal insight: generate an unusual, non-obvious angle on a problem (breaks local optima).",
            obj(
                vec![
                    ("task", json!({"type":"string","description":"Tatizo/mada ya kuchunguza kwa mtazamo mpya"})),
                    ("solvers", json!({"type":"integer","description":"Idadi ya wachambuzi (default 6)"})),
                ],
                vec!["task".into()],
            ),
        )
    }
}

pub struct SwarmV3RemindHandler;
#[async_trait]
impl ToolHandler for SwarmV3RemindHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        post_json("/api/swarm/v3/remind", &p).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "remind_cycle",
            "Remind cycle: spaced-repetition reminder scheduling for memory items.",
            obj(
                vec![
                    ("topic", json!({"type":"string","description":"Mada ya mzunguko wa ubunifu"})),
                    ("dreams", json!({"type":"integer","description":"Idadi ya mitazamo/ndoto (default 8)"})),
                ],
                vec!["topic".into()],
            ),
        )
    }
}

pub struct SwarmV3DescentHandler;
#[async_trait]
impl ToolHandler for SwarmV3DescentHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        post_json("/api/swarm/v3/descent", &p).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "agent_descent",
            "Agent descent: run gradient-style descent over agent proposals to converge on the best solution.",
            obj(
                vec![
                    ("task", json!({"type":"string"})),
                    ("proposers", json!({"type":"integer","description":"Idadi ya watoa mapendekezo (default 8)"})),
                    ("iterations", json!({"type":"integer","description":"Hatua za descent"})),
                ],
                vec!["task".into()],
            ),
        )
    }
}

pub struct SwarmV3MemoryWriteHandler;
#[async_trait]
impl ToolHandler for SwarmV3MemoryWriteHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        post_json("/api/swarm/v3/memory", &p).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "memory_write",
            "Write an item to 6-tier memory (levels: working/short/long/episodic/semantic/procedural).",
            obj(
                vec![
                    ("tier", json!({"type":"string","description":"working|episodic|semantic|procedural|emotional|meta"})),
                    ("key", json!({"type":"string","description":"Ufunguo wa kumbukumbu"})),
                    ("value", json!({"type":"string"})),
                    ("weight", json!({"type":"number","description":"Uzito 0..1 (default 0.5)"})),
                ],
                vec!["tier".into(), "key".into()],
            ),
        )
    }
}

pub struct SwarmV3MemoryRecallHandler;
#[async_trait]
impl ToolHandler for SwarmV3MemoryRecallHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        let q = p["q"].as_str().or_else(|| p["query"].as_str()).unwrap_or("");
        let tier = p["tier"].as_str().unwrap_or("");
        get_json(&format!("/api/swarm/v3/memory/recall?q={q}&tier={tier}")).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "memory_recall",
            "Recall from 6-tier memory by query (optionally filtered by tier).",
            obj(
                vec![
                    ("q", json!({"type":"string","description":"Neno la kutafuta"})),
                    ("tier", json!({"type":"string"})),
                ],
                vec!["q".into()],
            ),
        )
    }
}

pub struct SwarmV3FlowHandler;
#[async_trait]
impl ToolHandler for SwarmV3FlowHandler {
    async fn execute(&self, p: Value) -> Result<String, ToolError> {
        post_json("/api/swarm/v3/flow", &p).await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "megaflow_plan",
            "MegaFlow: plan a very large multi-stage pipeline (e.g. 10k tasks) with dependencies.",
            obj(
                vec![
                    ("tasks", json!({"type":"integer","description":"Idadi ya tasks (default 1000; mf. 10000)"})),
                    ("model_slots", json!({"type":"integer"})),
                    ("agent_slots", json!({"type":"integer"})),
                    ("env_slots", json!({"type":"integer"})),
                ],
                vec![],
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<&'static str> {
        vec![
            "swarm_v3_status",
            "warp_cortex_plan",
            "orthogonal_insight",
            "remind_cycle",
            "agent_descent",
            "memory_write",
            "memory_recall",
            "megaflow_plan",
        ]
    }

    #[test]
    fn schemas_zina_majina_sahihi() {
        let handlers: Vec<Box<dyn ToolHandler>> = vec![
            Box::new(SwarmV3StatusHandler),
            Box::new(SwarmV3CortexHandler),
            Box::new(SwarmV3OrthogonalHandler),
            Box::new(SwarmV3RemindHandler),
            Box::new(SwarmV3DescentHandler),
            Box::new(SwarmV3MemoryWriteHandler),
            Box::new(SwarmV3MemoryRecallHandler),
            Box::new(SwarmV3FlowHandler),
        ];
        let got: Vec<String> = handlers.iter().map(|h| h.schema().name.clone()).collect();
        assert_eq!(got, names());
        for h in &handlers {
            assert!(!h.schema().description.trim().is_empty());
        }
    }
}
