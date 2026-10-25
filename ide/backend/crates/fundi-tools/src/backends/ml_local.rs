//! Local ML inference backend (Rust/Go/Elixir pekee — hakuna Python).
//!
//! Inference ya ML ya ndani ilitegemea Python (ultralytics, transformers,
//! deepface, clip, sam2). MTECHOS ni Rust/Go/Elixir pekee, hivyo backend
//! hii sasa hurudisha hali ya wazi ya "unavailable" pamoja na maelekezo.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::tools::ml_local::MlLocalBackend;
use fundi_core::ToolError;

pub const ML_TASKS: &[&str] = &[
    "yolo_detect",
    "classify_image",
    "face_recognize",
    "clip_match",
    "segment_image",
];

pub struct LocalMlBackend {
    /// Root of `server/agent` — used to locate `vendor/` and `third_party/`.
    repo_root: PathBuf,
}

impl LocalMlBackend {
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    fn vendor_root(&self) -> PathBuf {
        self.repo_root.join("vendor")
    }

    fn third_party_root(&self) -> PathBuf {
        self.repo_root.join("third_party")
    }

    fn validate_task(task: &str) -> Result<(), ToolError> {
        if ML_TASKS.contains(&task) {
            Ok(())
        } else {
            Err(ToolError::InvalidParams(format!(
                "Unknown ML task '{}'. Allowed: {}",
                task,
                ML_TASKS.join(", ")
            )))
        }
    }
}

impl Default for LocalMlBackend {
    fn default() -> Self {
        // CARGO_MANIFEST_DIR = <repo>/server/agent/crates/fundi-tools → 3 up = repo.
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = here
            .ancestors()
            .nth(3)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        if repo_root.join("vendor").exists() || repo_root.join("third_party").exists() {
            Self::new(repo_root)
        } else {
            let fallback = std::env::current_dir()
                .ok()
                .and_then(|d| {
                    d.ancestors()
                        .find(|p| p.join("vendor").exists() || p.join("third_party").exists())
                        .map(|p| p.to_path_buf())
                })
                .unwrap_or(repo_root);
            Self::new(fallback)
        }
    }
}

#[async_trait]
impl MlLocalBackend for LocalMlBackend {
    async fn run(
        &self,
        task: &str,
        params: &Value,
        _timeout: Option<u64>,
    ) -> Result<String, ToolError> {
        Self::validate_task(task)?;

        // MTECHOS ni Rust/Go/Elixir pekee. Inference ya ML ya ndani ilitegemea
        // Python (ultralytics/transformers/deepface/clip/sam2); hiyo imeondolewa.
        // Hurudisha hali ya wazi badala ya kujifanya inaendesha.
        Ok(json!({
            "task": task,
            "status": "unavailable",
            "reason": "Local ML backend ya Python imeondolewa — MTECHOS ni Rust/Go/Elixir pekee.",
            "hint": "Endesha inference kwenye mazingira yako ya ML (torch/ultralytics) na tumia matokeo; au tumia endpoint ya nje.",
            "params": params,
            "vendor_root": self.vendor_root().display().to_string(),
            "third_party_root": self.third_party_root().display().to_string(),
        })
        .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_root_default_nonempty() {
        let b = LocalMlBackend::default();
        assert!(!b.vendor_root().as_os_str().is_empty());
        assert!(!b.third_party_root().as_os_str().is_empty());
    }

    #[test]
    fn test_validate_task() {
        assert!(LocalMlBackend::validate_task("yolo_detect").is_ok());
        assert!(LocalMlBackend::validate_task("not_a_task").is_err());
    }

    #[tokio::test]
    async fn test_run_reports_unavailable() {
        let b = LocalMlBackend::default();
        let out = b
            .run("yolo_detect", &json!({"image": "/tmp/x.jpg"}), None)
            .await
            .unwrap();
        assert!(out.contains("unavailable"));
    }
}
