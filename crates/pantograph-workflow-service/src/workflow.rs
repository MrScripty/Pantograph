use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::capabilities;

/// Node/port value binding used for workflow inputs and outputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPortBinding {
    pub node_id: String,
    pub port_id: String,
    pub value: serde_json::Value,
}

/// Explicit output node+port target to demand in a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowOutputTarget {
    pub node_id: String,
    pub port_id: String,
}

/// Request contract for generic workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub inputs: Vec<WorkflowPortBinding>,
    #[serde(default)]
    pub output_targets: Option<Vec<WorkflowOutputTarget>>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub run_id: Option<String>,
}

/// Workflow run response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunResponse {
    pub run_id: String,
    pub outputs: Vec<WorkflowPortBinding>,
    pub timing_ms: u128,
}

/// Workflow capabilities request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowCapabilitiesRequest {
    pub workflow_id: String,
}

/// Host model descriptor used to build capability model inventory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowHostModelDescriptor {
    #[serde(default)]
    pub model_type: Option<String>,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
}

/// Model inventory item included in workflow capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowCapabilityModel {
    pub model_id: String,
    #[serde(default)]
    pub model_revision_or_hash: Option<String>,
    #[serde(default)]
    pub model_type: Option<String>,
    pub node_ids: Vec<String>,
    pub roles: Vec<String>,
}

/// Host capability payload consumed by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRuntimeRequirements {
    #[serde(default)]
    pub estimated_peak_vram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_peak_ram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_min_vram_mb: Option<u64>,
    #[serde(default)]
    pub estimated_min_ram_mb: Option<u64>,
    pub estimation_confidence: String,
    pub required_models: Vec<String>,
    pub required_backends: Vec<String>,
    pub required_extensions: Vec<String>,
}

impl Default for WorkflowRuntimeRequirements {
    fn default() -> Self {
        Self {
            estimated_peak_vram_mb: None,
            estimated_peak_ram_mb: None,
            estimated_min_vram_mb: None,
            estimated_min_ram_mb: None,
            estimation_confidence: "unknown".to_string(),
            required_models: Vec::new(),
            required_backends: Vec::new(),
            required_extensions: Vec::new(),
        }
    }
}

/// Host capability payload consumed by the service.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowHostCapabilities {
    pub max_input_bindings: usize,
    pub max_output_targets: usize,
    pub max_value_bytes: usize,
    pub runtime_requirements: WorkflowRuntimeRequirements,
    #[serde(default)]
    pub models: Vec<WorkflowCapabilityModel>,
}

/// Workflow capabilities response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowCapabilitiesResponse {
    pub max_input_bindings: usize,
    pub max_output_targets: usize,
    pub max_value_bytes: usize,
    pub runtime_requirements: WorkflowRuntimeRequirements,
    #[serde(default)]
    pub models: Vec<WorkflowCapabilityModel>,
}

/// Workflow I/O discovery request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoRequest {
    pub workflow_id: String,
}

/// Workflow I/O discovery response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoResponse {
    #[serde(default)]
    pub inputs: Vec<WorkflowIoNode>,
    #[serde(default)]
    pub outputs: Vec<WorkflowIoNode>,
}

/// Workflow preflight request for static, non-runtime validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPreflightRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub inputs: Vec<WorkflowPortBinding>,
    #[serde(default)]
    pub output_targets: Option<Vec<WorkflowOutputTarget>>,
}

/// Input surface reference used by preflight diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowInputTarget {
    pub node_id: String,
    pub port_id: String,
}

/// Workflow preflight response for static request validation diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPreflightResponse {
    #[serde(default)]
    pub missing_required_inputs: Vec<WorkflowInputTarget>,
    #[serde(default)]
    pub invalid_targets: Vec<WorkflowOutputTarget>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub can_run: bool,
}

/// One workflow node available as an external input or output surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoNode {
    pub node_id: String,
    pub node_type: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub ports: Vec<WorkflowIoPort>,
}

/// I/O port metadata exposed to workflow API consumers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowIoPort {
    pub port_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub multiple: Option<bool>,
}

/// Session creation request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionCreateRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub usage_profile: Option<String>,
}

/// Session creation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionCreateResponse {
    pub session_id: String,
}

/// Session run request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionRunRequest {
    pub session_id: String,
    #[serde(default)]
    pub inputs: Vec<WorkflowPortBinding>,
    #[serde(default)]
    pub output_targets: Option<Vec<WorkflowOutputTarget>>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub run_id: Option<String>,
}

/// Session close request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionCloseRequest {
    pub session_id: String,
}

/// Session close response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionCloseResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowErrorCode {
    InvalidRequest,
    WorkflowNotFound,
    CapabilityViolation,
    RuntimeNotReady,
    SessionNotFound,
    SessionEvicted,
    SchedulerBusy,
    OutputNotProduced,
    RuntimeTimeout,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowErrorEnvelope {
    pub code: WorkflowErrorCode,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkflowServiceError {
    #[error("invalid_request: {0}")]
    InvalidRequest(String),

    #[error("workflow_not_found: {0}")]
    WorkflowNotFound(String),

    #[error("capability_violation: {0}")]
    CapabilityViolation(String),

    #[error("runtime_not_ready: {0}")]
    RuntimeNotReady(String),

    #[error("session_not_found: {0}")]
    SessionNotFound(String),

    #[error("session_evicted: {0}")]
    SessionEvicted(String),

    #[error("scheduler_busy: {0}")]
    SchedulerBusy(String),

    #[error("output_not_produced: {0}")]
    OutputNotProduced(String),

    #[error("runtime_timeout: {0}")]
    RuntimeTimeout(String),

    #[error("internal_error: {0}")]
    Internal(String),
}

impl WorkflowServiceError {
    pub fn code(&self) -> WorkflowErrorCode {
        match self {
            WorkflowServiceError::InvalidRequest(_) => WorkflowErrorCode::InvalidRequest,
            WorkflowServiceError::WorkflowNotFound(_) => WorkflowErrorCode::WorkflowNotFound,
            WorkflowServiceError::CapabilityViolation(_) => WorkflowErrorCode::CapabilityViolation,
            WorkflowServiceError::RuntimeNotReady(_) => WorkflowErrorCode::RuntimeNotReady,
            WorkflowServiceError::SessionNotFound(_) => WorkflowErrorCode::SessionNotFound,
            WorkflowServiceError::SessionEvicted(_) => WorkflowErrorCode::SessionEvicted,
            WorkflowServiceError::SchedulerBusy(_) => WorkflowErrorCode::SchedulerBusy,
            WorkflowServiceError::OutputNotProduced(_) => WorkflowErrorCode::OutputNotProduced,
            WorkflowServiceError::RuntimeTimeout(_) => WorkflowErrorCode::RuntimeTimeout,
            WorkflowServiceError::Internal(_) => WorkflowErrorCode::InternalError,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            WorkflowServiceError::InvalidRequest(message)
            | WorkflowServiceError::WorkflowNotFound(message)
            | WorkflowServiceError::CapabilityViolation(message)
            | WorkflowServiceError::RuntimeNotReady(message)
            | WorkflowServiceError::SessionNotFound(message)
            | WorkflowServiceError::SessionEvicted(message)
            | WorkflowServiceError::SchedulerBusy(message)
            | WorkflowServiceError::OutputNotProduced(message)
            | WorkflowServiceError::RuntimeTimeout(message)
            | WorkflowServiceError::Internal(message) => message,
        }
    }

    pub fn to_envelope(&self) -> WorkflowErrorEnvelope {
        WorkflowErrorEnvelope {
            code: self.code(),
            message: self.message().to_string(),
        }
    }

    pub fn to_envelope_json(&self) -> String {
        serde_json::to_string(&self.to_envelope()).unwrap_or_else(|_| {
            r#"{"code":"internal_error","message":"failed to serialize workflow error envelope"}"#
                .to_string()
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunOptions {
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WorkflowRunHandle {
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl WorkflowRunHandle {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowIoDirection {
    Input,
    Output,
}

/// Trait boundary for host/runtime concerns needed by workflow service.
#[async_trait]
pub trait WorkflowHost: Send + Sync {
    /// Candidate roots that may contain `.pantograph/workflows/<workflow_id>.json`.
    fn workflow_roots(&self) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Upper bound for request input bindings.
    fn max_input_bindings(&self) -> usize {
        capabilities::DEFAULT_MAX_INPUT_BINDINGS
    }

    /// Upper bound for explicit output target count.
    fn max_output_targets(&self) -> usize {
        capabilities::DEFAULT_MAX_OUTPUT_TARGETS
    }

    /// Upper bound for each bound value payload, in bytes.
    fn max_value_bytes(&self) -> usize {
        capabilities::DEFAULT_MAX_VALUE_BYTES
    }

    /// Backend identifier used when workflow data does not declare one.
    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok("unknown".to_string())
    }

    /// Optional model metadata for runtime requirement estimation.
    async fn model_metadata(
        &self,
        _model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        Ok(None)
    }

    /// Optional model descriptor for capability inventory.
    async fn model_descriptor(
        &self,
        _model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        Ok(None)
    }

    /// Resolve workflow identity and fail if it is unknown to the host.
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots()).map(|_| ())
    }

    /// Return capability limits and model support metadata.
    async fn workflow_capabilities(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots())?;
        let required_models = capabilities::extract_required_models(stored.nodes());
        let mut required_backends = capabilities::extract_required_backends(stored.nodes());
        if required_backends.is_empty() {
            required_backends.push(self.default_backend_name().await?);
        }
        required_backends.sort();
        required_backends.dedup();

        let required_extensions =
            capabilities::extract_required_extensions(stored.nodes(), !required_models.is_empty());
        let mut model_metadata = HashMap::new();
        for model_id in &required_models {
            if let Some(metadata) = self.model_metadata(model_id).await? {
                model_metadata.insert(model_id.clone(), metadata);
            }
        }

        let (
            estimated_peak_vram_mb,
            estimated_peak_ram_mb,
            estimated_min_vram_mb,
            estimated_min_ram_mb,
            estimation_confidence,
        ) = capabilities::estimate_memory_requirements(&required_models, &model_metadata);
        let model_usages = capabilities::extract_model_usages(stored.nodes());
        let mut models = Vec::with_capacity(model_usages.len());
        for usage in model_usages {
            let descriptor = self.model_descriptor(&usage.model_id).await?;
            let model_revision_or_hash = descriptor
                .as_ref()
                .and_then(|record| capabilities::select_preferred_hash(&record.hashes));
            let model_type = descriptor.and_then(|record| {
                record
                    .model_type
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
            });

            models.push(WorkflowCapabilityModel {
                model_id: usage.model_id,
                model_revision_or_hash,
                model_type,
                node_ids: usage.node_ids,
                roles: usage.roles,
            });
        }

        Ok(WorkflowHostCapabilities {
            max_input_bindings: self.max_input_bindings(),
            max_output_targets: self.max_output_targets(),
            max_value_bytes: self.max_value_bytes(),
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb,
                estimated_peak_ram_mb,
                estimated_min_vram_mb,
                estimated_min_ram_mb,
                estimation_confidence,
                required_models,
                required_backends,
                required_extensions,
            },
            models,
        })
    }

    /// Discover externally bindable input and output nodes for a workflow.
    async fn workflow_io(
        &self,
        workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots())?;
        derive_workflow_io(stored.nodes())
    }

    /// Execute one workflow run and return output port bindings.
    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError>;
}

const DEFAULT_MAX_SESSIONS: usize = 8;
const WORKFLOW_CANCEL_GRACE_WINDOW_MS: u64 = 250;

#[derive(Debug, Clone)]
struct WorkflowSessionState {
    workflow_id: String,
    usage_profile: Option<String>,
    in_use: bool,
    access_tick: u64,
    run_count: u64,
}

#[derive(Debug)]
struct WorkflowSessionStore {
    max_sessions: usize,
    tick: u64,
    active: HashMap<String, WorkflowSessionState>,
    evicted: HashSet<String>,
}

impl WorkflowSessionStore {
    fn new(max_sessions: usize) -> Self {
        Self {
            max_sessions: max_sessions.max(1),
            tick: 0,
            active: HashMap::new(),
            evicted: HashSet::new(),
        }
    }

    fn next_tick(&mut self) -> u64 {
        self.tick = self.tick.saturating_add(1);
        self.tick
    }

    fn create_session(
        &mut self,
        workflow_id: String,
        usage_profile: Option<String>,
    ) -> Result<String, WorkflowServiceError> {
        if self.active.len() >= self.max_sessions {
            let evict_id = self
                .active
                .iter()
                .filter(|(_, state)| !state.in_use)
                .min_by_key(|(_, state)| (state.access_tick, state.run_count))
                .map(|(session_id, _)| session_id.clone());
            if let Some(session_id) = evict_id {
                self.active.remove(&session_id);
                self.evicted.insert(session_id);
            } else {
                return Err(WorkflowServiceError::SchedulerBusy(
                    "no schedulable capacity; all sessions are currently in use".to_string(),
                ));
            }
        }

        let session_id = Uuid::new_v4().to_string();
        let state = WorkflowSessionState {
            workflow_id,
            usage_profile,
            in_use: false,
            access_tick: self.next_tick(),
            run_count: 0,
        };
        self.active.insert(session_id.clone(), state);
        self.evicted.remove(&session_id);
        Ok(session_id)
    }

    fn begin_run(&mut self, session_id: &str) -> Result<String, WorkflowServiceError> {
        if self.evicted.contains(session_id) {
            return Err(WorkflowServiceError::SessionEvicted(format!(
                "session '{}' was evicted by scheduler",
                session_id
            )));
        }

        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        if state.in_use {
            return Err(WorkflowServiceError::SchedulerBusy(format!(
                "session '{}' is already running",
                session_id
            )));
        }

        state.in_use = true;
        state.access_tick = tick;
        let _usage_profile = state.usage_profile.as_deref();
        Ok(state.workflow_id.clone())
    }

    fn end_run(&mut self, session_id: &str) {
        let tick = self.next_tick();
        if let Some(state) = self.active.get_mut(session_id) {
            state.in_use = false;
            state.access_tick = tick;
            state.run_count = state.run_count.saturating_add(1);
        }
    }

    fn close_session(&mut self, session_id: &str) -> Result<(), WorkflowServiceError> {
        if self.active.remove(session_id).is_some() {
            self.evicted.remove(session_id);
            return Ok(());
        }

        if self.evicted.remove(session_id) {
            return Err(WorkflowServiceError::SessionEvicted(format!(
                "session '{}' was evicted by scheduler",
                session_id
            )));
        }

        Err(WorkflowServiceError::SessionNotFound(format!(
            "session '{}' not found",
            session_id
        )))
    }
}

/// Service entrypoint for workflow API operations.
#[derive(Clone)]
pub struct WorkflowService {
    session_store: Arc<Mutex<WorkflowSessionStore>>,
}

impl Default for WorkflowService {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowService {
    pub fn new() -> Self {
        Self::with_max_sessions(DEFAULT_MAX_SESSIONS)
    }

    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self {
            session_store: Arc::new(Mutex::new(WorkflowSessionStore::new(max_sessions))),
        }
    }

    pub async fn create_workflow_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionCreateRequest,
    ) -> Result<WorkflowSessionCreateResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;

        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        let session_id = store.create_session(
            request.workflow_id,
            request
                .usage_profile
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
        )?;
        Ok(WorkflowSessionCreateResponse { session_id })
    }

    pub async fn run_workflow_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let workflow_id = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.begin_run(&session_id)?
        };

        let run_result = self
            .workflow_run(
                host,
                WorkflowRunRequest {
                    workflow_id,
                    inputs: request.inputs,
                    output_targets: request.output_targets,
                    timeout_ms: request.timeout_ms,
                    run_id: request.run_id,
                },
            )
            .await;

        if let Ok(mut store) = self.session_store.lock() {
            store.end_run(&session_id);
        }

        run_result
    }

    pub async fn close_workflow_session(
        &self,
        request: WorkflowSessionCloseRequest,
    ) -> Result<WorkflowSessionCloseResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.close_session(session_id)?;
        Ok(WorkflowSessionCloseResponse { ok: true })
    }

    pub async fn workflow_run<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        validate_timeout_ms(request.timeout_ms)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }

        host.validate_workflow(&request.workflow_id).await?;
        if let Some(targets) = request.output_targets.as_ref() {
            let io = host.workflow_io(&request.workflow_id).await?;
            validate_workflow_io(&io)?;
            validate_output_targets_against_io(targets, &io)?;
        }
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;

        if request.inputs.len() > capabilities.max_input_bindings {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "input binding count {} exceeds max_input_bindings {}",
                request.inputs.len(),
                capabilities.max_input_bindings
            )));
        }

        if let Some(targets) = request.output_targets.as_ref() {
            if targets.len() > capabilities.max_output_targets {
                return Err(WorkflowServiceError::CapabilityViolation(format!(
                    "output target count {} exceeds max_output_targets {}",
                    targets.len(),
                    capabilities.max_output_targets
                )));
            }
        }

        for binding in &request.inputs {
            validate_payload_size(binding, capabilities.max_value_bytes)?;
        }

        let started = Instant::now();
        let run_options = WorkflowRunOptions {
            timeout_ms: request.timeout_ms,
        };
        let run_handle = WorkflowRunHandle::new();
        let mut run_future = Box::pin(host.run_workflow(
            &request.workflow_id,
            &request.inputs,
            request.output_targets.as_deref(),
            run_options,
            run_handle.clone(),
        ));
        let outputs = if let Some(timeout_ms) = request.timeout_ms {
            let timeout = tokio::time::sleep(Duration::from_millis(timeout_ms));
            tokio::pin!(timeout);
            tokio::select! {
                result = &mut run_future => result?,
                _ = &mut timeout => {
                    run_handle.cancel();
                    let cancel_grace = tokio::time::sleep(Duration::from_millis(WORKFLOW_CANCEL_GRACE_WINDOW_MS));
                    tokio::pin!(cancel_grace);
                    tokio::select! {
                        _ = &mut run_future => {}
                        _ = &mut cancel_grace => {}
                    }
                    return Err(WorkflowServiceError::RuntimeTimeout(format!(
                        "workflow run exceeded timeout_ms {}",
                        timeout_ms
                    )));
                }
            }
        } else {
            run_future.await?
        };

        if let Some(targets) = request.output_targets.as_ref() {
            validate_requested_outputs_produced(targets, &outputs)?;
        } else if outputs.is_empty() {
            return Err(WorkflowServiceError::Internal(
                "workflow execution returned zero outputs".to_string(),
            ));
        }

        validate_bindings(&outputs, "outputs")?;
        for binding in &outputs {
            validate_payload_size(binding, capabilities.max_value_bytes)?;
        }

        let run_id = request
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        Ok(WorkflowRunResponse {
            run_id,
            outputs,
            timing_ms: started.elapsed().as_millis(),
        })
    }

    pub async fn workflow_get_capabilities<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowCapabilitiesRequest,
    ) -> Result<WorkflowCapabilitiesResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
        Ok(WorkflowCapabilitiesResponse {
            max_input_bindings: capabilities.max_input_bindings,
            max_output_targets: capabilities.max_output_targets,
            max_value_bytes: capabilities.max_value_bytes,
            runtime_requirements: capabilities.runtime_requirements,
            models: capabilities.models,
        })
    }

    pub async fn workflow_get_io<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowIoRequest,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;
        let io = host.workflow_io(&request.workflow_id).await?;
        validate_workflow_io(&io)?;
        Ok(io)
    }

    pub async fn workflow_preflight<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowPreflightRequest,
    ) -> Result<WorkflowPreflightResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }

        host.validate_workflow(&request.workflow_id).await?;
        let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
        if request.inputs.len() > capabilities.max_input_bindings {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "input binding count {} exceeds max_input_bindings {}",
                request.inputs.len(),
                capabilities.max_input_bindings
            )));
        }
        if let Some(targets) = request.output_targets.as_ref() {
            if targets.len() > capabilities.max_output_targets {
                return Err(WorkflowServiceError::CapabilityViolation(format!(
                    "output target count {} exceeds max_output_targets {}",
                    targets.len(),
                    capabilities.max_output_targets
                )));
            }
        }
        for binding in &request.inputs {
            validate_payload_size(binding, capabilities.max_value_bytes)?;
        }

        let io = host.workflow_io(&request.workflow_id).await?;
        validate_workflow_io(&io)?;

        let supplied_inputs = request
            .inputs
            .iter()
            .map(|binding| (binding.node_id.as_str(), binding.port_id.as_str()))
            .collect::<HashSet<_>>();
        let required_inputs = derive_required_external_inputs(&io);
        let mut missing_required_inputs = required_inputs
            .iter()
            .filter(|required| {
                !supplied_inputs.contains(&(required.node_id.as_str(), required.port_id.as_str()))
            })
            .cloned()
            .collect::<Vec<_>>();
        missing_required_inputs.sort_by(|a, b| {
            a.node_id
                .cmp(&b.node_id)
                .then_with(|| a.port_id.cmp(&b.port_id))
        });

        let invalid_targets = request
            .output_targets
            .as_deref()
            .map(|targets| collect_invalid_output_targets(targets, &io))
            .unwrap_or_default();

        let warnings = collect_preflight_warnings(&io);
        let can_run = missing_required_inputs.is_empty() && invalid_targets.is_empty();

        Ok(WorkflowPreflightResponse {
            missing_required_inputs,
            invalid_targets,
            warnings,
            can_run,
        })
    }
}

fn validate_timeout_ms(timeout_ms: Option<u64>) -> Result<(), WorkflowServiceError> {
    if matches!(timeout_ms, Some(0)) {
        return Err(WorkflowServiceError::InvalidRequest(
            "timeout_ms must be greater than zero when provided".to_string(),
        ));
    }
    Ok(())
}

fn validate_workflow_id(workflow_id: &str) -> Result<(), WorkflowServiceError> {
    if workflow_id.trim().is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(
            "workflow_id must be non-empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_bindings(
    bindings: &[WorkflowPortBinding],
    field_name: &str,
) -> Result<(), WorkflowServiceError> {
    let mut seen = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "{}.{}.node_id must be non-empty",
                field_name, index
            )));
        }
        if binding.port_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "{}.{}.port_id must be non-empty",
                field_name, index
            )));
        }
        if !seen.insert((binding.node_id.as_str(), binding.port_id.as_str())) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "{} has duplicate binding '{}.{}'",
                field_name, binding.node_id, binding.port_id
            )));
        }
    }
    Ok(())
}

fn validate_output_targets(targets: &[WorkflowOutputTarget]) -> Result<(), WorkflowServiceError> {
    let mut seen = HashSet::new();
    for (index, target) in targets.iter().enumerate() {
        if target.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets.{}.node_id must be non-empty",
                index
            )));
        }
        if target.port_id.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets.{}.port_id must be non-empty",
                index
            )));
        }
        if !seen.insert((target.node_id.as_str(), target.port_id.as_str())) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets has duplicate target '{}.{}'",
                target.node_id, target.port_id
            )));
        }
    }
    Ok(())
}

fn validate_output_targets_against_io(
    targets: &[WorkflowOutputTarget],
    io: &WorkflowIoResponse,
) -> Result<(), WorkflowServiceError> {
    let discovered_outputs = discovered_output_target_set(io);

    for (index, target) in targets.iter().enumerate() {
        let key = (target.node_id.clone(), target.port_id.clone());
        if !discovered_outputs.contains(&key) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "output_targets.{} references non-discoverable output '{}.{}'",
                index, target.node_id, target.port_id
            )));
        }
    }

    Ok(())
}

fn discovered_output_target_set(io: &WorkflowIoResponse) -> HashSet<(String, String)> {
    io.outputs
        .iter()
        .flat_map(|node| {
            node.ports
                .iter()
                .map(|port| (node.node_id.clone(), port.port_id.clone()))
        })
        .collect()
}

fn collect_invalid_output_targets(
    targets: &[WorkflowOutputTarget],
    io: &WorkflowIoResponse,
) -> Vec<WorkflowOutputTarget> {
    let discovered_outputs = discovered_output_target_set(io);
    let mut invalid_targets = targets
        .iter()
        .filter(|target| !discovered_outputs.contains(&(target.node_id.clone(), target.port_id.clone())))
        .cloned()
        .collect::<Vec<_>>();
    invalid_targets.sort_by(|a, b| a.node_id.cmp(&b.node_id).then_with(|| a.port_id.cmp(&b.port_id)));
    invalid_targets
}

fn derive_required_external_inputs(io: &WorkflowIoResponse) -> Vec<WorkflowInputTarget> {
    let mut required_inputs = io
        .inputs
        .iter()
        .flat_map(|node| {
            node.ports.iter().filter_map(move |port| {
                if port.required == Some(true) {
                    Some(WorkflowInputTarget {
                        node_id: node.node_id.clone(),
                        port_id: port.port_id.clone(),
                    })
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();
    required_inputs.sort_by(|a, b| a.node_id.cmp(&b.node_id).then_with(|| a.port_id.cmp(&b.port_id)));
    required_inputs
}

fn collect_preflight_warnings(io: &WorkflowIoResponse) -> Vec<String> {
    let mut warnings = io
        .inputs
        .iter()
        .flat_map(|node| {
            node.ports.iter().filter_map(move |port| {
                if port.required.is_none() {
                    Some(format!(
                        "input surface '{}.{}' does not declare required metadata; preflight treats it as optional",
                        node.node_id, port.port_id
                    ))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();
    warnings.sort();
    warnings.push(
        "preflight performs static validation only; runtime availability is evaluated by workflow_run"
            .to_string(),
    );
    warnings
}

fn validate_requested_outputs_produced(
    targets: &[WorkflowOutputTarget],
    outputs: &[WorkflowPortBinding],
) -> Result<(), WorkflowServiceError> {
    let produced = outputs
        .iter()
        .map(|binding| (binding.node_id.as_str(), binding.port_id.as_str()))
        .collect::<HashSet<_>>();

    for target in targets {
        let key = (target.node_id.as_str(), target.port_id.as_str());
        if !produced.contains(&key) {
            return Err(WorkflowServiceError::OutputNotProduced(format!(
                "requested output target '{}.{}' was not produced",
                target.node_id, target.port_id
            )));
        }
    }

    Ok(())
}

fn validate_payload_size(
    binding: &WorkflowPortBinding,
    max_value_bytes: usize,
) -> Result<(), WorkflowServiceError> {
    let payload_bytes = serde_json::to_vec(&binding.value)
        .map_err(|e| WorkflowServiceError::InvalidRequest(format!("invalid binding value: {}", e)))?
        .len();

    if payload_bytes > max_value_bytes {
        return Err(WorkflowServiceError::CapabilityViolation(format!(
            "binding '{}.{}' payload size {} exceeds max_value_bytes {}",
            binding.node_id, binding.port_id, payload_bytes, max_value_bytes
        )));
    }

    Ok(())
}

fn validate_workflow_io(io: &WorkflowIoResponse) -> Result<(), WorkflowServiceError> {
    validate_workflow_io_nodes(&io.inputs, "inputs")?;
    validate_workflow_io_nodes(&io.outputs, "outputs")?;
    Ok(())
}

fn validate_workflow_io_nodes(
    nodes: &[WorkflowIoNode],
    field_name: &str,
) -> Result<(), WorkflowServiceError> {
    for (node_index, node) in nodes.iter().enumerate() {
        if node.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.node_id must be non-empty",
                field_name, node_index
            )));
        }
        if node.node_type.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.node_type must be non-empty",
                field_name, node_index
            )));
        }
        if node.ports.is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.ports must contain at least one entry for node '{}'",
                field_name, node_index, node.node_id
            )));
        }
        let mut seen_port_ids = HashSet::new();
        for (port_index, port) in node.ports.iter().enumerate() {
            if port.port_id.trim().is_empty() {
                return Err(WorkflowServiceError::Internal(format!(
                    "{}.{}.ports.{}.port_id must be non-empty",
                    field_name, node_index, port_index
                )));
            }
            if !seen_port_ids.insert(port.port_id.as_str()) {
                return Err(WorkflowServiceError::Internal(format!(
                    "{}.{}.ports has duplicate port_id '{}' for node '{}'",
                    field_name, node_index, port.port_id, node.node_id
                )));
            }
        }
    }
    Ok(())
}

fn derive_workflow_io(
    nodes: &[capabilities::StoredGraphNode],
) -> Result<WorkflowIoResponse, WorkflowServiceError> {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    for node in nodes {
        let Some(direction) = classify_workflow_io_direction(node) else {
            continue;
        };
        let entry = build_workflow_io_node(node, direction)?;
        match direction {
            WorkflowIoDirection::Input => inputs.push(entry),
            WorkflowIoDirection::Output => outputs.push(entry),
        }
    }

    inputs.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    outputs.sort_by(|a, b| a.node_id.cmp(&b.node_id));

    Ok(WorkflowIoResponse { inputs, outputs })
}

fn classify_workflow_io_direction(
    node: &capabilities::StoredGraphNode,
) -> Option<WorkflowIoDirection> {
    let category = extract_nested_trimmed_str(node.data(), &["definition", "category"])
        .map(|v| v.to_ascii_lowercase());
    match category.as_deref() {
        Some("input") => return Some(WorkflowIoDirection::Input),
        Some("output") => return Some(WorkflowIoDirection::Output),
        _ => {}
    }

    let node_type = node.node_type().to_ascii_lowercase();
    if node_type.ends_with("-input") {
        return Some(WorkflowIoDirection::Input);
    }
    if node_type.ends_with("-output") {
        return Some(WorkflowIoDirection::Output);
    }

    None
}

fn build_workflow_io_node(
    node: &capabilities::StoredGraphNode,
    direction: WorkflowIoDirection,
) -> Result<WorkflowIoNode, WorkflowServiceError> {
    let name = extract_nested_trimmed_str(node.data(), &["name"])
        .or_else(|| extract_nested_trimmed_str(node.data(), &["label"]))
        .or_else(|| extract_nested_trimmed_str(node.data(), &["definition", "label"]));
    let description = extract_nested_trimmed_str(node.data(), &["description"])
        .or_else(|| extract_nested_trimmed_str(node.data(), &["definition", "description"]));
    let ports = derive_workflow_io_ports(node, direction)?;

    Ok(WorkflowIoNode {
        node_id: node.id().to_string(),
        node_type: node.node_type().to_string(),
        name,
        description,
        ports,
    })
}

fn derive_workflow_io_ports(
    node: &capabilities::StoredGraphNode,
    direction: WorkflowIoDirection,
) -> Result<Vec<WorkflowIoPort>, WorkflowServiceError> {
    let key = match direction {
        WorkflowIoDirection::Input => "inputs",
        WorkflowIoDirection::Output => "outputs",
    };

    let mut ports = ports_from_definition(node, key)?;
    ports.sort_by(|a, b| a.port_id.cmp(&b.port_id));
    Ok(ports)
}

fn ports_from_definition(
    node: &capabilities::StoredGraphNode,
    key: &str,
) -> Result<Vec<WorkflowIoPort>, WorkflowServiceError> {
    let entries = node
        .data()
        .get("definition")
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "workflow I/O schema error: node '{}' missing definition.{}",
                node.id(),
                key
            ))
        })?;
    if entries.is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "workflow I/O schema error: node '{}' has empty definition.{}",
            node.id(),
            key
        )));
    }

    let mut seen_port_ids = HashSet::new();
    let mut ports = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let port_id = entry
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "workflow I/O schema error: node '{}' {}.{} has invalid id",
                    node.id(),
                    key,
                    index
                ))
            })?
            .to_string();
        if !seen_port_ids.insert(port_id.clone()) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "workflow I/O schema error: node '{}' {} has duplicate port id '{}'",
                node.id(),
                key,
                port_id
            )));
        }

        let name = entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                entry
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            });

        let description = entry
            .get("description")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        let data_type = entry
            .get("data_type")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        ports.push(WorkflowIoPort {
            port_id,
            name,
            description,
            data_type,
            required: entry.get("required").and_then(serde_json::Value::as_bool),
            multiple: entry.get("multiple").and_then(serde_json::Value::as_bool),
        })
    }

    Ok(ports)
}

fn extract_nested_trimmed_str(data: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut cursor = data;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct MockWorkflowHost {
        capabilities: WorkflowHostCapabilities,
        omit_requested_target_output: bool,
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
                        required_backends: vec!["llamacpp".to_string()],
                        required_extensions: vec!["inference_gateway".to_string()],
                    },
                    models: vec![WorkflowCapabilityModel {
                        model_id: "model-a".to_string(),
                        model_revision_or_hash: Some("sha256:hash-model-a".to_string()),
                        model_type: Some("embedding".to_string()),
                        node_ids: vec!["node-1".to_string()],
                        roles: vec!["embedding".to_string(), "inference".to_string()],
                    }],
                },
                omit_requested_target_output: false,
            }
        }

        fn with_missing_requested_output(max_input_bindings: usize, max_value_bytes: usize) -> Self {
            Self {
                omit_requested_target_output: true,
                ..Self::new(max_input_bindings, max_value_bytes)
            }
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
                },
            }
        }
    }

    struct PreflightHost {
        capabilities: WorkflowHostCapabilities,
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
                },
            }
        }
    }

    struct DefaultCapabilitiesHost {
        workflow_root: PathBuf,
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
    impl WorkflowHost for TimeoutAwareHost {
        async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
            Ok(())
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
                    return Err(WorkflowServiceError::RuntimeTimeout(
                        "workflow run cancelled".to_string(),
                    ));
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }

    #[async_trait]
    impl WorkflowHost for MockWorkflowHost {
        async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
            if workflow_id == "wf-missing" {
                return Err(WorkflowServiceError::WorkflowNotFound(
                    workflow_id.to_string(),
                ));
            }
            Ok(())
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

        async fn run_workflow(
            &self,
            _workflow_id: &str,
            inputs: &[WorkflowPortBinding],
            output_targets: Option<&[WorkflowOutputTarget]>,
            _run_options: WorkflowRunOptions,
            _run_handle: WorkflowRunHandle,
        ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
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
            timeout_ms: None,
            run_id: Some("run-1".to_string()),
        };

        let json = serde_json::to_value(&req).expect("serialize request");
        assert_eq!(json["workflow_id"], "wf-1");
        assert_eq!(json["inputs"][0]["node_id"], "input-1");
        assert_eq!(json["output_targets"][0]["port_id"], "text");
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
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("expected runtime error");

        assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
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
        assert!(response.inputs[0]
            .ports
            .iter()
            .all(|port| port.port_id != "legacy-out"));

        assert_eq!(response.outputs.len(), 1);
        assert_eq!(response.outputs[0].node_id, "text-output-1");
        assert_eq!(response.outputs[0].ports.len(), 1);
        assert_eq!(response.outputs[0].ports[0].port_id, "text");
        assert!(response.outputs[0]
            .ports
            .iter()
            .all(|port| port.port_id != "stream"));

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
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("expected output_not_produced");

        assert!(matches!(err, WorkflowServiceError::OutputNotProduced(_)));
        assert!(err
            .to_string()
            .contains("requested output target 'text-output-1.text' was not produced"));
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
                },
            )
            .await
            .expect("preflight response");

        assert!(!response.can_run);
        assert_eq!(response.missing_required_inputs.len(), 1);
        assert_eq!(response.missing_required_inputs[0].node_id, "text-input-1");
        assert_eq!(response.missing_required_inputs[0].port_id, "text");
        assert_eq!(response.invalid_targets.len(), 1);
        assert_eq!(response.invalid_targets[0].node_id, "text-output-1");
        assert_eq!(response.invalid_targets[0].port_id, "stream");
        assert!(response
            .warnings
            .iter()
            .any(|warning| warning.contains("does not declare required metadata")));
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
                },
            )
            .await
            .expect("preflight response");

        assert!(response.can_run);
        assert!(response.missing_required_inputs.is_empty());
        assert!(response.invalid_targets.is_empty());
        assert!(response
            .warnings
            .iter()
            .any(|warning| warning.contains("static validation only")));
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
                },
            )
            .await
            .expect_err("duplicate targets should fail");

        assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
        assert!(err.to_string().contains("duplicate target"));
    }

    #[test]
    fn workflow_service_error_envelope_roundtrip() {
        let err = WorkflowServiceError::OutputNotProduced(
            "requested output target 'vector-output-1.vector' was not produced".to_string(),
        );

        let envelope = err.to_envelope();
        assert_eq!(envelope.code, WorkflowErrorCode::OutputNotProduced);
        assert!(envelope.message.contains("vector-output-1.vector"));

        let json = err.to_envelope_json();
        let parsed: WorkflowErrorEnvelope =
            serde_json::from_str(&json).expect("parse workflow error envelope");
        assert_eq!(parsed.code, WorkflowErrorCode::OutputNotProduced);
        assert!(parsed.message.contains("vector-output-1.vector"));
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
                },
            )
            .await
            .expect("create session");

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
                    timeout_ms: None,
                    run_id: Some("session-run-1".to_string()),
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
            .close_workflow_session(WorkflowSessionCloseRequest {
                session_id: created.session_id.clone(),
            })
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
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("closed session should not run");
        assert!(matches!(err, WorkflowServiceError::SessionNotFound(_)));
    }

    #[tokio::test]
    async fn workflow_session_returns_evicted_when_displaced() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::with_max_sessions(1);

        let first = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: None,
                },
            )
            .await
            .expect("create first");

        let _second = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: None,
                },
            )
            .await
            .expect("create second");

        let err = service
            .run_workflow_session(
                &host,
                WorkflowSessionRunRequest {
                    session_id: first.session_id,
                    inputs: Vec::new(),
                    output_targets: None,
                    timeout_ms: None,
                    run_id: None,
                },
            )
            .await
            .expect_err("evicted session should fail");
        assert!(matches!(err, WorkflowServiceError::SessionEvicted(_)));
    }

    #[tokio::test]
    async fn workflow_session_create_returns_scheduler_busy_when_capacity_locked() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::with_max_sessions(1);
        let created = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: None,
                },
            )
            .await
            .expect("create session");

        {
            let mut store = service.session_store.lock().expect("lock session store");
            let state = store
                .active
                .get_mut(&created.session_id)
                .expect("existing active session");
            state.in_use = true;
        }

        let err = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: None,
                },
            )
            .await
            .expect_err("scheduler should be busy");
        assert!(matches!(err, WorkflowServiceError::SchedulerBusy(_)));
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
            vec!["llamacpp"]
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
}
