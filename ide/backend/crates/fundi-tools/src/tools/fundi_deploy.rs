//! Fundi Deploy tools: REST client for Fundi Deploy (:8080) so Fundi Agent
//! can list computers/jobs, select OS, start deploy, approve/cancel, VPN/cloud.
//!
//! Environment:
//!   - FUNDI_DEPLOY_URL (default http://127.0.0.1:8080)
//!   - FUNDI_DEPLOY_TOKEN (optional bearer token)
//!   - FUNDI_DEPLOY_ALLOW_AUTO_APPROVE=1  (required to pass auto_approve:true)
//!   - FUNDI_DEPLOY_YOLO=1               (lab only: skip HITL markers)
//!
//! Security (production):
//!   - deploy_start / deploy_approve_job require HITL unless YOLO lab mode
//!   - auto_approve is denied unless FUNDI_DEPLOY_ALLOW_AUTO_APPROVE=1
//!   - Aligns with Akili V4 ethics: dangerous actions need human approval

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn};

use fundi_core::{tool_schema, JsonSchema, ToolError, ToolHandler, ToolSchema};

fn base_url() -> String {
    let url = std::env::var("FUNDI_DEPLOY_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    url.trim_end_matches('/').to_string()
}

fn token() -> Option<String> {
    std::env::var("FUNDI_DEPLOY_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn yolo_lab_mode() -> bool {
    matches!(
        std::env::var("FUNDI_DEPLOY_YOLO").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn allow_auto_approve_env() -> bool {
    matches!(
        std::env::var("FUNDI_DEPLOY_ALLOW_AUTO_APPROVE").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

/// HITL: require explicit approval flag from caller (agent/UI) unless lab YOLO.
fn require_hitl_param(params: &Value, action: &str) -> Result<(), ToolError> {
    if yolo_lab_mode() {
        warn!(target: "fundi_deploy", action = %action, "YOLO lab mode — HITL skipped");
        return Ok(());
    }
    let approved = params
        .get("hitl_approved")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || params
            .get("user_approved")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
    if !approved {
        return Err(ToolError::ExecutionFailed(format!(
            "HITL required for '{}'. Pass hitl_approved=true after user confirms. \
             (Lab only: FUNDI_DEPLOY_YOLO=1)",
            action
        )));
    }
    Ok(())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
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
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ToolError::ExecutionFailed(format!("HTTP {}: {}", status, text)));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    let v: serde_json::Result<Value> = serde_json::from_slice(&bytes);
    match v {
        Ok(parsed) => serde_json::to_string_pretty(&parsed)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string())),
        Err(_) => String::from_utf8(bytes.to_vec())
            .map_err(|e| ToolError::ExecutionFailed(e.to_string())),
    }
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
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ToolError::ExecutionFailed(format!("HTTP {}: {}", status, text)));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
    let v: serde_json::Result<Value> = serde_json::from_slice(&bytes);
    match v {
        Ok(parsed) => serde_json::to_string_pretty(&parsed)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string())),
        Err(_) => String::from_utf8(bytes.to_vec())
            .map_err(|e| ToolError::ExecutionFailed(e.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Read-only tools (no HITL)
// ---------------------------------------------------------------------------

pub struct DeployListComputersHandler;
impl DeployListComputersHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployListComputersHandler {
    async fn execute(&self, _params: Value) -> Result<String, ToolError> {
        get_json("/computers").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "deploy_list_computers",
            "List all computers/devices registered in Fundi Deploy (MTECH OS).",
            JsonSchema::object(IndexMap::new(), vec![]),
        )
    }
}

pub struct DeployListJobsHandler;
impl DeployListJobsHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployListJobsHandler {
    async fn execute(&self, _params: Value) -> Result<String, ToolError> {
        get_json("/jobs").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "deploy_list_jobs",
            "List all deployment jobs in Fundi Deploy (status, progress, device, OS).",
            JsonSchema::object(IndexMap::new(), vec![]),
        )
    }
}

pub struct DeployListImagesHandler;
impl DeployListImagesHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployListImagesHandler {
    async fn execute(&self, _params: Value) -> Result<String, ToolError> {
        get_json("/images").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "deploy_list_images",
            "List available OS images (Windows, Linux, etc.) for deployment.",
            JsonSchema::object(IndexMap::new(), vec![]),
        )
    }
}

pub struct DeploySupervisorSummaryHandler;
impl DeploySupervisorSummaryHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeploySupervisorSummaryHandler {
    async fn execute(&self, _params: Value) -> Result<String, ToolError> {
        get_json("/supervisor/summary").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "deploy_supervisor_summary",
            "Get deployment pipeline summary (counts per state/OS type, throughput).",
            JsonSchema::object(IndexMap::new(), vec![]),
        )
    }
}

pub struct DeployOsSelectHandler;
impl DeployOsSelectHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployOsSelectHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let specs = params
            .get("specs")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'specs' parameter".into()))?;
        let need = params
            .get("need")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let body = json!({ "specs": specs, "need": need });
        post_json("/os/select", &body).await
    }
    fn schema(&self) -> ToolSchema {
        let mut props = IndexMap::new();
        props.insert(
            "specs".into(),
            json!({
                "type": "string",
                "description": "PC specs, e.g. '8GB RAM, Core i5 10th gen, 256GB SSD'"
            }),
        );
        props.insert(
            "need".into(),
            json!({
                "type": "string",
                "description": "Required software/usage, e.g. 'Office, Photoshop, school' or 'server'"
            }),
        );
        tool_schema(
            "deploy_os_select",
            "Use AI + rules to recommend the best OS image for a PC given its specs and use case.",
            JsonSchema::object(props, vec!["specs".into()]),
        )
    }
}

pub struct DeployCloudStatusHandler;
impl DeployCloudStatusHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployCloudStatusHandler {
    async fn execute(&self, _params: Value) -> Result<String, ToolError> {
        get_json("/cloud/status").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "deploy_cloud_status",
            "Show multi-tenant cloud sync status (online, outbox count, tenant, last sync).",
            JsonSchema::object(IndexMap::new(), vec![]),
        )
    }
}

pub struct DeployVpnStatusHandler;
impl DeployVpnStatusHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployVpnStatusHandler {
    async fn execute(&self, _params: Value) -> Result<String, ToolError> {
        get_json("/api/vpn/status").await
    }
    fn schema(&self) -> ToolSchema {
        tool_schema(
            "deploy_vpn_status",
            "Show WireGuard VPN status (wg0 state, endpoint, IP, peers, running).",
            JsonSchema::object(IndexMap::new(), vec![]),
        )
    }
}

// ---------------------------------------------------------------------------
// Mutating tools — HITL required (production)
// ---------------------------------------------------------------------------

pub struct DeployApproveJobHandler;
impl DeployApproveJobHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployApproveJobHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        require_hitl_param(&params, "deploy_approve_job")?;
        let job_id = params
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'job_id' parameter".into()))?;
        info!(target: "fundi_deploy", job_id = %job_id, "HITL approved — approving job");
        let path = format!("/jobs/{}/approve", job_id.trim());
        post_json(&path, &json!({})).await
    }
    fn schema(&self) -> ToolSchema {
        let mut props = IndexMap::new();
        props.insert(
            "job_id".into(),
            json!({
                "type": "string",
                "description": "Deployment job ID (UUID) to approve"
            }),
        );
        props.insert(
            "hitl_approved".into(),
            json!({
                "type": "boolean",
                "description": "Must be true after human confirms (required in production)"
            }),
        );
        tool_schema(
            "deploy_approve_job",
            "Approve an awaiting-approval deployment job. Requires hitl_approved=true.",
            JsonSchema::object(props, vec!["job_id".into(), "hitl_approved".into()]),
        )
    }
}

pub struct DeployCancelJobHandler;
impl DeployCancelJobHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployCancelJobHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        // Cancel is safer than approve/start but still mutates — require HITL
        require_hitl_param(&params, "deploy_cancel_job")?;
        let job_id = params
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'job_id' parameter".into()))?;
        info!(target: "fundi_deploy", job_id = %job_id, "HITL approved — cancelling job");
        let path = format!("/jobs/{}/cancel", job_id.trim());
        post_json(&path, &json!({})).await
    }
    fn schema(&self) -> ToolSchema {
        let mut props = IndexMap::new();
        props.insert(
            "job_id".into(),
            json!({
                "type": "string",
                "description": "Deployment job ID (UUID) to cancel"
            }),
        );
        props.insert(
            "hitl_approved".into(),
            json!({
                "type": "boolean",
                "description": "Must be true after human confirms"
            }),
        );
        tool_schema(
            "deploy_cancel_job",
            "Cancel a running/pending deployment job. Requires hitl_approved=true.",
            JsonSchema::object(props, vec!["job_id".into(), "hitl_approved".into()]),
        )
    }
}

pub struct DeployStartHandler;
impl DeployStartHandler {
    pub fn new() -> Self {
        Self
    }
}
#[async_trait]
impl ToolHandler for DeployStartHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        require_hitl_param(&params, "deploy_start")?;

        let macs = params
            .get("macs")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'macs' array parameter".into()))?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>();
        if macs.is_empty() {
            return Err(ToolError::InvalidParams(
                "Parameter 'macs' must be a non-empty array of MAC addresses".into(),
            ));
        }

        let os = params
            .get("os")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");
        let reason = params
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("Fundi Agent — user HITL approved");

        let mut auto_approve = params
            .get("auto_approve")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Production: never allow auto_approve unless explicit env
        if auto_approve && !allow_auto_approve_env() && !yolo_lab_mode() {
            warn!(
                target: "fundi_deploy",
                "auto_approve=true blocked — set FUNDI_DEPLOY_ALLOW_AUTO_APPROVE=1 for lab"
            );
            auto_approve = false;
        }

        info!(
            target: "fundi_deploy",
            macs = ?macs,
            os = %os,
            auto_approve,
            "Starting deploy pipeline (HITL ok)"
        );

        let body = json!({
            "computers": macs
                .into_iter()
                .map(|mac| {
                    json!({
                        "mac": mac,
                        "name": "",
                    })
                })
                .collect::<Vec<_>>(),
            "os": os,
            "reason": reason,
            "auto_approve": auto_approve,
        });
        post_json("/deploy", &body).await
    }

    fn schema(&self) -> ToolSchema {
        let mut props = IndexMap::new();
        props.insert(
            "macs".into(),
            json!({
                "type": "array",
                "items": { "type": "string" },
                "description": "List of MAC addresses to deploy to, e.g. ['aa:bb:cc:dd:ee:ff']"
            }),
        );
        props.insert(
            "os".into(),
            json!({
                "type": "string",
                "description": "OS name or image ID (e.g. 'windows11', 'ubuntu24'); use 'auto' to let AI select."
            }),
        );
        props.insert(
            "reason".into(),
            json!({
                "type": "string",
                "description": "Reason for deployment (logged to job history)."
            }),
        );
        props.insert(
            "auto_approve".into(),
            json!({
                "type": "boolean",
                "description": "Skip server-side HITL (blocked unless FUNDI_DEPLOY_ALLOW_AUTO_APPROVE=1)."
            }),
        );
        props.insert(
            "hitl_approved".into(),
            json!({
                "type": "boolean",
                "description": "Must be true after human confirms deploy (required in production)."
            }),
        );
        tool_schema(
            "deploy_start",
            "Start OS deployment for target devices by MAC. Requires hitl_approved=true. auto_approve is blocked in production.",
            JsonSchema::object(props, vec!["macs".into(), "hitl_approved".into()]),
        )
    }
}
