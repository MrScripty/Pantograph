use super::*;
use crate::WorkflowSchedulerRuntimeCapacityPressure;
use crate::technical_fit::{
    WorkflowTechnicalFitReason, WorkflowTechnicalFitReasonCode, WorkflowTechnicalFitSelectionMode,
};
use crate::{WorkflowGraph, WorkflowGraphEditSessionCreateRequest};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

mod contracts;
mod runtime_preflight;
mod scheduler_snapshot;
mod session_admission;
mod session_capacity;
mod session_execution;
mod session_queue;
mod session_runtime_preflight;
mod session_stale_cleanup;
mod workflow_io;
mod workflow_preflight;
mod workflow_run;

struct MockWorkflowHost {
    capabilities: WorkflowHostCapabilities,
    omit_requested_target_output: bool,
    emit_invalid_output_binding: bool,
    technical_fit_decision: Option<WorkflowTechnicalFitDecision>,
    recorded_run_options: Arc<Mutex<Vec<WorkflowRunOptions>>>,
}

impl MockWorkflowHost {
    fn new(max_input_bindings: usize, max_value_bytes: usize) -> Self {
        Self {
            capabilities: WorkflowHostCapabilities {
                max_input_bindings,
                max_output_targets: 16,
                max_value_bytes,
                runtime_requirements: WorkflowRuntimeRequirements {
                    estimated_peak_vram_mb: Some(1024),
                    estimated_peak_ram_mb: Some(2048),
                    estimated_min_vram_mb: Some(512),
                    estimated_min_ram_mb: Some(1024),
                    estimation_confidence: "estimated".to_string(),
                    required_models: vec!["model-a".to_string()],
                    required_backends: vec!["llama_cpp".to_string()],
                    required_extensions: vec!["inference_gateway".to_string()],
                },
                models: vec![WorkflowCapabilityModel {
                    model_id: "model-a".to_string(),
                    model_revision_or_hash: Some("sha256:hash-model-a".to_string()),
                    model_type: Some("embedding".to_string()),
                    node_ids: vec!["node-1".to_string()],
                    roles: vec!["embedding".to_string(), "inference".to_string()],
                }],
                runtime_capabilities: vec![ready_runtime_capability()],
            },
            omit_requested_target_output: false,
            emit_invalid_output_binding: false,
            technical_fit_decision: None,
            recorded_run_options: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_missing_requested_output(max_input_bindings: usize, max_value_bytes: usize) -> Self {
        Self {
            omit_requested_target_output: true,
            ..Self::new(max_input_bindings, max_value_bytes)
        }
    }

    fn with_invalid_output_binding(max_input_bindings: usize, max_value_bytes: usize) -> Self {
        Self {
            emit_invalid_output_binding: true,
            ..Self::new(max_input_bindings, max_value_bytes)
        }
    }

    fn with_technical_fit_decision(
        max_input_bindings: usize,
        max_value_bytes: usize,
        technical_fit_decision: WorkflowTechnicalFitDecision,
    ) -> Self {
        Self {
            technical_fit_decision: Some(technical_fit_decision),
            ..Self::new(max_input_bindings, max_value_bytes)
        }
    }
}

struct InspectionHost {
    calls: Arc<Mutex<Vec<(String, String)>>>,
    state: Option<WorkflowGraphSessionStateView>,
}

#[async_trait]
impl WorkflowHost for InspectionHost {
    async fn workflow_session_inspection_state(
        &self,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<Option<WorkflowGraphSessionStateView>, WorkflowServiceError> {
        self.calls
            .lock()
            .expect("inspection host calls lock poisoned")
            .push((session_id.to_string(), workflow_id.to_string()));
        Ok(self.state.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        unreachable!("inspection host does not execute workflow runs")
    }
}

fn ready_runtime_capability() -> WorkflowRuntimeCapability {
    WorkflowRuntimeCapability {
        runtime_id: "llama_cpp".to_string(),
        display_name: "llama.cpp".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: true,
        source_kind: WorkflowRuntimeSourceKind::Managed,
        selected: true,
        readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
        selected_version: Some("b8248".to_string()),
        supports_external_connection: true,
        backend_keys: vec!["llamacpp".to_string(), "llama.cpp".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }
}

fn ready_pytorch_runtime_capability() -> WorkflowRuntimeCapability {
    WorkflowRuntimeCapability {
        runtime_id: "pytorch".to_string(),
        display_name: "PyTorch".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: true,
        source_kind: WorkflowRuntimeSourceKind::Managed,
        selected: true,
        readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
        selected_version: Some("2.6.0".to_string()),
        supports_external_connection: true,
        backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }
}

struct TimeoutAwareHost {
    cancelled: Arc<AtomicBool>,
    capabilities: WorkflowHostCapabilities,
}

impl TimeoutAwareHost {
    fn new(cancelled: Arc<AtomicBool>) -> Self {
        Self {
            cancelled,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: Vec::new(),
            },
        }
    }
}

#[derive(Clone)]
struct BlockingRunHost {
    capabilities: WorkflowHostCapabilities,
    started_runs: Arc<AtomicUsize>,
    first_run_started: Arc<Notify>,
    release_first_run: Arc<Notify>,
}

impl BlockingRunHost {
    fn new() -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            started_runs: Arc::new(AtomicUsize::new(0)),
            first_run_started: Arc::new(Notify::new()),
            release_first_run: Arc::new(Notify::new()),
        }
    }

    async fn wait_for_first_run_started(&self) {
        if self.started_runs.load(Ordering::SeqCst) > 0 {
            return;
        }
        self.first_run_started.notified().await;
    }

    fn release_first_run(&self) {
        self.release_first_run.notify_waiters();
    }
}

#[derive(Clone)]
struct AdmissionGatedHost {
    capabilities: WorkflowHostCapabilities,
    admission_open: Arc<AtomicBool>,
}

impl AdmissionGatedHost {
    fn new(admission_open: Arc<AtomicBool>) -> Self {
        Self {
            capabilities: MockWorkflowHost::new(8, 1024).capabilities,
            admission_open,
        }
    }
}

struct RecordingRuntimeHost {
    retention_hints: Arc<Mutex<Vec<WorkflowSessionRetentionHint>>>,
    capabilities: WorkflowHostCapabilities,
}

impl RecordingRuntimeHost {
    fn new(retention_hints: Arc<Mutex<Vec<WorkflowSessionRetentionHint>>>) -> Self {
        Self {
            retention_hints,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![ready_runtime_capability()],
            },
        }
    }
}

struct SelectingRuntimeHost {
    selected_session_id: String,
    unloads: Arc<Mutex<Vec<(String, WorkflowSessionUnloadReason)>>>,
    capabilities: WorkflowHostCapabilities,
}

impl SelectingRuntimeHost {
    fn new(
        selected_session_id: String,
        unloads: Arc<Mutex<Vec<(String, WorkflowSessionUnloadReason)>>>,
    ) -> Self {
        Self {
            selected_session_id,
            unloads,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![ready_runtime_capability()],
            },
        }
    }
}

struct AffinityRuntimeHost {
    unloads: Arc<Mutex<Vec<String>>>,
    capabilities: WorkflowHostCapabilities,
    required_backends_by_workflow: HashMap<String, Vec<String>>,
    required_models_by_workflow: HashMap<String, Vec<String>>,
}

impl AffinityRuntimeHost {
    fn new(unloads: Arc<Mutex<Vec<String>>>) -> Self {
        Self {
            unloads,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![
                    ready_runtime_capability(),
                    ready_pytorch_runtime_capability(),
                ],
            },
            required_backends_by_workflow: HashMap::new(),
            required_models_by_workflow: HashMap::new(),
        }
    }

    fn with_runtime_affinity(
        unloads: Arc<Mutex<Vec<String>>>,
        required_backends_by_workflow: HashMap<String, Vec<String>>,
        required_models_by_workflow: HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            unloads,
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: vec![
                    ready_runtime_capability(),
                    ready_pytorch_runtime_capability(),
                ],
            },
            required_backends_by_workflow,
            required_models_by_workflow,
        }
    }
}

struct PreflightHost {
    capabilities: WorkflowHostCapabilities,
    technical_fit_decision: Option<WorkflowTechnicalFitDecision>,
}

impl PreflightHost {
    fn new() -> Self {
        Self {
            capabilities: WorkflowHostCapabilities {
                max_input_bindings: 16,
                max_output_targets: 16,
                max_value_bytes: 4096,
                runtime_requirements: WorkflowRuntimeRequirements::default(),
                models: Vec::new(),
                runtime_capabilities: Vec::new(),
            },
            technical_fit_decision: None,
        }
    }

    fn with_technical_fit_decision(
        capabilities: WorkflowHostCapabilities,
        technical_fit_decision: WorkflowTechnicalFitDecision,
    ) -> Self {
        Self {
            capabilities,
            technical_fit_decision: Some(technical_fit_decision),
        }
    }
}

struct DefaultCapabilitiesHost {
    workflow_root: PathBuf,
}

struct CountingPreflightHost {
    workflow_capabilities_calls: Arc<AtomicUsize>,
    runtime_capabilities_calls: Arc<AtomicUsize>,
    graph_fingerprint: Arc<Mutex<String>>,
    technical_fit_requests: Arc<Mutex<Vec<WorkflowTechnicalFitRequest>>>,
}

#[derive(Clone)]
struct MockSchedulerDiagnosticsProvider {
    diagnostics: WorkflowSchedulerRuntimeRegistryDiagnostics,
    requests: Arc<Mutex<Vec<WorkflowSchedulerRuntimeDiagnosticsRequest>>>,
}

#[async_trait]
impl WorkflowSchedulerDiagnosticsProvider for MockSchedulerDiagnosticsProvider {
    async fn scheduler_runtime_registry_diagnostics(
        &self,
        request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
    ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError> {
        self.requests
            .lock()
            .expect("scheduler diagnostics requests lock poisoned")
            .push(request.clone());
        Ok(Some(self.diagnostics.clone()))
    }
}

#[async_trait]
impl WorkflowHost for DefaultCapabilitiesHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        vec![self.workflow_root.clone()]
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok("fallback-backend".to_string())
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        if model_id == "model-a" {
            Ok(Some(serde_json::json!({
                "size_bytes": 2_u64 * 1024_u64 * 1024_u64
            })))
        } else {
            Ok(None)
        }
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        if model_id == "model-a" {
            Ok(Some(WorkflowHostModelDescriptor {
                model_type: Some("embedding".to_string()),
                hashes: HashMap::from([
                    ("blake3".to_string(), "bbb".to_string()),
                    ("sha256".to_string(), "abc123".to_string()),
                ]),
            }))
        } else {
            Ok(None)
        }
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            return Ok(targets
                .iter()
                .map(|target| WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value: serde_json::json!("ok"),
                })
                .collect());
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for CountingPreflightHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok(self
            .graph_fingerprint
            .lock()
            .expect("graph fingerprint lock poisoned")
            .clone())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        self.workflow_capabilities_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(WorkflowHostCapabilities {
            max_input_bindings: 8,
            max_output_targets: 8,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                required_backends: vec!["llama_cpp".to_string()],
                ..WorkflowRuntimeRequirements::default()
            },
            models: Vec::new(),
            runtime_capabilities: vec![ready_runtime_capability()],
        })
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        self.runtime_capabilities_calls
            .fetch_add(1, Ordering::SeqCst);
        Ok(vec![ready_runtime_capability()])
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: Vec::new(),
            outputs: vec![WorkflowIoNode {
                node_id: "text-output-1".to_string(),
                node_type: "text-output".to_string(),
                name: Some("Output".to_string()),
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
        })
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }

    async fn workflow_technical_fit_decision(
        &self,
        request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        self.technical_fit_requests
            .lock()
            .expect("technical-fit requests lock poisoned")
            .push(request.clone());
        Ok(None)
    }
}

#[async_trait]
impl WorkflowHost for TimeoutAwareHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("timeout-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        loop {
            if run_handle.is_cancelled() {
                self.cancelled.store(true, Ordering::SeqCst);
                return Err(WorkflowServiceError::Cancelled(
                    "workflow run cancelled".to_string(),
                ));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

#[async_trait]
impl WorkflowHost for BlockingRunHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("blocking-run-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if self.started_runs.fetch_add(1, Ordering::SeqCst) == 0 {
            self.first_run_started.notify_waiters();
            self.release_first_run.notified().await;
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for AdmissionGatedHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("admission-gated-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn can_load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<bool, WorkflowServiceError> {
        Ok(self.admission_open.load(Ordering::SeqCst))
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for RecordingRuntimeHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("recording-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.retention_hints
            .lock()
            .expect("retention hints lock poisoned")
            .push(retention_hint);
        Ok(())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for SelectingRuntimeHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("selection-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn select_runtime_unload_candidate(
        &self,
        _target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        Ok(candidates
            .iter()
            .find(|candidate| candidate.session_id == self.selected_session_id)
            .cloned())
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        reason: WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.unloads
            .lock()
            .expect("unloads lock poisoned")
            .push((session_id.to_string(), reason));
        Ok(())
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for AffinityRuntimeHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("affinity-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        let mut capabilities = self.capabilities.clone();
        capabilities.runtime_requirements.required_backends = self
            .required_backends_by_workflow
            .get(workflow_id)
            .cloned()
            .unwrap_or_default();
        capabilities.runtime_requirements.required_models = self
            .required_models_by_workflow
            .get(workflow_id)
            .cloned()
            .unwrap_or_default();
        Ok(capabilities)
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        _reason: WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.unloads
            .lock()
            .expect("unloads lock poisoned")
            .push(session_id.to_string());
        Ok(())
    }

    async fn load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for MockWorkflowHost {
    fn max_input_bindings(&self) -> usize {
        self.capabilities.max_input_bindings
    }

    fn max_output_targets(&self) -> usize {
        self.capabilities.max_output_targets
    }

    fn max_value_bytes(&self) -> usize {
        self.capabilities.max_value_bytes
    }

    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        if workflow_id == "wf-missing" {
            return Err(WorkflowServiceError::WorkflowNotFound(
                workflow_id.to_string(),
            ));
        }
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("mock-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: vec![WorkflowIoNode {
                node_id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                name: Some("Input".to_string()),
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "text-output-1".to_string(),
                node_type: "text-output".to_string(),
                name: Some("Output".to_string()),
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
        })
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(self.capabilities.runtime_capabilities.clone())
    }

    async fn workflow_technical_fit_decision(
        &self,
        _request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        Ok(self.technical_fit_decision.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        self.recorded_run_options
            .lock()
            .expect("run options lock poisoned")
            .push(run_options);

        if inputs.iter().any(|binding| {
            binding
                .value
                .as_str()
                .map(|value| value.contains("runtime-error"))
                .unwrap_or(false)
        }) {
            return Err(WorkflowServiceError::RuntimeNotReady(
                "backend not ready".to_string(),
            ));
        }

        if let Some(targets) = output_targets {
            if self.omit_requested_target_output && !targets.is_empty() {
                return Ok(Vec::new());
            }
            let mut outputs = Vec::with_capacity(targets.len());
            for target in targets {
                let value = inputs
                    .iter()
                    .find(|binding| {
                        binding.node_id == target.node_id && binding.port_id == target.port_id
                    })
                    .map(|binding| binding.value.clone())
                    .unwrap_or(serde_json::Value::Null);

                outputs.push(WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value,
                });
            }
            return Ok(outputs);
        }

        if self.emit_invalid_output_binding {
            return Ok(vec![WorkflowPortBinding {
                node_id: "text-output-1".to_string(),
                port_id: String::new(),
                value: serde_json::json!("invalid"),
            }]);
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("default output"),
        }])
    }
}

#[async_trait]
impl WorkflowHost for PreflightHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("preflight-graph".to_string())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(self.capabilities.clone())
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: vec![WorkflowIoNode {
                node_id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                name: Some("Prompt".to_string()),
                description: None,
                ports: vec![
                    WorkflowIoPort {
                        port_id: "text".to_string(),
                        name: Some("Text".to_string()),
                        description: None,
                        data_type: Some("string".to_string()),
                        required: Some(true),
                        multiple: Some(false),
                    },
                    WorkflowIoPort {
                        port_id: "tone".to_string(),
                        name: Some("Tone".to_string()),
                        description: None,
                        data_type: Some("string".to_string()),
                        required: None,
                        multiple: Some(false),
                    },
                ],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "text-output-1".to_string(),
                node_type: "text-output".to_string(),
                name: Some("Answer".to_string()),
                description: None,
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
        })
    }

    async fn workflow_technical_fit_decision(
        &self,
        _request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        Ok(self.technical_fit_decision.clone())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        _output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        _run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        Ok(vec![WorkflowPortBinding {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("ok"),
        }])
    }
}

#[tokio::test]
async fn capabilities_returns_host_capabilities() {
    let host = MockWorkflowHost::new(8, 4096);
    let service = WorkflowService::new();
    let response = service
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("capabilities");

    assert_eq!(response.max_input_bindings, 8);
    assert_eq!(response.max_output_targets, 16);
    assert_eq!(response.max_value_bytes, 4096);
    assert_eq!(
        response.runtime_requirements.estimated_peak_ram_mb,
        Some(2048)
    );
    assert_eq!(response.runtime_requirements.required_models.len(), 1);
    assert_eq!(response.models.len(), 1);
    assert_eq!(response.models[0].model_id, "model-a");
}

#[tokio::test]
async fn workflow_session_create_returns_scheduler_busy_at_capacity() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(1);

    let _first = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first");

    let err = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect_err("second session should fail at capacity");
    assert_eq!(
        err.to_envelope().details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::session_capacity_reached(1, 1),
        ))
    );
}

#[tokio::test]
async fn workflow_session_capacity_is_released_after_close() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(1);
    let first = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let err = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect_err("scheduler should be busy at session capacity");
    assert_eq!(
        err.to_envelope().details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::session_capacity_reached(1, 1),
        ))
    );

    let closed = service
        .close_workflow_session(
            &host,
            WorkflowSessionCloseRequest {
                session_id: first.session_id,
            },
        )
        .await
        .expect("close session");
    assert!(closed.ok);

    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session after close");

    let status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("get status");
    assert_eq!(status.session.session_kind, WorkflowSessionKind::Workflow);
    assert!(!status.session.keep_alive);
}

#[tokio::test]
async fn workflow_session_create_surfaces_runtime_capacity_details_when_no_unload_candidate_available()
 {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(2, 1);
    let loaded = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-loaded".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create loaded keep-alive session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let queue_id = store
            .enqueue_run(
                &loaded.session_id,
                &WorkflowSessionRunRequest {
                    session_id: loaded.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("run-loaded".to_string()),
                    priority: None,
                },
            )
            .expect("enqueue run for loaded session");
        let dequeued = store
            .begin_queued_run(&loaded.session_id, &queue_id)
            .expect("begin queued run");
        assert!(
            dequeued.is_some(),
            "loaded session should transition into an active run"
        );
    }

    let err = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-blocked".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect_err("second keep-alive session should fail while loaded capacity is pinned");
    assert_eq!(
        err.to_envelope().details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(1, 1, 0),
        ))
    );
}

#[tokio::test]
async fn invalidate_all_session_runtimes_clears_loaded_state_for_active_sessions() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let first = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create first session");
    let second = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-2".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create second session");
    let third = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-3".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create third session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .mark_runtime_loaded(&first.session_id, true)
            .expect("mark first runtime loaded");
        store
            .mark_runtime_loaded(&second.session_id, true)
            .expect("mark second runtime loaded");
    }

    let mut invalidated = service
        .invalidate_all_session_runtimes()
        .expect("invalidate session runtimes");
    invalidated.sort();

    let mut expected = vec![first.session_id.clone(), second.session_id.clone()];
    expected.sort();
    assert_eq!(invalidated, expected);

    let first_status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: first.session_id,
        })
        .await
        .expect("first session status");
    let second_status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: second.session_id,
        })
        .await
        .expect("second session status");
    let third_status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: third.session_id,
        })
        .await
        .expect("third session status");

    assert_eq!(
        first_status.session.state,
        WorkflowSessionState::IdleUnloaded
    );
    assert_eq!(
        second_status.session.state,
        WorkflowSessionState::IdleUnloaded
    );
    assert_eq!(
        third_status.session.state,
        WorkflowSessionState::IdleUnloaded
    );
}

#[tokio::test]
async fn default_capabilities_derive_runtime_requirements_from_workflow() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-service-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-default.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": {
                "name": "Default Capability Test"
            },
            "graph": {
                "nodes": [
                    {
                        "id": "node-1",
                        "node_type": "text-input",
                        "data": {
                            "model_id": "model-a",
                            "backend_key": "llamacpp",
                            "embedding": true
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    }
                ],
                "edges": []
            }
        })
        .to_string(),
    )
    .expect("write workflow");

    let host = DefaultCapabilitiesHost { workflow_root };
    let response = WorkflowService::new()
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest {
                workflow_id: "wf-default".to_string(),
            },
        )
        .await
        .expect("capabilities response");

    assert_eq!(
        response.max_input_bindings,
        capabilities::DEFAULT_MAX_INPUT_BINDINGS
    );
    assert_eq!(
        response.max_output_targets,
        capabilities::DEFAULT_MAX_OUTPUT_TARGETS
    );
    assert_eq!(
        response.max_value_bytes,
        capabilities::DEFAULT_MAX_VALUE_BYTES
    );
    assert_eq!(
        response.runtime_requirements.required_models,
        vec!["model-a"]
    );
    assert_eq!(
        response.runtime_requirements.required_backends,
        vec!["llama_cpp"]
    );
    assert_eq!(
        response.runtime_requirements.required_extensions,
        vec!["inference_gateway".to_string(), "pumas_api".to_string()]
    );
    assert_eq!(response.models.len(), 1);
    assert_eq!(response.models[0].model_id, "model-a");
    assert_eq!(response.models[0].model_type.as_deref(), Some("embedding"));
    assert_eq!(
        response.models[0].model_revision_or_hash.as_deref(),
        Some("sha256:abc123")
    );
    assert_eq!(response.models[0].node_ids, vec!["node-1".to_string()]);
    assert_eq!(response.models[0].roles, vec!["embedding".to_string()]);
    assert_eq!(response.runtime_requirements.estimated_peak_ram_mb, Some(2));
    assert_eq!(response.runtime_requirements.estimated_min_ram_mb, Some(2));
    assert_eq!(
        response.runtime_requirements.estimation_confidence,
        "estimated_from_model_sizes"
    );

    let _ = fs::remove_dir_all(temp_root);
}
