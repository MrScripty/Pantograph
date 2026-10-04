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
    capabilities, WorkflowErrorCode, WorkflowErrorDetails, WorkflowErrorEnvelope, WorkflowHost,
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
            readiness_state: Some(
                pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
            ),
            selected_version: None,
            supports_external_connection: true,
            backend_keys: self.backend_keys.clone(),
            backend_capability_facts: None,
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
            return Err(WorkflowServiceError::Cancelled(
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
                WorkflowServiceError::Cancelled("workflow run cancelled".to_string())
            } else {
                WorkflowServiceError::RuntimeNotReady(e.to_string())
            }
        })?;
        if !response.status().is_success() {
            return Err(workflow_error_from_http_response(response).await?);
        }

        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;

        parse_workflow_outputs_payload(&payload)
    }
}

async fn workflow_error_from_http_response(
    response: reqwest::Response,
) -> Result<WorkflowServiceError, WorkflowServiceError> {
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
    Ok(map_workflow_error_envelope(envelope))
}

fn map_workflow_error_envelope(envelope: WorkflowErrorEnvelope) -> WorkflowServiceError {
    let scheduler_details = envelope.details.and_then(|details| match details {
        WorkflowErrorDetails::Scheduler(details) => Some(details),
        WorkflowErrorDetails::Graph(_) => None,
    });
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
        WorkflowErrorCode::Cancelled => WorkflowServiceError::Cancelled(envelope.message),
        WorkflowErrorCode::SessionNotFound => {
            WorkflowServiceError::SessionNotFound(envelope.message)
        }
        WorkflowErrorCode::SessionEvicted => WorkflowServiceError::SessionEvicted(envelope.message),
        WorkflowErrorCode::QueueItemNotFound => {
            WorkflowServiceError::QueueItemNotFound(envelope.message)
        }
        WorkflowErrorCode::SchedulerBusy => {
            if let Some(details) = scheduler_details {
                WorkflowServiceError::scheduler_busy_with_details(envelope.message, details)
            } else {
                WorkflowServiceError::scheduler_busy(envelope.message)
            }
        }
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
        WorkflowErrorCode, WorkflowGraphErrorDetails, WorkflowOutputTarget, WorkflowServiceError,
    };
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

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

    fn spawn_single_workflow_server(
        status_code: u16,
        body: serde_json::Value,
    ) -> (String, std::thread::JoinHandle<serde_json::Value>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking accept");
        let addr = listener.local_addr().expect("local addr");
        let body_text = body.to_string();
        let reason = if status_code == 200 { "OK" } else { "ERROR" };

        let handle = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "HTTP transport was never called before the test-server deadline"
                        );
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("test server accept failed: {error}"),
                }
            };
            stream.set_nonblocking(false).expect("set blocking read");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set timeout");
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            reader.read_line(&mut line).expect("read request line");
            assert_eq!(line, "POST /v1/workflow/run HTTP/1.1\r\n");
            let mut content_length = None;
            loop {
                line.clear();
                assert!(reader.read_line(&mut line).expect("read request header") > 0);
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length =
                            Some(value.trim().parse::<usize>().expect("content length"));
                    }
                }
            }
            let content_length = content_length.expect("JSON request has content length");
            assert!(content_length <= 8192, "unexpected test request size");
            let mut request_body = vec![0_u8; content_length];
            reader
                .read_exact(&mut request_body)
                .expect("read complete JSON request");
            drop(reader);

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
            serde_json::from_slice(&request_body).expect("request body is JSON")
        });

        (format!("http://{}", addr), handle)
    }

    #[tokio::test]
    async fn transport_preserves_empty_outputs_for_a_requested_target() {
        let workflow_id = "wf-output-not-produced";

        let payload = serde_json::json!({
            "workflow_run_id": "adapter-run-1",
            "outputs": [],
            "timing_ms": 2
        });
        let (base_url, server_thread) = spawn_single_workflow_server(200, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let outputs = host
            .run_workflow(
                workflow_id,
                &[],
                Some(&[WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                WorkflowRunOptions {
                    timeout_ms: Some(2_000),
                    ..WorkflowRunOptions::default()
                },
                WorkflowRunHandle::new(),
            )
            .await
            .expect("transport preserves a valid empty outputs array");

        let request = server_thread.join().expect("join server");
        assert_eq!(
            request,
            serde_json::json!({
                "workflow_id": workflow_id,
                "inputs": [],
                "output_targets": [{"node_id": "vector-output-1", "port_id": "vector"}],
            })
        );
        assert!(outputs.is_empty());
    }

    #[tokio::test]
    async fn transport_maps_output_not_produced_error_envelope() {
        let workflow_id = "wf-output-not-produced";
        let payload = serde_json::json!({
            "code": "output_not_produced",
            "message": "requested output target 'vector-output-1.vector' was not produced"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(422, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let err = host
            .run_workflow(
                workflow_id,
                &[],
                None,
                WorkflowRunOptions {
                    timeout_ms: Some(2_000),
                    ..WorkflowRunOptions::default()
                },
                WorkflowRunHandle::new(),
            )
            .await
            .expect_err("422 envelope should map to output_not_produced");

        server_thread.join().expect("join server");
        match err {
            WorkflowServiceError::OutputNotProduced(message) => {
                assert_eq!(
                    message,
                    "requested output target 'vector-output-1.vector' was not produced"
                );
            }
            other => panic!("expected output_not_produced, got {}", other),
        }
    }

    #[tokio::test]
    async fn workflow_run_maps_non_2xx_error_envelope_to_service_error() {
        let workflow_id = "wf-runtime-not-ready";
        let payload = serde_json::json!({
            "code": "runtime_not_ready",
            "message": "backend unavailable"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(503, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let err = host
            .run_workflow(
                workflow_id,
                &[],
                None,
                WorkflowRunOptions {
                    timeout_ms: Some(2_000),
                    ..WorkflowRunOptions::default()
                },
                WorkflowRunHandle::new(),
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
    async fn workflow_run_maps_cancelled_error_envelope_to_service_error() {
        let workflow_id = "wf-cancelled";
        let payload = serde_json::json!({
            "code": "cancelled",
            "message": "workflow run cancelled"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(409, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let err = host
            .run_workflow(
                workflow_id,
                &[],
                None,
                WorkflowRunOptions {
                    timeout_ms: Some(2_000),
                    ..WorkflowRunOptions::default()
                },
                WorkflowRunHandle::new(),
            )
            .await
            .expect_err("409 cancelled envelope should map to cancelled");

        server_thread.join().expect("join server");
        match err {
            WorkflowServiceError::Cancelled(message) => {
                assert_eq!(message, "workflow run cancelled");
            }
            other => panic!("expected cancelled, got {}", other),
        }
    }

    #[tokio::test]
    async fn workflow_run_maps_invalid_request_error_envelope_to_service_error() {
        let workflow_id = "wf-invalid-request";
        let payload = serde_json::json!({
            "code": "invalid_request",
            "message": "workflow requires interactive input"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(400, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let err = host
            .run_workflow(
                workflow_id,
                &[],
                None,
                WorkflowRunOptions {
                    timeout_ms: Some(2_000),
                    ..WorkflowRunOptions::default()
                },
                WorkflowRunHandle::new(),
            )
            .await
            .expect_err("400 invalid_request envelope should map to invalid request");

        server_thread.join().expect("join server");
        match err {
            WorkflowServiceError::InvalidRequest(message) => {
                assert_eq!(message, "workflow requires interactive input");
            }
            other => panic!("expected invalid request, got {}", other),
        }
    }

    #[tokio::test]
    async fn workflow_run_rejects_non_envelope_non_2xx_error_payload() {
        let workflow_id = "wf-malformed-error";
        let payload = serde_json::json!({
            "error": "backend unavailable"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(502, payload);

        let host = FrontendHttpWorkflowHost::new(
            base_url,
            None,
            Vec::new(),
            DEFAULT_MAX_INPUT_BINDINGS,
            DEFAULT_MAX_OUTPUT_TARGETS,
            DEFAULT_MAX_VALUE_BYTES,
            DEFAULT_BACKEND_NAME.to_string(),
        )
        .expect("build frontend host");

        let err = host
            .run_workflow(
                workflow_id,
                &[],
                None,
                WorkflowRunOptions {
                    timeout_ms: Some(2_000),
                    ..WorkflowRunOptions::default()
                },
                WorkflowRunHandle::new(),
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
                WorkflowErrorCode::Cancelled,
                WorkflowServiceError::Cancelled("x".to_string()),
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
                WorkflowServiceError::scheduler_busy("x"),
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
                details: None,
                diagnostics: None,
            });
            assert_eq!(mapped.to_envelope(), expected.to_envelope());
        }
    }

    #[test]
    fn map_workflow_error_envelope_ignores_graph_details_for_scheduler_busy() {
        let mapped = map_workflow_error_envelope(WorkflowErrorEnvelope {
            code: WorkflowErrorCode::SchedulerBusy,
            message: "x".to_string(),
            details: Some(WorkflowErrorDetails::Graph(WorkflowGraphErrorDetails {
                graph_diagnostics: Vec::new(),
            })),
            diagnostics: None,
        });

        assert_eq!(
            mapped.to_envelope(),
            WorkflowServiceError::scheduler_busy("x").to_envelope()
        );
    }

    #[tokio::test]
    async fn workflow_error_from_http_response_rejects_non_envelope_payloads() {
        let payload = serde_json::json!({
            "error": "backend unavailable"
        });
        let (base_url, server_thread) = spawn_single_workflow_server(502, payload);

        let response = reqwest::Client::new()
            .post(format!("{}/v1/workflow/run", base_url))
            .json(&serde_json::json!({}))
            .send()
            .await
            .expect("request should succeed");

        let error = workflow_error_from_http_response(response)
            .await
            .expect_err("non-envelope payload should be rejected");

        server_thread.join().expect("join server");
        assert!(matches!(error, WorkflowServiceError::Internal(_)));
        assert!(error
            .to_string()
            .contains("expected workflow error envelope JSON"));
    }
}
