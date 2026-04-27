use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};
use serde::{Deserialize, Serialize};

use crate::util::{validate_optional_text, validate_required_text, MAX_ID_LEN, MAX_JSON_LEN};
use crate::DiagnosticsLedgerError;

pub const DEFAULT_PAGE_SIZE: u32 = 100;
pub const MAX_PAGE_SIZE: u32 = 500;
pub const DEFAULT_STANDARD_RETENTION_DAYS: u32 = 365;
pub const MAX_RETENTION_DAYS: u32 = 3650;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionGuaranteeLevel {
    ManagedFull,
    ManagedPartial,
    EscapeHatchDetected,
    UnsafeOrUnobserved,
}

impl ExecutionGuaranteeLevel {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::ManagedFull => "managed_full",
            Self::ManagedPartial => "managed_partial",
            Self::EscapeHatchDetected => "escape_hatch_detected",
            Self::UnsafeOrUnobserved => "unsafe_or_unobserved",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "managed_full" => Ok(Self::ManagedFull),
            "managed_partial" => Ok(Self::ManagedPartial),
            "escape_hatch_detected" => Ok(Self::EscapeHatchDetected),
            "unsafe_or_unobserved" => Ok(Self::UnsafeOrUnobserved),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "guarantee_level",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageEventStatus {
    Completed,
    Failed,
    Partial,
    Cancelled,
}

impl UsageEventStatus {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Partial => "partial",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "partial" => Ok(Self::Partial),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DiagnosticsLedgerError::InvalidField { field: "status" }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    Standard,
}

impl RetentionClass {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Standard => "standard",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "standard" => Ok(Self::Standard),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "retention_class",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputModality {
    Text,
    Image,
    Audio,
    Video,
    Embeddings,
    Structured,
}

impl OutputModality {
    pub(crate) fn as_db(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Embeddings => "embeddings",
            Self::Structured => "structured",
        }
    }

    pub(crate) fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            "embeddings" => Ok(Self::Embeddings),
            "structured" => Ok(Self::Structured),
            _ => Err(DiagnosticsLedgerError::InvalidField { field: "modality" }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMeasurementUnavailableReason {
    NotProduced,
    UnsupportedModality,
    TokenizerUnavailable,
    MetadataUnavailable,
    RuntimeDidNotReport,
    OutputTruncated,
    OutputRedacted,
    ExecutionFailedBeforeMeasurement,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelIdentity {
    pub model_id: String,
    pub model_revision: Option<String>,
    pub model_hash: Option<String>,
    pub model_modality: Option<String>,
    pub runtime_backend: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseSnapshot {
    pub license_value: Option<String>,
    pub source_metadata_json: Option<String>,
    pub model_metadata_snapshot_json: Option<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelOutputMeasurement {
    pub modality: OutputModality,
    pub item_count: Option<u64>,
    pub character_count: Option<u64>,
    pub byte_size: Option<u64>,
    pub token_count: Option<u64>,
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub pixel_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub sample_rate_hz: Option<u64>,
    pub channels: Option<u64>,
    pub frame_count: Option<u64>,
    pub vector_count: Option<u64>,
    pub dimensions: Option<u64>,
    pub numeric_representation: Option<String>,
    pub top_level_shape: Option<String>,
    pub schema_id: Option<String>,
    pub schema_digest: Option<String>,
    pub unavailable_reasons: Vec<OutputMeasurementUnavailableReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageLineage {
    pub node_id: String,
    pub node_type: String,
    pub port_ids: Vec<String>,
    pub composed_parent_chain: Vec<String>,
    pub effective_contract_version: Option<String>,
    pub effective_contract_digest: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLicenseUsageEvent {
    pub usage_event_id: UsageEventId,
    pub client_id: ClientId,
    pub client_session_id: ClientSessionId,
    pub bucket_id: BucketId,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub model: ModelIdentity,
    pub lineage: UsageLineage,
    pub license_snapshot: LicenseSnapshot,
    pub output_measurement: ModelOutputMeasurement,
    pub guarantee_level: ExecutionGuaranteeLevel,
    pub status: UsageEventStatus,
    pub retention_class: RetentionClass,
    pub started_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub correlation_id: Option<String>,
}

impl ModelLicenseUsageEvent {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("model_id", &self.model.model_id, MAX_ID_LEN)?;
        validate_required_text("node_id", &self.lineage.node_id, MAX_ID_LEN)?;
        validate_required_text("node_type", &self.lineage.node_type, MAX_ID_LEN)?;
        validate_optional_text(
            "model_metadata_snapshot_json",
            self.license_snapshot
                .model_metadata_snapshot_json
                .as_deref(),
            MAX_JSON_LEN,
        )?;
        validate_optional_text(
            "source_metadata_json",
            self.license_snapshot.source_metadata_json.as_deref(),
            MAX_JSON_LEN,
        )?;
        validate_optional_text(
            "metadata_json",
            self.lineage.metadata_json.as_deref(),
            MAX_JSON_LEN,
        )?;
        if let Some(completed_at_ms) = self.completed_at_ms {
            if completed_at_ms < self.started_at_ms {
                return Err(DiagnosticsLedgerError::InvalidTimeRange);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsRetentionPolicy {
    pub policy_id: String,
    pub policy_version: u32,
    pub retention_class: RetentionClass,
    pub retention_days: u32,
    pub applied_at_ms: i64,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateRetentionPolicyCommand {
    pub retention_class: RetentionClass,
    pub retention_days: u32,
    pub explanation: String,
}

impl UpdateRetentionPolicyCommand {
    pub(crate) fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        if self.retention_days == 0 || self.retention_days > MAX_RETENTION_DAYS {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "retention_days",
            });
        }
        validate_required_text("explanation", &self.explanation, MAX_JSON_LEN)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsQuery {
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub node_id: Option<String>,
    pub node_contract_version: Option<String>,
    pub node_contract_digest: Option<String>,
    pub model_id: Option<String>,
    pub license_value: Option<String>,
    pub guarantee_level: Option<ExecutionGuaranteeLevel>,
    pub started_at_ms: Option<i64>,
    pub ended_before_ms: Option<i64>,
    pub page: u32,
    pub page_size: u32,
}

impl Default for DiagnosticsQuery {
    fn default() -> Self {
        Self {
            client_id: None,
            client_session_id: None,
            bucket_id: None,
            workflow_run_id: None,
            workflow_id: None,
            workflow_version_id: None,
            workflow_semantic_version: None,
            node_id: None,
            node_contract_version: None,
            node_contract_digest: None,
            model_id: None,
            license_value: None,
            guarantee_level: None,
            started_at_ms: None,
            ended_before_ms: None,
            page: 0,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }
}

impl DiagnosticsQuery {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        if self.page_size > MAX_PAGE_SIZE {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.page_size,
                max: MAX_PAGE_SIZE,
            });
        }
        if matches!(
            (self.started_at_ms, self.ended_before_ms),
            (Some(start), Some(end)) if end <= start
        ) {
            return Err(DiagnosticsLedgerError::InvalidTimeRange);
        }
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "node_contract_version",
            self.node_contract_version.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "node_contract_digest",
            self.node_contract_digest.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("model_id", self.model_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "workflow_semantic_version",
            self.workflow_semantic_version.as_deref(),
            MAX_ID_LEN,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsProjection {
    pub events: Vec<ModelLicenseUsageEvent>,
    pub page: u32,
    pub page_size: u32,
    pub may_have_pruned_usage: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneUsageEventsCommand {
    pub retention_class: RetentionClass,
    pub prune_completed_before_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneUsageEventsResult {
    pub pruned_event_count: u64,
    pub retention_class: RetentionClass,
    pub prune_completed_before_ms: i64,
}
