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

mod scheduler_snapshot;
mod session_queue;

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
                runtime_capabilities: vec![ready_runtime_capability()],
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
                runtime_capabilities: vec![ready_runtime_capability()],
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

#[test]
fn request_roundtrip_uses_snake_case() {
    let req = WorkflowRunRequest {
        workflow_id: "wf-1".to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("hello"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
        override_selection: Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama.cpp".to_string()),
        }),
        timeout_ms: None,
        run_id: Some("run-1".to_string()),
    };

    let json = serde_json::to_value(&req).expect("serialize request");
    assert_eq!(json["workflow_id"], "wf-1");
    assert_eq!(json["inputs"][0]["node_id"], "input-1");
    assert_eq!(json["output_targets"][0]["port_id"], "text");
    assert_eq!(json["override_selection"]["model_id"], "model-a");
    assert_eq!(json["override_selection"]["backend_key"], "llama.cpp");
}

#[test]
fn response_roundtrip_preserves_outputs() {
    let res = WorkflowRunResponse {
        run_id: "run-1".to_string(),
        outputs: vec![WorkflowPortBinding {
            node_id: "vector-output-1".to_string(),
            port_id: "vector".to_string(),
            value: serde_json::json!([0.1, 0.2, 0.3]),
        }],
        timing_ms: 5,
    };

    let json = serde_json::to_string(&res).expect("serialize response");
    let parsed: WorkflowRunResponse = serde_json::from_str(&json).expect("parse response");
    assert_eq!(parsed.run_id, "run-1");
    assert_eq!(parsed.outputs[0].node_id, "vector-output-1");
}

#[test]
fn workflow_io_roundtrip_uses_snake_case() {
    let response = WorkflowIoResponse {
        inputs: vec![WorkflowIoNode {
            node_id: "text-input-1".to_string(),
            node_type: "text-input".to_string(),
            name: Some("Prompt".to_string()),
            description: Some("Prompt input".to_string()),
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
    };

    let json = serde_json::to_value(&response).expect("serialize workflow io");
    assert_eq!(json["inputs"][0]["node_id"], "text-input-1");
    assert_eq!(json["outputs"][0]["ports"][0]["port_id"], "text");

    let parsed: WorkflowIoResponse =
        serde_json::from_value(json).expect("parse workflow io response");
    assert_eq!(parsed.inputs[0].name.as_deref(), Some("Prompt"));
    assert_eq!(
        parsed.outputs[0].ports[0].data_type.as_deref(),
        Some("string")
    );
}

#[tokio::test]
async fn workflow_run_returns_host_outputs() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();
    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: Some("run-xyz".to_string()),
            },
        )
        .await
        .expect("workflow_run");

    assert_eq!(response.run_id, "run-xyz");
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].value, serde_json::json!("hello world"));
}

#[tokio::test]
async fn workflow_run_fails_when_host_returns_runtime_error() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("runtime-error object"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("expected runtime error");

    assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
}

#[tokio::test]
async fn workflow_run_honors_blocking_backend_technical_fit_decision() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        10,
        256,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![
                WorkflowTechnicalFitReason::new(
                    WorkflowTechnicalFitReasonCode::MissingRuntimeState,
                    None,
                ),
                WorkflowTechnicalFitReason::new(
                    WorkflowTechnicalFitReasonCode::ConservativeFallback,
                    None,
                ),
            ],
        },
    );
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("technical-fit decision should block run");

    assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
    assert!(
        err.to_string()
            .contains("technical-fit could not select a ready runtime")
    );
}

#[tokio::test]
async fn workflow_run_returns_internal_when_host_emits_invalid_output_shape() {
    let host = MockWorkflowHost::with_invalid_output_binding(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("invalid host output should be internal");

    assert!(matches!(err, WorkflowServiceError::Internal(_)));
    assert!(
        err.to_string()
            .contains("outputs.0.port_id must be non-empty")
    );
}

#[tokio::test]
async fn workflow_run_rejects_zero_timeout_ms() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: Some(0),
                run_id: None,
            },
        )
        .await
        .expect_err("expected invalid timeout");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("timeout_ms"));
}

#[tokio::test]
async fn workflow_run_timeout_cancels_host_within_grace_window() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let host = TimeoutAwareHost::new(cancelled.clone());
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-timeout".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: Some(25),
                run_id: None,
            },
        )
        .await
        .expect_err("expected timeout");

    assert!(matches!(err, WorkflowServiceError::RuntimeTimeout(_)));
    assert!(cancelled.load(Ordering::SeqCst));
}

#[tokio::test]
async fn workflow_run_rejects_empty_node_id() {
    let host = MockWorkflowHost::new(10, 256);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("bad"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("expected invalid request");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[tokio::test]
async fn workflow_run_rejects_oversized_payload() {
    let host = MockWorkflowHost::new(10, 8);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("this is too large"),
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("expected capability violation");

    assert!(matches!(err, WorkflowServiceError::CapabilityViolation(_)));
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
async fn workflow_get_io_derives_inputs_and_outputs_from_workflow() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "name": "Prompt",
                            "description": "Prompt supplied by the caller",
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "label": "Text Input",
                                "description": "Provides text input",
                                "inputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "legacy-out",
                                        "label": "Legacy Out",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    },
                    {
                        "id": "text-output-1",
                        "node_type": "text-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "io_binding_origin": "client_session",
                                "label": "Text Output",
                                "description": "Displays text output",
                                "inputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    },
                                    {
                                        "id": "stream",
                                        "label": "Stream",
                                        "data_type": "stream",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
                        },
                        "position": { "x": 120.0, "y": 0.0 }
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
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io".to_string(),
            },
        )
        .await
        .expect("workflow io response");

    assert_eq!(response.inputs.len(), 1);
    assert_eq!(response.inputs[0].node_id, "text-input-1");
    assert_eq!(response.inputs[0].name.as_deref(), Some("Prompt"));
    assert_eq!(
        response.inputs[0].description.as_deref(),
        Some("Prompt supplied by the caller")
    );
    assert_eq!(response.inputs[0].ports.len(), 1);
    assert_eq!(response.inputs[0].ports[0].port_id, "text");
    assert_eq!(
        response.inputs[0].ports[0].data_type.as_deref(),
        Some("string")
    );
    assert!(
        response.inputs[0]
            .ports
            .iter()
            .all(|port| port.port_id != "legacy-out")
    );

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "text-output-1");
    assert_eq!(response.outputs[0].ports.len(), 1);
    assert_eq!(response.outputs[0].ports[0].port_id, "text");
    assert!(
        response.outputs[0]
            .ports
            .iter()
            .all(|port| port.port_id != "stream")
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_missing_directional_ports() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-invalid.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Invalid" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "outputs": [
                                    { "id": "text", "label": "Text", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-invalid".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject missing directional ports");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("text-input-1"));
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_skips_integrated_io_nodes() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-integrated.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Integrated" },
            "graph": {
                "nodes": [
                    {
                        "id": "puma-lib-1",
                        "node_type": "puma-lib",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "integrated",
                                "inputs": [],
                                "outputs": [
                                    { "id": "model_path", "label": "Model Path", "data_type": "string" }
                                ]
                            }
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
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-integrated".to_string(),
            },
        )
        .await
        .expect("workflow io should skip integrated io nodes");

    assert!(response.inputs.is_empty());
    assert!(response.outputs.is_empty());
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_missing_io_binding_origin() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-missing-origin.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Missing Origin" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "inputs": [
                                    { "id": "text", "label": "Text", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-missing-origin".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject missing io_binding_origin");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(
        err.to_string()
            .contains("missing definition.io_binding_origin")
    );
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_invalid_or_duplicate_port_ids() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-dup.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Duplicates" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-output-1",
                        "node_type": "text-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "io_binding_origin": "client_session",
                                "outputs": [
                                    { "id": "text", "label": "Text", "data_type": "string" },
                                    { "id": "text", "label": "Text 2", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-dup".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject duplicate port ids");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate port id 'text'"));
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_get_io_rejects_whitespace_port_ids() {
    let temp_root = std::env::temp_dir()
        .join("pantograph-workflow-io-tests")
        .join(uuid::Uuid::new_v4().to_string());
    let workflow_root = temp_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflow_root).expect("create workflow root");
    let workflow_path = workflow_root.join("wf-io-whitespace.json");
    fs::write(
        &workflow_path,
        serde_json::json!({
            "metadata": { "name": "Workflow I/O Whitespace" },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "inputs": [
                                    { "id": "   ", "label": "Text", "data_type": "string" }
                                ]
                            }
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
    let err = WorkflowService::new()
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-io-whitespace".to_string(),
            },
        )
        .await
        .expect_err("workflow io should reject whitespace port ids");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("text-input-1"));
    let _ = fs::remove_dir_all(temp_root);
}

#[tokio::test]
async fn workflow_run_accepts_discovered_output_targets() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();

    let io = service
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("workflow io");
    let target_node = &io.outputs[0];
    let target_port = &target_node.ports[0];

    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: target_node.node_id.clone(),
                    port_id: target_port.port_id.clone(),
                    value: serde_json::json!("ok"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: target_node.node_id.clone(),
                    port_id: target_port.port_id.clone(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect("workflow run with discovered target");

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, target_node.node_id);
    assert_eq!(response.outputs[0].port_id, target_port.port_id);
}

#[tokio::test]
async fn workflow_run_rejects_non_discovered_output_targets() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "stream".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("non-discovered target should fail early");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[tokio::test]
async fn workflow_run_returns_output_not_produced_when_target_missing() {
    let host = MockWorkflowHost::with_missing_requested_output(8, 1024);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("expected output_not_produced");

    assert!(matches!(err, WorkflowServiceError::OutputNotProduced(_)));
    assert!(
        err.to_string()
            .contains("requested output target 'text-output-1.text' was not produced")
    );
}

#[tokio::test]
async fn workflow_run_rejects_duplicate_input_bindings() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![
                    WorkflowPortBinding {
                        node_id: "text-input-1".to_string(),
                        port_id: "text".to_string(),
                        value: serde_json::json!("first"),
                    },
                    WorkflowPortBinding {
                        node_id: "text-input-1".to_string(),
                        port_id: "text".to_string(),
                        value: serde_json::json!("second"),
                    },
                ],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("duplicate bindings should fail");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate binding"));
}

#[tokio::test]
async fn workflow_preflight_reports_missing_required_inputs_and_invalid_targets() {
    let host = PreflightHost::new();
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "stream".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(!response.can_run);
    assert_eq!(response.graph_fingerprint, "preflight-graph");
    assert_eq!(response.missing_required_inputs.len(), 1);
    assert_eq!(response.missing_required_inputs[0].node_id, "text-input-1");
    assert_eq!(response.missing_required_inputs[0].port_id, "text");
    assert_eq!(response.invalid_targets.len(), 1);
    assert_eq!(response.invalid_targets[0].node_id, "text-output-1");
    assert_eq!(response.invalid_targets[0].port_id, "stream");
    assert!(
        response
            .warnings
            .iter()
            .any(|warning| warning.contains("does not declare required metadata"))
    );
}

#[tokio::test]
async fn workflow_preflight_can_run_when_inputs_and_targets_are_valid() {
    let host = PreflightHost::new();
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(response.can_run);
    assert_eq!(response.graph_fingerprint, "preflight-graph");
    assert!(response.missing_required_inputs.is_empty());
    assert!(response.invalid_targets.is_empty());
    assert!(
        response
            .warnings
            .iter()
            .any(|warning| warning.contains("does not declare required metadata"))
    );
}

#[tokio::test]
async fn workflow_preflight_surfaces_backend_technical_fit_decision() {
    let host = PreflightHost::with_technical_fit_decision(
        WorkflowHostCapabilities {
            max_input_bindings: 16,
            max_output_targets: 16,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "estimated".to_string(),
                required_models: Vec::new(),
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: Vec::new(),
        },
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("llama_cpp".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::ConservativeFallback,
                Some("llama_cpp"),
            )],
        },
    );
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(response.can_run);
    assert_eq!(
        response.technical_fit_decision,
        Some(WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("llama_cpp".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason {
                code: WorkflowTechnicalFitReasonCode::ConservativeFallback,
                candidate_id: Some("llama_cpp".to_string()),
            }],
        })
    );
    assert!(response.blocking_runtime_issues.is_empty());
    assert!(response.runtime_warnings.iter().any(|issue| {
        issue
            .message
            .contains("selected 'llama_cpp' conservatively")
    }));
}

#[tokio::test]
async fn workflow_preflight_blocks_selected_technical_fit_runtime_when_capability_is_not_ready() {
    let host = PreflightHost::with_technical_fit_decision(
        WorkflowHostCapabilities {
            max_input_bindings: 16,
            max_output_targets: 16,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "estimated".to_string(),
                required_models: Vec::new(),
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![WorkflowRuntimeCapability {
                runtime_id: "llama_cpp".to_string(),
                display_name: "llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: false,
                configured: false,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: true,
                readiness_state: Some(WorkflowRuntimeReadinessState::Failed),
                selected_version: Some("b8248".to_string()),
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: Some("validation failed".to_string()),
            }],
        },
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("llama_cpp".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::ConservativeFallback,
                Some("llama_cpp"),
            )],
        },
    );
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    assert!(!response.can_run);
    assert_eq!(response.blocking_runtime_issues.len(), 1);
    assert!(
        response.blocking_runtime_issues[0]
            .message
            .contains("validation failed")
    );
}

#[tokio::test]
async fn workflow_preflight_rejects_duplicate_output_targets() {
    let host = PreflightHost::new();
    let service = WorkflowService::new();

    let err = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![
                    WorkflowOutputTarget {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                    },
                    WorkflowOutputTarget {
                        node_id: "text-output-1".to_string(),
                        port_id: "text".to_string(),
                    },
                ]),
                override_selection: None,
            },
        )
        .await
        .expect_err("duplicate targets should fail");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate target"));
}

#[tokio::test]
async fn workflow_preflight_normalizes_override_selection_into_technical_fit_request() {
    let technical_fit_requests = Arc::new(Mutex::new(Vec::new()));
    let host = CountingPreflightHost {
        workflow_capabilities_calls: Arc::new(AtomicUsize::new(0)),
        runtime_capabilities_calls: Arc::new(AtomicUsize::new(0)),
        graph_fingerprint: Arc::new(Mutex::new("graph-a".to_string())),
        technical_fit_requests: technical_fit_requests.clone(),
    };
    let service = WorkflowService::new();

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: Some(WorkflowTechnicalFitOverride {
                    model_id: Some(" model-a ".to_string()),
                    backend_key: Some("llama.cpp".to_string()),
                }),
            },
        )
        .await
        .expect("preflight response");

    assert!(response.can_run);

    let requests = technical_fit_requests
        .lock()
        .expect("technical-fit requests lock poisoned");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].override_selection,
        Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        })
    );
}

#[test]
fn runtime_preflight_prefers_selected_runtime_over_non_selected_match() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama_cpp".to_string()],
        &[
            WorkflowRuntimeCapability {
                runtime_id: "managed-llama".to_string(),
                display_name: "Managed llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: false,
                readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
                selected_version: Some("b8248".to_string()),
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string(), "llama.cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
            WorkflowRuntimeCapability {
                runtime_id: "remote-llama".to_string(),
                display_name: "Remote llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: false,
                configured: false,
                can_install: false,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::Host,
                selected: true,
                readiness_state: Some(WorkflowRuntimeReadinessState::Unknown),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: Some("remote host is not configured".to_string()),
            },
        ],
    );

    assert_eq!(runtime_warnings.len(), 1);
    assert_eq!(blocking_runtime_issues.len(), 1);
    assert_eq!(blocking_runtime_issues[0].runtime_id, "remote-llama");
    assert!(blocking_runtime_issues[0].message.contains(
        "workflow requires backend 'llama_cpp' but Remote llama.cpp: remote host is not configured"
    ));
}

#[test]
fn runtime_preflight_uses_ready_fallback_when_no_runtime_is_selected() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama_cpp".to_string()],
        &[
            WorkflowRuntimeCapability {
                runtime_id: "missing-llama".to_string(),
                display_name: "Missing llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Missing,
                available: false,
                configured: false,
                can_install: true,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: false,
                readiness_state: Some(WorkflowRuntimeReadinessState::Missing),
                selected_version: None,
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: vec!["llama-server".to_string()],
                unavailable_reason: None,
            },
            ready_runtime_capability(),
        ],
    );

    assert!(runtime_warnings.is_empty());
    assert!(blocking_runtime_issues.is_empty());
}

#[test]
fn runtime_preflight_matches_legacy_backend_aliases_against_canonical_capabilities() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama.cpp".to_string(), "PyTorch".to_string()],
        &[
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
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
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
                selected_version: None,
                supports_external_connection: true,
                backend_keys: vec!["torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
        ],
    );

    assert!(runtime_warnings.is_empty());
    assert!(blocking_runtime_issues.is_empty());
}

#[test]
fn runtime_preflight_reports_selected_version_readiness_context() {
    let (runtime_warnings, blocking_runtime_issues) = evaluate_runtime_preflight(
        &["llama_cpp".to_string()],
        &[WorkflowRuntimeCapability {
            runtime_id: "llama_cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: false,
            configured: false,
            can_install: false,
            can_remove: true,
            source_kind: WorkflowRuntimeSourceKind::Managed,
            selected: true,
            readiness_state: Some(WorkflowRuntimeReadinessState::Validating),
            selected_version: Some("b8248".to_string()),
            supports_external_connection: true,
            backend_keys: vec!["llama_cpp".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }],
    );

    assert_eq!(runtime_warnings.len(), 1);
    assert_eq!(blocking_runtime_issues.len(), 1);
    assert!(
        blocking_runtime_issues[0]
            .message
            .contains("selected version 'b8248' is validating")
    );
}

#[test]
fn workflow_service_error_envelope_roundtrip() {
    let err = WorkflowServiceError::OutputNotProduced(
        "requested output target 'vector-output-1.vector' was not produced".to_string(),
    );

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::OutputNotProduced);
    assert!(envelope.message.contains("vector-output-1.vector"));
    assert_eq!(envelope.details, None);

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(parsed.code, WorkflowErrorCode::OutputNotProduced);
    assert!(parsed.message.contains("vector-output-1.vector"));
    assert_eq!(parsed.details, None);
}

#[test]
fn workflow_service_cancelled_envelope_roundtrip() {
    let err = WorkflowServiceError::Cancelled("workflow run cancelled".to_string());

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::Cancelled);
    assert_eq!(envelope.message, "workflow run cancelled");
    assert_eq!(envelope.details, None);

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(parsed.code, WorkflowErrorCode::Cancelled);
    assert_eq!(parsed.message, "workflow run cancelled");
    assert_eq!(parsed.details, None);
}

#[test]
fn workflow_service_scheduler_busy_envelope_includes_structured_details() {
    let err = WorkflowServiceError::scheduler_runtime_capacity_exhausted(2, 2, 0);

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::SchedulerBusy);
    assert_eq!(
        envelope.details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(2, 2, 0),
        ))
    );

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(
        parsed.details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(2, 2, 0),
        ))
    );
}

#[tokio::test]
async fn workflow_session_lifecycle_create_run_close() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("generic-run".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create session");
    assert_eq!(created.runtime_capabilities.len(), 1);

    let response = service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello session"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: Some("session-run-1".to_string()),
                priority: None,
            },
        )
        .await
        .expect("run session");
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("hello session")
    );

    let closed = service
        .close_workflow_session(
            &host,
            WorkflowSessionCloseRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .expect("close session");
    assert!(closed.ok);

    let err = service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id,
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect_err("closed session should not run");
    assert!(matches!(err, WorkflowServiceError::SessionNotFound(_)));
}

#[tokio::test]
async fn workflow_session_run_passes_logical_session_id_in_run_options() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(2);

    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create keep-alive session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello session"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: Some("session-run-options".to_string()),
                priority: None,
            },
        )
        .await
        .expect("run keep-alive session");

    let recorded = host
        .recorded_run_options
        .lock()
        .expect("run options lock poisoned");
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].workflow_session_id.as_deref(),
        Some(created.session_id.as_str())
    );
    assert_eq!(recorded[0].timeout_ms, None);
}

#[tokio::test]
async fn keep_alive_session_loads_runtime_with_keep_alive_retention_hint() {
    let retention_hints = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingRuntimeHost::new(retention_hints.clone());
    let service = WorkflowService::with_max_sessions(2);

    service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create keep-alive session");

    assert_eq!(
        *retention_hints
            .lock()
            .expect("retention hints lock poisoned"),
        vec![WorkflowSessionRetentionHint::KeepAlive]
    );
}

#[tokio::test]
async fn one_shot_session_run_loads_runtime_with_ephemeral_retention_hint() {
    let retention_hints = Arc::new(Mutex::new(Vec::new()));
    let host = RecordingRuntimeHost::new(retention_hints.clone());
    let service = WorkflowService::with_max_sessions(2);

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
        .expect("create one-shot session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id,
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run one-shot session");

    assert_eq!(
        *retention_hints
            .lock()
            .expect("retention hints lock poisoned"),
        vec![WorkflowSessionRetentionHint::Ephemeral]
    );
}

#[test]
fn loaded_runtime_capacity_limit_clamps_to_valid_session_bounds() {
    let service = WorkflowService::with_capacity_limits(4, 4);

    service
        .set_loaded_runtime_capacity_limit(Some(2))
        .expect("set lower loaded-runtime capacity");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        2
    );

    service
        .set_loaded_runtime_capacity_limit(Some(0))
        .expect("clamp loaded-runtime capacity to minimum");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        1
    );

    service
        .set_loaded_runtime_capacity_limit(Some(99))
        .expect("clamp loaded-runtime capacity to session limit");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        4
    );

    service
        .set_loaded_runtime_capacity_limit(None)
        .expect("reset loaded-runtime capacity to session limit");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        4
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_uses_host_selected_candidate() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let service = WorkflowService::with_capacity_limits(3, 2);

    let first = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("first".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create first keep-alive session");
    let second = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-2".to_string(),
                usage_profile: Some("second".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create second keep-alive session");
    let third = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-3".to_string(),
                usage_profile: Some("third".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create third session");
    let third_session_id = third.session_id.clone();

    let selecting_host = SelectingRuntimeHost::new(second.session_id.clone(), unloads.clone());

    service
        .run_workflow_session(
            &selecting_host,
            WorkflowSessionRunRequest {
                session_id: third_session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run third session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first(),
        Some(&(
            second.session_id.clone(),
            WorkflowSessionUnloadReason::CapacityRebalance,
        ))
    );
    assert!(
        unloads
            .iter()
            .any(|(session_id, _)| session_id == &third_session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|(session_id, _)| session_id == &first.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_affine_idle_runtime_by_default() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::new(unloads.clone());
    let service = WorkflowService::with_capacity_limits(3, 2);

    let affine = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create affine keep-alive session");
    let non_affine = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other".to_string(),
                usage_profile: Some("batch".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create non-affine keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(non_affine.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &affine.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_shared_model_idle_runtime() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::with_runtime_affinity(
        unloads.clone(),
        HashMap::from([
            ("wf-target".to_string(), vec!["llama_cpp".to_string()]),
            ("wf-shared-model".to_string(), vec!["llama_cpp".to_string()]),
            ("wf-other-model".to_string(), vec!["pytorch".to_string()]),
        ]),
        HashMap::from([
            ("wf-target".to_string(), vec!["model-a".to_string()]),
            ("wf-shared-model".to_string(), vec!["model-a".to_string()]),
            ("wf-other-model".to_string(), vec!["model-b".to_string()]),
        ]),
    );
    let service = WorkflowService::with_capacity_limits(3, 2);

    let shared_model = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared-model".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create shared-model keep-alive session");
    let other_model = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other-model".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create other-model keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-target".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(other_model.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &shared_model.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_shared_backend_idle_runtime() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::with_runtime_affinity(
        unloads.clone(),
        HashMap::from([
            ("wf-target".to_string(), vec!["llama_cpp".to_string()]),
            (
                "wf-shared-backend".to_string(),
                vec!["llama_cpp".to_string()],
            ),
            ("wf-other-backend".to_string(), vec!["pytorch".to_string()]),
        ]),
        HashMap::from([
            ("wf-target".to_string(), vec!["model-a".to_string()]),
            ("wf-shared-backend".to_string(), vec!["model-z".to_string()]),
            ("wf-other-backend".to_string(), vec!["model-a".to_string()]),
        ]),
    );
    let service = WorkflowService::with_capacity_limits(3, 2);

    let shared_backend = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared-backend".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create shared-backend keep-alive session");
    let other_backend = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other-backend".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create other-backend keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-target".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(other_backend.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &shared_backend.session_id)
    );
}

#[tokio::test]
async fn workflow_session_run_waits_for_runtime_capacity_before_admission() {
    let host = BlockingRunHost::new();
    let service = WorkflowService::with_capacity_limits(2, 1);

    let first = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-first".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-second".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");

    let first_service = service.clone();
    let first_host = host.clone();
    let first_session_id = first.session_id.clone();
    let first_run = tokio::spawn(async move {
        first_service
            .run_workflow_session(
                &first_host,
                WorkflowSessionRunRequest {
                    session_id: first_session_id,
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("run-first".to_string()),
                    priority: Some(1),
                },
            )
            .await
    });

    host.wait_for_first_run_started().await;

    let second_service = service.clone();
    let second_host = host.clone();
    let second_session_id = second.session_id.clone();
    let mut second_run = tokio::spawn(async move {
        second_service
            .run_workflow_session(
                &second_host,
                WorkflowSessionRunRequest {
                    session_id: second_session_id,
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("run-second".to_string()),
                    priority: Some(1),
                },
            )
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: second.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot while waiting");
    let diagnostics = snapshot
        .diagnostics
        .as_ref()
        .expect("scheduler diagnostics while waiting");

    assert_eq!(snapshot.session.state, WorkflowSessionState::IdleUnloaded);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowSessionQueueItemStatus::Pending
    );
    assert_eq!(
        snapshot.items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity)
    );
    assert_eq!(
        diagnostics.next_admission_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity)
    );
    assert_eq!(diagnostics.next_admission_wait_ms, None);
    assert_eq!(diagnostics.next_admission_not_before_ms, None);
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut second_run)
            .await
            .is_err(),
        "second run should remain queued until capacity becomes available"
    );

    host.release_first_run();

    let first_response = first_run
        .await
        .expect("first run join")
        .expect("first run response");
    let second_response = second_run
        .await
        .expect("second run join")
        .expect("second run response");

    assert_eq!(first_response.outputs.len(), 1);
    assert_eq!(second_response.outputs.len(), 1);
}

#[tokio::test]
async fn workflow_session_run_waits_for_runtime_admission_before_dequeue() {
    let admission_open = Arc::new(AtomicBool::new(false));
    let host = AdmissionGatedHost::new(admission_open.clone());
    let service = WorkflowService::with_capacity_limits(1, 1);

    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-gated".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create gated session");

    let run_service = service.clone();
    let run_host = host.clone();
    let session_id = created.session_id.clone();
    let mut run = tokio::spawn(async move {
        run_service
            .run_workflow_session(
                &run_host,
                WorkflowSessionRunRequest {
                    session_id,
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("run-gated".to_string()),
                    priority: Some(1),
                },
            )
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;

    let before_snapshot_ms = unix_timestamp_ms();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot while admission is blocked");
    let after_snapshot_ms = unix_timestamp_ms();
    let diagnostics = snapshot
        .diagnostics
        .as_ref()
        .expect("scheduler diagnostics while admission is blocked");

    assert_eq!(snapshot.session.state, WorkflowSessionState::IdleUnloaded);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowSessionQueueItemStatus::Pending
    );
    assert_eq!(
        snapshot.items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission)
    );
    assert_eq!(
        diagnostics.next_admission_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission)
    );
    assert_eq!(diagnostics.next_admission_wait_ms, Some(10));
    let next_admission_not_before_ms = diagnostics
        .next_admission_not_before_ms
        .expect("runtime-admission wait timestamp");
    assert!(next_admission_not_before_ms >= before_snapshot_ms.saturating_add(10));
    assert!(next_admission_not_before_ms <= after_snapshot_ms.saturating_add(10));
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut run)
            .await
            .is_err(),
        "run should remain queued until runtime admission opens"
    );

    admission_open.store(true, Ordering::SeqCst);

    let response = run
        .await
        .expect("run join")
        .expect("run response after admission opens");
    assert_eq!(response.outputs.len(), 1);
}

#[tokio::test]
async fn workflow_session_runtime_preflight_is_cached_until_graph_changes() {
    let workflow_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let runtime_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let graph_fingerprint = Arc::new(Mutex::new("graph-a".to_string()));
    let technical_fit_requests = Arc::new(Mutex::new(Vec::new()));
    let host = CountingPreflightHost {
        workflow_capabilities_calls: workflow_capabilities_calls.clone(),
        runtime_capabilities_calls: runtime_capabilities_calls.clone(),
        graph_fingerprint: graph_fingerprint.clone(),
        technical_fit_requests,
    };
    let service = WorkflowService::with_max_sessions(1);

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
        .expect("create session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("first run");
    assert_eq!(workflow_capabilities_calls.load(Ordering::SeqCst), 1);

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("second run");
    assert_eq!(
        workflow_capabilities_calls.load(Ordering::SeqCst),
        1,
        "unchanged graph should reuse cached preflight"
    );
    assert_eq!(runtime_capabilities_calls.load(Ordering::SeqCst), 3);

    *graph_fingerprint
        .lock()
        .expect("graph fingerprint lock poisoned") = "graph-b".to_string();

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id,
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("third run after graph change");
    assert_eq!(
        workflow_capabilities_calls.load(Ordering::SeqCst),
        2,
        "graph change should invalidate cached preflight"
    );
}

#[tokio::test]
async fn workflow_session_runtime_preflight_cache_invalidates_on_override_selection_change() {
    let workflow_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let runtime_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let technical_fit_requests = Arc::new(Mutex::new(Vec::new()));
    let host = CountingPreflightHost {
        workflow_capabilities_calls: workflow_capabilities_calls.clone(),
        runtime_capabilities_calls: runtime_capabilities_calls.clone(),
        graph_fingerprint: Arc::new(Mutex::new("graph-a".to_string())),
        technical_fit_requests: technical_fit_requests.clone(),
    };
    let service = WorkflowService::with_max_sessions(1);

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
        .expect("create session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: Some(WorkflowTechnicalFitOverride {
                    model_id: None,
                    backend_key: Some("llama.cpp".to_string()),
                }),
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("first run");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: created.session_id,
                inputs: Vec::new(),
                output_targets: None,
                override_selection: Some(WorkflowTechnicalFitOverride {
                    model_id: Some("model-a".to_string()),
                    backend_key: Some("llama.cpp".to_string()),
                }),
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("second run");

    let requests = technical_fit_requests
        .lock()
        .expect("technical-fit requests lock poisoned");
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].override_selection,
        Some(WorkflowTechnicalFitOverride {
            model_id: None,
            backend_key: Some("llama_cpp".to_string()),
        })
    );
    assert_eq!(
        requests[1].override_selection,
        Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        })
    );
    assert_eq!(
        workflow_capabilities_calls.load(Ordering::SeqCst),
        2,
        "override changes should invalidate cached preflight"
    );
    assert_eq!(runtime_capabilities_calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn keep_alive_session_create_blocks_when_runtime_preflight_fails() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        8,
        1024,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::MissingRuntimeState,
                None,
            )],
        },
    );
    let service = WorkflowService::with_max_sessions(1);

    let err = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect_err("keep-alive session create should fail when runtime preflight blocks");

    assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .active
            .len(),
        0,
        "failed keep-alive create should roll back session creation"
    );
}

#[tokio::test]
async fn keep_alive_enable_blocks_when_runtime_preflight_fails() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        8,
        1024,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::MissingRuntimeState,
                None,
            )],
        },
    );
    let service = WorkflowService::with_max_sessions(1);
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create unloaded session");

    let err = service
        .workflow_set_session_keep_alive(
            &host,
            WorkflowSessionKeepAliveRequest {
                session_id: created.session_id.clone(),
                keep_alive: true,
            },
        )
        .await
        .expect_err("keep-alive enable should fail when runtime preflight blocks");

    assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
    let summary = service
        .session_store
        .lock()
        .expect("session store lock poisoned")
        .session_summary(&created.session_id)
        .expect("session summary after failed keep-alive enable");
    assert_eq!(summary.state, WorkflowSessionState::IdleUnloaded);
    assert!(!summary.keep_alive);
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
async fn workflow_cleanup_stale_sessions_removes_idle_non_keep_alive_session() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
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
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    let response = service
        .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert_eq!(
        response.cleaned_session_ids,
        vec![created.session_id.clone()]
    );
    let err = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect_err("cleaned session should be removed");
    assert!(matches!(err, WorkflowServiceError::SessionNotFound(_)));

    let second_response = service
        .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("second cleanup stale sessions");
    assert!(
        second_response.cleaned_session_ids.is_empty(),
        "repeat cleanup should be idempotent once the stale session is gone"
    );
}

#[tokio::test]
async fn workflow_cleanup_stale_sessions_keeps_session_with_queued_work() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
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
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("queued-run-1".to_string()),
                    priority: Some(1),
                },
            )
            .expect("enqueue run");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    let response = service
        .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert!(
        response.cleaned_session_ids.is_empty(),
        "queued sessions should remain scheduler-visible until the queue drains"
    );

    let session_id = created.session_id.clone();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
        .await
        .expect("scheduler snapshot");
    assert_eq!(snapshot.session.session_id, created.session_id);
    assert_eq!(snapshot.session.queued_runs, 1);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowSessionQueueItemStatus::Pending
    );
}

#[tokio::test]
async fn workflow_cleanup_stale_sessions_keeps_keep_alive_session() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    let response = service
        .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert!(response.cleaned_session_ids.is_empty());
    let status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("keep-alive session should remain accessible");
    assert!(status.session.keep_alive);
}

#[tokio::test]
async fn workflow_get_session_inspection_uses_host_owned_live_state_view() {
    let create_host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_session(
            &create_host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow session");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let inspection_state = WorkflowGraphSessionStateView::new(
        node_engine::WorkflowSessionResidencyState::Warm,
        Vec::new(),
        None,
        None,
    );
    let inspection_host = InspectionHost {
        calls: calls.clone(),
        state: Some(inspection_state.clone()),
    };

    let response = service
        .workflow_get_session_inspection(
            &inspection_host,
            WorkflowSessionInspectionRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .expect("inspect workflow session");

    assert_eq!(response.session.session_id, created.session_id);
    assert_eq!(response.session.workflow_id, "wf-1");
    assert_eq!(response.workflow_session_state, Some(inspection_state));
    assert_eq!(
        calls
            .lock()
            .expect("inspection host calls lock poisoned")
            .as_slice(),
        &[(created.session_id, "wf-1".to_string())]
    );
}

#[tokio::test]
async fn workflow_cleanup_stale_sessions_respects_recent_status_reads() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
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
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("status read should refresh session access");

    let response = service
        .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert!(response.cleaned_session_ids.is_empty());
    let status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("recently accessed session should remain accessible");
    assert_eq!(status.session.state, WorkflowSessionState::IdleUnloaded);
}

#[tokio::test]
async fn workflow_stale_cleanup_worker_removes_stale_sessions() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_session_stale_cleanup_worker(WorkflowSessionStaleCleanupWorkerConfig {
            interval: Duration::from_millis(10),
            idle_timeout: Duration::from_millis(20),
        })
        .expect("spawn stale cleanup worker");
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
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let removed = {
                let store = service
                    .session_store
                    .lock()
                    .expect("session store lock poisoned");
                !store.active.contains_key(&created.session_id)
            };
            if removed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("worker should remove stale workflow session");

    worker.shutdown().await;
}

#[tokio::test]
async fn workflow_stale_cleanup_worker_keeps_sessions_with_queued_work() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_session_stale_cleanup_worker(WorkflowSessionStaleCleanupWorkerConfig {
            interval: Duration::from_millis(10),
            idle_timeout: Duration::from_millis(20),
        })
        .expect("spawn stale cleanup worker");
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
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("queued-run-1".to_string()),
                    priority: Some(1),
                },
            )
            .expect("enqueue run");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    tokio::time::sleep(Duration::from_millis(80)).await;

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot");
    assert_eq!(snapshot.session.session_id, created.session_id);
    assert_eq!(snapshot.session.queued_runs, 1);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowSessionQueueItemStatus::Pending
    );

    worker.shutdown().await;
}

#[tokio::test]
async fn workflow_stale_cleanup_worker_shutdown_stops_future_cleanup() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_session_stale_cleanup_worker(WorkflowSessionStaleCleanupWorkerConfig {
            interval: Duration::from_secs(1),
            idle_timeout: Duration::from_millis(20),
        })
        .expect("spawn stale cleanup worker");
    worker.shutdown().await;
    worker.shutdown().await;

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
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    let status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("shutdown worker should not remove stale sessions");
    assert_eq!(status.session.state, WorkflowSessionState::IdleUnloaded);
}

#[test]
fn workflow_stale_cleanup_worker_requires_active_tokio_runtime() {
    let service = Arc::new(WorkflowService::new());
    let err = match service.spawn_workflow_session_stale_cleanup_worker(
        WorkflowSessionStaleCleanupWorkerConfig::default(),
    ) {
        Ok(_) => panic!("spawn should fail without an active tokio runtime"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        WorkflowServiceError::Internal(ref message)
            if message.contains("requires an active Tokio runtime")
    ));
}

#[test]
fn workflow_stale_cleanup_worker_accepts_explicit_runtime_handle() {
    let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_session_stale_cleanup_worker_with_handle(
            WorkflowSessionStaleCleanupWorkerConfig::default(),
            runtime.handle().clone(),
        )
        .expect("spawn stale cleanup worker with explicit runtime handle");

    runtime.block_on(async move {
        worker.shutdown().await;
    });
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
