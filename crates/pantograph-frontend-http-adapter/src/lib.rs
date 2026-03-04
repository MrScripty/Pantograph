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
    use pantograph_workflow_service::{
        WorkflowOutputTarget, WorkflowRunRequest, WorkflowService, WorkflowServiceError,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    fn create_temp_workflow_root_with_output(workflow_id: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pantograph-frontend-http-tests-{suffix}"));
        let workflows_dir = root.join(".pantograph").join("workflows");
        std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");

        let workflow_json = serde_json::json!({
            "version": "1.0",
            "metadata": {
                "name": "Output Workflow"
            },
            "graph": {
                "nodes": [
                    {
                        "id": "vector-output-1",
                        "node_type": "vector-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "outputs": [
                                    {
                                        "id": "vector",
                                        "data_type": "embedding",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    }
                ],
                "edges": []
            }
        });

        let file_path = workflows_dir.join(format!("{}.json", workflow_id));
        std::fs::write(
            file_path,
            serde_json::to_vec(&workflow_json).expect("serialize workflow"),
        )
        .expect("write workflow");
        root
    }

    fn spawn_single_workflow_server(
        status_code: u16,
        body: serde_json::Value,
    ) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let body_text = body.to_string();
        let reason = if status_code == 200 { "OK" } else { "ERROR" };

        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set timeout");
            let mut request_buf = [0_u8; 8192];
            let _ = stream.read(&mut request_buf);

            let response = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_code,
                reason,
                body_text.len(),
                body_text
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        (format!("http://{}", addr), handle)
    }

    #[tokio::test]
    async fn workflow_run_returns_output_not_produced_for_missing_target_output() {
        let workflow_id = "wf-output-not-produced";
        let workflow_root = create_temp_workflow_root_with_output(workflow_id)
            .join(".pantograph")
            .join("workflows");

        let payload = serde_json::json!({
            "run_id": "adapter-run-1",
            "outputs": [],
            "timing_ms": 2
        });
        let (base_url, server_thread) = spawn_single_workflow_server(200, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            vec![workflow_root],
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let err = WorkflowService::new()
            .workflow_run(
                &host,
                WorkflowRunRequest {
                    workflow_id: workflow_id.to_string(),
                    inputs: Vec::new(),
                    output_targets: Some(vec![WorkflowOutputTarget {
                        node_id: "vector-output-1".to_string(),
                        port_id: "vector".to_string(),
                    }]),
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("missing output should return output_not_produced");

        server_thread.join().expect("join server");
        assert!(matches!(err, WorkflowServiceError::OutputNotProduced(_)));
    }
}
