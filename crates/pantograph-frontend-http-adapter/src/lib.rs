//! Frontend-only HTTP adapter for workflow service execution.
//!
//! This crate is intentionally separate from headless API bindings so URL-based
//! HTTP integration remains an explicit opt-in for modular GUI embedding.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pantograph_workflow_service::{
    capabilities, WorkflowHost, WorkflowHostModelDescriptor, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowRunHandle, WorkflowRunOptions, WorkflowServiceError,
};

pub const DEFAULT_BACKEND_NAME: &str = "openai-compatible";
pub const DEFAULT_MAX_INPUT_BINDINGS: usize = capabilities::DEFAULT_MAX_INPUT_BINDINGS;
pub const DEFAULT_MAX_OUTPUT_TARGETS: usize = capabilities::DEFAULT_MAX_OUTPUT_TARGETS;
pub const DEFAULT_MAX_VALUE_BYTES: usize = capabilities::DEFAULT_MAX_VALUE_BYTES;

#[derive(Debug, thiserror::Error)]
pub enum FrontendHttpWorkflowHostError {
    #[error("invalid base_url '{base_url}': {reason}")]
    InvalidUrl {
        base_url: String,
        reason: String,
    },
    #[error("unsupported URL scheme '{scheme}' in base_url '{base_url}'")]
    UnsupportedScheme {
        base_url: String,
        scheme: String,
    },
    #[error("base_url '{base_url}' is missing a host")]
    MissingHost { base_url: String },
}

/// Workflow host that proxies workflow_run through HTTP.
///
/// This adapter is for frontend/modular GUI transport integration, not for
/// framework headless workflow consumers.
pub struct FrontendHttpWorkflowHost {
    base_url: String,
    workflow_roots: Vec<PathBuf>,
    max_input_bindings: usize,
    max_output_targets: usize,
    max_value_bytes: usize,
    backend_name: String,
    pumas_api: Option<Arc<pumas_library::PumasApi>>,
    http_client: reqwest::Client,
}

impl FrontendHttpWorkflowHost {
    pub fn with_defaults(
        base_url: String,
        pumas_api: Option<Arc<pumas_library::PumasApi>>,
        manifest_dir: &Path,
    ) -> Result<Self, FrontendHttpWorkflowHostError> {
        Self::new(
            base_url,
            pumas_api,
            capabilities::default_workflow_roots(manifest_dir),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
    }

    pub fn new(
        base_url: String,
        pumas_api: Option<Arc<pumas_library::PumasApi>>,
        workflow_roots: Vec<PathBuf>,
        max_input_bindings: usize,
        max_output_targets: usize,
        max_value_bytes: usize,
        backend_name: String,
    ) -> Result<Self, FrontendHttpWorkflowHostError> {
        let base_url = normalize_base_url(base_url)?;
        Ok(Self {
            base_url,
            workflow_roots,
            max_input_bindings,
            max_output_targets,
            max_value_bytes,
            backend_name: backend_name.trim().to_string(),
            pumas_api,
            http_client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl WorkflowHost for FrontendHttpWorkflowHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        self.workflow_roots.clone()
    }

    fn max_input_bindings(&self) -> usize {
        self.max_input_bindings
    }

    fn max_output_targets(&self) -> usize {
        self.max_output_targets
    }

    fn max_value_bytes(&self) -> usize {
        self.max_value_bytes
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok(self.backend_name.clone())
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        let Some(api) = &self.pumas_api else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| m.metadata))
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        let Some(api) = &self.pumas_api else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| WorkflowHostModelDescriptor {
            model_type: Some(m.model_type.trim().to_string()).filter(|v| !v.is_empty()),
            hashes: m.hashes,
        }))
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if run_handle.is_cancelled() {
            return Err(WorkflowServiceError::RuntimeTimeout(
                "workflow run cancelled before dispatch".to_string(),
            ));
        }

        let url = format!("{}/v1/workflow/run", self.base_url);
        let body = serde_json::json!({
            "workflow_id": workflow_id,
            "inputs": inputs,
            "output_targets": output_targets,
        });

        let mut request = self.http_client.post(&url).json(&body);
        if let Some(timeout_ms) = run_options.timeout_ms {
            request = request.timeout(Duration::from_millis(timeout_ms));
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                WorkflowServiceError::RuntimeTimeout("frontend HTTP workflow request timed out".to_string())
            } else if run_handle.is_cancelled() {
                WorkflowServiceError::RuntimeTimeout("workflow run cancelled".to_string())
            } else {
                WorkflowServiceError::RuntimeNotReady(e.to_string())
            }
        })?;
        if !response.status().is_success() {
            return Err(WorkflowServiceError::Internal(format!(
                "workflow api error {}",
                response.status()
            )));
        }

        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;

        parse_workflow_outputs_payload(&payload)
    }
}

pub fn parse_workflow_outputs_payload(
    payload: &serde_json::Value,
) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
    let outputs = payload
        .get("outputs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| WorkflowServiceError::Internal("missing outputs array".to_string()))?;

    let mut bindings = Vec::with_capacity(outputs.len());
    for (index, output) in outputs.iter().enumerate() {
        let node_id = output
            .get("node_id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                WorkflowServiceError::Internal(format!(
                    "invalid outputs[{}].node_id",
                    index
                ))
            })?
            .to_string();
        let port_id = output
            .get("port_id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                WorkflowServiceError::Internal(format!(
                    "invalid outputs[{}].port_id",
                    index
                ))
            })?
            .to_string();

        let value = output
            .get("value")
            .cloned()
            .ok_or_else(|| WorkflowServiceError::Internal(format!("missing outputs[{}].value", index)))?;

        bindings.push(WorkflowPortBinding {
            node_id,
            port_id,
            value,
        });
    }

    Ok(bindings)
}

fn normalize_base_url(raw_base_url: String) -> Result<String, FrontendHttpWorkflowHostError> {
    let trimmed = raw_base_url.trim().to_string();
    let parsed = reqwest::Url::parse(&trimmed).map_err(|e| {
        FrontendHttpWorkflowHostError::InvalidUrl {
            base_url: trimmed.clone(),
            reason: e.to_string(),
        }
    })?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(FrontendHttpWorkflowHostError::UnsupportedScheme {
                base_url: trimmed,
                scheme: other.to_string(),
            });
        }
    }

    if parsed.host_str().is_none() {
        return Err(FrontendHttpWorkflowHostError::MissingHost { base_url: trimmed });
    }

    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_workflow_outputs_payload_rejects_missing_fields() {
        let payload = serde_json::json!({
            "outputs": [{ "node_id": "node-1", "value": "oops" }]
        });
        let err = parse_workflow_outputs_payload(&payload).expect_err("must reject malformed outputs");
        assert!(err.to_string().contains("port_id"));
    }

    #[test]
    fn normalize_base_url_rejects_non_http_schemes() {
        let err = normalize_base_url("file:///tmp/server".to_string())
            .expect_err("must reject unsupported scheme");
        assert!(matches!(
            err,
            FrontendHttpWorkflowHostError::UnsupportedScheme { .. }
        ));
    }

    #[test]
    fn normalize_base_url_trims_trailing_slash() {
        let normalized = normalize_base_url("http://127.0.0.1:8080/".to_string())
            .expect("normalize");
        assert_eq!(normalized, "http://127.0.0.1:8080");
    }
}
