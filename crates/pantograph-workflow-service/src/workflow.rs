use async_trait::async_trait;
use pantograph_runtime_identity::canonical_runtime_backend_key;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::capabilities;
use crate::graph::{
    GraphSessionStore, WorkflowGraph, WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest,
    WorkflowGraphConnectRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCloseResponse, WorkflowGraphEditSessionCreateRequest,
    WorkflowGraphEditSessionCreateResponse, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphEditSessionGraphResponse, WorkflowGraphGetConnectionCandidatesRequest,
    WorkflowGraphInsertNodeAndConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest,
    WorkflowGraphListResponse, WorkflowGraphLoadRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest, WorkflowGraphSaveResponse,
    WorkflowGraphSessionStateView, WorkflowGraphStore, WorkflowGraphUndoRedoStateRequest,
    WorkflowGraphUndoRedoStateResponse, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest,
};
use crate::scheduler::{
    unix_timestamp_ms, WorkflowSessionPreflightCache, WorkflowSessionStore,
    WORKFLOW_SESSION_QUEUE_POLL_MS,
};
use crate::technical_fit::{
    WorkflowTechnicalFitDecision, WorkflowTechnicalFitOverride, WorkflowTechnicalFitRequest,
};

mod runtime_preflight;
mod session_runtime;

pub(crate) use self::runtime_preflight::evaluate_runtime_preflight;
use self::runtime_preflight::{collect_preflight_warnings, format_runtime_not_ready_message};

#[cfg(test)]
use crate::graph::WorkflowSessionKind;

pub(crate) use crate::scheduler::scheduler_snapshot_trace_execution_id;
pub use crate::scheduler::{
    select_runtime_unload_candidate_by_affinity, WorkflowSchedulerAdmissionOutcome,
    WorkflowSchedulerDecisionReason, WorkflowSchedulerRuntimeRegistryDiagnostics,
    WorkflowSchedulerRuntimeWarmupDecision, WorkflowSchedulerRuntimeWarmupReason,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowSessionInspectionRequest, WorkflowSessionInspectionResponse,
    WorkflowSessionKeepAliveRequest, WorkflowSessionKeepAliveResponse,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueCancelResponse,
    WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRetentionHint,
    WorkflowSessionRuntimeSelectionTarget, WorkflowSessionRuntimeUnloadCandidate,
    WorkflowSessionStaleCleanupRequest, WorkflowSessionStaleCleanupResponse,
    WorkflowSessionStaleCleanupWorker, WorkflowSessionStaleCleanupWorkerConfig,
    WorkflowSessionState, WorkflowSessionStatusRequest, WorkflowSessionStatusResponse,
    WorkflowSessionSummary, WorkflowSessionUnloadReason,
};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<WorkflowTechnicalFitOverride>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRuntimeInstallState {
    Installed,
    SystemProvided,
    Missing,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRuntimeSourceKind {
    #[default]
    Unknown,
    Managed,
    System,
    Host,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRuntimeReadinessState {
    Unknown,
    Missing,
    Downloading,
    Extracting,
    Validating,
    Ready,
    Failed,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRuntimeCapability {
    pub runtime_id: String,
    pub display_name: String,
    pub install_state: WorkflowRuntimeInstallState,
    pub available: bool,
    pub configured: bool,
    pub can_install: bool,
    pub can_remove: bool,
    #[serde(default)]
    pub source_kind: WorkflowRuntimeSourceKind,
    #[serde(default)]
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_state: Option<WorkflowRuntimeReadinessState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_version: Option<String>,
    #[serde(default)]
    pub supports_external_connection: bool,
    #[serde(default)]
    pub backend_keys: Vec<String>,
    #[serde(default)]
    pub missing_files: Vec<String>,
    #[serde(default)]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRuntimeIssue {
    pub runtime_id: String,
    pub display_name: String,
    pub required_backend_key: String,
    pub message: String,
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
    #[serde(default)]
    pub runtime_capabilities: Vec<WorkflowRuntimeCapability>,
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
    #[serde(default)]
    pub runtime_capabilities: Vec<WorkflowRuntimeCapability>,
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

/// Workflow preflight request for request-shape and runtime-readiness validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPreflightRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub inputs: Vec<WorkflowPortBinding>,
    #[serde(default)]
    pub output_targets: Option<Vec<WorkflowOutputTarget>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<WorkflowTechnicalFitOverride>,
}

/// Input surface reference used by preflight diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowInputTarget {
    pub node_id: String,
    pub port_id: String,
}

/// Workflow preflight response for request validation and runtime-readiness diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPreflightResponse {
    #[serde(default)]
    pub missing_required_inputs: Vec<WorkflowInputTarget>,
    #[serde(default)]
    pub invalid_targets: Vec<WorkflowOutputTarget>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub graph_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_fit_decision: Option<WorkflowTechnicalFitDecision>,
    #[serde(default)]
    pub runtime_warnings: Vec<WorkflowRuntimeIssue>,
    #[serde(default)]
    pub blocking_runtime_issues: Vec<WorkflowRuntimeIssue>,
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
    #[serde(default)]
    pub keep_alive: bool,
}

/// Session creation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionCreateResponse {
    pub session_id: String,
    #[serde(default)]
    pub runtime_capabilities: Vec<WorkflowRuntimeCapability>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<WorkflowTechnicalFitOverride>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
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
    Cancelled,
    SessionNotFound,
    SessionEvicted,
    QueueItemNotFound,
    SchedulerBusy,
    OutputNotProduced,
    RuntimeTimeout,
    InternalError,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSchedulerErrorReason {
    SessionCapacityReached,
    RuntimeCapacityExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSchedulerErrorDetails {
    pub reason: WorkflowSchedulerErrorReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_session_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_sessions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_session_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_loaded_sessions: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reclaimable_loaded_session_count: Option<usize>,
}

impl WorkflowSchedulerErrorDetails {
    pub fn session_capacity_reached(active_session_count: usize, max_sessions: usize) -> Self {
        Self {
            reason: WorkflowSchedulerErrorReason::SessionCapacityReached,
            active_session_count: Some(active_session_count),
            max_sessions: Some(max_sessions),
            loaded_session_count: None,
            max_loaded_sessions: None,
            reclaimable_loaded_session_count: None,
        }
    }

    pub fn runtime_capacity_exhausted(
        loaded_session_count: usize,
        max_loaded_sessions: usize,
        reclaimable_loaded_session_count: usize,
    ) -> Self {
        Self {
            reason: WorkflowSchedulerErrorReason::RuntimeCapacityExhausted,
            active_session_count: None,
            max_sessions: None,
            loaded_session_count: Some(loaded_session_count),
            max_loaded_sessions: Some(max_loaded_sessions),
            reclaimable_loaded_session_count: Some(reclaimable_loaded_session_count),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowErrorDetails {
    Scheduler(WorkflowSchedulerErrorDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowErrorEnvelope {
    pub code: WorkflowErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<WorkflowErrorDetails>,
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

    #[error("cancelled: {0}")]
    Cancelled(String),

    #[error("session_not_found: {0}")]
    SessionNotFound(String),

    #[error("session_evicted: {0}")]
    SessionEvicted(String),

    #[error("queue_item_not_found: {0}")]
    QueueItemNotFound(String),

    #[error("scheduler_busy: {message}")]
    SchedulerBusy {
        message: String,
        details: Option<WorkflowSchedulerErrorDetails>,
    },

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
            WorkflowServiceError::Cancelled(_) => WorkflowErrorCode::Cancelled,
            WorkflowServiceError::SessionNotFound(_) => WorkflowErrorCode::SessionNotFound,
            WorkflowServiceError::SessionEvicted(_) => WorkflowErrorCode::SessionEvicted,
            WorkflowServiceError::QueueItemNotFound(_) => WorkflowErrorCode::QueueItemNotFound,
            WorkflowServiceError::SchedulerBusy { .. } => WorkflowErrorCode::SchedulerBusy,
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
            | WorkflowServiceError::Cancelled(message)
            | WorkflowServiceError::SessionNotFound(message)
            | WorkflowServiceError::SessionEvicted(message)
            | WorkflowServiceError::QueueItemNotFound(message)
            | WorkflowServiceError::OutputNotProduced(message)
            | WorkflowServiceError::RuntimeTimeout(message)
            | WorkflowServiceError::Internal(message) => message,
            WorkflowServiceError::SchedulerBusy { message, .. } => message,
        }
    }

    pub fn details(&self) -> Option<WorkflowErrorDetails> {
        match self {
            WorkflowServiceError::SchedulerBusy {
                details: Some(details),
                ..
            } => Some(WorkflowErrorDetails::Scheduler(details.clone())),
            _ => None,
        }
    }

    pub fn scheduler_busy(message: impl Into<String>) -> Self {
        Self::SchedulerBusy {
            message: message.into(),
            details: None,
        }
    }

    pub fn scheduler_busy_with_details(
        message: impl Into<String>,
        details: WorkflowSchedulerErrorDetails,
    ) -> Self {
        Self::SchedulerBusy {
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn scheduler_session_capacity_reached(
        active_session_count: usize,
        max_sessions: usize,
    ) -> Self {
        Self::scheduler_busy_with_details(
            format!(
                "session capacity {} reached; close an existing session before creating another",
                max_sessions
            ),
            WorkflowSchedulerErrorDetails::session_capacity_reached(
                active_session_count,
                max_sessions,
            ),
        )
    }

    pub fn scheduler_runtime_capacity_exhausted(
        loaded_session_count: usize,
        max_loaded_sessions: usize,
        reclaimable_loaded_session_count: usize,
    ) -> Self {
        Self::scheduler_busy_with_details(
            "runtime capacity exhausted; no idle session runtime available for unload",
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(
                loaded_session_count,
                max_loaded_sessions,
                reclaimable_loaded_session_count,
            ),
        )
    }

    pub fn to_envelope(&self) -> WorkflowErrorEnvelope {
        WorkflowErrorEnvelope {
            code: self.code(),
            message: self.message().to_string(),
            details: self.details(),
        }
    }

    pub fn to_envelope_json(&self) -> String {
        serde_json::to_string(&self.to_envelope()).unwrap_or_else(|_| {
            r#"{"code":"internal_error","message":"failed to serialize workflow error envelope"}"#
                .to_string()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunOptions {
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_session_id: Option<String>,
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

    /// Report runtime capability state for the current host.
    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        Ok(Vec::new())
    }

    /// Resolve workflow identity and fail if it is unknown to the host.
    async fn validate_workflow(&self, workflow_id: &str) -> Result<(), WorkflowServiceError> {
        capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots()).map(|_| ())
    }

    /// Return the current graph fingerprint for session-scoped preflight caching.
    async fn workflow_graph_fingerprint(
        &self,
        workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        capabilities::workflow_graph_fingerprint(workflow_id, &self.workflow_roots())
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
            required_backends.push(canonical_runtime_backend_key(
                &self.default_backend_name().await?,
            ));
        }
        required_backends.sort();
        required_backends.dedup();

        let required_extensions = capabilities::extract_required_extensions(
            stored.nodes(),
            stored.edges(),
            !required_models.is_empty(),
        );
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
            runtime_capabilities: self.runtime_capabilities().await?,
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

    /// Load session runtime resources (model runtime, worker state) when needed.
    async fn can_load_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _usage_profile: Option<&str>,
        _retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<bool, WorkflowServiceError> {
        Ok(true)
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

    /// Unload session runtime resources when scheduler rebalances or session closes.
    async fn unload_session_runtime(
        &self,
        _session_id: &str,
        _workflow_id: &str,
        _reason: WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn select_runtime_unload_candidate(
        &self,
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        Ok(select_runtime_unload_candidate_by_affinity(
            target, candidates,
        ))
    }

    async fn workflow_technical_fit_decision(
        &self,
        _request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        Ok(None)
    }

    /// Optional backend-owned live workflow-session inspection surface for
    /// node memory, checkpoint, and residency state.
    async fn workflow_session_inspection_state(
        &self,
        _session_id: &str,
        _workflow_id: &str,
    ) -> Result<Option<WorkflowGraphSessionStateView>, WorkflowServiceError> {
        Ok(None)
    }
}

/// Backend-owned request for additive scheduler diagnostics that depend on a
/// runtime-registry or another host-specific runtime fact source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSchedulerRuntimeDiagnosticsRequest {
    pub session_id: String,
    pub workflow_id: String,
    pub usage_profile: Option<String>,
    pub keep_alive: bool,
    pub runtime_loaded: bool,
    pub next_admission_queue_id: Option<String>,
    pub reclaim_candidates: Vec<WorkflowSessionRuntimeUnloadCandidate>,
}

/// Optional backend provider for additive scheduler diagnostics that require
/// host/runtime state outside the canonical queue store.
#[async_trait]
pub trait WorkflowSchedulerDiagnosticsProvider: Send + Sync {
    async fn scheduler_runtime_registry_diagnostics(
        &self,
        _request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
    ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError> {
        Ok(None)
    }
}

const DEFAULT_MAX_SESSIONS: usize = 8;
const WORKFLOW_CANCEL_GRACE_WINDOW_MS: u64 = 250;

/// Service entrypoint for workflow API operations.
#[derive(Clone)]
pub struct WorkflowService {
    session_store: Arc<Mutex<WorkflowSessionStore>>,
    graph_session_store: Arc<GraphSessionStore>,
    scheduler_diagnostics_provider:
        Arc<Mutex<Option<Arc<dyn WorkflowSchedulerDiagnosticsProvider>>>>,
}

impl Default for WorkflowService {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowService {
    pub fn new() -> Self {
        Self::with_capacity_limits(DEFAULT_MAX_SESSIONS, DEFAULT_MAX_SESSIONS)
    }

    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self::with_capacity_limits(max_sessions, max_sessions)
    }

    pub fn with_capacity_limits(max_sessions: usize, max_loaded_sessions: usize) -> Self {
        Self {
            session_store: Arc::new(Mutex::new(WorkflowSessionStore::new(
                max_sessions,
                max_loaded_sessions,
            ))),
            graph_session_store: Arc::new(GraphSessionStore::new()),
            scheduler_diagnostics_provider: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_scheduler_diagnostics_provider(
        &self,
        provider: Option<Arc<dyn WorkflowSchedulerDiagnosticsProvider>>,
    ) -> Result<(), WorkflowServiceError> {
        let mut guard = self.scheduler_diagnostics_provider.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "scheduler diagnostics provider lock poisoned".to_string(),
            )
        })?;
        *guard = provider;
        Ok(())
    }

    pub fn set_loaded_runtime_capacity_limit(
        &self,
        max_loaded_sessions: Option<usize>,
    ) -> Result<(), WorkflowServiceError> {
        let mut store = self.session_store_guard()?;
        store.max_loaded_sessions = max_loaded_sessions
            .unwrap_or(store.max_sessions)
            .max(1)
            .min(store.max_sessions);
        Ok(())
    }

    pub(crate) fn session_store_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, WorkflowSessionStore>, WorkflowServiceError> {
        self.session_store
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("session store lock poisoned".to_string()))
    }

    pub async fn create_workflow_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionCreateRequest,
    ) -> Result<WorkflowSessionCreateResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;

        let session_id = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.create_session(
                request.workflow_id.clone(),
                request
                    .usage_profile
                    .clone()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty()),
                Vec::new(),
                Vec::new(),
                request.keep_alive,
            )?
        };

        if request.keep_alive {
            if let Err(error) = self
                .refresh_session_runtime_affinity_basis(host, &session_id, &request.workflow_id)
                .await
            {
                if let Ok(mut rollback_store) = self.session_store.lock() {
                    let _ = rollback_store.close_session(&session_id);
                }
                return Err(error);
            }
            if let Err(error) = self.ensure_session_runtime_loaded(host, &session_id).await {
                if let Ok(mut rollback_store) = self.session_store.lock() {
                    let _ = rollback_store.close_session(&session_id);
                }
                return Err(error);
            }
        }

        Ok(WorkflowSessionCreateResponse {
            session_id,
            runtime_capabilities: host.runtime_capabilities().await?,
        })
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
        validate_timeout_ms(request.timeout_ms)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }

        let queue_id = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.enqueue_run(&session_id, &request)?
        };

        let queued_run = loop {
            let session_ready_to_load = {
                let mut store = self.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal("session store lock poisoned".to_string())
                })?;
                if !store.queued_run_is_admission_candidate(&session_id, &queue_id)? {
                    None
                } else {
                    Some(store.session_summary(&session_id)?)
                }
            };
            if let Some(session) = session_ready_to_load {
                let retention_hint = if session.keep_alive {
                    WorkflowSessionRetentionHint::KeepAlive
                } else {
                    WorkflowSessionRetentionHint::Ephemeral
                };
                if !host
                    .can_load_session_runtime(
                        &session.session_id,
                        &session.workflow_id,
                        session.usage_profile.as_deref(),
                        retention_hint,
                    )
                    .await?
                {
                    if let Ok(mut store) = self.session_store.lock() {
                        let _ = store.set_queue_decision_reason_if_present(
                            &session_id,
                            &queue_id,
                            WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission,
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(WORKFLOW_SESSION_QUEUE_POLL_MS)).await;
                    continue;
                }
            }

            let maybe_queued = {
                let mut store = self.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal("session store lock poisoned".to_string())
                })?;
                store.begin_queued_run(&session_id, &queue_id)?
            };
            if let Some(queued) = maybe_queued {
                break queued;
            }
            tokio::time::sleep(Duration::from_millis(WORKFLOW_SESSION_QUEUE_POLL_MS)).await;
        };

        let preflight_cache = match self
            .ensure_session_runtime_preflight(
                host,
                &session_id,
                &queued_run.workflow_id,
                queued_run.queued.override_selection.clone(),
            )
            .await
        {
            Ok(cache) => cache,
            Err(error) => {
                if let Ok(mut store) = self.session_store.lock() {
                    let _ = store.finish_run(&session_id, &queue_id);
                }
                return Err(error);
            }
        };

        if let Err(error) = self.ensure_session_runtime_loaded(host, &session_id).await {
            if let Ok(mut store) = self.session_store.lock() {
                let _ = store.finish_run(&session_id, &queue_id);
            }
            return Err(error);
        }

        let run_result = self
            .workflow_run_internal(
                host,
                WorkflowRunRequest {
                    workflow_id: queued_run.workflow_id,
                    inputs: queued_run.queued.inputs,
                    output_targets: queued_run.queued.output_targets,
                    override_selection: queued_run.queued.override_selection,
                    timeout_ms: queued_run.queued.timeout_ms,
                    run_id: queued_run.queued.run_id,
                },
                Some(preflight_cache),
                Some(session_id.clone()),
            )
            .await;

        let finish_state = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.finish_run(&session_id, &queue_id)?
        };
        if finish_state.unload_runtime {
            host.unload_session_runtime(
                &session_id,
                &finish_state.workflow_id,
                WorkflowSessionUnloadReason::KeepAliveDisabled,
            )
            .await?;
        }

        run_result
    }

    pub async fn workflow_get_session_status(
        &self,
        request: WorkflowSessionStatusRequest,
    ) -> Result<WorkflowSessionStatusResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.touch_session(session_id)?;
        let session = store.session_summary(session_id)?;
        Ok(WorkflowSessionStatusResponse { session })
    }

    pub async fn workflow_get_session_inspection<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionInspectionRequest,
    ) -> Result<WorkflowSessionInspectionResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let session = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.touch_session(session_id)?;
            store.session_summary(session_id)?
        };
        let workflow_session_state = host
            .workflow_session_inspection_state(session_id, &session.workflow_id)
            .await?;
        Ok(WorkflowSessionInspectionResponse {
            session,
            workflow_session_state,
        })
    }

    pub async fn workflow_list_session_queue(
        &self,
        request: WorkflowSessionQueueListRequest,
    ) -> Result<WorkflowSessionQueueListResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.touch_session(session_id)?;
        let items = store.list_queue(session_id)?;
        Ok(WorkflowSessionQueueListResponse {
            session_id: session_id.to_string(),
            items,
        })
    }

    pub async fn workflow_get_scheduler_snapshot(
        &self,
        request: WorkflowSchedulerSnapshotRequest,
    ) -> Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let scheduler_diagnostics_provider = self
            .scheduler_diagnostics_provider
            .lock()
            .map_err(|_| {
                WorkflowServiceError::Internal(
                    "scheduler diagnostics provider lock poisoned".to_string(),
                )
            })?
            .clone();

        let workflow_snapshot = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            match store
                .touch_session(session_id)
                .and_then(|_| store.session_summary(session_id))
            {
                Ok(session) => {
                    let items = store.list_queue(session_id)?;
                    let runtime_diagnostics_request =
                        store.scheduler_runtime_diagnostics_request(session_id)?;
                    let diagnostics = store.scheduler_snapshot_diagnostics(session_id)?;
                    Some((session, items, diagnostics, runtime_diagnostics_request))
                }
                Err(WorkflowServiceError::SessionNotFound(_)) => None,
                Err(error) => return Err(error),
            }
        };

        if let Some((session, items, mut diagnostics, runtime_diagnostics_request)) =
            workflow_snapshot
        {
            if let Some(provider) = scheduler_diagnostics_provider.as_ref() {
                diagnostics.runtime_registry = provider
                    .scheduler_runtime_registry_diagnostics(&runtime_diagnostics_request)
                    .await?;
            }
            return Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some(session.workflow_id.clone()),
                session_id: session_id.to_string(),
                trace_execution_id: scheduler_snapshot_trace_execution_id(&items),
                session,
                items,
                diagnostics: Some(diagnostics),
            });
        }

        self.graph_session_store
            .get_scheduler_snapshot(session_id)
            .await
    }

    pub async fn workflow_cleanup_stale_sessions(
        &self,
        request: WorkflowSessionStaleCleanupRequest,
    ) -> Result<WorkflowSessionStaleCleanupResponse, WorkflowServiceError> {
        if request.idle_timeout_ms == 0 {
            return Err(WorkflowServiceError::InvalidRequest(
                "idle_timeout_ms must be greater than zero".to_string(),
            ));
        }

        let now_ms = unix_timestamp_ms();
        let candidates = {
            let store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.stale_cleanup_candidates(now_ms, request.idle_timeout_ms)
        };

        let mut cleaned_session_ids = Vec::new();
        for candidate in candidates {
            let cleaned = {
                let mut store = self.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal("session store lock poisoned".to_string())
                })?;
                store.close_stale_session_if_unchanged(&candidate, now_ms, request.idle_timeout_ms)
            };
            if cleaned {
                cleaned_session_ids.push(candidate.session_id);
            }
        }

        Ok(WorkflowSessionStaleCleanupResponse {
            cleaned_session_ids,
        })
    }

    pub fn spawn_workflow_session_stale_cleanup_worker(
        self: &Arc<Self>,
        config: WorkflowSessionStaleCleanupWorkerConfig,
    ) -> Result<WorkflowSessionStaleCleanupWorker, WorkflowServiceError> {
        if config.interval.is_zero() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow-session stale cleanup interval must be greater than zero".to_string(),
            ));
        }
        if config.idle_timeout.is_zero() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow-session stale cleanup idle timeout must be greater than zero".to_string(),
            ));
        }

        let idle_timeout_ms = config.idle_timeout.as_millis().min(u128::from(u64::MAX)) as u64;
        let interval = config.interval;
        let service = Arc::clone(self);
        let runtime_handle = tokio::runtime::Handle::try_current().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow-session stale cleanup worker requires an active Tokio runtime"
                    .to_string(),
            )
        })?;
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = runtime_handle.spawn(async move {
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = tokio::time::sleep(interval) => {
                        let _ = service
                            .workflow_cleanup_stale_sessions(WorkflowSessionStaleCleanupRequest {
                                idle_timeout_ms,
                            })
                            .await;
                    }
                }
            }
        });

        Ok(WorkflowSessionStaleCleanupWorker::new(
            shutdown_tx,
            join_handle,
        ))
    }

    pub async fn workflow_cancel_session_queue_item(
        &self,
        request: WorkflowSessionQueueCancelRequest,
    ) -> Result<WorkflowSessionQueueCancelResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let queue_id = request.queue_id.trim();
        if queue_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "queue_id must be non-empty".to_string(),
            ));
        }

        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.cancel_queue_item(session_id, queue_id)?;
        Ok(WorkflowSessionQueueCancelResponse { ok: true })
    }

    pub async fn workflow_reprioritize_session_queue_item(
        &self,
        request: WorkflowSessionQueueReprioritizeRequest,
    ) -> Result<WorkflowSessionQueueReprioritizeResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let queue_id = request.queue_id.trim();
        if queue_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "queue_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.reprioritize_queue_item(session_id, queue_id, request.priority)?;
        Ok(WorkflowSessionQueueReprioritizeResponse { ok: true })
    }

    pub async fn workflow_set_session_keep_alive<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionKeepAliveRequest,
    ) -> Result<WorkflowSessionKeepAliveResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let (state_after_update, unload_workflow_id) = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.set_keep_alive(&session_id, request.keep_alive)?
        };

        if let Some(workflow_id) = unload_workflow_id {
            host.unload_session_runtime(
                &session_id,
                &workflow_id,
                WorkflowSessionUnloadReason::KeepAliveDisabled,
            )
            .await?;
        } else if request.keep_alive
            && matches!(state_after_update, WorkflowSessionState::IdleUnloaded)
        {
            let workflow_id = {
                let store = self.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal("session store lock poisoned".to_string())
                })?;
                store.session_summary(&session_id)?.workflow_id
            };
            self.refresh_session_runtime_affinity_basis(host, &session_id, &workflow_id)
                .await?;
            self.ensure_session_runtime_loaded(host, &session_id)
                .await?;
        }

        let state = {
            let store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.session_summary(&session_id)?.state
        };
        Ok(WorkflowSessionKeepAliveResponse {
            session_id,
            keep_alive: request.keep_alive,
            state,
        })
    }

    pub async fn close_workflow_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionCloseRequest,
    ) -> Result<WorkflowSessionCloseResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let close_state = {
            let mut store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            store.close_session(&session_id)?
        };
        if close_state.runtime_loaded {
            host.unload_session_runtime(
                &session_id,
                &close_state.workflow_id,
                WorkflowSessionUnloadReason::SessionClosed,
            )
            .await?;
        }

        Ok(WorkflowSessionCloseResponse { ok: true })
    }

    pub async fn workflow_run<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_run_internal(host, request, None, None).await
    }

    async fn workflow_run_internal<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowRunRequest,
        cached_preflight: Option<WorkflowSessionPreflightCache>,
        workflow_session_id: Option<String>,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        validate_timeout_ms(request.timeout_ms)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }
        let override_selection = request
            .override_selection
            .as_ref()
            .and_then(WorkflowTechnicalFitOverride::normalized);

        let max_input_bindings = host.max_input_bindings();
        let max_output_targets = host.max_output_targets();
        let max_value_bytes = host.max_value_bytes();

        host.validate_workflow(&request.workflow_id).await?;
        if let Some(targets) = request.output_targets.as_ref() {
            let io = host.workflow_io(&request.workflow_id).await?;
            validate_workflow_io(&io)?;
            validate_output_targets_against_io(targets, &io)?;
        }
        let blocking_runtime_issues = if let Some(cache) = cached_preflight.as_ref() {
            cache.blocking_runtime_issues.clone()
        } else {
            let capabilities = host.workflow_capabilities(&request.workflow_id).await?;
            self.workflow_runtime_preflight_assessment(
                host,
                &request.workflow_id,
                &capabilities,
                override_selection,
            )
            .await?
            .blocking_runtime_issues
        };

        if !blocking_runtime_issues.is_empty() {
            return Err(WorkflowServiceError::RuntimeNotReady(
                format_runtime_not_ready_message(&blocking_runtime_issues),
            ));
        }

        if request.inputs.len() > max_input_bindings {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "input binding count {} exceeds max_input_bindings {}",
                request.inputs.len(),
                max_input_bindings
            )));
        }

        if let Some(targets) = request.output_targets.as_ref() {
            if targets.len() > max_output_targets {
                return Err(WorkflowServiceError::CapabilityViolation(format!(
                    "output target count {} exceeds max_output_targets {}",
                    targets.len(),
                    max_output_targets
                )));
            }
        }

        for binding in &request.inputs {
            validate_payload_size(binding, max_value_bytes)?;
        }

        let started = Instant::now();
        let run_options = WorkflowRunOptions {
            timeout_ms: request.timeout_ms,
            workflow_session_id,
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

        validate_host_output_bindings(&outputs, "outputs")?;
        for binding in &outputs {
            validate_payload_size(binding, max_value_bytes)?;
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
            runtime_capabilities: capabilities.runtime_capabilities,
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
        let graph_fingerprint = host
            .workflow_graph_fingerprint(&request.workflow_id)
            .await?;
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

        let runtime_preflight = self
            .workflow_runtime_preflight_assessment(
                host,
                &request.workflow_id,
                &capabilities,
                request
                    .override_selection
                    .as_ref()
                    .and_then(WorkflowTechnicalFitOverride::normalized),
            )
            .await?;
        let warnings = collect_preflight_warnings(
            &io,
            &runtime_preflight.runtime_warnings,
            &runtime_preflight.blocking_runtime_issues,
        );
        let can_run = missing_required_inputs.is_empty()
            && invalid_targets.is_empty()
            && runtime_preflight.blocking_runtime_issues.is_empty();

        Ok(WorkflowPreflightResponse {
            missing_required_inputs,
            invalid_targets,
            warnings,
            graph_fingerprint,
            technical_fit_decision: runtime_preflight.technical_fit_decision,
            runtime_warnings: runtime_preflight.runtime_warnings,
            blocking_runtime_issues: runtime_preflight.blocking_runtime_issues,
            can_run,
        })
    }

    pub async fn workflow_graph_create_edit_session(
        &self,
        request: WorkflowGraphEditSessionCreateRequest,
    ) -> Result<WorkflowGraphEditSessionCreateResponse, WorkflowServiceError> {
        Ok(self.graph_session_store.create_session(request.graph).await)
    }

    pub async fn workflow_graph_close_edit_session(
        &self,
        request: WorkflowGraphEditSessionCloseRequest,
    ) -> Result<WorkflowGraphEditSessionCloseResponse, WorkflowServiceError> {
        self.graph_session_store
            .close_session(&request.session_id)
            .await
    }

    pub async fn workflow_graph_get_edit_session_graph(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store
            .get_session_graph(&request.session_id)
            .await
    }

    pub async fn workflow_graph_get_undo_redo_state(
        &self,
        request: WorkflowGraphUndoRedoStateRequest,
    ) -> Result<WorkflowGraphUndoRedoStateResponse, WorkflowServiceError> {
        self.graph_session_store
            .get_undo_redo_state(&request.session_id)
            .await
    }

    pub async fn workflow_graph_update_node_data(
        &self,
        request: WorkflowGraphUpdateNodeDataRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.update_node_data(request).await
    }

    pub async fn workflow_graph_update_node_position(
        &self,
        request: WorkflowGraphUpdateNodePositionRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.update_node_position(request).await
    }

    pub async fn workflow_graph_add_node(
        &self,
        request: WorkflowGraphAddNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.add_node(request).await
    }

    pub async fn workflow_graph_remove_node(
        &self,
        request: WorkflowGraphRemoveNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.remove_node(request).await
    }

    pub async fn workflow_graph_add_edge(
        &self,
        request: WorkflowGraphAddEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.add_edge(request).await
    }

    pub async fn workflow_graph_remove_edge(
        &self,
        request: WorkflowGraphRemoveEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.remove_edge(request).await
    }

    pub async fn workflow_graph_undo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.undo(request).await
    }

    pub async fn workflow_graph_redo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.redo(request).await
    }

    pub async fn workflow_graph_get_connection_candidates(
        &self,
        request: WorkflowGraphGetConnectionCandidatesRequest,
    ) -> Result<crate::graph::ConnectionCandidatesResponse, WorkflowServiceError> {
        self.graph_session_store
            .get_connection_candidates(request)
            .await
    }

    pub async fn workflow_graph_connect(
        &self,
        request: WorkflowGraphConnectRequest,
    ) -> Result<crate::graph::ConnectionCommitResponse, WorkflowServiceError> {
        self.graph_session_store.connect(request).await
    }

    pub async fn workflow_graph_insert_node_and_connect(
        &self,
        request: WorkflowGraphInsertNodeAndConnectRequest,
    ) -> Result<crate::graph::InsertNodeConnectionResponse, WorkflowServiceError> {
        self.graph_session_store
            .insert_node_and_connect(request)
            .await
    }

    pub async fn workflow_graph_preview_node_insert_on_edge(
        &self,
        request: WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    ) -> Result<crate::graph::EdgeInsertionPreviewResponse, WorkflowServiceError> {
        self.graph_session_store
            .preview_node_insert_on_edge(request)
            .await
    }

    pub async fn workflow_graph_insert_node_on_edge(
        &self,
        request: WorkflowGraphInsertNodeOnEdgeRequest,
    ) -> Result<crate::graph::InsertNodeOnEdgeResponse, WorkflowServiceError> {
        self.graph_session_store.insert_node_on_edge(request).await
    }

    pub fn workflow_graph_save<S: WorkflowGraphStore>(
        &self,
        store: &S,
        request: WorkflowGraphSaveRequest,
    ) -> Result<WorkflowGraphSaveResponse, WorkflowServiceError> {
        let path = store.save_workflow(request.name, request.graph)?;
        Ok(WorkflowGraphSaveResponse { path })
    }

    pub fn workflow_graph_load<S: WorkflowGraphStore>(
        &self,
        store: &S,
        request: WorkflowGraphLoadRequest,
    ) -> Result<crate::graph::WorkflowFile, WorkflowServiceError> {
        store.load_workflow(request.path)
    }

    pub fn workflow_graph_list<S: WorkflowGraphStore>(
        &self,
        store: &S,
    ) -> Result<WorkflowGraphListResponse, WorkflowServiceError> {
        let workflows = store.list_workflows()?;
        Ok(WorkflowGraphListResponse { workflows })
    }

    pub async fn workflow_graph_get_runtime_snapshot(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        let response = self
            .graph_session_store
            .get_session_graph(session_id)
            .await?;
        Ok(response.graph)
    }

    pub async fn workflow_graph_mark_edit_session_running(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.graph_session_store.mark_running(session_id).await
    }

    pub async fn workflow_graph_mark_edit_session_finished(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.graph_session_store.finish_run(session_id).await
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

pub(crate) fn validate_workflow_id(workflow_id: &str) -> Result<(), WorkflowServiceError> {
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

fn validate_host_output_bindings(
    bindings: &[WorkflowPortBinding],
    field_name: &str,
) -> Result<(), WorkflowServiceError> {
    let mut seen = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        if binding.node_id.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.node_id must be non-empty",
                field_name, index
            )));
        }
        if binding.port_id.trim().is_empty() {
            return Err(WorkflowServiceError::Internal(format!(
                "{}.{}.port_id must be non-empty",
                field_name, index
            )));
        }
        if !seen.insert((binding.node_id.as_str(), binding.port_id.as_str())) {
            return Err(WorkflowServiceError::Internal(format!(
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
        .filter(|target| {
            !discovered_outputs.contains(&(target.node_id.clone(), target.port_id.clone()))
        })
        .cloned()
        .collect::<Vec<_>>();
    invalid_targets.sort_by(|a, b| {
        a.node_id
            .cmp(&b.node_id)
            .then_with(|| a.port_id.cmp(&b.port_id))
    });
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
    required_inputs.sort_by(|a, b| {
        a.node_id
            .cmp(&b.node_id)
            .then_with(|| a.port_id.cmp(&b.port_id))
    });
    required_inputs
}

fn compute_runtime_capability_fingerprint(
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> String {
    let mut normalized = runtime_capabilities.to_vec();
    normalized.sort_by(|a, b| a.runtime_id.cmp(&b.runtime_id));
    for capability in &mut normalized {
        capability.backend_keys.sort();
        capability.missing_files.sort();
    }

    let encoded = serde_json::to_string(&normalized).unwrap_or_default();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in encoded.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
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
        let Some(direction) = classify_workflow_io_direction(node)? else {
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
) -> Result<Option<WorkflowIoDirection>, WorkflowServiceError> {
    let category = extract_nested_trimmed_str(node.data(), &["definition", "category"])
        .map(|v| v.to_ascii_lowercase());
    let Some(direction) = (match category.as_deref() {
        Some("input") => Some(WorkflowIoDirection::Input),
        Some("output") => Some(WorkflowIoDirection::Output),
        _ => None,
    }) else {
        return Ok(None);
    };

    let origin = extract_nested_trimmed_str(node.data(), &["definition", "io_binding_origin"])
        .map(|v| v.to_ascii_lowercase())
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "workflow I/O schema error: node '{}' missing definition.io_binding_origin",
                node.id()
            ))
        })?;

    match origin.as_str() {
        "client_session" => Ok(Some(direction)),
        "integrated" => Ok(None),
        _ => Err(WorkflowServiceError::InvalidRequest(format!(
            "workflow I/O schema error: node '{}' has invalid definition.io_binding_origin '{}'",
            node.id(),
            origin
        ))),
    }
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
    use crate::technical_fit::{
        WorkflowTechnicalFitReason, WorkflowTechnicalFitReasonCode,
        WorkflowTechnicalFitSelectionMode,
    };
    use crate::WorkflowSchedulerRuntimeCapacityPressure;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use tokio::sync::Notify;

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

        fn with_missing_requested_output(
            max_input_bindings: usize,
            max_value_bytes: usize,
        ) -> Self {
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
        ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError>
        {
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
        assert!(err
            .to_string()
            .contains("technical-fit could not select a ready runtime"));
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
        assert!(err
            .to_string()
            .contains("outputs.0.port_id must be non-empty"));
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
        assert!(err
            .to_string()
            .contains("missing definition.io_binding_origin"));
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
                    override_selection: None,
                },
            )
            .await
            .expect("preflight response");

        assert!(response.can_run);
        assert_eq!(response.graph_fingerprint, "preflight-graph");
        assert!(response.missing_required_inputs.is_empty());
        assert!(response.invalid_targets.is_empty());
        assert!(response
            .warnings
            .iter()
            .any(|warning| warning.contains("does not declare required metadata")));
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
        assert!(blocking_runtime_issues[0]
            .message
            .contains("Remote llama.cpp is not configured"));
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
        assert!(blocking_runtime_issues[0]
            .message
            .contains("selected version 'b8248' is validating"));
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
        assert!(unloads
            .iter()
            .any(|(session_id, _)| session_id == &third_session_id));
        assert!(!unloads
            .iter()
            .any(|(session_id, _)| session_id == &first.session_id));
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
        assert!(unloads
            .iter()
            .any(|session_id| session_id == &target.session_id));
        assert!(!unloads
            .iter()
            .any(|session_id| session_id == &affine.session_id));
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
        assert!(unloads
            .iter()
            .any(|session_id| session_id == &target.session_id));
        assert!(!unloads
            .iter()
            .any(|session_id| session_id == &shared_model.session_id));
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
        assert!(unloads
            .iter()
            .any(|session_id| session_id == &target.session_id));
        assert!(!unloads
            .iter()
            .any(|session_id| session_id == &shared_backend.session_id));
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
    async fn workflow_session_create_surfaces_runtime_capacity_details_when_no_unload_candidate_available(
    ) {
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

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_returns_workflow_session_summary() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
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
            .expect("create workflow session");

        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id.clone(),
            })
            .await
            .expect("scheduler snapshot");

        assert_eq!(snapshot.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(snapshot.session_id, created.session_id);
        assert_eq!(snapshot.session.session_kind, WorkflowSessionKind::Workflow);
        assert_eq!(snapshot.session.workflow_id, "wf-1");
        assert_eq!(
            snapshot.session.usage_profile.as_deref(),
            Some("interactive")
        );
        assert_eq!(snapshot.trace_execution_id, None);
        assert!(snapshot.items.is_empty());
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_returns_edit_session_lifecycle() {
        let service = WorkflowService::new();
        let created = service
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: WorkflowGraph::new(),
            })
            .await
            .expect("create edit session");

        let idle_snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id.clone(),
            })
            .await
            .expect("idle edit snapshot");
        assert_eq!(idle_snapshot.workflow_id, None);
        assert_eq!(
            idle_snapshot.session.session_kind,
            WorkflowSessionKind::Edit
        );
        assert_eq!(
            idle_snapshot.session.state,
            WorkflowSessionState::IdleLoaded
        );
        assert_eq!(idle_snapshot.session.queued_runs, 0);
        assert_eq!(idle_snapshot.session.run_count, 0);
        assert_eq!(idle_snapshot.trace_execution_id, None);
        assert!(idle_snapshot.items.is_empty());

        service
            .workflow_graph_mark_edit_session_running(&created.session_id)
            .await
            .expect("mark running");

        let running_snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id.clone(),
            })
            .await
            .expect("running edit snapshot");
        assert_eq!(
            running_snapshot.session.session_kind,
            WorkflowSessionKind::Edit
        );
        assert_eq!(
            running_snapshot.session.state,
            WorkflowSessionState::Running
        );
        assert_eq!(running_snapshot.session.queued_runs, 1);
        assert_eq!(running_snapshot.items.len(), 1);
        assert_eq!(
            running_snapshot.items[0].status,
            WorkflowSessionQueueItemStatus::Running
        );
        let started_at_ms = running_snapshot.items[0]
            .enqueued_at_ms
            .expect("edit session running item should expose start time");
        assert_eq!(
            running_snapshot.items[0].dequeued_at_ms,
            Some(started_at_ms)
        );
        assert_eq!(
            running_snapshot.items[0].run_id.as_deref(),
            Some(created.session_id.as_str())
        );
        assert_eq!(
            running_snapshot.trace_execution_id.as_deref(),
            Some(created.session_id.as_str())
        );

        service
            .workflow_graph_mark_edit_session_finished(&created.session_id)
            .await
            .expect("finish running edit session");

        let completed_snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id,
            })
            .await
            .expect("completed edit snapshot");
        assert_eq!(
            completed_snapshot.session.state,
            WorkflowSessionState::IdleLoaded
        );
        assert_eq!(completed_snapshot.session.queued_runs, 0);
        assert_eq!(completed_snapshot.session.run_count, 1);
        assert_eq!(completed_snapshot.trace_execution_id, None);
        assert!(completed_snapshot.items.is_empty());
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_exposes_single_visible_queue_run_as_trace_execution() {
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
                        priority: None,
                    },
                )
                .expect("enqueue run");
        }

        let session_id = created.session_id.clone();
        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
            .await
            .expect("scheduler snapshot");

        assert_eq!(snapshot.trace_execution_id.as_deref(), Some("queued-run-1"));
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(
            snapshot.items[0].status,
            WorkflowSessionQueueItemStatus::Pending
        );
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_exposes_next_admission_diagnostics() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::with_capacity_limits(2, 1);
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
            .expect("create workflow session");

        let queue_id = {
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
                .expect("enqueue run")
        };

        let session_id = created.session_id.clone();
        let before_snapshot_ms = unix_timestamp_ms();
        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
            .await
            .expect("scheduler snapshot");
        let after_snapshot_ms = unix_timestamp_ms();
        let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

        assert_eq!(diagnostics.loaded_session_count, 0);
        assert_eq!(diagnostics.max_loaded_sessions, 1);
        assert_eq!(diagnostics.reclaimable_loaded_session_count, 0);
        assert_eq!(
            diagnostics.runtime_capacity_pressure,
            WorkflowSchedulerRuntimeCapacityPressure::Available
        );
        assert!(!diagnostics.active_run_blocks_admission);
        assert_eq!(
            diagnostics.next_admission_queue_id.as_deref(),
            Some(queue_id.as_str())
        );
        assert_eq!(diagnostics.next_admission_bypassed_queue_id, None);
        assert_eq!(diagnostics.next_admission_after_runs, Some(0));
        assert_eq!(diagnostics.next_admission_wait_ms, Some(0));
        let next_admission_not_before_ms = diagnostics
            .next_admission_not_before_ms
            .expect("immediate admission not-before timestamp");
        assert!(next_admission_not_before_ms >= before_snapshot_ms);
        assert!(next_admission_not_before_ms <= after_snapshot_ms);
        assert_eq!(
            diagnostics.next_admission_reason,
            Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
        );
        assert_eq!(diagnostics.runtime_registry, None);
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_merges_runtime_registry_diagnostics_from_provider() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::with_capacity_limits(3, 1);
        let requests = Arc::new(Mutex::new(Vec::new()));
        service
            .set_scheduler_diagnostics_provider(Some(Arc::new(MockSchedulerDiagnosticsProvider {
                diagnostics: WorkflowSchedulerRuntimeRegistryDiagnostics {
                    target_runtime_id: Some("llama_cpp".to_string()),
                    reclaim_candidate_session_id: Some("session-loaded".to_string()),
                    reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
                    next_warmup_decision: Some(
                        WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,
                    ),
                    next_warmup_reason: Some(
                        WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady,
                    ),
                },
                requests: requests.clone(),
            })))
            .expect("scheduler diagnostics provider should be installed");

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
            .expect("create loaded session");
        let created = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-queued".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                },
            )
            .await
            .expect("create queued session");

        let queue_id = {
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
                .expect("enqueue run")
        };

        let session_id = created.session_id.clone();
        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
            .await
            .expect("scheduler snapshot");
        let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

        assert_eq!(
            diagnostics.runtime_registry,
            Some(WorkflowSchedulerRuntimeRegistryDiagnostics {
                target_runtime_id: Some("llama_cpp".to_string()),
                reclaim_candidate_session_id: Some("session-loaded".to_string()),
                reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
                next_warmup_decision: Some(
                    WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,
                ),
                next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady,),
            })
        );
        let recorded_requests = requests
            .lock()
            .expect("scheduler diagnostics requests lock poisoned");
        assert_eq!(recorded_requests.len(), 1);
        assert_eq!(recorded_requests[0].session_id, created.session_id);
        assert_eq!(recorded_requests[0].workflow_id, "wf-queued");
        assert_eq!(
            recorded_requests[0].usage_profile.as_deref(),
            Some("interactive")
        );
        assert!(!recorded_requests[0].keep_alive);
        assert!(!recorded_requests[0].runtime_loaded);
        assert_eq!(
            recorded_requests[0].next_admission_queue_id.as_deref(),
            Some(queue_id.as_str())
        );
        assert_eq!(recorded_requests[0].reclaim_candidates.len(), 1);
        assert_eq!(
            recorded_requests[0].reclaim_candidates[0].session_id,
            loaded.session_id
        );
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_reports_bypassed_queue_head_for_warm_reuse() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
        let created = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                },
            )
            .await
            .expect("create workflow session");

        let (cold_head_queue_id, warm_queue_id) = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .update_runtime_affinity_basis(
                    &created.session_id,
                    vec!["llama_cpp".to_string()],
                    vec!["model-a".to_string()],
                )
                .expect("update runtime affinity basis");
            store
                .mark_runtime_loaded(&created.session_id, true)
                .expect("mark runtime loaded");
            let cold_head_queue_id = store
                .enqueue_run(
                    &created.session_id,
                    &WorkflowSessionRunRequest {
                        session_id: created.session_id.clone(),
                        inputs: Vec::new(),
                        output_targets: None,
                        override_selection: Some(WorkflowTechnicalFitOverride {
                            model_id: Some("model-b".to_string()),
                            backend_key: Some("pytorch".to_string()),
                        }),
                        timeout_ms: None,
                        run_id: Some("cold-head".to_string()),
                        priority: Some(1),
                    },
                )
                .expect("enqueue cold head");
            let warm_queue_id = store
                .enqueue_run(
                    &created.session_id,
                    &WorkflowSessionRunRequest {
                        session_id: created.session_id.clone(),
                        inputs: Vec::new(),
                        output_targets: None,
                        override_selection: None,
                        timeout_ms: None,
                        run_id: Some("warm-follow".to_string()),
                        priority: Some(1),
                    },
                )
                .expect("enqueue warm follow");
            (cold_head_queue_id, warm_queue_id)
        };

        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id,
            })
            .await
            .expect("scheduler snapshot");
        let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

        assert_eq!(
            diagnostics.next_admission_queue_id.as_deref(),
            Some(warm_queue_id.as_str())
        );
        assert_eq!(
            diagnostics.next_admission_bypassed_queue_id.as_deref(),
            Some(cold_head_queue_id.as_str())
        );
        assert_eq!(
            diagnostics.next_admission_reason,
            Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
        );
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_marks_rebalance_required_when_idle_runtime_can_be_reclaimed(
    ) {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::with_capacity_limits(3, 1);
        let _loaded = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-loaded".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                },
            )
            .await
            .expect("create loaded session");
        let created = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-queued".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                },
            )
            .await
            .expect("create queued session");

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
        }

        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id,
            })
            .await
            .expect("scheduler snapshot");
        let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

        assert_eq!(diagnostics.loaded_session_count, 1);
        assert_eq!(diagnostics.max_loaded_sessions, 1);
        assert_eq!(diagnostics.reclaimable_loaded_session_count, 1);
        assert_eq!(
            diagnostics.runtime_capacity_pressure,
            WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired
        );
        assert_eq!(
            snapshot.session.state,
            WorkflowSessionState::IdleUnloaded,
            "the queued session should still be unloaded before admission"
        );
    }

    #[tokio::test]
    async fn workflow_get_scheduler_snapshot_omits_trace_execution_for_ambiguous_pending_queue() {
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
            for run_id in ["queued-run-1", "queued-run-2"] {
                store
                    .enqueue_run(
                        &created.session_id,
                        &WorkflowSessionRunRequest {
                            session_id: created.session_id.clone(),
                            inputs: Vec::new(),
                            output_targets: None,
                            override_selection: None,
                            timeout_ms: None,
                            run_id: Some(run_id.to_string()),
                            priority: None,
                        },
                    )
                    .expect("enqueue run");
            }
        }

        let snapshot = service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id,
            })
            .await
            .expect("scheduler snapshot");

        assert_eq!(snapshot.trace_execution_id, None);
        assert_eq!(snapshot.items.len(), 2);
        assert!(snapshot
            .items
            .iter()
            .all(|item| item.status == WorkflowSessionQueueItemStatus::Pending));
    }

    #[tokio::test]
    async fn workflow_session_queue_items_include_authoritative_timestamps() {
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

        let request = WorkflowSessionRunRequest {
            session_id: created.session_id.clone(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
            timeout_ms: None,
            run_id: Some("queued-run-1".to_string()),
            priority: None,
        };

        let queue_id = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .enqueue_run(&created.session_id, &request)
                .expect("enqueue run")
        };

        let pending_items = {
            let store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .list_queue(&created.session_id)
                .expect("list pending queue items")
        };
        assert_eq!(pending_items.len(), 1);
        assert_eq!(pending_items[0].queue_id, queue_id);
        assert_eq!(pending_items[0].run_id.as_deref(), Some("queued-run-1"));
        assert!(pending_items[0].enqueued_at_ms.is_some());
        assert!(pending_items[0].dequeued_at_ms.is_none());
        assert_eq!(pending_items[0].queue_position, Some(0));
        assert_eq!(
            pending_items[0].scheduler_admission_outcome,
            Some(WorkflowSchedulerAdmissionOutcome::Queued)
        );
        assert_eq!(
            pending_items[0].status,
            WorkflowSessionQueueItemStatus::Pending
        );
        assert_eq!(
            pending_items[0].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::HighestPriorityFirst)
        );

        let running_items = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .begin_queued_run(&created.session_id, &queue_id)
                .expect("begin queued run");
            store
                .list_queue(&created.session_id)
                .expect("list running queue items")
        };
        assert_eq!(running_items.len(), 1);
        assert_eq!(running_items[0].queue_id, queue_id);
        assert_eq!(
            running_items[0].status,
            WorkflowSessionQueueItemStatus::Running
        );
        assert_eq!(
            running_items[0].enqueued_at_ms,
            pending_items[0].enqueued_at_ms
        );
        assert_eq!(running_items[0].queue_position, Some(0));
        assert_eq!(
            running_items[0].scheduler_admission_outcome,
            Some(WorkflowSchedulerAdmissionOutcome::Admitted)
        );
        assert!(running_items[0].dequeued_at_ms.is_some());
        assert!(
            running_items[0]
                .dequeued_at_ms
                .expect("dequeued timestamp present")
                >= running_items[0]
                    .enqueued_at_ms
                    .expect("enqueued timestamp present")
        );
        assert_eq!(
            running_items[0].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
        );
    }

    #[tokio::test]
    async fn workflow_session_queue_marks_loaded_compatible_admission_as_warm_reuse() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
        let created = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                },
            )
            .await
            .expect("create workflow session");

        let queue_id = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .mark_runtime_loaded(&created.session_id, true)
                .expect("mark runtime loaded");
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
                .expect("enqueue run")
        };

        let running_items = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .begin_queued_run(&created.session_id, &queue_id)
                .expect("begin queued run");
            store
                .list_queue(&created.session_id)
                .expect("list running queue items")
        };

        assert_eq!(running_items.len(), 1);
        assert_eq!(running_items[0].queue_id, queue_id);
        assert_eq!(
            running_items[0].scheduler_admission_outcome,
            Some(WorkflowSchedulerAdmissionOutcome::Admitted)
        );
        assert_eq!(
            running_items[0].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
        );
    }

    #[tokio::test]
    async fn workflow_session_queue_prefers_bounded_warm_reuse_over_same_priority_cold_head() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
        let created = service
            .create_workflow_session(
                &host,
                WorkflowSessionCreateRequest {
                    workflow_id: "wf-1".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                },
            )
            .await
            .expect("create workflow session");

        let (cold_head_queue_id, warm_queue_id) = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .update_runtime_affinity_basis(
                    &created.session_id,
                    vec!["llama_cpp".to_string()],
                    vec!["model-a".to_string()],
                )
                .expect("update runtime affinity basis");
            store
                .mark_runtime_loaded(&created.session_id, true)
                .expect("mark runtime loaded");
            let cold_head_queue_id = store
                .enqueue_run(
                    &created.session_id,
                    &WorkflowSessionRunRequest {
                        session_id: created.session_id.clone(),
                        inputs: Vec::new(),
                        output_targets: None,
                        override_selection: Some(WorkflowTechnicalFitOverride {
                            model_id: Some("model-b".to_string()),
                            backend_key: Some("pytorch".to_string()),
                        }),
                        timeout_ms: None,
                        run_id: Some("cold-head".to_string()),
                        priority: Some(1),
                    },
                )
                .expect("enqueue cold head");
            let warm_queue_id = store
                .enqueue_run(
                    &created.session_id,
                    &WorkflowSessionRunRequest {
                        session_id: created.session_id.clone(),
                        inputs: Vec::new(),
                        output_targets: None,
                        override_selection: None,
                        timeout_ms: None,
                        run_id: Some("warm-follow".to_string()),
                        priority: Some(1),
                    },
                )
                .expect("enqueue warm follow");
            (cold_head_queue_id, warm_queue_id)
        };

        let running_items = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .begin_queued_run(&created.session_id, &warm_queue_id)
                .expect("begin queued run");
            store
                .list_queue(&created.session_id)
                .expect("list running queue items")
        };

        assert_eq!(running_items.len(), 2);
        assert_eq!(running_items[0].queue_id, warm_queue_id);
        assert_eq!(
            running_items[0].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
        );
        assert_eq!(running_items[1].queue_id, cold_head_queue_id);
        assert_eq!(
            running_items[1].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::HighestPriorityFirst)
        );
    }

    #[tokio::test]
    async fn workflow_session_queue_items_expose_authoritative_queue_positions() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
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
            .expect("create workflow session");

        let first_queue_id = {
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
                        priority: Some(10),
                    },
                )
                .expect("enqueue first run")
        };
        let second_queue_id = {
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
                        run_id: Some("queued-run-2".to_string()),
                        priority: Some(5),
                    },
                )
                .expect("enqueue second run")
        };

        let pending_items = {
            let store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .list_queue(&created.session_id)
                .expect("list pending queue items")
        };
        assert_eq!(pending_items.len(), 2);
        assert_eq!(pending_items[0].queue_id, first_queue_id);
        assert_eq!(pending_items[0].queue_position, Some(0));
        assert_eq!(pending_items[1].queue_id, second_queue_id);
        assert_eq!(pending_items[1].queue_position, Some(1));

        let running_items = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .begin_queued_run(&created.session_id, &first_queue_id)
                .expect("begin first run");
            store
                .list_queue(&created.session_id)
                .expect("list queue after begin")
        };
        assert_eq!(running_items.len(), 2);
        assert_eq!(running_items[0].queue_id, first_queue_id);
        assert_eq!(running_items[0].queue_position, Some(0));
        assert_eq!(running_items[1].queue_id, second_queue_id);
        assert_eq!(running_items[1].queue_position, Some(1));
    }

    #[tokio::test]
    async fn workflow_session_queue_promotes_starved_runs_before_newer_higher_priority_runs() {
        let host = MockWorkflowHost::new(8, 1024);
        let service = WorkflowService::new();
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
            .expect("create workflow session");

        let low_priority_queue_id = {
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
                        run_id: Some("older-low-priority".to_string()),
                        priority: Some(0),
                    },
                )
                .expect("enqueue low priority run")
        };

        {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            for run_id in [
                "newer-high-priority-1",
                "newer-high-priority-2",
                "newer-high-priority-3",
                "newer-high-priority-4",
            ] {
                store
                    .enqueue_run(
                        &created.session_id,
                        &WorkflowSessionRunRequest {
                            session_id: created.session_id.clone(),
                            inputs: Vec::new(),
                            output_targets: None,
                            override_selection: None,
                            timeout_ms: None,
                            run_id: Some(run_id.to_string()),
                            priority: Some(2),
                        },
                    )
                    .expect("enqueue higher priority run");
            }
        }

        let pending_items = {
            let store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .list_queue(&created.session_id)
                .expect("list starved queue items")
        };
        assert_eq!(pending_items.len(), 5);
        assert_eq!(pending_items[0].queue_id, low_priority_queue_id);
        assert_eq!(pending_items[0].queue_position, Some(0));
        assert_eq!(
            pending_items[0].scheduler_admission_outcome,
            Some(WorkflowSchedulerAdmissionOutcome::Queued)
        );
        assert_eq!(
            pending_items[0].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::StarvationProtection)
        );
        assert_eq!(
            pending_items[1].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::FifoPriorityTieBreak)
        );

        let running_items = {
            let mut store = service
                .session_store
                .lock()
                .expect("session store lock poisoned");
            store
                .begin_queued_run(&created.session_id, &low_priority_queue_id)
                .expect("admit starved queue item");
            store
                .list_queue(&created.session_id)
                .expect("list running queue items")
        };
        assert_eq!(running_items[0].queue_id, low_priority_queue_id);
        assert_eq!(
            running_items[0].scheduler_admission_outcome,
            Some(WorkflowSchedulerAdmissionOutcome::Admitted)
        );
        assert_eq!(
            running_items[0].scheduler_decision_reason,
            Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
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
}
