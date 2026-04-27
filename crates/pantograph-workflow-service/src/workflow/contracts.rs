use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use pantograph_diagnostics_ledger::DiagnosticsLedgerError;
use pantograph_runtime_attribution::AttributionError;

use crate::graph::{
    WorkflowExecutableTopology, WorkflowGraph, WorkflowGraphRunSettings,
    WorkflowPresentationMetadata,
};
use crate::technical_fit::{WorkflowTechnicalFitDecision, WorkflowTechnicalFitOverride};

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
#[serde(deny_unknown_fields)]
pub struct WorkflowRunRequest {
    pub workflow_id: String,
    pub workflow_semantic_version: String,
    #[serde(default)]
    pub inputs: Vec<WorkflowPortBinding>,
    #[serde(default)]
    pub output_targets: Option<Vec<WorkflowOutputTarget>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<WorkflowTechnicalFitOverride>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Workflow run response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct WorkflowRunResponse {
    pub workflow_run_id: String,
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

/// Request a historic workflow graph using the immutable run snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct WorkflowRunGraphQueryRequest {
    pub workflow_run_id: String,
}

/// Response for historic run graph lookup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunGraphQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_graph: Option<WorkflowRunGraphProjection>,
}

/// Historic workflow graph reconstructed from versioned execution,
/// presentation, and per-run settings records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowRunGraphProjection {
    pub workflow_run_id: String,
    pub workflow_id: String,
    pub workflow_version_id: String,
    pub workflow_presentation_revision_id: String,
    pub workflow_semantic_version: String,
    pub workflow_execution_fingerprint: String,
    pub snapshot_created_at_ms: i64,
    pub workflow_version_created_at_ms: i64,
    pub presentation_revision_created_at_ms: i64,
    pub graph: WorkflowGraph,
    pub executable_topology: WorkflowExecutableTopology,
    pub presentation_metadata: WorkflowPresentationMetadata,
    pub graph_settings: WorkflowGraphRunSettings,
}

/// Request local Network page state for the current Pantograph instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct WorkflowLocalNetworkStatusQueryRequest {
    #[serde(default = "default_include_network_interfaces")]
    pub include_network_interfaces: bool,
    #[serde(default = "default_include_disks")]
    pub include_disks: bool,
}

fn default_include_network_interfaces() -> bool {
    true
}

fn default_include_disks() -> bool {
    true
}

/// Local-first Network page response. Peer nodes are intentionally empty until
/// the future Iroh transport supplies trusted peer records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalNetworkStatusQueryResponse {
    pub local_node: WorkflowLocalNetworkNodeStatus,
    #[serde(default)]
    pub peer_nodes: Vec<WorkflowPeerNetworkNodeStatus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNetworkTransportState {
    LocalOnly,
    PeerNetworkingUnavailable,
    PairingRequired,
    Connected,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalNetworkNodeStatus {
    pub node_id: String,
    pub display_name: String,
    pub captured_at_ms: u64,
    pub transport_state: WorkflowNetworkTransportState,
    pub system: WorkflowLocalSystemMetrics,
    pub scheduler_load: WorkflowLocalSchedulerLoad,
    #[serde(default)]
    pub degradation_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPeerNetworkNodeStatus {
    pub node_id: String,
    pub display_name: String,
    pub transport_state: WorkflowNetworkTransportState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalSystemMetrics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kernel_version: Option<String>,
    pub cpu: WorkflowLocalCpuMetrics,
    pub memory: WorkflowLocalMemoryMetrics,
    #[serde(default)]
    pub disks: Vec<WorkflowLocalDiskMetrics>,
    #[serde(default)]
    pub network_interfaces: Vec<WorkflowLocalNetworkInterfaceMetrics>,
    pub gpu: WorkflowLocalGpuMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalCpuMetrics {
    pub logical_core_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_usage_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalMemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalDiskMetrics {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalNetworkInterfaceMetrics {
    pub name: String,
    pub total_received_bytes: u64,
    pub total_transmitted_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalGpuMetrics {
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowLocalSchedulerLoad {
    pub max_sessions: usize,
    pub active_session_count: usize,
    pub max_loaded_sessions: usize,
    pub loaded_session_count: usize,
    pub active_run_count: usize,
    pub queued_run_count: usize,
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
    /// Legacy structural fingerprint used for preflight/cache invalidation.
    /// It is not workflow-version identity; version-aware diagnostics must use
    /// workflow_version_id and node behavior-version facts instead.
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
pub struct WorkflowExecutionSessionCreateRequest {
    pub workflow_id: String,
    #[serde(default)]
    pub usage_profile: Option<String>,
    #[serde(default)]
    pub keep_alive: bool,
}

/// Session creation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionCreateResponse {
    pub session_id: String,
    #[serde(default)]
    pub runtime_capabilities: Vec<WorkflowRuntimeCapability>,
}

/// Session run request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionRunRequest {
    pub session_id: String,
    pub workflow_semantic_version: String,
    #[serde(default)]
    pub inputs: Vec<WorkflowPortBinding>,
    #[serde(default)]
    pub output_targets: Option<Vec<WorkflowOutputTarget>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_selection: Option<WorkflowTechnicalFitOverride>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub priority: Option<i32>,
}

/// Session close request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionCloseRequest {
    pub session_id: String,
}

/// Session close response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutionSessionCloseResponse {
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

impl From<AttributionError> for WorkflowServiceError {
    fn from(error: AttributionError) -> Self {
        match error {
            AttributionError::Storage(_) | AttributionError::UnsupportedSchemaVersion { .. } => {
                Self::Internal(error.to_string())
            }
            _ => Self::InvalidRequest(error.to_string()),
        }
    }
}

impl From<DiagnosticsLedgerError> for WorkflowServiceError {
    fn from(error: DiagnosticsLedgerError) -> Self {
        match error {
            DiagnosticsLedgerError::MissingField { .. }
            | DiagnosticsLedgerError::FieldTooLong { .. }
            | DiagnosticsLedgerError::InvalidField { .. }
            | DiagnosticsLedgerError::InvalidTimeRange
            | DiagnosticsLedgerError::QueryLimitExceeded { .. }
            | DiagnosticsLedgerError::UnsupportedEventKind { .. }
            | DiagnosticsLedgerError::InvalidEventSource { .. }
            | DiagnosticsLedgerError::EventPayloadTooLarge { .. } => {
                Self::InvalidRequest(error.to_string())
            }
            DiagnosticsLedgerError::UnsupportedSchemaVersion { .. }
            | DiagnosticsLedgerError::Storage(_)
            | DiagnosticsLedgerError::Serialization(_) => Self::Internal(error.to_string()),
        }
    }
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
    pub workflow_execution_session_id: Option<String>,
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

impl Default for WorkflowRunHandle {
    fn default() -> Self {
        Self::new()
    }
}
