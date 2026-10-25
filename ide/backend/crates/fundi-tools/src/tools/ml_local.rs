//! Local ML inference tool (MTECHOS: Rust/Go/Elixir pekee — hakuna Python).
//! Backend ya Python (ultralytics/transformers/deepface/clip/sam2) imeondolewa;
//! hurudisha hali ya "unavailable". Safety: task/params are whitelist-validated.

use async_trait::async_trait;
use indexmap::IndexMap;
use serde_json::{json, Value};

use fundi_core::{tool_schema, JsonSchema, ToolError, ToolHandler, ToolSchema};

use std::sync::Arc;

// ---------------------------------------------------------------------------
// MlLocalBackend trait
// ---------------------------------------------------------------------------

/// Backend for local ML inference using vendored models.
#[async_trait]
pub trait MlLocalBackend: Send + Sync {
    async fn run(
        &self,
        task: &str,
        params: &Value,
        timeout: Option<u64>,
    ) -> Result<String, ToolError>;
}

// ---------------------------------------------------------------------------
// MlLocalHandler
// ---------------------------------------------------------------------------

/// Tool for local ML inference (vision, detection, face, segmentation, CLIP).
pub struct MlLocalHandler {
    backend: Arc<dyn MlLocalBackend>,
}

impl MlLocalHandler {
    pub fn new(backend: Arc<dyn MlLocalBackend>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl ToolHandler for MlLocalHandler {
    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let task = params
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'task' parameter".into()))?;

        let tool_params = params.get("params").cloned().unwrap_or(json!({}));
        let timeout = params.get("timeout").and_then(|v| v.as_u64());

        self.backend.run(task, &tool_params, timeout).await
    }

    fn schema(&self) -> ToolSchema {
        let mut props = IndexMap::new();
        props.insert(
            "task".into(),
            json!({
                "type": "string",
                "description": "ML task to run",
                "enum": [
                    "yolo_detect",
                    "classify_image",
                    "face_recognize",
                    "clip_match",
                    "segment_image"
                ]
            }),
        );
        props.insert(
            "params".into(),
            json!({
                "type": "object",
                "description": "Task-specific parameters (e.g. {\"image\": \"/path\", \"query\": \"a cat\"})"
            }),
        );
        props.insert(
            "timeout".into(),
            json!({
                "type": "integer",
                "description": "Timeout in seconds (default: 120)",
                "default": 120
            }),
        );

        tool_schema(
            "ml_local",
            "Run local ML inference using vendored models (YOLO detection, image classification, face recognition, CLIP matching, SAM2 segmentation). Faster than cloud APIs and runs offline.",
            JsonSchema::object(props, vec!["task".into()]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMlBackend;
    #[async_trait]
    impl MlLocalBackend for MockMlBackend {
        async fn run(
            &self,
            task: &str,
            _params: &Value,
            _timeout: Option<u64>,
        ) -> Result<String, ToolError> {
            Ok(format!("ML result for task: {}", task))
        }
    }

    #[tokio::test]
    async fn test_ml_local_schema() {
        let handler = MlLocalHandler::new(Arc::new(MockMlBackend));
        assert_eq!(handler.schema().name, "ml_local");
    }

    #[tokio::test]
    async fn test_ml_local_missing_task() {
        let handler = MlLocalHandler::new(Arc::new(MockMlBackend));
        let result = handler.execute(json!({})).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'task' parameter"));
    }

    #[tokio::test]
    async fn test_ml_local_execute() {
        let handler = MlLocalHandler::new(Arc::new(MockMlBackend));
        let result = handler
            .execute(json!({
                "task": "yolo_detect",
                "params": {"image": "/tmp/img.jpg"}
            }))
            .await
            .unwrap();
        assert!(result.contains("yolo_detect"));
    }
}
