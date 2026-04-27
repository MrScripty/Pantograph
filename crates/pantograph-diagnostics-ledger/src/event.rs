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
pub const RUN_LIST_PROJECTION_VERSION: i64 = 1;
pub const RUN_DETAIL_PROJECTION_NAME: &str = "run_detail";
pub const RUN_DETAIL_PROJECTION_VERSION: i64 = 1;
pub const IO_ARTIFACT_PROJECTION_NAME: &str = "io_artifact";
pub const IO_ARTIFACT_PROJECTION_VERSION: i64 = 1;
pub const LIBRARY_USAGE_PROJECTION_NAME: &str = "library_usage";
pub const LIBRARY_USAGE_PROJECTION_VERSION: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventKind {
    SchedulerEstimateProduced,
    SchedulerQueuePlacement,
    RunStarted,
    RunTerminal,
    RunSnapshotAccepted,
    IoArtifactObserved,
    LibraryAssetAccessed,
    RetentionPolicyChanged,
    RuntimeCapabilityObserved,
}

impl DiagnosticEventKind {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::SchedulerEstimateProduced => "scheduler.estimate_produced",
            Self::SchedulerQueuePlacement => "scheduler.queue_placement",
            Self::RunStarted => "run.started",
            Self::RunTerminal => "run.terminal",
            Self::RunSnapshotAccepted => "run.snapshot_accepted",
            Self::IoArtifactObserved => "io.artifact_observed",
            Self::LibraryAssetAccessed => "library.asset_accessed",
            Self::RetentionPolicyChanged => "retention.policy_changed",
            Self::RuntimeCapabilityObserved => "runtime.capability_observed",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "scheduler.estimate_produced" => Ok(Self::SchedulerEstimateProduced),
            "scheduler.queue_placement" => Ok(Self::SchedulerQueuePlacement),
            "run.started" => Ok(Self::RunStarted),
            "run.terminal" => Ok(Self::RunTerminal),
            "run.snapshot_accepted" => Ok(Self::RunSnapshotAccepted),
            "io.artifact_observed" => Ok(Self::IoArtifactObserved),
            "library.asset_accessed" => Ok(Self::LibraryAssetAccessed),
            "retention.policy_changed" => Ok(Self::RetentionPolicyChanged),
            "runtime.capability_observed" => Ok(Self::RuntimeCapabilityObserved),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "payload_type")]
pub enum DiagnosticEventPayload {
    SchedulerEstimateProduced(SchedulerEstimateProducedPayload),
    SchedulerQueuePlacement(SchedulerQueuePlacementPayload),
    RunStarted(RunStartedPayload),
    RunTerminal(RunTerminalPayload),
    RunSnapshotAccepted(RunSnapshotAcceptedPayload),
    IoArtifactObserved(IoArtifactObservedPayload),
    LibraryAssetAccessed(LibraryAssetAccessedPayload),
    RetentionPolicyChanged(RetentionPolicyChangedPayload),
    RuntimeCapabilityObserved(RuntimeCapabilityObservedPayload),
}

impl DiagnosticEventPayload {
    pub fn event_kind(&self) -> DiagnosticEventKind {
        match self {
            Self::SchedulerEstimateProduced(_) => DiagnosticEventKind::SchedulerEstimateProduced,
            Self::SchedulerQueuePlacement(_) => DiagnosticEventKind::SchedulerQueuePlacement,
            Self::RunStarted(_) => DiagnosticEventKind::RunStarted,
            Self::RunTerminal(_) => DiagnosticEventKind::RunTerminal,
            Self::RunSnapshotAccepted(_) => DiagnosticEventKind::RunSnapshotAccepted,
            Self::IoArtifactObserved(_) => DiagnosticEventKind::IoArtifactObserved,
            Self::LibraryAssetAccessed(_) => DiagnosticEventKind::LibraryAssetAccessed,
            Self::RetentionPolicyChanged(_) => DiagnosticEventKind::RetentionPolicyChanged,
            Self::RuntimeCapabilityObserved(_) => DiagnosticEventKind::RuntimeCapabilityObserved,
        }
    }

    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        match self {
            Self::SchedulerEstimateProduced(payload) => payload.validate(),
            Self::SchedulerQueuePlacement(payload) => payload.validate(),
            Self::RunStarted(payload) => payload.validate(),
            Self::RunTerminal(payload) => payload.validate(),
            Self::RunSnapshotAccepted(payload) => payload.validate(),
            Self::IoArtifactObserved(payload) => payload.validate(),
            Self::LibraryAssetAccessed(payload) => payload.validate(),
            Self::RetentionPolicyChanged(payload) => payload.validate(),
            Self::RuntimeCapabilityObserved(payload) => payload.validate(),
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
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct IoArtifactObservedPayload {
    pub artifact_id: String,
    pub artifact_role: String,
    pub media_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<String>,
}

impl IoArtifactObservedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("artifact_id", &self.artifact_id, MAX_ID_LEN)?;
        validate_required_text("artifact_role", &self.artifact_role, MAX_ID_LEN)?;
        validate_optional_text("media_type", self.media_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("content_hash", self.content_hash.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LibraryAssetAccessedPayload {
    pub asset_id: String,
    pub operation: String,
    pub cache_status: Option<String>,
    pub network_bytes: Option<u64>,
}

impl LibraryAssetAccessedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("asset_id", &self.asset_id, MAX_ID_LEN)?;
        validate_required_text("operation", &self.operation, MAX_ID_LEN)?;
        validate_optional_text("cache_status", self.cache_status.as_deref(), MAX_ID_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RetentionPolicyChangedPayload {
    pub policy_id: String,
    pub reason: String,
}

impl RetentionPolicyChangedPayload {
    fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("policy_id", &self.policy_id, MAX_ID_LEN)?;
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
        validate_optional_text(
            "workflow_semantic_version",
            self.workflow_semantic_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "scheduler_policy_id",
            self.scheduler_policy_id.as_deref(),
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
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
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
    pub workflow_presentation_revision_id: Option<String>,
    pub latest_estimate_json: Option<String>,
    pub latest_queue_placement_json: Option<String>,
    pub started_payload_json: Option<String>,
    pub terminal_payload_json: Option<String>,
    pub terminal_error: Option<String>,
    pub timeline_event_count: u64,
    pub last_event_seq: i64,
    pub last_updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoArtifactProjectionQuery {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub node_id: Option<String>,
    pub artifact_role: Option<String>,
    pub media_type: Option<String>,
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
    pub media_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<String>,
    pub payload_ref: Option<String>,
    pub retention_policy_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryUsageProjectionQuery {
    pub asset_id: Option<String>,
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub after_event_seq: Option<i64>,
    pub limit: u32,
}

impl Default for LibraryUsageProjectionQuery {
    fn default() -> Self {
        Self {
            asset_id: None,
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
        validate_optional_text("asset_id", self.asset_id.as_deref(), MAX_ID_LEN)
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

fn validate_event_scope(
    request: &DiagnosticEventAppendRequest,
) -> Result<(), DiagnosticsLedgerError> {
    match request.payload.event_kind() {
        DiagnosticEventKind::SchedulerEstimateProduced
        | DiagnosticEventKind::SchedulerQueuePlacement
        | DiagnosticEventKind::RunStarted
        | DiagnosticEventKind::RunTerminal
        | DiagnosticEventKind::RunSnapshotAccepted
        | DiagnosticEventKind::IoArtifactObserved => {
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
        DiagnosticEventKind::RuntimeCapabilityObserved => matches!(
            source_component,
            DiagnosticEventSourceComponent::Runtime | DiagnosticEventSourceComponent::LocalObserver
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
