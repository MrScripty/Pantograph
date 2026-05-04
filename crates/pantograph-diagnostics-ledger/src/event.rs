use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use serde::{Deserialize, Serialize};

use crate::util::{validate_optional_text, validate_required_text, MAX_ID_LEN, MAX_JSON_LEN};
use crate::DiagnosticsLedgerError;

pub const DIAGNOSTIC_EVENT_SCHEMA_VERSION: i64 = 1;
pub const MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES: usize = 8_192;
pub const SCHEDULER_TIMELINE_PROJECTION_NAME: &str = "scheduler_timeline";
pub const SCHEDULER_TIMELINE_PROJECTION_VERSION: i64 = 4;
pub const RUN_LIST_PROJECTION_NAME: &str = "run_list";
pub const RUN_LIST_PROJECTION_VERSION: i64 = 7;
pub const RUN_DETAIL_PROJECTION_NAME: &str = "run_detail";
pub const RUN_DETAIL_PROJECTION_VERSION: i64 = 6;
pub const IO_ARTIFACT_PROJECTION_NAME: &str = "io_artifact";
pub const IO_ARTIFACT_PROJECTION_VERSION: i64 = 6;
pub const LIBRARY_USAGE_PROJECTION_NAME: &str = "library_usage";
pub const LIBRARY_USAGE_PROJECTION_VERSION: i64 = 1;
pub const NODE_STATUS_PROJECTION_NAME: &str = "node_status";
pub const NODE_STATUS_PROJECTION_VERSION: i64 = 4;
pub const MAX_DIAGNOSTIC_ERROR_TEXT_LEN: usize = 4_096;
pub const MAX_DIAGNOSTIC_ERROR_CAUSE_COUNT: usize = 8;
pub const MAX_DIAGNOSTIC_ERROR_CAUSE_LEN: usize = 1_024;
pub const MAX_INFERENCE_OPTION_DIAGNOSTICS: usize = 64;
pub const MAX_INFERENCE_COMPATIBILITY_ISSUES: usize = 32;
pub const MAX_INFERENCE_KV_CACHE_REASON_LEN: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventKind {
    SchedulerEstimateProduced,
    SchedulerQueuePlacement,
    SchedulerQueueControl,
    SchedulerRunDelayed,
    SchedulerModelLifecycleChanged,
    SchedulerRunAdmitted,
    SchedulerReservationChanged,
    RunStarted,
    RunTerminal,
    RunSnapshotAccepted,
    IoArtifactObserved,
    RetentionArtifactStateChanged,
    LibraryAssetAccessed,
    RetentionPolicyChanged,
    RuntimeCapabilityObserved,
    NodeExecutionStatus,
    InferenceExecutionDiagnosticObserved,
    DiagnosticErrorOccurred,
}

impl DiagnosticEventKind {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::SchedulerEstimateProduced => "scheduler.estimate_produced",
            Self::SchedulerQueuePlacement => "scheduler.queue_placement",
            Self::SchedulerQueueControl => "scheduler.queue_control",
            Self::SchedulerRunDelayed => "scheduler.run_delayed",
            Self::SchedulerModelLifecycleChanged => "scheduler.model_lifecycle_changed",
            Self::SchedulerRunAdmitted => "scheduler.run_admitted",
            Self::SchedulerReservationChanged => "scheduler.reservation_changed",
            Self::RunStarted => "run.started",
            Self::RunTerminal => "run.terminal",
            Self::RunSnapshotAccepted => "run.snapshot_accepted",
            Self::IoArtifactObserved => "io.artifact_observed",
            Self::RetentionArtifactStateChanged => "retention.artifact_state_changed",
            Self::LibraryAssetAccessed => "library.asset_accessed",
            Self::RetentionPolicyChanged => "retention.policy_changed",
            Self::RuntimeCapabilityObserved => "runtime.capability_observed",
            Self::NodeExecutionStatus => "node.execution_status",
            Self::InferenceExecutionDiagnosticObserved => "inference.execution_diagnostic_observed",
            Self::DiagnosticErrorOccurred => "diagnostic.error_occurred",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "scheduler.estimate_produced" => Ok(Self::SchedulerEstimateProduced),
            "scheduler.queue_placement" => Ok(Self::SchedulerQueuePlacement),
            "scheduler.queue_control" => Ok(Self::SchedulerQueueControl),
            "scheduler.run_delayed" => Ok(Self::SchedulerRunDelayed),
            "scheduler.model_lifecycle_changed" => Ok(Self::SchedulerModelLifecycleChanged),
            "scheduler.run_admitted" => Ok(Self::SchedulerRunAdmitted),
            "scheduler.reservation_changed" => Ok(Self::SchedulerReservationChanged),
            "run.started" => Ok(Self::RunStarted),
            "run.terminal" => Ok(Self::RunTerminal),
            "run.snapshot_accepted" => Ok(Self::RunSnapshotAccepted),
            "io.artifact_observed" => Ok(Self::IoArtifactObserved),
            "retention.artifact_state_changed" => Ok(Self::RetentionArtifactStateChanged),
            "library.asset_accessed" => Ok(Self::LibraryAssetAccessed),
            "retention.policy_changed" => Ok(Self::RetentionPolicyChanged),
            "runtime.capability_observed" => Ok(Self::RuntimeCapabilityObserved),
            "node.execution_status" => Ok(Self::NodeExecutionStatus),
            "inference.execution_diagnostic_observed" => {
                Ok(Self::InferenceExecutionDiagnosticObserved)
            }
            "diagnostic.error_occurred" => Ok(Self::DiagnosticErrorOccurred),
            _ => Err(DiagnosticsLedgerError::UnsupportedEventKind {
                event_kind: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventSourceComponent {
    Scheduler,
    WorkflowService,
    Runtime,
    NodeExecution,
    Retention,
    Library,
    LocalObserver,
}

impl DiagnosticEventSourceComponent {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Scheduler => "scheduler",
            Self::WorkflowService => "workflow_service",
            Self::Runtime => "runtime",
            Self::NodeExecution => "node_execution",
            Self::Retention => "retention",
            Self::Library => "library",
            Self::LocalObserver => "local_observer",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "scheduler" => Ok(Self::Scheduler),
            "workflow_service" => Ok(Self::WorkflowService),
            "runtime" => Ok(Self::Runtime),
            "node_execution" => Ok(Self::NodeExecution),
            "retention" => Ok(Self::Retention),
            "library" => Ok(Self::Library),
            "local_observer" => Ok(Self::LocalObserver),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "source_component",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventPrivacyClass {
    SystemMetadata,
    UserMetadata,
    SensitiveReference,
}

impl DiagnosticEventPrivacyClass {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::SystemMetadata => "system_metadata",
            Self::UserMetadata => "user_metadata",
            Self::SensitiveReference => "sensitive_reference",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "system_metadata" => Ok(Self::SystemMetadata),
            "user_metadata" => Ok(Self::UserMetadata),
            "sensitive_reference" => Ok(Self::SensitiveReference),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "privacy_class",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventRetentionClass {
    AuditMetadata,
    PayloadReference,
}

impl DiagnosticEventRetentionClass {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::AuditMetadata => "audit_metadata",
            Self::PayloadReference => "payload_reference",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "audit_metadata" => Ok(Self::AuditMetadata),
            "payload_reference" => Ok(Self::PayloadReference),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "event_retention_class",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoArtifactRetentionState {
    Retained,
    MetadataOnly,
    External,
    Truncated,
    TooLarge,
    Expired,
    Deleted,
}

impl IoArtifactRetentionState {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Retained => "retained",
            Self::MetadataOnly => "metadata_only",
            Self::External => "external",
            Self::Truncated => "truncated",
            Self::TooLarge => "too_large",
            Self::Expired => "expired",
            Self::Deleted => "deleted",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "retained" => Ok(Self::Retained),
            "metadata_only" => Ok(Self::MetadataOnly),
            "external" => Ok(Self::External),
            "truncated" => Ok(Self::Truncated),
            "too_large" => Ok(Self::TooLarge),
            "expired" => Ok(Self::Expired),
            "deleted" => Ok(Self::Deleted),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "retention_state",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "payload_type")]
pub enum DiagnosticEventPayload {
    SchedulerEstimateProduced(SchedulerEstimateProducedPayload),
    SchedulerQueuePlacement(SchedulerQueuePlacementPayload),
    SchedulerQueueControl(SchedulerQueueControlPayload),
    SchedulerRunDelayed(SchedulerRunDelayedPayload),
    SchedulerModelLifecycleChanged(SchedulerModelLifecycleChangedPayload),
    SchedulerRunAdmitted(SchedulerRunAdmittedPayload),
    SchedulerReservationChanged(SchedulerReservationChangedPayload),
    RunStarted(RunStartedPayload),
    RunTerminal(RunTerminalPayload),
    RunSnapshotAccepted(RunSnapshotAcceptedPayload),
    IoArtifactObserved(IoArtifactObservedPayload),
    RetentionArtifactStateChanged(RetentionArtifactStateChangedPayload),
    LibraryAssetAccessed(LibraryAssetAccessedPayload),
    RetentionPolicyChanged(RetentionPolicyChangedPayload),
    RuntimeCapabilityObserved(RuntimeCapabilityObservedPayload),
    NodeExecutionStatus(NodeExecutionStatusPayload),
    InferenceExecutionDiagnosticObserved(InferenceExecutionDiagnosticObservedPayload),
    DiagnosticErrorOccurred(DiagnosticErrorOccurredPayload),
}

impl DiagnosticEventPayload {
    pub fn event_kind(&self) -> DiagnosticEventKind {
        match self {
            Self::SchedulerEstimateProduced(_) => DiagnosticEventKind::SchedulerEstimateProduced,
            Self::SchedulerQueuePlacement(_) => DiagnosticEventKind::SchedulerQueuePlacement,
            Self::SchedulerQueueControl(_) => DiagnosticEventKind::SchedulerQueueControl,
            Self::SchedulerRunDelayed(_) => DiagnosticEventKind::SchedulerRunDelayed,
            Self::SchedulerModelLifecycleChanged(_) => {
                DiagnosticEventKind::SchedulerModelLifecycleChanged
            }
            Self::SchedulerRunAdmitted(_) => DiagnosticEventKind::SchedulerRunAdmitted,
            Self::SchedulerReservationChanged(_) => {
                DiagnosticEventKind::SchedulerReservationChanged
            }
            Self::RunStarted(_) => DiagnosticEventKind::RunStarted,
            Self::RunTerminal(_) => DiagnosticEventKind::RunTerminal,
            Self::RunSnapshotAccepted(_) => DiagnosticEventKind::RunSnapshotAccepted,
            Self::IoArtifactObserved(_) => DiagnosticEventKind::IoArtifactObserved,
            Self::RetentionArtifactStateChanged(_) => {
                DiagnosticEventKind::RetentionArtifactStateChanged
            }
            Self::LibraryAssetAccessed(_) => DiagnosticEventKind::LibraryAssetAccessed,
            Self::RetentionPolicyChanged(_) => DiagnosticEventKind::RetentionPolicyChanged,
            Self::RuntimeCapabilityObserved(_) => DiagnosticEventKind::RuntimeCapabilityObserved,
            Self::NodeExecutionStatus(_) => DiagnosticEventKind::NodeExecutionStatus,
            Self::InferenceExecutionDiagnosticObserved(_) => {
                DiagnosticEventKind::InferenceExecutionDiagnosticObserved
            }
            Self::DiagnosticErrorOccurred(_) => DiagnosticEventKind::DiagnosticErrorOccurred,
        }
    }

    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        match self {
            Self::SchedulerEstimateProduced(payload) => payload.validate(),
            Self::SchedulerQueuePlacement(payload) => payload.validate(),
            Self::SchedulerQueueControl(payload) => payload.validate(),
            Self::SchedulerRunDelayed(payload) => payload.validate(),
            Self::SchedulerModelLifecycleChanged(payload) => payload.validate(),
            Self::SchedulerRunAdmitted(payload) => payload.validate(),
            Self::SchedulerReservationChanged(payload) => payload.validate(),
            Self::RunStarted(payload) => payload.validate(),
            Self::RunTerminal(payload) => payload.validate(),
            Self::RunSnapshotAccepted(payload) => payload.validate(),
            Self::IoArtifactObserved(payload) => payload.validate(),
            Self::RetentionArtifactStateChanged(payload) => payload.validate(),
            Self::LibraryAssetAccessed(payload) => payload.validate(),
            Self::RetentionPolicyChanged(payload) => payload.validate(),
            Self::RuntimeCapabilityObserved(payload) => payload.validate(),
            Self::NodeExecutionStatus(payload) => payload.validate(),
            Self::InferenceExecutionDiagnosticObserved(payload) => payload.validate(),
            Self::DiagnosticErrorOccurred(payload) => payload.validate(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerEstimateProducedPayload {
    pub estimate_version: String,
    pub confidence: String,
    pub estimated_queue_wait_ms: Option<u64>,
    pub estimated_duration_ms: Option<u64>,
    #[serde(default)]
    pub model_cache_state: Option<SchedulerModelCacheState>,
    #[serde(default)]
    pub blocking_conditions: Vec<SchedulerEstimateBlockingCondition>,
    #[serde(default)]
    pub missing_asset_ids: Vec<String>,
    #[serde(default)]
    pub candidate_runtime_ids: Vec<String>,
    #[serde(default)]
    pub candidate_device_ids: Vec<String>,
    #[serde(default)]
    pub candidate_network_node_ids: Vec<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
}

impl SchedulerEstimateProducedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("estimate_version", &self.estimate_version, MAX_ID_LEN)?;
        validate_required_text("confidence", &self.confidence, MAX_ID_LEN)?;
        validate_text_list("missing_asset_ids", &self.missing_asset_ids)?;
        validate_text_list("candidate_runtime_ids", &self.candidate_runtime_ids)?;
        validate_text_list("candidate_device_ids", &self.candidate_device_ids)?;
        validate_text_list(
            "candidate_network_node_ids",
            &self.candidate_network_node_ids,
        )?;
        validate_text_list("reasons", &self.reasons)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerEstimateBlockingCondition {
    RuntimeAdmissionPending,
    QueueBacklog,
    RuntimeUnavailable,
    ModelCacheUnknown,
    MissingAsset,
}

impl SchedulerEstimateBlockingCondition {
    pub(crate) fn summary(self) -> &'static str {
        match self {
            Self::RuntimeAdmissionPending => "runtime admission pending",
            Self::QueueBacklog => "queue backlog",
            Self::RuntimeUnavailable => "runtime unavailable",
            Self::ModelCacheUnknown => "model cache unknown",
            Self::MissingAsset => "missing asset",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerQueuePlacementPayload {
    pub queue_position: u32,
    pub priority: i32,
    pub scheduler_policy_id: String,
}

impl SchedulerQueuePlacementPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("scheduler_policy_id", &self.scheduler_policy_id, MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerQueueControlAction {
    Cancel,
    PushToFront,
    Reprioritize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerQueueControlOutcome {
    Accepted,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerQueueControlActorScope {
    BackendControlApi,
    ClientSession,
    GuiAdmin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerQueueControlPayload {
    pub action: SchedulerQueueControlAction,
    pub outcome: SchedulerQueueControlOutcome,
    pub actor_scope: SchedulerQueueControlActorScope,
    #[serde(default)]
    pub requested_session_id: Option<String>,
    #[serde(default)]
    pub effective_session_id: Option<String>,
    pub previous_queue_position: Option<u32>,
    pub previous_priority: Option<i32>,
    pub new_priority: Option<i32>,
    pub reason: Option<String>,
}

impl SchedulerQueueControlPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text(
            "requested_session_id",
            self.requested_session_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "effective_session_id",
            self.effective_session_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("queue_control_reason", self.reason.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerRunDelayedPayload {
    pub reason: String,
    pub delayed_until_ms: Option<i64>,
    pub fairness_context: Option<String>,
}

impl SchedulerRunDelayedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("delay_reason", &self.reason, MAX_ID_LEN)?;
        validate_optional_text(
            "fairness_context",
            self.fairness_context.as_deref(),
            MAX_JSON_LEN,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerRunAdmittedPayload {
    pub queue_wait_ms: Option<u64>,
    pub decision_reason: String,
    #[serde(default)]
    pub selected_runtime_id: Option<String>,
    #[serde(default)]
    pub selected_device_id: Option<String>,
    #[serde(default)]
    pub selected_network_node_id: Option<String>,
    #[serde(default)]
    pub reserved_model_ids: Vec<String>,
}

impl SchedulerRunAdmittedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text(
            "admission_decision_reason",
            &self.decision_reason,
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_runtime_id",
            self.selected_runtime_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_device_id",
            self.selected_device_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_network_node_id",
            self.selected_network_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_text_list("reserved_model_ids", &self.reserved_model_ids)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerReservationTransition {
    Created,
    Released,
}

impl SchedulerReservationTransition {
    fn summary(self) -> &'static str {
        match self {
            Self::Created => "reservation created",
            Self::Released => "reservation released",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerReservationResourceKind {
    RuntimeSlot,
}

impl SchedulerReservationResourceKind {
    fn summary(self) -> &'static str {
        match self {
            Self::RuntimeSlot => "runtime slot",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerReservationChangedPayload {
    pub transition: SchedulerReservationTransition,
    pub reservation_id: String,
    pub resource_kind: SchedulerReservationResourceKind,
    #[serde(default)]
    pub selected_runtime_id: Option<String>,
    #[serde(default)]
    pub selected_device_id: Option<String>,
    #[serde(default)]
    pub selected_network_node_id: Option<String>,
    #[serde(default)]
    pub reserved_model_ids: Vec<String>,
    pub reason: Option<String>,
}

impl SchedulerReservationChangedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("reservation_id", &self.reservation_id, MAX_ID_LEN)?;
        validate_optional_text(
            "selected_runtime_id",
            self.selected_runtime_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_device_id",
            self.selected_device_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_network_node_id",
            self.selected_network_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_text_list("reserved_model_ids", &self.reserved_model_ids)?;
        validate_optional_text("reservation_reason", self.reason.as_deref(), MAX_ID_LEN)
    }

    pub(crate) fn summary(&self) -> String {
        format!(
            "{} {}",
            self.resource_kind.summary(),
            self.transition.summary()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerModelLifecycleTransition {
    LoadRequested,
    LoadDependencyResolved,
    LoadStarted,
    LoadCompleted,
    LoadFailed,
    UnloadScheduled,
    UnloadCancelled,
    UnloadStarted,
    UnloadCompleted,
    UnloadFailed,
}

impl SchedulerModelLifecycleTransition {
    fn summary(self) -> &'static str {
        match self {
            Self::LoadRequested => "model load requested",
            Self::LoadDependencyResolved => "model load dependency resolved",
            Self::LoadStarted => "model load started",
            Self::LoadCompleted => "model load completed",
            Self::LoadFailed => "model load failed",
            Self::UnloadScheduled => "model unload scheduled",
            Self::UnloadCancelled => "model unload cancelled",
            Self::UnloadStarted => "model unload started",
            Self::UnloadCompleted => "model unload completed",
            Self::UnloadFailed => "model unload failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerModelCacheState {
    Unknown,
    NotRequired,
    CacheHit,
    CacheMiss,
    LoadRequested,
    Loaded,
    UnloadRequested,
    Unloaded,
    Failed,
}

impl SchedulerModelCacheState {
    pub fn for_lifecycle_transition(transition: SchedulerModelLifecycleTransition) -> Self {
        match transition {
            SchedulerModelLifecycleTransition::LoadRequested
            | SchedulerModelLifecycleTransition::LoadDependencyResolved
            | SchedulerModelLifecycleTransition::LoadStarted => Self::LoadRequested,
            SchedulerModelLifecycleTransition::LoadCompleted => Self::Loaded,
            SchedulerModelLifecycleTransition::LoadFailed => Self::Failed,
            SchedulerModelLifecycleTransition::UnloadScheduled
            | SchedulerModelLifecycleTransition::UnloadCancelled
            | SchedulerModelLifecycleTransition::UnloadStarted => Self::UnloadRequested,
            SchedulerModelLifecycleTransition::UnloadCompleted => Self::Unloaded,
            SchedulerModelLifecycleTransition::UnloadFailed => Self::Failed,
        }
    }

    pub(crate) fn summary(self) -> &'static str {
        match self {
            Self::Unknown => "cache state unknown",
            Self::NotRequired => "model not required",
            Self::CacheHit => "model cache hit",
            Self::CacheMiss => "model cache miss",
            Self::LoadRequested => "model load requested",
            Self::Loaded => "model loaded",
            Self::UnloadRequested => "model unload requested",
            Self::Unloaded => "model unloaded",
            Self::Failed => "model cache failed",
        }
    }

    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::NotRequired => "not_required",
            Self::CacheHit => "cache_hit",
            Self::CacheMiss => "cache_miss",
            Self::LoadRequested => "load_requested",
            Self::Loaded => "loaded",
            Self::UnloadRequested => "unload_requested",
            Self::Unloaded => "unloaded",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerModelLifecycleChangedPayload {
    pub transition: SchedulerModelLifecycleTransition,
    #[serde(default)]
    pub cache_state: Option<SchedulerModelCacheState>,
    pub reason: Option<String>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_error_event_id: Option<String>,
}

impl SchedulerModelLifecycleChangedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("model_lifecycle_reason", self.reason.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("model_lifecycle_error", self.error.as_deref(), MAX_JSON_LEN)?;
        validate_optional_text(
            "canonical_error_event_id",
            self.canonical_error_event_id.as_deref(),
            MAX_ID_LEN,
        )
    }

    pub(crate) fn summary(&self) -> &'static str {
        self.transition.summary()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunStartedPayload {
    pub queue_wait_ms: Option<u64>,
    pub scheduler_decision_reason: Option<String>,
}

impl RunStartedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text(
            "scheduler_decision_reason",
            self.scheduler_decision_reason.as_deref(),
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTerminalStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunTerminalPayload {
    pub status: RunTerminalStatus,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_error_event_id: Option<String>,
}

impl RunTerminalPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("error", self.error.as_deref(), MAX_JSON_LEN)?;
        validate_optional_text(
            "canonical_error_event_id",
            self.canonical_error_event_id.as_deref(),
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunSnapshotAcceptedPayload {
    pub workflow_run_snapshot_id: String,
    pub workflow_presentation_revision_id: String,
    pub workflow_execution_session_id: String,
    pub node_versions: Vec<RunSnapshotNodeVersionPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RunSnapshotNodeVersionPayload {
    pub node_id: String,
    pub node_type: String,
    pub contract_version: String,
    pub behavior_digest: String,
}

impl RunSnapshotAcceptedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text(
            "workflow_run_snapshot_id",
            &self.workflow_run_snapshot_id,
            MAX_ID_LEN,
        )?;
        validate_required_text(
            "workflow_presentation_revision_id",
            &self.workflow_presentation_revision_id,
            MAX_ID_LEN,
        )?;
        validate_required_text(
            "workflow_execution_session_id",
            &self.workflow_execution_session_id,
            MAX_ID_LEN,
        )?;
        for node_version in &self.node_versions {
            node_version.validate()?;
        }
        Ok(())
    }
}

impl RunSnapshotNodeVersionPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("node_id", &self.node_id, MAX_ID_LEN)?;
        validate_required_text("node_type", &self.node_type, MAX_ID_LEN)?;
        validate_required_text("contract_version", &self.contract_version, MAX_ID_LEN)?;
        validate_required_text("behavior_digest", &self.behavior_digest, MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoArtifactRole {
    NodeInput,
    NodeOutput,
    WorkflowInput,
    WorkflowOutput,
}

impl IoArtifactRole {
    pub(crate) fn as_db(&self) -> &'static str {
        match self {
            Self::NodeInput => "node_input",
            Self::NodeOutput => "node_output",
            Self::WorkflowInput => "workflow_input",
            Self::WorkflowOutput => "workflow_output",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoArtifactPayloadKind {
    Text,
    Image,
    Audio,
    Video,
    #[serde(rename = "3d")]
    ThreeD,
    LargeTable,
    GenericBinary,
    Structured,
}

impl IoArtifactPayloadKind {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::ThreeD => "3d",
            Self::LargeTable => "large_table",
            Self::GenericBinary => "generic_binary",
            Self::Structured => "structured",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            "3d" => Ok(Self::ThreeD),
            "large_table" => Ok(Self::LargeTable),
            "generic_binary" => Ok(Self::GenericBinary),
            "structured" => Ok(Self::Structured),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "io_artifact_payload_kind",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoArtifactLifecycleState {
    Declared,
    Writing,
    Streaming,
    Finalizing,
    Retained,
    Failed,
    Expired,
    Deleted,
}

impl IoArtifactLifecycleState {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Declared => "declared",
            Self::Writing => "writing",
            Self::Streaming => "streaming",
            Self::Finalizing => "finalizing",
            Self::Retained => "retained",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Deleted => "deleted",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "declared" => Ok(Self::Declared),
            "writing" => Ok(Self::Writing),
            "streaming" => Ok(Self::Streaming),
            "finalizing" => Ok(Self::Finalizing),
            "retained" => Ok(Self::Retained),
            "failed" => Ok(Self::Failed),
            "expired" => Ok(Self::Expired),
            "deleted" => Ok(Self::Deleted),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "io_artifact_lifecycle_state",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoArtifactAccessMode {
    Read,
    Download,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IoArtifactConversionStatus {
    Converted,
    PassedThrough,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IoArtifactConversionDependency {
    pub dependency_id: String,
    pub active_version: String,
    pub lease_id: String,
    pub lease_holder: String,
}

impl IoArtifactConversionDependency {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text(
            "artifact_conversion_dependency_id",
            &self.dependency_id,
            MAX_ID_LEN,
        )?;
        validate_required_text(
            "artifact_conversion_dependency_active_version",
            &self.active_version,
            MAX_ID_LEN,
        )?;
        validate_required_text("artifact_conversion_lease_id", &self.lease_id, MAX_ID_LEN)?;
        validate_required_text(
            "artifact_conversion_lease_holder",
            &self.lease_holder,
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IoArtifactFormatMetadata {
    pub format_id: String,
    pub media_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crf: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converter_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_status: Option<IoArtifactConversionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conversion_dependencies: Vec<IoArtifactConversionDependency>,
}

impl IoArtifactFormatMetadata {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("artifact_format_id", &self.format_id, MAX_ID_LEN)?;
        validate_required_text("artifact_media_type", &self.media_type, MAX_ID_LEN)?;
        validate_optional_text("artifact_codec_id", self.codec_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("artifact_bit_depth", self.bit_depth.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "artifact_color_profile_id",
            self.color_profile_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "artifact_converter_id",
            self.converter_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "artifact_converter_version",
            self.converter_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "artifact_library_version",
            self.library_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "artifact_conversion_id",
            self.conversion_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "artifact_conversion_command_id",
            self.conversion_command_id.as_deref(),
            MAX_ID_LEN,
        )?;
        for dependency in &self.conversion_dependencies {
            dependency.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IoArtifactObservedPayload {
    pub artifact_id: String,
    pub artifact_role: IoArtifactRole,
    #[serde(default)]
    pub producer_node_id: Option<String>,
    #[serde(default)]
    pub producer_port_id: Option<String>,
    #[serde(default)]
    pub consumer_node_id: Option<String>,
    #[serde(default)]
    pub consumer_port_id: Option<String>,
    pub media_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<String>,
    #[serde(default)]
    pub retention_state: Option<IoArtifactRetentionState>,
    #[serde(default)]
    pub retention_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_kind: Option<IoArtifactPayloadKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_state: Option<IoArtifactLifecycleState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub access_modes: Vec<IoArtifactAccessMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<IoArtifactFormatMetadata>,
}

impl IoArtifactObservedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("artifact_id", &self.artifact_id, MAX_ID_LEN)?;
        validate_optional_text(
            "producer_node_id",
            self.producer_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "producer_port_id",
            self.producer_port_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "consumer_node_id",
            self.consumer_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "consumer_port_id",
            self.consumer_port_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("media_type", self.media_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("content_hash", self.content_hash.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "retention_reason",
            self.retention_reason.as_deref(),
            MAX_JSON_LEN,
        )?;
        validate_optional_text("read_handle", self.read_handle.as_deref(), MAX_JSON_LEN)?;
        validate_optional_text("stream_handle", self.stream_handle.as_deref(), MAX_JSON_LEN)?;
        if let Some(format) = &self.format {
            format.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryAssetOperation {
    Access,
    Delete,
    Download,
    Import,
    RunUsage,
    Search,
}

impl LibraryAssetOperation {
    pub(crate) fn as_db(&self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Delete => "delete",
            Self::Download => "download",
            Self::Import => "import",
            Self::RunUsage => "run_usage",
            Self::Search => "search",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryAssetCacheStatus {
    Hit,
    Miss,
    NotApplicable,
    Unknown,
}

impl LibraryAssetCacheStatus {
    pub(crate) fn as_db(&self) -> &'static str {
        match self {
            Self::Hit => "hit",
            Self::Miss => "miss",
            Self::NotApplicable => "not_applicable",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LibraryAssetAccessedPayload {
    pub asset_id: String,
    pub operation: LibraryAssetOperation,
    pub cache_status: Option<LibraryAssetCacheStatus>,
    pub network_bytes: Option<u64>,
}

impl LibraryAssetAccessedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_library_resource_id("asset_id", &self.asset_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionPolicyActorScope {
    GuiAdmin,
    Maintenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RetentionPolicyChangedPayload {
    pub policy_id: String,
    pub policy_version: u32,
    pub retention_days: u32,
    pub actor_scope: RetentionPolicyActorScope,
    pub reason: String,
}

impl RetentionPolicyChangedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("policy_id", &self.policy_id, MAX_ID_LEN)?;
        if self.policy_version == 0 || self.retention_days == 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "retention_policy",
            });
        }
        validate_required_text("reason", &self.reason, MAX_JSON_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RetentionArtifactStateChangedPayload {
    pub artifact_id: String,
    pub retention_state: IoArtifactRetentionState,
    pub actor_scope: RetentionPolicyActorScope,
    pub reason: String,
}

impl RetentionArtifactStateChangedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("artifact_id", &self.artifact_id, MAX_ID_LEN)?;
        validate_required_text("reason", &self.reason, MAX_JSON_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeCapabilityObservedPayload {
    pub runtime_id: String,
    pub runtime_version: Option<String>,
    pub status: String,
}

impl RuntimeCapabilityObservedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("runtime_id", &self.runtime_id, MAX_ID_LEN)?;
        validate_optional_text(
            "runtime_version",
            self.runtime_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_required_text("status", &self.status, MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecutionProjectionStatus {
    Queued,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

impl NodeExecutionProjectionStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "waiting" => Ok(Self::Waiting),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "node_execution_status",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NodeExecutionStatusPayload {
    pub status: NodeExecutionProjectionStatus,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
}

impl NodeExecutionStatusPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("error", self.error.as_deref(), MAX_JSON_LEN)?;
        validate_optional_text("task_id", self.task_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "selected_backend_key",
            self.selected_backend_key.as_deref(),
            MAX_ID_LEN,
        )?;
        if let (Some(started_at_ms), Some(completed_at_ms)) =
            (self.started_at_ms, self.completed_at_ms)
        {
            if completed_at_ms < started_at_ms {
                return Err(DiagnosticsLedgerError::InvalidField {
                    field: "completed_at_ms",
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceExecutionDiagnosticObservedPayload {
    pub request_id: String,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle_event_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_artifact_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<InferenceUsageDiagnosticSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_handle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kv_cache: Option<InferenceKvCacheDiagnosticSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_report: Option<InferenceCompatibilityReportDiagnosticSummary>,
    #[serde(default)]
    pub compatibility_issue_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatibility_issues: Vec<InferenceCompatibilityIssueDiagnosticSummary>,
    #[serde(default)]
    pub option_support_counts: InferenceOptionSupportCounts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_diagnostics: Vec<InferenceOptionDiagnosticSummary>,
}

impl InferenceExecutionDiagnosticObservedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("request_id", &self.request_id, MAX_ID_LEN)?;
        validate_required_text("task_id", &self.task_id, MAX_ID_LEN)?;
        validate_optional_text(
            "lifecycle_phase",
            self.lifecycle_phase.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "lifecycle_event_kind",
            self.lifecycle_event_kind.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_backend_key",
            self.selected_backend_key.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_backend_family",
            self.selected_backend_family.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_device_id",
            self.selected_device_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "resolved_artifact_kind",
            self.resolved_artifact_kind.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "cache_handle_id",
            self.cache_handle_id.as_deref(),
            MAX_ID_LEN,
        )?;
        if self.option_diagnostics.len() > MAX_INFERENCE_OPTION_DIAGNOSTICS {
            return Err(DiagnosticsLedgerError::FieldTooLong {
                field: "option_diagnostics",
                max_len: MAX_INFERENCE_OPTION_DIAGNOSTICS,
            });
        }
        if self.compatibility_issues.len() > MAX_INFERENCE_COMPATIBILITY_ISSUES {
            return Err(DiagnosticsLedgerError::FieldTooLong {
                field: "compatibility_issues",
                max_len: MAX_INFERENCE_COMPATIBILITY_ISSUES,
            });
        }
        if let Some(report) = self.compatibility_report.as_ref() {
            report.validate()?;
        }
        if let Some(usage) = self.usage.as_ref() {
            usage.validate()?;
        }
        if let Some(kv_cache) = self.kv_cache.as_ref() {
            kv_cache.validate()?;
        }
        for issue in &self.compatibility_issues {
            issue.validate()?;
        }
        for diagnostic in &self.option_diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceUsageDiagnosticSummary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

impl InferenceUsageDiagnosticSummary {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        if matches!(
            (
                self.prompt_tokens,
                self.completion_tokens,
                self.total_tokens
            ),
            (None, None, None)
        ) {
            return Err(DiagnosticsLedgerError::MissingField { field: "usage" });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceKvCacheDiagnosticSummary {
    pub action: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl InferenceKvCacheDiagnosticSummary {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("kv_cache.action", &self.action, MAX_ID_LEN)?;
        validate_required_text("kv_cache.outcome", &self.outcome, MAX_ID_LEN)?;
        validate_optional_text("kv_cache.cache_id", self.cache_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "kv_cache.backend_key",
            self.backend_key.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "kv_cache.reuse_source",
            self.reuse_source.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "kv_cache.reason",
            self.reason.as_deref(),
            MAX_INFERENCE_KV_CACHE_REASON_LEN,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceCompatibilityReportDiagnosticSummary {
    pub status: String,
    pub compatible: bool,
    pub task: String,
    pub model_source: String,
    pub preprocessing: String,
    pub postprocessing: String,
}

impl InferenceCompatibilityReportDiagnosticSummary {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("compatibility_status", &self.status, MAX_ID_LEN)?;
        validate_required_text("compatibility_task", &self.task, MAX_ID_LEN)?;
        validate_required_text("compatibility_model_source", &self.model_source, MAX_ID_LEN)?;
        validate_required_text(
            "compatibility_preprocessing",
            &self.preprocessing,
            MAX_ID_LEN,
        )?;
        validate_required_text(
            "compatibility_postprocessing",
            &self.postprocessing,
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceCompatibilityIssueDiagnosticSummary {
    pub kind: String,
    pub phase: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl InferenceCompatibilityIssueDiagnosticSummary {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("compatibility_issue_kind", &self.kind, MAX_ID_LEN)?;
        validate_required_text("compatibility_issue_phase", &self.phase, MAX_ID_LEN)?;
        validate_required_text("compatibility_issue_message", &self.message, MAX_JSON_LEN)?;
        validate_optional_text(
            "compatibility_issue_model_id",
            self.model_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "compatibility_issue_path",
            self.path.as_deref(),
            MAX_JSON_LEN,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceOptionSupportCounts {
    #[serde(default)]
    pub honored: u32,
    #[serde(default)]
    pub mapped: u32,
    #[serde(default)]
    pub defaulted: u32,
    #[serde(default)]
    pub ignored: u32,
    #[serde(default)]
    pub unsupported: u32,
    #[serde(default)]
    pub rejected: u32,
    #[serde(default)]
    pub conflict: u32,
    #[serde(default)]
    pub model_unavailable: u32,
    #[serde(default)]
    pub backend_unavailable: u32,
    #[serde(default)]
    pub requires_model_support: u32,
    #[serde(default)]
    pub requires_backend_support: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InferenceOptionDiagnosticSummary {
    pub option_path: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl InferenceOptionDiagnosticSummary {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("option_path", &self.option_path, MAX_ID_LEN)?;
        validate_required_text("option_support_state", &self.state, MAX_ID_LEN)?;
        validate_optional_text("backend_key", self.backend_key.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "option_diagnostic_message",
            self.message.as_deref(),
            MAX_JSON_LEN,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticErrorScopeKind {
    Run,
    Node,
    RuntimeModel,
    Scheduler,
    Artifact,
    Projection,
    Transport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticErrorSeverity {
    Warning,
    Error,
    Fatal,
}

impl DiagnosticErrorSeverity {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            "fatal" => Ok(Self::Fatal),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "error_severity",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticErrorRecoverability {
    Recoverable,
    Retryable,
    Unrecoverable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DiagnosticErrorLocation {
    pub component: Option<String>,
    pub operation: Option<String>,
    pub module_path: Option<String>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl DiagnosticErrorLocation {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text(
            "error_location_component",
            self.component.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "error_location_operation",
            self.operation.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "error_location_module_path",
            self.module_path.as_deref(),
            MAX_JSON_LEN,
        )?;
        validate_optional_text("error_location_file", self.file.as_deref(), MAX_JSON_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiagnosticErrorOccurredPayload {
    pub phase: String,
    pub scope: DiagnosticErrorScopeKind,
    pub severity: DiagnosticErrorSeverity,
    pub code: String,
    pub message: String,
    pub technical_message: Option<String>,
    #[serde(default)]
    pub cause_chain: Vec<String>,
    pub recoverability: DiagnosticErrorRecoverability,
    #[serde(default)]
    pub location: DiagnosticErrorLocation,
    #[serde(default)]
    pub related_event_ids: Vec<String>,
    pub caused_by_event_id: Option<String>,
}

impl DiagnosticErrorOccurredPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("error_phase", &self.phase, MAX_ID_LEN)?;
        validate_required_text("error_code", &self.code, MAX_ID_LEN)?;
        validate_required_text(
            "error_message",
            &self.message,
            MAX_DIAGNOSTIC_ERROR_TEXT_LEN,
        )?;
        validate_optional_text(
            "error_technical_message",
            self.technical_message.as_deref(),
            MAX_DIAGNOSTIC_ERROR_TEXT_LEN,
        )?;
        if self.cause_chain.len() > MAX_DIAGNOSTIC_ERROR_CAUSE_COUNT {
            return Err(DiagnosticsLedgerError::FieldTooLong {
                field: "error_cause_chain",
                max_len: MAX_DIAGNOSTIC_ERROR_CAUSE_COUNT,
            });
        }
        for cause in &self.cause_chain {
            validate_required_text("error_cause", cause, MAX_DIAGNOSTIC_ERROR_CAUSE_LEN)?;
        }
        self.location.validate()?;
        validate_text_list("related_event_ids", &self.related_event_ids)?;
        validate_optional_text(
            "caused_by_event_id",
            self.caused_by_event_id.as_deref(),
            MAX_ID_LEN,
        )
    }

    pub(crate) fn summary(&self) -> &str {
        self.message.as_str()
    }
}

pub fn sanitize_diagnostic_error_text(value: &str, max_len: usize) -> String {
    let mut sanitized = String::with_capacity(value.len().min(max_len));
    for ch in value.chars() {
        let replacement = if ch.is_control() { ' ' } else { ch };
        if sanitized.len() + replacement.len_utf8() > max_len {
            break;
        }
        sanitized.push(replacement);
    }

    if sanitized.trim().is_empty() && !value.is_empty() {
        "error text contained only control characters".to_string()
    } else {
        sanitized
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEventAppendRequest {
    pub source_component: DiagnosticEventSourceComponent,
    pub source_instance_id: Option<String>,
    pub occurred_at_ms: i64,
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub node_id: Option<String>,
    pub node_type: Option<String>,
    pub node_version: Option<String>,
    pub runtime_id: Option<String>,
    pub runtime_version: Option<String>,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
    pub privacy_class: DiagnosticEventPrivacyClass,
    pub retention_class: DiagnosticEventRetentionClass,
    pub payload_ref: Option<String>,
    pub payload: DiagnosticEventPayload,
}

impl DiagnosticEventAppendRequest {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text(
            "source_instance_id",
            self.source_instance_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "workflow_semantic_version",
            self.workflow_semantic_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("node_type", self.node_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("node_version", self.node_version.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("runtime_id", self.runtime_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "runtime_version",
            self.runtime_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("model_id", self.model_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("model_version", self.model_version.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "scheduler_policy_id",
            self.scheduler_policy_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "retention_policy_id",
            self.retention_policy_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_payload_ref(self.payload_ref.as_deref())?;
        self.payload.validate()?;
        validate_event_scope(self)?;
        validate_event_source(self.payload.event_kind(), self.source_component)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEventRecord {
    pub event_seq: i64,
    pub event_id: String,
    pub event_kind: DiagnosticEventKind,
    pub schema_version: i64,
    pub source_component: DiagnosticEventSourceComponent,
    pub source_instance_id: Option<String>,
    pub occurred_at_ms: i64,
    pub recorded_at_ms: i64,
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub node_id: Option<String>,
    pub node_type: Option<String>,
    pub node_version: Option<String>,
    pub runtime_id: Option<String>,
    pub runtime_version: Option<String>,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
    pub privacy_class: DiagnosticEventPrivacyClass,
    pub retention_class: DiagnosticEventRetentionClass,
    pub payload_hash: String,
    pub payload_size_bytes: u64,
    pub payload_ref: Option<String>,
    pub payload_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStatus {
    Current,
    Rebuilding,
    NeedsRebuild,
    Failed,
}

impl ProjectionStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Rebuilding => "rebuilding",
            Self::NeedsRebuild => "needs_rebuild",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "current" => Ok(Self::Current),
            "rebuilding" => Ok(Self::Rebuilding),
            "needs_rebuild" => Ok(Self::NeedsRebuild),
            "failed" => Ok(Self::Failed),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "projection_status",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionStateRecord {
    pub projection_name: String,
    pub projection_version: i64,
    pub last_applied_event_seq: i64,
    pub status: ProjectionStatus,
    pub rebuilt_at_ms: Option<i64>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionStateUpdate {
    pub projection_name: String,
    pub projection_version: i64,
    pub last_applied_event_seq: i64,
    pub status: ProjectionStatus,
    pub rebuilt_at_ms: Option<i64>,
}

impl ProjectionStateUpdate {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("projection_name", &self.projection_name, MAX_ID_LEN)?;
        if self.projection_version <= 0 || self.last_applied_event_seq < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "projection_state",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerTimelineProjectionQuery {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
    pub scheduler_policy_id: Option<String>,
    pub after_event_seq: Option<i64>,
    pub limit: u32,
}

impl Default for SchedulerTimelineProjectionQuery {
    fn default() -> Self {
        Self {
            workflow_run_id: None,
            workflow_id: None,
            scheduler_policy_id: None,
            after_event_seq: None,
            limit: 100,
        }
    }
}

impl SchedulerTimelineProjectionQuery {
    pub fn validate(&self, max_limit: u32) -> Result<(), DiagnosticsLedgerError> {
        if self.limit > max_limit {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.limit,
                max: max_limit,
            });
        }
        if self.after_event_seq.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "after_event_seq",
            });
        }
        validate_optional_text(
            "scheduler_policy_id",
            self.scheduler_policy_id.as_deref(),
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerTimelineProjectionRecord {
    pub event_seq: i64,
    pub event_id: String,
    pub event_kind: DiagnosticEventKind,
    pub source_component: DiagnosticEventSourceComponent,
    pub occurred_at_ms: i64,
    pub recorded_at_ms: i64,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
    pub summary: String,
    pub detail: Option<String>,
    pub error_severity: Option<DiagnosticErrorSeverity>,
    pub error_phase: Option<String>,
    pub related_event_ids: Vec<String>,
    pub payload_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunListProjectionStatus {
    Accepted,
    Future,
    Scheduled,
    Queued,
    Delayed,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl RunListProjectionStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Future => "future",
            Self::Scheduled => "scheduled",
            Self::Queued => "queued",
            Self::Delayed => "delayed",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "accepted" => Ok(Self::Accepted),
            "future" => Ok(Self::Future),
            "scheduled" => Ok(Self::Scheduled),
            "queued" => Ok(Self::Queued),
            "delayed" => Ok(Self::Delayed),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "run_list_status",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunListProjectionQuery {
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub status: Option<RunListProjectionStatus>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
    pub selected_runtime_id: Option<String>,
    pub selected_device_id: Option<String>,
    pub selected_network_node_id: Option<String>,
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub accepted_at_from_ms: Option<i64>,
    pub accepted_at_to_ms: Option<i64>,
    pub error_severity: Option<DiagnosticErrorSeverity>,
    pub error_phase: Option<String>,
    pub after_event_seq: Option<i64>,
    pub limit: u32,
}

impl Default for RunListProjectionQuery {
    fn default() -> Self {
        Self {
            workflow_id: None,
            workflow_version_id: None,
            workflow_semantic_version: None,
            status: None,
            scheduler_policy_id: None,
            retention_policy_id: None,
            selected_runtime_id: None,
            selected_device_id: None,
            selected_network_node_id: None,
            client_id: None,
            client_session_id: None,
            bucket_id: None,
            accepted_at_from_ms: None,
            accepted_at_to_ms: None,
            error_severity: None,
            error_phase: None,
            after_event_seq: None,
            limit: 100,
        }
    }
}

impl RunListProjectionQuery {
    pub fn validate(&self, max_limit: u32) -> Result<(), DiagnosticsLedgerError> {
        if self.limit > max_limit {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.limit,
                max: max_limit,
            });
        }
        if self.after_event_seq.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "after_event_seq",
            });
        }
        if self.accepted_at_from_ms.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "accepted_at_from_ms",
            });
        }
        if self.accepted_at_to_ms.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "accepted_at_to_ms",
            });
        }
        if let (Some(from_ms), Some(to_ms)) = (self.accepted_at_from_ms, self.accepted_at_to_ms) {
            if from_ms > to_ms {
                return Err(DiagnosticsLedgerError::InvalidField {
                    field: "accepted_at_range",
                });
            }
        }
        validate_optional_text(
            "workflow_semantic_version",
            self.workflow_semantic_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "scheduler_policy_id",
            self.scheduler_policy_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "retention_policy_id",
            self.retention_policy_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_runtime_id",
            self.selected_runtime_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_device_id",
            self.selected_device_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_network_node_id",
            self.selected_network_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("error_phase", self.error_phase.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunListProjectionRecord {
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub status: RunListProjectionStatus,
    pub accepted_at_ms: Option<i64>,
    pub enqueued_at_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
    pub selected_runtime_id: Option<String>,
    pub selected_backend_key: Option<String>,
    pub selected_model_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub selected_device_id: Option<String>,
    pub selected_network_node_id: Option<String>,
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub workflow_execution_session_id: Option<String>,
    pub scheduler_queue_position: Option<u32>,
    pub scheduler_priority: Option<i32>,
    pub estimate_confidence: Option<String>,
    pub estimated_queue_wait_ms: Option<u64>,
    pub estimated_duration_ms: Option<u64>,
    pub model_cache_state: Option<SchedulerModelCacheState>,
    pub scheduler_reason: Option<String>,
    pub latest_error_event_id: Option<String>,
    pub latest_error_severity: Option<DiagnosticErrorSeverity>,
    pub latest_error_phase: Option<String>,
    pub latest_error_code: Option<String>,
    pub latest_error_message: Option<String>,
    pub fatal_error_event_id: Option<String>,
    pub error_count: u64,
    pub warning_count: u64,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunListFacetKind {
    WorkflowVersion,
    Status,
    SchedulerPolicy,
    RetentionPolicy,
    SelectedRuntime,
    SelectedDevice,
    SelectedNetworkNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunListFacetRecord {
    pub facet_kind: RunListFacetKind,
    pub facet_value: String,
    pub run_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDetailProjectionQuery {
    pub workflow_run_id: WorkflowRunId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunDetailProjectionRecord {
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub status: RunListProjectionStatus,
    pub accepted_at_ms: Option<i64>,
    pub enqueued_at_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
    pub selected_runtime_id: Option<String>,
    pub selected_backend_key: Option<String>,
    pub selected_model_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub selected_device_id: Option<String>,
    pub selected_network_node_id: Option<String>,
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub workflow_run_snapshot_id: Option<String>,
    pub workflow_execution_session_id: Option<String>,
    pub workflow_presentation_revision_id: Option<String>,
    pub latest_estimate_json: Option<String>,
    pub latest_queue_placement_json: Option<String>,
    pub started_payload_json: Option<String>,
    pub terminal_payload_json: Option<String>,
    pub terminal_error: Option<String>,
    pub scheduler_queue_position: Option<u32>,
    pub scheduler_priority: Option<i32>,
    pub estimate_confidence: Option<String>,
    pub estimated_queue_wait_ms: Option<u64>,
    pub estimated_duration_ms: Option<u64>,
    pub model_cache_state: Option<SchedulerModelCacheState>,
    pub scheduler_reason: Option<String>,
    pub latest_error_event_id: Option<String>,
    pub latest_error_severity: Option<DiagnosticErrorSeverity>,
    pub latest_error_phase: Option<String>,
    pub latest_error_code: Option<String>,
    pub latest_error_message: Option<String>,
    pub fatal_error_event_id: Option<String>,
    pub error_count: u64,
    pub warning_count: u64,
    pub timeline_event_count: u64,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoArtifactProjectionQuery {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub node_id: Option<String>,
    pub producer_node_id: Option<String>,
    pub consumer_node_id: Option<String>,
    pub artifact_role: Option<String>,
    pub media_type: Option<String>,
    pub retention_state: Option<IoArtifactRetentionState>,
    pub retention_policy_id: Option<String>,
    pub runtime_id: Option<String>,
    pub selected_backend_key: Option<String>,
    pub model_id: Option<String>,
    pub after_event_seq: Option<i64>,
    pub limit: u32,
}

impl IoArtifactProjectionQuery {
    pub fn validate(&self, max_limit: u32) -> Result<(), DiagnosticsLedgerError> {
        if self.limit > max_limit {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.limit,
                max: max_limit,
            });
        }
        if self.after_event_seq.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "after_event_seq",
            });
        }
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "producer_node_id",
            self.producer_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "consumer_node_id",
            self.consumer_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("artifact_role", self.artifact_role.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("media_type", self.media_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "retention_policy_id",
            self.retention_policy_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("runtime_id", self.runtime_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "selected_backend_key",
            self.selected_backend_key.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("model_id", self.model_id.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoArtifactRetentionSummaryQuery {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub node_id: Option<String>,
    pub producer_node_id: Option<String>,
    pub consumer_node_id: Option<String>,
    pub artifact_role: Option<String>,
    pub media_type: Option<String>,
    pub retention_policy_id: Option<String>,
    pub runtime_id: Option<String>,
    pub selected_backend_key: Option<String>,
    pub model_id: Option<String>,
}

impl IoArtifactRetentionSummaryQuery {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "producer_node_id",
            self.producer_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "consumer_node_id",
            self.consumer_node_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("artifact_role", self.artifact_role.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("media_type", self.media_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "retention_policy_id",
            self.retention_policy_id.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("runtime_id", self.runtime_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "selected_backend_key",
            self.selected_backend_key.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("model_id", self.model_id.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoArtifactRetentionSummaryRecord {
    pub retention_state: IoArtifactRetentionState,
    pub artifact_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoArtifactProjectionRecord {
    pub event_seq: i64,
    pub event_id: String,
    pub occurred_at_ms: i64,
    pub recorded_at_ms: i64,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub node_id: Option<String>,
    pub node_type: Option<String>,
    pub node_version: Option<String>,
    pub runtime_id: Option<String>,
    pub runtime_version: Option<String>,
    pub selected_backend_key: Option<String>,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub artifact_id: String,
    pub artifact_role: String,
    pub producer_node_id: Option<String>,
    pub producer_port_id: Option<String>,
    pub consumer_node_id: Option<String>,
    pub consumer_port_id: Option<String>,
    pub media_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<String>,
    pub payload_ref: Option<String>,
    pub retention_state: IoArtifactRetentionState,
    pub retention_reason: Option<String>,
    pub retention_policy_id: Option<String>,
    pub payload_kind: Option<IoArtifactPayloadKind>,
    pub lifecycle_state: Option<IoArtifactLifecycleState>,
    pub access_modes: Vec<IoArtifactAccessMode>,
    pub read_handle: Option<String>,
    pub stream_handle: Option<String>,
    pub format: Option<IoArtifactFormatMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStatusProjectionQuery {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub node_id: Option<String>,
    pub status: Option<NodeExecutionProjectionStatus>,
    pub after_event_seq: Option<i64>,
    pub limit: u32,
}

impl Default for NodeStatusProjectionQuery {
    fn default() -> Self {
        Self {
            workflow_run_id: None,
            node_id: None,
            status: None,
            after_event_seq: None,
            limit: 250,
        }
    }
}

impl NodeStatusProjectionQuery {
    pub fn validate(&self, max_limit: u32) -> Result<(), DiagnosticsLedgerError> {
        if self.limit > max_limit {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.limit,
                max: max_limit,
            });
        }
        if self.after_event_seq.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "after_event_seq",
            });
        }
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStatusProjectionRecord {
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub node_id: String,
    pub node_type: Option<String>,
    pub node_version: Option<String>,
    pub runtime_id: Option<String>,
    pub runtime_version: Option<String>,
    pub task_id: Option<String>,
    pub selected_backend_key: Option<String>,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub status: NodeExecutionProjectionStatus,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    pub error_event_id: Option<String>,
    pub error_severity: Option<DiagnosticErrorSeverity>,
    pub error_phase: Option<String>,
    pub error_code: Option<String>,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryUsageProjectionQuery {
    pub asset_id: Option<String>,
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub after_event_seq: Option<i64>,
    pub limit: u32,
}

impl Default for LibraryUsageProjectionQuery {
    fn default() -> Self {
        Self {
            asset_id: None,
            workflow_run_id: None,
            workflow_id: None,
            workflow_version_id: None,
            after_event_seq: None,
            limit: 100,
        }
    }
}

impl LibraryUsageProjectionQuery {
    pub fn validate(&self, max_limit: u32) -> Result<(), DiagnosticsLedgerError> {
        if self.limit > max_limit {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.limit,
                max: max_limit,
            });
        }
        if self.after_event_seq.unwrap_or(0) < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "after_event_seq",
            });
        }
        if let Some(asset_id) = self.asset_id.as_deref() {
            validate_library_resource_id("asset_id", asset_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryUsageProjectionRecord {
    pub asset_id: String,
    pub total_access_count: u64,
    pub run_access_count: u64,
    pub total_network_bytes: u64,
    pub last_accessed_at_ms: i64,
    pub last_operation: String,
    pub last_cache_status: Option<String>,
    pub last_workflow_run_id: Option<WorkflowRunId>,
    pub last_workflow_id: Option<WorkflowId>,
    pub last_workflow_version_id: Option<WorkflowVersionId>,
    pub last_workflow_semantic_version: Option<String>,
    pub last_client_id: Option<ClientId>,
    pub last_client_session_id: Option<ClientSessionId>,
    pub last_bucket_id: Option<BucketId>,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

fn validate_text_list(
    field: &'static str,
    values: &[String],
) -> Result<(), DiagnosticsLedgerError> {
    for value in values {
        validate_required_text(field, value, MAX_ID_LEN)?;
    }
    Ok(())
}

fn validate_payload_ref(value: Option<&str>) -> Result<(), DiagnosticsLedgerError> {
    validate_optional_text("payload_ref", value, MAX_JSON_LEN)?;
    let Some(value) = value else {
        return Ok(());
    };
    if value.trim() != value || value.chars().any(char::is_whitespace) {
        return Err(DiagnosticsLedgerError::InvalidField {
            field: "payload_ref",
        });
    }
    let allowed_scheme = ["artifact://", "pumas://", "pantograph://"]
        .iter()
        .find(|scheme| value.starts_with(**scheme));
    let Some(scheme) = allowed_scheme else {
        return Err(DiagnosticsLedgerError::InvalidField {
            field: "payload_ref",
        });
    };
    let reference = &value[scheme.len()..];
    if reference.is_empty()
        || reference.starts_with('/')
        || reference.contains('\\')
        || reference
            .split('/')
            .any(|segment| segment == "." || segment == "..")
    {
        return Err(DiagnosticsLedgerError::InvalidField {
            field: "payload_ref",
        });
    }
    Ok(())
}

fn validate_library_resource_id(
    field: &'static str,
    value: &str,
) -> Result<(), DiagnosticsLedgerError> {
    validate_required_text(field, value, MAX_ID_LEN)?;
    if value.trim() != value || value.chars().any(char::is_whitespace) {
        return Err(DiagnosticsLedgerError::InvalidField { field });
    }
    if value.contains("://")
        && !["pumas://", "pantograph://", "hf://"]
            .iter()
            .any(|scheme| value.starts_with(*scheme))
    {
        return Err(DiagnosticsLedgerError::InvalidField { field });
    }
    let reference = ["pumas://", "pantograph://", "hf://"]
        .iter()
        .find_map(|scheme| value.strip_prefix(*scheme))
        .unwrap_or(value);
    if reference.starts_with('/')
        || reference.is_empty()
        || reference.contains('\\')
        || reference
            .split('/')
            .any(|segment| segment == "." || segment == ".." || segment.is_empty())
    {
        return Err(DiagnosticsLedgerError::InvalidField { field });
    }
    Ok(())
}

fn validate_event_scope(
    request: &DiagnosticEventAppendRequest,
) -> Result<(), DiagnosticsLedgerError> {
    match request.payload.event_kind() {
        DiagnosticEventKind::SchedulerEstimateProduced
        | DiagnosticEventKind::SchedulerQueuePlacement
        | DiagnosticEventKind::SchedulerQueueControl
        | DiagnosticEventKind::SchedulerRunDelayed
        | DiagnosticEventKind::SchedulerModelLifecycleChanged
        | DiagnosticEventKind::SchedulerRunAdmitted
        | DiagnosticEventKind::SchedulerReservationChanged
        | DiagnosticEventKind::RunStarted
        | DiagnosticEventKind::RunTerminal
        | DiagnosticEventKind::RunSnapshotAccepted
        | DiagnosticEventKind::IoArtifactObserved
        | DiagnosticEventKind::RetentionArtifactStateChanged
        | DiagnosticEventKind::NodeExecutionStatus
        | DiagnosticEventKind::InferenceExecutionDiagnosticObserved => {
            if request.workflow_run_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "workflow_run_id",
                });
            }
            if request.workflow_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "workflow_id",
                });
            }
            if request.payload.event_kind() == DiagnosticEventKind::RetentionArtifactStateChanged
                && request.retention_policy_id.is_none()
            {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "retention_policy_id",
                });
            }
            if matches!(
                request.payload.event_kind(),
                DiagnosticEventKind::NodeExecutionStatus
                    | DiagnosticEventKind::InferenceExecutionDiagnosticObserved
            ) && request.node_id.is_none()
            {
                return Err(DiagnosticsLedgerError::MissingField { field: "node_id" });
            }
            if request.payload.event_kind() == DiagnosticEventKind::SchedulerModelLifecycleChanged
                && request.model_id.is_none()
            {
                return Err(DiagnosticsLedgerError::MissingField { field: "model_id" });
            }
        }
        DiagnosticEventKind::DiagnosticErrorOccurred => {
            let DiagnosticEventPayload::DiagnosticErrorOccurred(payload) = &request.payload else {
                return Err(DiagnosticsLedgerError::InvalidField {
                    field: "event_kind",
                });
            };
            if !matches!(
                payload.scope,
                DiagnosticErrorScopeKind::Transport | DiagnosticErrorScopeKind::Projection
            ) {
                if request.workflow_run_id.is_none() {
                    return Err(DiagnosticsLedgerError::MissingField {
                        field: "workflow_run_id",
                    });
                }
                if request.workflow_id.is_none() {
                    return Err(DiagnosticsLedgerError::MissingField {
                        field: "workflow_id",
                    });
                }
            }
            validate_diagnostic_error_scope(payload.scope, request)?;
        }
        DiagnosticEventKind::RetentionPolicyChanged => {
            if request.retention_policy_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "retention_policy_id",
                });
            }
        }
        DiagnosticEventKind::RuntimeCapabilityObserved => {
            if request.runtime_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "runtime_id",
                });
            }
        }
        DiagnosticEventKind::LibraryAssetAccessed => {}
    }
    Ok(())
}

fn validate_diagnostic_error_scope(
    scope: DiagnosticErrorScopeKind,
    request: &DiagnosticEventAppendRequest,
) -> Result<(), DiagnosticsLedgerError> {
    match scope {
        DiagnosticErrorScopeKind::Run | DiagnosticErrorScopeKind::Transport => Ok(()),
        DiagnosticErrorScopeKind::Node => {
            if request.node_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField { field: "node_id" });
            }
            Ok(())
        }
        DiagnosticErrorScopeKind::RuntimeModel => {
            if request.runtime_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "runtime_id",
                });
            }
            Ok(())
        }
        DiagnosticErrorScopeKind::Scheduler => {
            if request.scheduler_policy_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "scheduler_policy_id",
                });
            }
            Ok(())
        }
        DiagnosticErrorScopeKind::Artifact => {
            if request.payload_ref.is_none() && request.node_id.is_none() {
                return Err(DiagnosticsLedgerError::MissingField {
                    field: "payload_ref_or_node_id",
                });
            }
            Ok(())
        }
        DiagnosticErrorScopeKind::Projection => Ok(()),
    }
}

fn validate_event_source(
    event_kind: DiagnosticEventKind,
    source_component: DiagnosticEventSourceComponent,
) -> Result<(), DiagnosticsLedgerError> {
    let allowed = match event_kind {
        DiagnosticEventKind::SchedulerEstimateProduced
        | DiagnosticEventKind::SchedulerQueuePlacement
        | DiagnosticEventKind::SchedulerQueueControl
        | DiagnosticEventKind::SchedulerRunDelayed
        | DiagnosticEventKind::SchedulerModelLifecycleChanged
        | DiagnosticEventKind::SchedulerRunAdmitted
        | DiagnosticEventKind::SchedulerReservationChanged
        | DiagnosticEventKind::RunStarted => {
            matches!(source_component, DiagnosticEventSourceComponent::Scheduler)
        }
        DiagnosticEventKind::RunSnapshotAccepted | DiagnosticEventKind::RunTerminal => {
            matches!(
                source_component,
                DiagnosticEventSourceComponent::WorkflowService
            )
        }
        DiagnosticEventKind::IoArtifactObserved => matches!(
            source_component,
            DiagnosticEventSourceComponent::WorkflowService
                | DiagnosticEventSourceComponent::Runtime
                | DiagnosticEventSourceComponent::NodeExecution
        ),
        DiagnosticEventKind::LibraryAssetAccessed => {
            matches!(source_component, DiagnosticEventSourceComponent::Library)
        }
        DiagnosticEventKind::RetentionPolicyChanged => {
            matches!(source_component, DiagnosticEventSourceComponent::Retention)
        }
        DiagnosticEventKind::RetentionArtifactStateChanged => {
            matches!(source_component, DiagnosticEventSourceComponent::Retention)
        }
        DiagnosticEventKind::RuntimeCapabilityObserved => matches!(
            source_component,
            DiagnosticEventSourceComponent::Runtime | DiagnosticEventSourceComponent::LocalObserver
        ),
        DiagnosticEventKind::NodeExecutionStatus => matches!(
            source_component,
            DiagnosticEventSourceComponent::NodeExecution | DiagnosticEventSourceComponent::Runtime
        ),
        DiagnosticEventKind::InferenceExecutionDiagnosticObserved => matches!(
            source_component,
            DiagnosticEventSourceComponent::NodeExecution
                | DiagnosticEventSourceComponent::Runtime
                | DiagnosticEventSourceComponent::WorkflowService
        ),
        DiagnosticEventKind::DiagnosticErrorOccurred => matches!(
            source_component,
            DiagnosticEventSourceComponent::Scheduler
                | DiagnosticEventSourceComponent::WorkflowService
                | DiagnosticEventSourceComponent::Runtime
                | DiagnosticEventSourceComponent::NodeExecution
                | DiagnosticEventSourceComponent::LocalObserver
        ),
    };
    if allowed {
        Ok(())
    } else {
        Err(DiagnosticsLedgerError::InvalidEventSource {
            event_kind: event_kind.as_db(),
            source_component: source_component.as_db(),
        })
    }
}
