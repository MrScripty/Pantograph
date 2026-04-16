//! Frontend-only HTTP adapter for workflow service execution.
//!
//! This crate is intentionally separate from headless API bindings so URL-based
//! HTTP integration remains an explicit opt-in for modular GUI embedding.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pantograph_runtime_identity::{
    backend_key_aliases, normalize_runtime_identifier_with_fallback,
};
use pantograph_workflow_service::{
    capabilities, WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowHost,
    WorkflowHostModelDescriptor, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunHandle,
    WorkflowRunOptions, WorkflowRuntimeCapability, WorkflowRuntimeInstallState,
    WorkflowRuntimeSourceKind, WorkflowServiceError,
};

pub const DEFAULT_BACKEND_NAME: &str = "openai-compatible";
pub const DEFAULT_MAX_INPUT_BINDINGS: usize = capabilities::DEFAULT_MAX_INPUT_BINDINGS;
pub const DEFAULT_MAX_OUTPUT_TARGETS: usize = capabilities::DEFAULT_MAX_OUTPUT_TARGETS;
pub const DEFAULT_MAX_VALUE_BYTES: usize = capabilities::DEFAULT_MAX_VALUE_BYTES;

#[derive(Debug, thiserror::Error)]
pub enum FrontendHttpWorkflowHostError {
    #[error("invalid base_url '{base_url}': {reason}")]
    InvalidUrl { base_url: String, reason: String },
    #[error("unsupported URL scheme '{scheme}' in base_url '{base_url}'")]
    UnsupportedScheme { base_url: String, scheme: String },
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
    backend_runtime_id: String,
    backend_keys: Vec<String>,
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
        let backend_name = backend_name.trim().to_string();
        let backend_runtime_id =
            normalize_runtime_identifier_with_fallback(&backend_name, DEFAULT_BACKEND_NAME);
        let backend_keys = backend_key_aliases(&backend_name, &backend_runtime_id);
        Ok(Self {
            base_url,
            workflow_roots,
            max_input_bindings,
            max_output_targets,
            max_value_bytes,
            backend_name,
            backend_runtime_id,
            backend_keys,
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
        Ok(self.backend_runtime_id.clone())
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

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(vec![WorkflowRuntimeCapability {
            runtime_id: self.backend_runtime_id.clone(),
            display_name: self.backend_name.clone(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Host,
            selected: true,
            supports_external_connection: true,
            backend_keys: self.backend_keys.clone(),
            missing_files: Vec::new(),
            unavailable_reason: None,
        }])
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
                WorkflowServiceError::RuntimeTimeout(
                    "frontend HTTP workflow request timed out".to_string(),
                )
            } else if run_handle.is_cancelled() {
                WorkflowServiceError::RuntimeTimeout("workflow run cancelled".to_string())
            } else {
                WorkflowServiceError::RuntimeNotReady(e.to_string())
            }
        })?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.map_err(|e| {
                WorkflowServiceError::Internal(format!(
                    "workflow api error {} (failed to read error payload: {})",
                    status, e
                ))
            })?;
            let envelope: WorkflowErrorEnvelope = serde_json::from_str(&body).map_err(|e| {
                WorkflowServiceError::Internal(format!(
                    "workflow api error {} (expected workflow error envelope JSON: {}; body: {})",
                    status, e, body
                ))
            })?;
            return Err(map_workflow_error_envelope(envelope));
        }

        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;

        parse_workflow_outputs_payload(&payload)
    }
}

fn map_workflow_error_envelope(envelope: WorkflowErrorEnvelope) -> WorkflowServiceError {
    match envelope.code {
        WorkflowErrorCode::InvalidRequest => WorkflowServiceError::InvalidRequest(envelope.message),
        WorkflowErrorCode::WorkflowNotFound => {
            WorkflowServiceError::WorkflowNotFound(envelope.message)
        }
        WorkflowErrorCode::CapabilityViolation => {
            WorkflowServiceError::CapabilityViolation(envelope.message)
        }
        WorkflowErrorCode::RuntimeNotReady => {
            WorkflowServiceError::RuntimeNotReady(envelope.message)
        }
        WorkflowErrorCode::SessionNotFound => {
            WorkflowServiceError::SessionNotFound(envelope.message)
        }
        WorkflowErrorCode::SessionEvicted => WorkflowServiceError::SessionEvicted(envelope.message),
        WorkflowErrorCode::QueueItemNotFound => {
            WorkflowServiceError::QueueItemNotFound(envelope.message)
        }
        WorkflowErrorCode::SchedulerBusy => WorkflowServiceError::SchedulerBusy(envelope.message),
        WorkflowErrorCode::OutputNotProduced => {
            WorkflowServiceError::OutputNotProduced(envelope.message)
        }
        WorkflowErrorCode::RuntimeTimeout => WorkflowServiceError::RuntimeTimeout(envelope.message),
        WorkflowErrorCode::InternalError => WorkflowServiceError::Internal(envelope.message),
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
                WorkflowServiceError::Internal(format!("invalid outputs[{}].node_id", index))
            })?
            .to_string();
        let port_id = output
            .get("port_id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                WorkflowServiceError::Internal(format!("invalid outputs[{}].port_id", index))
            })?
            .to_string();

        let value = output.get("value").cloned().ok_or_else(|| {
            WorkflowServiceError::Internal(format!("missing outputs[{}].value", index))
        })?;

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
    let parsed =
        reqwest::Url::parse(&trimmed).map_err(|e| FrontendHttpWorkflowHostError::InvalidUrl {
            base_url: trimmed.clone(),
            reason: e.to_string(),
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
        WorkflowErrorCode, WorkflowOutputTarget, WorkflowRunRequest, WorkflowService,
        WorkflowServiceError,
    };
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_workflow_outputs_payload_rejects_missing_fields() {
        let payload = serde_json::json!({
            "outputs": [{ "node_id": "node-1", "value": "oops" }]
        });
        let err =
            parse_workflow_outputs_payload(&payload).expect_err("must reject malformed outputs");
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
        let normalized =
            normalize_base_url("http://127.0.0.1:8080/".to_string()).expect("normalize");
        assert_eq!(normalized, "http://127.0.0.1:8080");
    }

    #[test]
    fn normalize_backend_runtime_id_stabilizes_backend_aliases() {
        assert_eq!(
            normalize_runtime_identifier_with_fallback(" llama.cpp ", DEFAULT_BACKEND_NAME),
            "llama_cpp"
        );
        assert_eq!(
            normalize_runtime_identifier_with_fallback("OpenAI Compatible", DEFAULT_BACKEND_NAME),
            "openai_compatible"
        );
        assert_eq!(
            normalize_runtime_identifier_with_fallback("", DEFAULT_BACKEND_NAME),
            "openai_compatible"
        );
    }

    #[test]
    fn backend_key_aliases_include_stable_and_display_forms() {
        let aliases = backend_key_aliases("llama.cpp", "llama_cpp");
        assert_eq!(
            aliases,
            vec![
                "llama.cpp".to_string(),
                "llama_cpp".to_string(),
                "llamacpp".to_string()
            ]
        );
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
                    override_selection: None,
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("missing output should return output_not_produced");

        server_thread.join().expect("join server");
        assert!(matches!(err, WorkflowServiceError::OutputNotProduced(_)));
    }

    #[tokio::test]
    async fn workflow_run_maps_non_2xx_error_envelope_to_service_error() {
        let workflow_id = "wf-runtime-not-ready";
        let workflow_root = create_temp_workflow_root_with_output(workflow_id)
            .join(".pantograph")
            .join("workflows");
        let payload = serde_json::json!({
            "code": "runtime_not_ready",
            "message": "backend unavailable"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(503, payload);

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
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("503 envelope should map to runtime_not_ready");

        server_thread.join().expect("join server");
        match err {
            WorkflowServiceError::RuntimeNotReady(message) => {
                assert_eq!(message, "backend unavailable");
            }
            other => panic!("expected runtime_not_ready, got {}", other),
        }
    }

    #[tokio::test]
    async fn workflow_run_rejects_non_envelope_non_2xx_error_payload() {
        let workflow_id = "wf-malformed-error";
        let workflow_root = create_temp_workflow_root_with_output(workflow_id)
            .join(".pantograph")
            .join("workflows");
        let payload = serde_json::json!({
            "error": "backend unavailable"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(502, payload);

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
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("non-envelope errors must not be silently remapped");

        server_thread.join().expect("join server");
        assert!(matches!(err, WorkflowServiceError::Internal(_)));
        assert!(err
            .to_string()
            .contains("expected workflow error envelope JSON"));
    }

    #[tokio::test]
    async fn frontend_http_host_reports_stable_runtime_identity() {
        let host = FrontendHttpWorkflowHost::new(
            "http://127.0.0.1:8080".to_string(),
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            "llama.cpp".to_string(),
        )
        .expect("build frontend host");

        assert_eq!(
            host.default_backend_name().await.expect("default backend"),
            "llama_cpp"
        );

        let runtime = host
            .runtime_capabilities()
            .await
            .expect("runtime capabilities");
        assert_eq!(runtime.len(), 1);
        assert_eq!(runtime[0].runtime_id, "llama_cpp");
        assert_eq!(runtime[0].display_name, "llama.cpp");
        assert_eq!(runtime[0].source_kind, WorkflowRuntimeSourceKind::Host);
        assert!(runtime[0].selected);
        assert!(runtime[0].supports_external_connection);
        assert_eq!(
            runtime[0].backend_keys,
            vec![
                "llama.cpp".to_string(),
                "llama_cpp".to_string(),
                "llamacpp".to_string()
            ]
        );
    }

    #[test]
    fn map_workflow_error_envelope_maps_all_codes() {
        let cases = [
            (
                WorkflowErrorCode::InvalidRequest,
                WorkflowServiceError::InvalidRequest("x".to_string()),
            ),
            (
                WorkflowErrorCode::WorkflowNotFound,
                WorkflowServiceError::WorkflowNotFound("x".to_string()),
            ),
            (
                WorkflowErrorCode::CapabilityViolation,
                WorkflowServiceError::CapabilityViolation("x".to_string()),
            ),
            (
                WorkflowErrorCode::RuntimeNotReady,
                WorkflowServiceError::RuntimeNotReady("x".to_string()),
            ),
            (
                WorkflowErrorCode::SessionNotFound,
                WorkflowServiceError::SessionNotFound("x".to_string()),
            ),
            (
                WorkflowErrorCode::SessionEvicted,
                WorkflowServiceError::SessionEvicted("x".to_string()),
            ),
            (
                WorkflowErrorCode::QueueItemNotFound,
                WorkflowServiceError::QueueItemNotFound("x".to_string()),
            ),
            (
                WorkflowErrorCode::SchedulerBusy,
                WorkflowServiceError::SchedulerBusy("x".to_string()),
            ),
            (
                WorkflowErrorCode::OutputNotProduced,
                WorkflowServiceError::OutputNotProduced("x".to_string()),
            ),
            (
                WorkflowErrorCode::RuntimeTimeout,
                WorkflowServiceError::RuntimeTimeout("x".to_string()),
            ),
            (
                WorkflowErrorCode::InternalError,
                WorkflowServiceError::Internal("x".to_string()),
            ),
        ];

        for (code, expected) in cases {
            let mapped = map_workflow_error_envelope(WorkflowErrorEnvelope {
                code,
                message: "x".to_string(),
            });
            assert_eq!(mapped.to_envelope(), expected.to_envelope());
        }
    }
}
