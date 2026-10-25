//! IDE tool — huwapa agent uwezo wa IDE kama tool HALISI ya SDK.
//!
//! Muundo: tool hii ni **thin client** kwa utekelezaji MMOJA (REST: `/api/ide/*`
//! kwenye `fundi-deploy`). Hatunakili mantiki — tunaunganisha. Agent huita tool
//! `ide` kama tool nyingine yoyote ya SDK.
//!
//!   audit        → POST /api/ide/code/audit/file
//!   test         → POST /api/ide/test/run
//!   build_targets→ GET  /api/ide/build/targets
//!   build_script → POST /api/ide/build/script
//!   speed        → POST /api/ide/build/speed
//!   devices      → GET  /api/ide/device/catalog
//!   lsp          → POST /api/ide/lsp/{action}
//!   debug        → POST /api/ide/dap/run
//!   e2e          → POST /api/ide/e2e/run
//!   engineer     → POST /api/ide/engineer/run
//!   pipeline     → POST /api/ide/pipeline/run

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use fundi_core::{tool_schema, JsonSchema, ToolError, ToolHandler, ToolSchema};

/// Tool inayotoa uwezo wa IDE kwa agent (thin client kwa `/api/ide/*`).
pub struct IdeToolHandler {
    base_url: String,
    client: reqwest::Client,
}

impl IdeToolHandler {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Hupata base URL kutoka env `FUNDI_IDE_URL`, kisha `FUNDI_DEPLOY_URL`.
    pub fn from_env() -> Self {
        let url = std::env::var("FUNDI_IDE_URL")
            .or_else(|_| std::env::var("FUNDI_DEPLOY_URL"))
            .unwrap_or_else(|_| "http://127.0.0.1:8080".into());
        Self::new(url)
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.get(format!("{}{}", self.base_url, path))
    }
    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.post(format!("{}{}", self.base_url, path))
    }

    async fn send(&self, rb: reqwest::RequestBuilder) -> Result<String, ToolError> {
        let resp = rb
            .timeout(std::time::Duration::from_secs(900))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("IDE request: {e}")))?;
        let body = resp
            .text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("IDE body: {e}")))?;
        Ok(body)
    }
}

#[async_trait]
impl ToolHandler for IdeToolHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let op = params
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'op' parameter".into()))?;
        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("");

        let req = match op {
            "audit" => self.post("/api/ide/code/audit/file").json(&json!({
                "path": path,
                "code": params.get("code"),
                "lang": params.get("lang"),
            })),
            "test" => self.post("/api/ide/test/run").json(&json!({
                "path": params.get("path"),
                "only": params.get("only").or_else(|| params.get("stack")),
                "with_lint": params.get("with_lint"),
                "with_fmt": params.get("with_fmt"),
            })),
            "build_targets" => self.get("/api/ide/build/targets"),
            "build_script" => self.post("/api/ide/build/script").json(&json!({
                "platform": params.get("platform"),
                "path": path,
            })),
            "speed" => self.post("/api/ide/build/speed").json(&json!({
                "files": params.get("files").unwrap_or(&json!(200)),
                "size": params.get("size").unwrap_or(&json!(2048)),
            })),
            "devices" => self.get("/api/ide/device/catalog"),
            "device_frame" => self.post("/api/ide/device/frame").json(&json!({
                "device": params.get("device"),
                "orientation": params.get("orientation"),
                "code": params.get("code"),
                "path": params.get("path"),
            })),
            "device_launch" => self.post("/api/ide/device/launch").json(&json!({
                "os": params.get("os").unwrap_or(&json!("android")),
                "avd": params.get("avd"),
                "udid": params.get("udid"),
            })),
            "build_run" => self.post("/api/ide/build/run").json(&json!({
                "platform": params.get("platform"),
                "stack": params.get("stack"),
                "path": path,
            })),
            "swarm" => self.post("/api/ide/swarm/run").json(&json!({
                "idea": params.get("idea"),
                "kind": params.get("kind").unwrap_or(&json!("website")),
                "lang": params.get("lang").unwrap_or(&json!("rust")),
                "path": path,
                "use_llm": params.get("use_llm"),
            })),
            "swarm_generate" => self.post("/api/ide/swarm/generate").json(&json!({
                "idea": params.get("idea"),
                "kind": params.get("kind").unwrap_or(&json!("website")),
                "lang": params.get("lang").unwrap_or(&json!("rust")),
            })),
            "lsp" => {
                let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("diagnostics");
                self.post(&format!("/api/ide/lsp/{action}")).json(&json!({
                    "action": action,
                    "path": path,
                    "language": params.get("lang"),
                    "code": params.get("code"),
                    "line": params.get("line"),
                    "character": params.get("character"),
                }))
            }
            "debug" => self.post("/api/ide/dap/run").json(&json!({
                "path": path,
                "breakpoints": params.get("breakpoints"),
                "steps": params.get("steps"),
                "stop_on_entry": params.get("stop_on_entry"),
            })),
            "e2e" => self.post("/api/ide/e2e/run").json(&json!({
                "url": params.get("url"),
                "name": params.get("name").unwrap_or(&json!("agent-smoke")),
                "install": params.get("install"),
            })),
            "engineer" => self.post("/api/ide/engineer/run").json(&json!({
                "path": params.get("path"),
                "tests": params.get("tests"),
                "lint": params.get("lint"),
                "lsp": params.get("lsp"),
                "debug": params.get("debug"),
                "e2e_url": params.get("e2e_url"),
                "platforms": params.get("platforms"),
            })),
            "pipeline" => self.post("/api/ide/pipeline/run").json(&json!({
                "idea": params.get("idea"),
                "kind": params.get("kind"),
                "lang": params.get("lang"),
                "path": path,
                "code": params.get("code"),
                "tests": params.get("tests"),
                "lsp": params.get("lsp"),
                "debug": params.get("debug"),
                "e2e_url": params.get("e2e_url"),
                "platforms": params.get("platforms"),
                "write": params.get("write"),
            })),
            other => {
                return Err(ToolError::InvalidParams(format!("op haijulikani: {other}")));
            }
        };
        self.send(req).await
    }

    fn schema(&self) -> ToolSchema {
        let mut props: IndexMap<String, Value> = IndexMap::new();
        props.insert(
            "op".into(),
            json!({
                "type": "string",
                "description": "Operesheni ya IDE",
                "enum": [
                    "audit", "test", "build_targets", "build_script", "build_run", "speed",
                    "devices", "device_frame", "device_launch",
                    "lsp", "debug", "e2e", "engineer", "pipeline", "swarm", "swarm_generate"
                ],
            }),
        );
        props.insert("path".into(), json!({ "type": "string", "description": "Path ya faili/mradi ndani ya workspace" }));
        props.insert("code".into(), json!({ "type": "string", "description": "Code ya kuchambua (op=audit)" }));
        props.insert("lang".into(), json!({ "type": "string", "description": "Lugha (op=audit)" }));
        props.insert("stack".into(), json!({ "type": "string", "description": "Stack id (op=test): rust/node/python/go/java/php/ruby/r/cpp (alias ya `only`)" }));
        props.insert("with_lint".into(), json!({ "type": "boolean", "description": "Endesha lint pia (op=test)" }));
        props.insert("with_fmt".into(), json!({ "type": "boolean", "description": "Endesha fmt check pia (op=test)" }));
        props.insert("action".into(), json!({ "type": "string", "description": "diagnostics|hover|completion (op=lsp)" }));
        props.insert("line".into(), json!({ "type": "integer", "description": "Mstari (op=lsp hover/completion)" }));
        props.insert("character".into(), json!({ "type": "integer", "description": "Herufi (op=lsp)" }));
        props.insert("platform".into(), json!({ "type": "string", "description": "Jukwaa (op=build_script/build_run)" }));
        props.insert("device".into(), json!({ "type": "string", "description": "Device id (op=device_frame), mfano iphone-16, pixel-8" }));
        props.insert("os".into(), json!({ "type": "string", "description": "android|ios (op=device_launch)" }));
        props.insert("orientation".into(), json!({ "type": "string", "description": "portrait|landscape (op=device_frame)" }));
        props.insert("avd".into(), json!({ "type": "string", "description": "Android AVD name (op=device_launch)" }));
        props.insert("udid".into(), json!({ "type": "string", "description": "iOS simulator UDID (op=device_launch)" }));
        props.insert("use_llm".into(), json!({ "type": "boolean", "description": "Tumia LLM kwenye swarm (op=swarm)" }));
        props.insert("url".into(), json!({ "type": "string", "description": "URL kwa E2E (op=e2e)" }));
        props.insert("breakpoints".into(), json!({ "type": "array", "items": { "type": "integer" }, "description": "Mistari ya breakpoint (op=debug)" }));
        props.insert("idea".into(), json!({ "type": "string", "description": "Idea/kazi (op=pipeline)" }));
        props.insert("platforms".into(), json!({ "type": "array", "items": { "type": "string" }, "description": "Majukwaa (op=engineer/pipeline)" }));
        props.insert("e2e_url".into(), json!({ "type": "string", "description": "URL ya E2E (op=engineer/pipeline)" }));

        tool_schema(
            "ide",
            "Zana ya IDE: audit ya usalama/bugs, testi halisi, build matrix (Windows/macOS/ARM/Linux), \
             kasi, vifaa vya Android/iPhone, LSP (IntelliSense), debugger (DAP), E2E (Playwright), \
             senior engineer, na autopilot pipeline. Inatumia endpoint za /api/ide/*.",
            JsonSchema::object(props, vec!["op".into()]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_op_enum() {
        let s = IdeToolHandler::new("http://127.0.0.1:8080").schema();
        assert_eq!(s.name, "ide");
        let ops = s.parameters.properties.unwrap().get("op").unwrap().clone();
        let en = ops["enum"].as_array().unwrap();
        assert!(en.iter().any(|v| v == "pipeline"));
        assert!(en.iter().any(|v| v == "lsp"));
    }

    #[tokio::test]
    async fn unknown_op_errors() {
        let h = IdeToolHandler::new("http://127.0.0.1:9");
        assert!(h.execute(json!({ "op": "nope" })).await.is_err());
    }
}
