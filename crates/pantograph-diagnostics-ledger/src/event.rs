use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use serde::{Deserialize, Serialize};

use crate::util::{validate_optional_text, validate_required_text, MAX_ID_LEN, MAX_JSON_LEN};
use crate::DiagnosticsLedgerError;

pub const DIAGNOSTIC_EVENT_SCHEMA_VERSION: i64 = 1;
pub const MAX_DIAGNOSTIC_EVENT_PAYLOAD_BYTES: usize = 8_192;
pub const SCHEDULER_TIMELINE_PROJECTION_NAME: &str = "scheduler_timeline";
pub const SCHEDULER_TIMELINE_PROJECTION_VERSION: i64 = 1;
pub const RUN_LIST_PROJECTION_NAME: &str = "run_list";
pub const RUN_LIST_PROJECTION_VERSION: i64 = 3;
pub const RUN_DETAIL_PROJECTION_NAME: &str = "run_detail";
pub const RUN_DETAIL_PROJECTION_VERSION: i64 = 2;
pub const IO_ARTIFACT_PROJECTION_NAME: &str = "io_artifact";
pub const IO_ARTIFACT_PROJECTION_VERSION: i64 = 4;
pub const LIBRARY_USAGE_PROJECTION_NAME: &str = "library_usage";
pub const LIBRARY_USAGE_PROJECTION_VERSION: i64 = 1;
pub const NODE_STATUS_PROJECTION_NAME: &str = "node_status";
pub const NODE_STATUS_PROJECTION_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventKind {
    SchedulerEstimateProduced,
    SchedulerQueuePlacement,
    SchedulerQueueControl,
    SchedulerRunDelayed,
    SchedulerModelLifecycleChanged,
    SchedulerRunAdmitted,
    RunStarted,
    RunTerminal,
    RunSnapshotAccepted,
    IoArtifactObserved,
    RetentionArtifactStateChanged,
    LibraryAssetAccessed,
    RetentionPolicyChanged,
    RuntimeCapabilityObserved,
    NodeExecutionStatus,
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
            Self::RunStarted => "run.started",
            Self::RunTerminal => "run.terminal",
            Self::RunSnapshotAccepted => "run.snapshot_accepted",
            Self::IoArtifactObserved => "io.artifact_observed",
            Self::RetentionArtifactStateChanged => "retention.artifact_state_changed",
            Self::LibraryAssetAccessed => "library.asset_accessed",
            Self::RetentionPolicyChanged => "retention.policy_changed",
            Self::RuntimeCapabilityObserved => "runtime.capability_observed",
            Self::NodeExecutionStatus => "node.execution_status",
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
            "run.started" => Ok(Self::RunStarted),
            "run.terminal" => Ok(Self::RunTerminal),
            "run.snapshot_accepted" => Ok(Self::RunSnapshotAccepted),
            "io.artifact_observed" => Ok(Self::IoArtifactObserved),
            "retention.artifact_state_changed" => Ok(Self::RetentionArtifactStateChanged),
            "library.asset_accessed" => Ok(Self::LibraryAssetAccessed),
            "retention.policy_changed" => Ok(Self::RetentionPolicyChanged),
            "runtime.capability_observed" => Ok(Self::RuntimeCapabilityObserved),
            "node.execution_status" => Ok(Self::NodeExecutionStatus),
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
    RunStarted(RunStartedPayload),
    RunTerminal(RunTerminalPayload),
    RunSnapshotAccepted(RunSnapshotAcceptedPayload),
    IoArtifactObserved(IoArtifactObservedPayload),
    RetentionArtifactStateChanged(RetentionArtifactStateChangedPayload),
    LibraryAssetAccessed(LibraryAssetAccessedPayload),
    RetentionPolicyChanged(RetentionPolicyChangedPayload),
    RuntimeCapabilityObserved(RuntimeCapabilityObservedPayload),
    NodeExecutionStatus(NodeExecutionStatusPayload),
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
            Self::RunStarted(payload) => payload.validate(),
            Self::RunTerminal(payload) => payload.validate(),
            Self::RunSnapshotAccepted(payload) => payload.validate(),
            Self::IoArtifactObserved(payload) => payload.validate(),
            Self::RetentionArtifactStateChanged(payload) => payload.validate(),
            Self::LibraryAssetAccessed(payload) => payload.validate(),
            Self::RetentionPolicyChanged(payload) => payload.validate(),
            Self::RuntimeCapabilityObserved(payload) => payload.validate(),
            Self::NodeExecutionStatus(payload) => payload.validate(),
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
    pub reasons: Vec<String>,
}

impl SchedulerEstimateProducedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("estimate_version", &self.estimate_version, MAX_ID_LEN)?;
        validate_required_text("confidence", &self.confidence, MAX_ID_LEN)?;
        validate_text_list("reasons", &self.reasons)
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
    pub previous_queue_position: Option<u32>,
    pub previous_priority: Option<i32>,
    pub new_priority: Option<i32>,
    pub reason: Option<String>,
}

impl SchedulerQueueControlPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
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
}

impl SchedulerRunAdmittedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text(
            "admission_decision_reason",
            &self.decision_reason,
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerModelLifecycleTransition {
    LoadRequested,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerModelLifecycleChangedPayload {
    pub transition: SchedulerModelLifecycleTransition,
    pub reason: Option<String>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

impl SchedulerModelLifecycleChangedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("model_lifecycle_reason", self.reason.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("model_lifecycle_error", self.error.as_deref(), MAX_JSON_LEN)
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
}

impl RunTerminalPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("error", self.error.as_deref(), MAX_JSON_LEN)
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
        )
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
}

impl NodeExecutionStatusPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("error", self.error.as_deref(), MAX_JSON_LEN)?;
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
    pub payload_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunListProjectionStatus {
    Accepted,
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
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub accepted_at_from_ms: Option<i64>,
    pub accepted_at_to_ms: Option<i64>,
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
            client_id: None,
            client_session_id: None,
            bucket_id: None,
            accepted_at_from_ms: None,
            accepted_at_to_ms: None,
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
        )
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
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub workflow_execution_session_id: Option<String>,
    pub scheduler_queue_position: Option<u32>,
    pub scheduler_priority: Option<i32>,
    pub estimate_confidence: Option<String>,
    pub estimated_queue_wait_ms: Option<u64>,
    pub estimated_duration_ms: Option<u64>,
    pub scheduler_reason: Option<String>,
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
    pub scheduler_reason: Option<String>,
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
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub status: NodeExecutionProjectionStatus,
    pub started_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
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
        | DiagnosticEventKind::RunStarted
        | DiagnosticEventKind::RunTerminal
        | DiagnosticEventKind::RunSnapshotAccepted
        | DiagnosticEventKind::IoArtifactObserved
        | DiagnosticEventKind::RetentionArtifactStateChanged
        | DiagnosticEventKind::NodeExecutionStatus => {
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
            if request.payload.event_kind() == DiagnosticEventKind::NodeExecutionStatus
                && request.node_id.is_none()
            {
                return Err(DiagnosticsLedgerError::MissingField { field: "node_id" });
            }
            if request.payload.event_kind() == DiagnosticEventKind::SchedulerModelLifecycleChanged
                && request.model_id.is_none()
            {
                return Err(DiagnosticsLedgerError::MissingField { field: "model_id" });
            }
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
