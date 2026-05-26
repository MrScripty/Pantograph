use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceAvailabilityStatus, InferenceDiagnosticSeverity, InferenceInterfaceDiagnostic,
    InferenceInterfaceFingerprint, InferenceTaskKind, ValidatedDraftGraphValidationSummary,
    WorkflowGraphRevision, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use pantograph_runtime_attribution::{
    WorkflowExecutableValidationSnapshotRecord as AttributionWorkflowExecutableValidationSnapshotRecord,
    WorkflowExecutableValidationSnapshotStoreRequest as AttributionWorkflowExecutableValidationSnapshotStoreRequest,
    WorkflowId, WorkflowVersionId, WorkflowVersionRecord,
};
use pantograph_scheduler::{
    SchedulerEstimateHint, SchedulerNodeId, SchedulerRuntimeDeviceConstraints,
    SchedulerTraitSetting,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use uuid::Uuid;

use super::task_graph::{
    WorkflowSchedulerInferenceTaskProjection, WorkflowSchedulerInferenceTaskProjections,
    WorkflowSchedulerReadyInferenceTaskProjection,
};
use crate::graph::{
    InferenceInterfaceNodeProjectionRecord, WorkflowGraphInferenceValidationPublication,
};

pub const WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION: u16 = 1;
pub const WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_NODES: usize = 512;
pub const WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_DIAGNOSTICS_PER_NODE: usize = 64;
pub const WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TRAIT_SETTINGS_PER_NODE: usize = 128;
pub const WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_ESTIMATE_HINTS_PER_NODE: usize = 64;
const WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TEXT_LEN: usize = 1024;
const SNAPSHOT_ID_PREFIX: &str = "wfvalsnap_";

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct WorkflowExecutableValidationSnapshotId(String);

impl WorkflowExecutableValidationSnapshotId {
    #[must_use]
    pub fn generate() -> Self {
        Self(format!("{SNAPSHOT_ID_PREFIX}{}", Uuid::new_v4()))
    }

    pub fn parse(
        value: impl AsRef<str>,
    ) -> Result<Self, WorkflowExecutableValidationSnapshotError> {
        let value = validate_identifier("validation_snapshot_id", value.as_ref())?;
        if !value.starts_with(SNAPSHOT_ID_PREFIX) {
            return Err(
                WorkflowExecutableValidationSnapshotError::InvalidIdentifier {
                    field: "validation_snapshot_id",
                },
            );
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for WorkflowExecutableValidationSnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("WorkflowExecutableValidationSnapshotId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for WorkflowExecutableValidationSnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for WorkflowExecutableValidationSnapshotId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for WorkflowExecutableValidationSnapshotId {
    type Err = WorkflowExecutableValidationSnapshotError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for WorkflowExecutableValidationSnapshotId {
    type Error = WorkflowExecutableValidationSnapshotError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for WorkflowExecutableValidationSnapshotId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for WorkflowExecutableValidationSnapshotId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowExecutableValidationSnapshotLookupRequest {
    pub workflow_version_id: WorkflowVersionId,
    pub workflow_execution_fingerprint: String,
    #[serde(default = "default_descriptor_contract_version")]
    pub descriptor_contract_version: u32,
}

impl WorkflowExecutableValidationSnapshotLookupRequest {
    pub fn validate(&self) -> Result<(), WorkflowExecutableValidationSnapshotError> {
        validate_text(
            "workflow_execution_fingerprint",
            &self.workflow_execution_fingerprint,
        )?;
        validate_descriptor_contract_version(self.descriptor_contract_version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowExecutableValidationSnapshotRecord {
    #[serde(default = "default_snapshot_schema_version")]
    pub schema_version: u16,
    pub validation_snapshot_id: WorkflowExecutableValidationSnapshotId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: WorkflowVersionId,
    pub workflow_semantic_version: String,
    pub workflow_execution_fingerprint: String,
    #[serde(default = "default_descriptor_contract_version")]
    pub descriptor_contract_version: u32,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub validation_summary: DraftGraphValidationSummary,
    pub nodes: Vec<WorkflowExecutableValidationSnapshotNode>,
}

impl WorkflowExecutableValidationSnapshotRecord {
    pub fn from_validation_publication(
        workflow_version: &WorkflowVersionRecord,
        validation_snapshot_id: WorkflowExecutableValidationSnapshotId,
        publication: &WorkflowGraphInferenceValidationPublication,
    ) -> Result<Self, WorkflowExecutableValidationSnapshotError> {
        let nodes = publication
            .node_projections
            .iter()
            .map(WorkflowExecutableValidationSnapshotNode::from_projection_record)
            .collect::<Result<Vec<_>, _>>()?;
        let snapshot = Self {
            schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
            validation_snapshot_id,
            workflow_id: workflow_version.workflow_id.clone(),
            workflow_version_id: workflow_version.workflow_version_id.clone(),
            workflow_semantic_version: workflow_version.semantic_version.clone(),
            workflow_execution_fingerprint: workflow_version.execution_fingerprint.clone(),
            descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            graph_revision: publication.validation_session.graph_revision.clone(),
            validation_session_id: publication.validation_session.validation_session_id.clone(),
            validation_summary: publication.validation_session.summary.clone(),
            nodes,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), WorkflowExecutableValidationSnapshotError> {
        if self.schema_version != WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION {
            return Err(
                WorkflowExecutableValidationSnapshotError::UnsupportedSchemaVersion {
                    actual: self.schema_version,
                    expected: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
                },
            );
        }
        validate_descriptor_contract_version(self.descriptor_contract_version)?;
        validate_text("workflow_semantic_version", &self.workflow_semantic_version)?;
        validate_text(
            "workflow_execution_fingerprint",
            &self.workflow_execution_fingerprint,
        )?;
        let summary =
            ValidatedDraftGraphValidationSummary::try_from(self.validation_summary.clone())
                .map_err(|error| {
                    WorkflowExecutableValidationSnapshotError::InvalidValidationSummary {
                        message: error.to_string(),
                    }
                })?;
        if !summary.as_summary().executable
            || summary.as_summary().status != DraftGraphValidationStatus::Executable
        {
            return Err(
                WorkflowExecutableValidationSnapshotError::NonExecutableSummary {
                    status: self.validation_summary.status,
                },
            );
        }
        validate_collection_len(
            "nodes",
            self.nodes.len(),
            WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_NODES,
        )?;
        if self.nodes.is_empty() {
            return Err(WorkflowExecutableValidationSnapshotError::MissingNodes);
        }

        let mut seen_nodes = BTreeSet::new();
        for node in &self.nodes {
            node.validate()?;
            if !seen_nodes.insert(node.node_id.clone()) {
                return Err(WorkflowExecutableValidationSnapshotError::DuplicateNode {
                    node_id: node.node_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn to_attribution_store_request(
        &self,
    ) -> Result<
        AttributionWorkflowExecutableValidationSnapshotStoreRequest,
        WorkflowExecutableValidationSnapshotError,
    > {
        self.validate()?;
        let compact_snapshot_json = serde_json::to_string(self).map_err(|error| {
            WorkflowExecutableValidationSnapshotError::SnapshotSerialization {
                message: error.to_string(),
            }
        })?;
        Ok(
            AttributionWorkflowExecutableValidationSnapshotStoreRequest {
                workflow_version_id: self.workflow_version_id.clone(),
                workflow_id: self.workflow_id.clone(),
                workflow_execution_fingerprint: self.workflow_execution_fingerprint.clone(),
                snapshot_schema_version: self.schema_version,
                descriptor_contract_version: self.descriptor_contract_version,
                graph_revision: self.graph_revision.as_str().to_string(),
                validation_session_id: self.validation_session_id.as_str().to_string(),
                validation_snapshot_id: self.validation_snapshot_id.as_str().to_string(),
                compact_snapshot_json,
            },
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowExecutableValidationSnapshotNode {
    pub node_id: WorkflowNodeId,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub task_kind: InferenceTaskKind,
    pub model_ref: PumasModelRef,
    #[serde(default)]
    pub constraints: SchedulerRuntimeDeviceConstraints,
    pub availability_status: InferenceAvailabilityStatus,
    pub validation_status: DraftGraphValidationStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_settings: Vec<SchedulerTraitSetting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimate_hints: Vec<SchedulerEstimateHint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_diagnostics: Vec<WorkflowExecutableValidationSnapshotDiagnostic>,
}

impl WorkflowExecutableValidationSnapshotNode {
    fn from_projection_record(
        record: &InferenceInterfaceNodeProjectionRecord,
    ) -> Result<Self, WorkflowExecutableValidationSnapshotError> {
        record.descriptor.validate().map_err(|error| {
            WorkflowExecutableValidationSnapshotError::InvalidDescriptor {
                node_id: record.node_id.clone(),
                message: error.to_string(),
            }
        })?;
        record.authored_snapshot.validate().map_err(|error| {
            WorkflowExecutableValidationSnapshotError::InvalidAuthoredSnapshot {
                node_id: record.node_id.clone(),
                message: error.to_string(),
            }
        })?;
        let node = Self {
            node_id: record.node_id.clone(),
            descriptor_fingerprint: record.descriptor.descriptor_fingerprint.clone(),
            task_kind: record.descriptor.task_kind.clone(),
            model_ref: record.descriptor.model_ref.clone(),
            constraints: SchedulerRuntimeDeviceConstraints {
                requested_runtime_id: record.runtime_constraint.clone(),
                requested_device_id: record.device_constraint.clone(),
            },
            availability_status: record.descriptor.availability.status,
            validation_status: record.validation_summary.status,
            trait_settings: Vec::new(),
            estimate_hints: Vec::new(),
            blocking_diagnostics: snapshot_diagnostics_from_descriptor(
                &record.descriptor.diagnostics,
            ),
        };
        node.validate()?;
        Ok(node)
    }

    pub fn validate(&self) -> Result<(), WorkflowExecutableValidationSnapshotError> {
        self.model_ref.validate().map_err(|error| {
            WorkflowExecutableValidationSnapshotError::InvalidModelRef {
                node_id: self.node_id.clone(),
                message: error.to_string(),
            }
        })?;
        validate_collection_len(
            "node.trait_settings",
            self.trait_settings.len(),
            WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TRAIT_SETTINGS_PER_NODE,
        )?;
        validate_collection_len(
            "node.estimate_hints",
            self.estimate_hints.len(),
            WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_ESTIMATE_HINTS_PER_NODE,
        )?;
        validate_collection_len(
            "node.blocking_diagnostics",
            self.blocking_diagnostics.len(),
            WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_DIAGNOSTICS_PER_NODE,
        )?;
        for diagnostic in &self.blocking_diagnostics {
            diagnostic.validate()?;
        }
        if self.validation_status != DraftGraphValidationStatus::Executable {
            return Err(
                WorkflowExecutableValidationSnapshotError::NonExecutableNode {
                    node_id: self.node_id.clone(),
                    status: self.validation_status,
                },
            );
        }
        if self.availability_status != InferenceAvailabilityStatus::Available {
            return Err(WorkflowExecutableValidationSnapshotError::UnavailableNode {
                node_id: self.node_id.clone(),
                status: self.availability_status,
            });
        }
        Ok(())
    }

    fn scheduler_projection(
        &self,
    ) -> Result<WorkflowSchedulerInferenceTaskProjection, WorkflowExecutableValidationSnapshotError>
    {
        let scheduler_node_id = SchedulerNodeId::parse(self.node_id.as_str()).map_err(|error| {
            WorkflowExecutableValidationSnapshotError::InvalidSchedulerNodeId {
                node_id: self.node_id.clone(),
                message: error.to_string(),
            }
        })?;
        let task_type = DependencyTaskId::parse(self.task_kind.as_str()).map_err(|error| {
            WorkflowExecutableValidationSnapshotError::InvalidTaskKind {
                node_id: self.node_id.clone(),
                message: error.to_string(),
            }
        })?;

        Ok(WorkflowSchedulerInferenceTaskProjection::Ready(
            WorkflowSchedulerReadyInferenceTaskProjection {
                node_id: scheduler_node_id,
                descriptor_fingerprint: self.descriptor_fingerprint.clone(),
                task_type,
                model_ref: self.model_ref.clone(),
                constraints: self.constraints.clone(),
                trait_settings: self.trait_settings.clone(),
                estimate_hints: self.estimate_hints.clone(),
            },
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowExecutableValidationSnapshotDiagnostic {
    pub severity: WorkflowExecutableValidationSnapshotDiagnosticSeverity,
    pub code: WorkflowExecutableValidationSnapshotDiagnosticCode,
    pub message: String,
}

impl WorkflowExecutableValidationSnapshotDiagnostic {
    pub fn validate(&self) -> Result<(), WorkflowExecutableValidationSnapshotError> {
        validate_text("diagnostic.message", &self.message)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowExecutableValidationSnapshotDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowExecutableValidationSnapshotDiagnosticCode {
    SnapshotMissing,
    SnapshotStoreUnavailable,
    SnapshotStale,
    SnapshotNonExecutable,
    ContractIncompatible,
    WorkflowVersionMismatch,
    WorkflowFingerprintMismatch,
    NodeUnavailable,
    NodeInvalid,
    ProjectionInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedWorkflowExecutableValidationSnapshotRecord(
    WorkflowExecutableValidationSnapshotRecord,
);

impl ValidatedWorkflowExecutableValidationSnapshotRecord {
    #[must_use]
    pub fn as_record(&self) -> &WorkflowExecutableValidationSnapshotRecord {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> WorkflowExecutableValidationSnapshotRecord {
        self.0
    }

    pub fn scheduler_inference_task_projections(
        &self,
    ) -> Result<WorkflowSchedulerInferenceTaskProjections, WorkflowExecutableValidationSnapshotError>
    {
        let records = self
            .0
            .nodes
            .iter()
            .map(WorkflowExecutableValidationSnapshotNode::scheduler_projection)
            .collect::<Result<Vec<_>, _>>()?;
        WorkflowSchedulerInferenceTaskProjections::from_records(records).map_err(|error| {
            WorkflowExecutableValidationSnapshotError::InvalidSchedulerProjection {
                message: error.to_string(),
            }
        })
    }

    pub fn to_attribution_store_request(
        &self,
    ) -> Result<
        AttributionWorkflowExecutableValidationSnapshotStoreRequest,
        WorkflowExecutableValidationSnapshotError,
    > {
        self.0.to_attribution_store_request()
    }

    pub fn from_attribution_record(
        stored: AttributionWorkflowExecutableValidationSnapshotRecord,
        request: &WorkflowExecutableValidationSnapshotLookupRequest,
    ) -> Result<Self, WorkflowExecutableValidationSnapshotError> {
        request.validate()?;
        if stored.workflow_version_id != request.workflow_version_id {
            return Err(
                WorkflowExecutableValidationSnapshotError::SnapshotMetadataMismatch {
                    field: "workflow_version_id",
                    workflow_version_id: request.workflow_version_id.clone(),
                },
            );
        }
        if stored.workflow_execution_fingerprint != request.workflow_execution_fingerprint {
            return Err(
                WorkflowExecutableValidationSnapshotError::WorkflowFingerprintMismatch {
                    workflow_version_id: request.workflow_version_id.clone(),
                },
            );
        }
        if stored.descriptor_contract_version != request.descriptor_contract_version {
            return Err(
                WorkflowExecutableValidationSnapshotError::DescriptorContractVersionMismatch {
                    expected: request.descriptor_contract_version,
                    actual: stored.descriptor_contract_version,
                },
            );
        }

        let snapshot: WorkflowExecutableValidationSnapshotRecord =
            serde_json::from_str(&stored.compact_snapshot_json).map_err(|error| {
                WorkflowExecutableValidationSnapshotError::SnapshotSerialization {
                    message: error.to_string(),
                }
            })?;
        let validated = Self::try_from(snapshot)?;
        validate_attribution_metadata(&stored, validated.as_record(), request)?;
        Ok(validated)
    }
}

impl TryFrom<WorkflowExecutableValidationSnapshotRecord>
    for ValidatedWorkflowExecutableValidationSnapshotRecord
{
    type Error = WorkflowExecutableValidationSnapshotError;

    fn try_from(value: WorkflowExecutableValidationSnapshotRecord) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryWorkflowExecutableValidationSnapshotStore {
    available: bool,
    snapshots_by_version:
        BTreeMap<WorkflowVersionId, ValidatedWorkflowExecutableValidationSnapshotRecord>,
}

impl InMemoryWorkflowExecutableValidationSnapshotStore {
    #[must_use]
    pub fn available() -> Self {
        Self {
            available: true,
            snapshots_by_version: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            available: false,
            snapshots_by_version: BTreeMap::new(),
        }
    }

    pub fn record_snapshot(
        &mut self,
        snapshot: WorkflowExecutableValidationSnapshotRecord,
    ) -> Result<(), WorkflowExecutableValidationSnapshotError> {
        if !self.available {
            return Err(WorkflowExecutableValidationSnapshotError::SnapshotStoreUnavailable);
        }
        let snapshot = ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(snapshot)?;
        self.snapshots_by_version
            .insert(snapshot.as_record().workflow_version_id.clone(), snapshot);
        Ok(())
    }

    pub fn lookup_executable_snapshot(
        &self,
        request: &WorkflowExecutableValidationSnapshotLookupRequest,
    ) -> Result<
        ValidatedWorkflowExecutableValidationSnapshotRecord,
        WorkflowExecutableValidationSnapshotError,
    > {
        if !self.available {
            return Err(WorkflowExecutableValidationSnapshotError::SnapshotStoreUnavailable);
        }
        request.validate()?;
        let snapshot = self
            .snapshots_by_version
            .get(&request.workflow_version_id)
            .ok_or_else(
                || WorkflowExecutableValidationSnapshotError::SnapshotMissing {
                    workflow_version_id: request.workflow_version_id.clone(),
                },
            )?;
        if snapshot.as_record().descriptor_contract_version != request.descriptor_contract_version {
            return Err(
                WorkflowExecutableValidationSnapshotError::DescriptorContractVersionMismatch {
                    expected: request.descriptor_contract_version,
                    actual: snapshot.as_record().descriptor_contract_version,
                },
            );
        }
        if snapshot.as_record().workflow_execution_fingerprint
            != request.workflow_execution_fingerprint
        {
            return Err(
                WorkflowExecutableValidationSnapshotError::WorkflowFingerprintMismatch {
                    workflow_version_id: request.workflow_version_id.clone(),
                },
            );
        }
        Ok(snapshot.clone())
    }
}

impl Default for InMemoryWorkflowExecutableValidationSnapshotStore {
    fn default() -> Self {
        Self::available()
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowExecutableValidationSnapshotError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("{field} contains {actual_len} items; maximum is {max_len}")]
    TooManyItems {
        field: &'static str,
        actual_len: usize,
        max_len: usize,
    },
    #[error(
        "unsupported executable validation snapshot schema version {actual}; expected {expected}"
    )]
    UnsupportedSchemaVersion { actual: u16, expected: u16 },
    #[error("unsupported descriptor contract version {actual}; expected {expected}")]
    UnsupportedDescriptorContractVersion { actual: u32, expected: u32 },
    #[error("descriptor contract version mismatch: stored {actual}, requested {expected}")]
    DescriptorContractVersionMismatch { expected: u32, actual: u32 },
    #[error("validation summary is invalid: {message}")]
    InvalidValidationSummary { message: String },
    #[error("validation summary is not executable: {status:?}")]
    NonExecutableSummary { status: DraftGraphValidationStatus },
    #[error("executable validation snapshot must contain at least one inference node")]
    MissingNodes,
    #[error("duplicate executable validation snapshot node '{node_id}'")]
    DuplicateNode { node_id: WorkflowNodeId },
    #[error("node '{node_id}' model ref is invalid: {message}")]
    InvalidModelRef {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("node '{node_id}' descriptor is invalid: {message}")]
    InvalidDescriptor {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("node '{node_id}' authored snapshot is invalid: {message}")]
    InvalidAuthoredSnapshot {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("node '{node_id}' is not executable: {status:?}")]
    NonExecutableNode {
        node_id: WorkflowNodeId,
        status: DraftGraphValidationStatus,
    },
    #[error("node '{node_id}' is unavailable: {status:?}")]
    UnavailableNode {
        node_id: WorkflowNodeId,
        status: InferenceAvailabilityStatus,
    },
    #[error("node '{node_id}' has invalid scheduler node id: {message}")]
    InvalidSchedulerNodeId {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("node '{node_id}' has invalid inference task kind: {message}")]
    InvalidTaskKind {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("scheduler projection is invalid: {message}")]
    InvalidSchedulerProjection { message: String },
    #[error("executable validation snapshot store is unavailable")]
    SnapshotStoreUnavailable,
    #[error(
        "executable validation snapshot is missing for workflow version '{workflow_version_id}'"
    )]
    SnapshotMissing {
        workflow_version_id: WorkflowVersionId,
    },
    #[error("workflow fingerprint does not match executable validation snapshot for workflow version '{workflow_version_id}'")]
    WorkflowFingerprintMismatch {
        workflow_version_id: WorkflowVersionId,
    },
    #[error("executable validation snapshot serialization failed: {message}")]
    SnapshotSerialization { message: String },
    #[error("stored executable validation snapshot metadata field '{field}' does not match compact snapshot for workflow version '{workflow_version_id}'")]
    SnapshotMetadataMismatch {
        field: &'static str,
        workflow_version_id: WorkflowVersionId,
    },
}

fn validate_attribution_metadata(
    stored: &AttributionWorkflowExecutableValidationSnapshotRecord,
    snapshot: &WorkflowExecutableValidationSnapshotRecord,
    request: &WorkflowExecutableValidationSnapshotLookupRequest,
) -> Result<(), WorkflowExecutableValidationSnapshotError> {
    if stored.workflow_id != snapshot.workflow_id {
        return stored_metadata_mismatch("workflow_id", request);
    }
    if stored.workflow_version_id != snapshot.workflow_version_id {
        return stored_metadata_mismatch("workflow_version_id", request);
    }
    if stored.workflow_execution_fingerprint != snapshot.workflow_execution_fingerprint {
        return Err(
            WorkflowExecutableValidationSnapshotError::WorkflowFingerprintMismatch {
                workflow_version_id: request.workflow_version_id.clone(),
            },
        );
    }
    if stored.snapshot_schema_version != snapshot.schema_version {
        return stored_metadata_mismatch("snapshot_schema_version", request);
    }
    if stored.descriptor_contract_version != snapshot.descriptor_contract_version {
        return Err(
            WorkflowExecutableValidationSnapshotError::DescriptorContractVersionMismatch {
                expected: request.descriptor_contract_version,
                actual: stored.descriptor_contract_version,
            },
        );
    }
    if stored.graph_revision != snapshot.graph_revision.as_str() {
        return stored_metadata_mismatch("graph_revision", request);
    }
    if stored.validation_session_id != snapshot.validation_session_id.as_str() {
        return stored_metadata_mismatch("validation_session_id", request);
    }
    if stored.validation_snapshot_id != snapshot.validation_snapshot_id.as_str() {
        return stored_metadata_mismatch("validation_snapshot_id", request);
    }
    Ok(())
}

fn stored_metadata_mismatch<T>(
    field: &'static str,
    request: &WorkflowExecutableValidationSnapshotLookupRequest,
) -> Result<T, WorkflowExecutableValidationSnapshotError> {
    Err(
        WorkflowExecutableValidationSnapshotError::SnapshotMetadataMismatch {
            field,
            workflow_version_id: request.workflow_version_id.clone(),
        },
    )
}

fn default_snapshot_schema_version() -> u16 {
    WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION
}

fn default_descriptor_contract_version() -> u32 {
    INFERENCE_INTERFACE_CONTRACT_VERSION
}

fn validate_descriptor_contract_version(
    value: u32,
) -> Result<(), WorkflowExecutableValidationSnapshotError> {
    if value != INFERENCE_INTERFACE_CONTRACT_VERSION {
        return Err(
            WorkflowExecutableValidationSnapshotError::UnsupportedDescriptorContractVersion {
                actual: value,
                expected: INFERENCE_INTERFACE_CONTRACT_VERSION,
            },
        );
    }
    Ok(())
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, WorkflowExecutableValidationSnapshotError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WorkflowExecutableValidationSnapshotError::MissingField { field });
    }
    if trimmed.len() > WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TEXT_LEN {
        return Err(WorkflowExecutableValidationSnapshotError::FieldTooLong {
            field,
            max_len: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(WorkflowExecutableValidationSnapshotError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_text(
    field: &'static str,
    value: &str,
) -> Result<(), WorkflowExecutableValidationSnapshotError> {
    if value.trim().is_empty() {
        return Err(WorkflowExecutableValidationSnapshotError::MissingField { field });
    }
    if value.len() > WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TEXT_LEN {
        return Err(WorkflowExecutableValidationSnapshotError::FieldTooLong {
            field,
            max_len: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_TEXT_LEN,
        });
    }
    if value.chars().any(char::is_control) {
        return Err(WorkflowExecutableValidationSnapshotError::InvalidText { field });
    }
    Ok(())
}

fn validate_collection_len(
    field: &'static str,
    actual_len: usize,
    max_len: usize,
) -> Result<(), WorkflowExecutableValidationSnapshotError> {
    if actual_len > max_len {
        return Err(WorkflowExecutableValidationSnapshotError::TooManyItems {
            field,
            actual_len,
            max_len,
        });
    }
    Ok(())
}

fn snapshot_diagnostics_from_descriptor(
    diagnostics: &[InferenceInterfaceDiagnostic],
) -> Vec<WorkflowExecutableValidationSnapshotDiagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == InferenceDiagnosticSeverity::Error)
        .map(
            |diagnostic| WorkflowExecutableValidationSnapshotDiagnostic {
                severity: WorkflowExecutableValidationSnapshotDiagnosticSeverity::Error,
                code: WorkflowExecutableValidationSnapshotDiagnosticCode::NodeInvalid,
                message: diagnostic.message.clone(),
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{DeviceIntentId, RuntimeIntentId};
    use pantograph_inference_interface_contracts::{
        AuthoredInferenceInterfaceSnapshot, DraftGraphEnqueueDisabledReason, InferenceAvailability,
        InferenceInterfaceDescriptor,
    };
    use pantograph_runtime_attribution::{WorkflowId, WorkflowVersionId};
    use pantograph_scheduler::{
        SchedulerEstimateHint, SchedulerEstimateHintKind, SchedulerTraitId, SchedulerTraitSetting,
        SchedulerTraitValue,
    };

    use crate::graph::{
        InferenceInterfaceNodeProjectionRecord, WorkflowGraphInferenceValidationPublication,
        WorkflowGraphInferenceValidationSession,
    };

    use super::*;

    #[test]
    fn compacts_validation_publication_into_executable_snapshot() {
        let workflow_version = workflow_version_fixture();
        let publication = publication_fixture();
        let snapshot_id = WorkflowExecutableValidationSnapshotId::parse(
            "wfvalsnap_00000000-0000-4000-8000-000000000002",
        )
        .expect("valid snapshot id");

        let snapshot = WorkflowExecutableValidationSnapshotRecord::from_validation_publication(
            &workflow_version,
            snapshot_id.clone(),
            &publication,
        )
        .expect("publication should compact to snapshot");

        assert_eq!(snapshot.validation_snapshot_id, snapshot_id);
        assert_eq!(
            snapshot.workflow_version_id,
            workflow_version.workflow_version_id
        );
        assert_eq!(
            snapshot.workflow_execution_fingerprint,
            workflow_version.execution_fingerprint
        );
        assert_eq!(
            snapshot.graph_revision,
            publication.validation_session.graph_revision
        );
        assert_eq!(
            snapshot.validation_summary.status,
            DraftGraphValidationStatus::Executable
        );
        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].node_id.as_str(), "infer_node");
        assert_eq!(snapshot.nodes[0].task_kind.as_str(), "image_generation");
        assert_eq!(
            snapshot.nodes[0]
                .constraints
                .requested_runtime_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("pytorch")
        );
        assert!(snapshot.nodes[0].trait_settings.is_empty());
        assert!(snapshot.nodes[0].estimate_hints.is_empty());
    }

    #[test]
    fn validates_and_projects_executable_snapshot() {
        let snapshot = snapshot_fixture();
        let validated =
            ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(snapshot.clone())
                .expect("snapshot should validate");
        let projections = validated
            .scheduler_inference_task_projections()
            .expect("snapshot should project scheduler tasks");
        let node_id = SchedulerNodeId::parse("infer_node").expect("valid scheduler node id");

        let projection = projections.get(&node_id).expect("projection exists");
        match projection {
            WorkflowSchedulerInferenceTaskProjection::Ready(ready) => {
                assert_eq!(ready.node_id, node_id);
                assert_eq!(ready.task_type.as_str(), "image_generation");
                assert_eq!(ready.model_ref.model_id, "pumas://model/stable-diffusion");
                assert_eq!(
                    ready
                        .constraints
                        .requested_runtime_id
                        .as_ref()
                        .map(|id| id.as_str()),
                    Some("pytorch")
                );
                assert_eq!(ready.trait_settings.len(), 1);
                assert_eq!(ready.estimate_hints.len(), 1);
            }
            WorkflowSchedulerInferenceTaskProjection::Blocked(_) => {
                panic!("executable snapshot must produce ready projection")
            }
        }

        assert_eq!(validated.as_record(), &snapshot);
    }

    #[test]
    fn rejects_non_executable_summary() {
        let mut snapshot = snapshot_fixture();
        snapshot.validation_summary = DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Blocked,
            executable: false,
            enqueue_disabled_reasons: vec![DraftGraphEnqueueDisabledReason::BlockingDiagnostics],
            diagnostics_count: 1,
            blocking_diagnostics_count: 1,
        };

        assert!(matches!(
            ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(snapshot),
            Err(
                WorkflowExecutableValidationSnapshotError::NonExecutableSummary {
                    status: DraftGraphValidationStatus::Blocked
                }
            )
        ));
    }

    #[test]
    fn rejects_bounded_contents_overflow() {
        let mut snapshot = snapshot_fixture();
        let node = snapshot.nodes[0].clone();
        snapshot.nodes = vec![node; WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_MAX_NODES + 1];

        assert!(matches!(
            ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(snapshot),
            Err(WorkflowExecutableValidationSnapshotError::TooManyItems { field: "nodes", .. })
        ));
    }

    #[test]
    fn lookup_fails_closed_when_store_unavailable() {
        let store = InMemoryWorkflowExecutableValidationSnapshotStore::unavailable();
        let request = lookup_request_fixture(&snapshot_fixture());

        assert!(matches!(
            store.lookup_executable_snapshot(&request),
            Err(WorkflowExecutableValidationSnapshotError::SnapshotStoreUnavailable)
        ));
    }

    #[test]
    fn lookup_fails_closed_when_snapshot_missing() {
        let store = InMemoryWorkflowExecutableValidationSnapshotStore::available();
        let request = lookup_request_fixture(&snapshot_fixture());

        assert!(matches!(
            store.lookup_executable_snapshot(&request),
            Err(WorkflowExecutableValidationSnapshotError::SnapshotMissing { .. })
        ));
    }

    #[test]
    fn lookup_rejects_fingerprint_mismatch() {
        let snapshot = snapshot_fixture();
        let mut store = InMemoryWorkflowExecutableValidationSnapshotStore::available();
        store
            .record_snapshot(snapshot.clone())
            .expect("snapshot should store");
        let mut request = lookup_request_fixture(&snapshot);
        request.workflow_execution_fingerprint = "different-fingerprint".to_string();

        assert!(matches!(
            store.lookup_executable_snapshot(&request),
            Err(WorkflowExecutableValidationSnapshotError::WorkflowFingerprintMismatch { .. })
        ));
    }

    fn lookup_request_fixture(
        snapshot: &WorkflowExecutableValidationSnapshotRecord,
    ) -> WorkflowExecutableValidationSnapshotLookupRequest {
        WorkflowExecutableValidationSnapshotLookupRequest {
            workflow_version_id: snapshot.workflow_version_id.clone(),
            workflow_execution_fingerprint: snapshot.workflow_execution_fingerprint.clone(),
            descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        }
    }

    fn snapshot_fixture() -> WorkflowExecutableValidationSnapshotRecord {
        WorkflowExecutableValidationSnapshotRecord {
            schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
            validation_snapshot_id: WorkflowExecutableValidationSnapshotId::parse(
                "wfvalsnap_00000000-0000-4000-8000-000000000001",
            )
            .expect("valid snapshot id"),
            workflow_id: WorkflowId::try_from(
                "workflow_00000000-0000-4000-8000-000000000001".to_string(),
            )
            .expect("valid workflow id"),
            workflow_version_id: WorkflowVersionId::try_from(
                "wfver_00000000-0000-4000-8000-000000000001".to_string(),
            )
            .expect("valid workflow version id"),
            workflow_semantic_version: "1".to_string(),
            workflow_execution_fingerprint: "workflow-fingerprint".to_string(),
            descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            graph_revision: WorkflowGraphRevision::parse("revision_1")
                .expect("valid graph revision"),
            validation_session_id: DraftGraphValidationSessionId::parse("validation_session_1")
                .expect("valid validation session id"),
            validation_summary: DraftGraphValidationSummary {
                status: DraftGraphValidationStatus::Executable,
                executable: true,
                enqueue_disabled_reasons: Vec::new(),
                diagnostics_count: 0,
                blocking_diagnostics_count: 0,
            },
            nodes: vec![WorkflowExecutableValidationSnapshotNode {
                node_id: WorkflowNodeId::parse("infer_node").expect("valid node id"),
                descriptor_fingerprint: InferenceInterfaceFingerprint::parse(
                    "descriptor_fingerprint_1",
                )
                .expect("valid descriptor fingerprint"),
                task_kind: InferenceTaskKind::parse("image_generation").expect("valid task kind"),
                model_ref: PumasModelRef {
                    model_id: "pumas://model/stable-diffusion".to_string(),
                    revision: Some("main".to_string()),
                    selected_artifact_id: Some("artifact-diffusers".to_string()),
                    selected_artifact_path: None,
                    migration_diagnostics: Vec::new(),
                },
                constraints: SchedulerRuntimeDeviceConstraints {
                    requested_runtime_id: Some(
                        RuntimeIntentId::parse("pytorch").expect("valid runtime id"),
                    ),
                    requested_device_id: Some(
                        DeviceIntentId::parse("cuda:0").expect("valid device id"),
                    ),
                },
                availability_status: InferenceAvailabilityStatus::Available,
                validation_status: DraftGraphValidationStatus::Executable,
                trait_settings: vec![SchedulerTraitSetting {
                    trait_id: SchedulerTraitId::parse("denoising_scheduler")
                        .expect("valid trait id"),
                    value: SchedulerTraitValue::String("euler".to_string()),
                }],
                estimate_hints: vec![SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakVramBytes,
                    value: 4_294_967_296,
                }],
                blocking_diagnostics: Vec::new(),
            }],
        }
    }

    fn workflow_version_fixture() -> WorkflowVersionRecord {
        WorkflowVersionRecord {
            workflow_version_id: WorkflowVersionId::try_from(
                "wfver_00000000-0000-4000-8000-000000000001".to_string(),
            )
            .expect("valid workflow version id"),
            workflow_id: WorkflowId::try_from(
                "workflow_00000000-0000-4000-8000-000000000001".to_string(),
            )
            .expect("valid workflow id"),
            semantic_version: "1".to_string(),
            execution_fingerprint: "workflow-fingerprint".to_string(),
            executable_topology_json: "{}".to_string(),
            created_at_ms: 1,
        }
    }

    fn publication_fixture() -> WorkflowGraphInferenceValidationPublication {
        let graph_revision =
            WorkflowGraphRevision::parse("revision_1").expect("valid graph revision");
        let validation_session_id = DraftGraphValidationSessionId::parse("validation_session_1")
            .expect("valid validation session id");
        let summary = DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Executable,
            executable: true,
            enqueue_disabled_reasons: Vec::new(),
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        };
        let descriptor = InferenceInterfaceDescriptor {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            model_ref: PumasModelRef {
                model_id: "pumas://model/stable-diffusion".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("artifact-diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            task_kind: InferenceTaskKind::parse("image_generation").expect("valid task kind"),
            descriptor_fingerprint: InferenceInterfaceFingerprint::parse(
                "descriptor_fingerprint_1",
            )
            .expect("valid descriptor fingerprint"),
            runtime_conditions: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            availability: InferenceAvailability::available(),
            diagnostics: Vec::new(),
        };
        WorkflowGraphInferenceValidationPublication {
            validation_session: WorkflowGraphInferenceValidationSession {
                contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
                validation_session_id: validation_session_id.clone(),
                graph_revision,
                latest_sequence: 0,
                summary: summary.clone(),
                events: Vec::new(),
            },
            node_projections: vec![InferenceInterfaceNodeProjectionRecord {
                node_id: WorkflowNodeId::parse("infer_node").expect("valid node id"),
                descriptor: descriptor.clone(),
                authored_snapshot: AuthoredInferenceInterfaceSnapshot {
                    contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
                    descriptor_fingerprint: descriptor.descriptor_fingerprint,
                    task_kind: descriptor.task_kind,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                },
                validation_summary: summary,
                runtime_constraint: Some(RuntimeIntentId::parse("pytorch").unwrap()),
                device_constraint: Some(DeviceIntentId::parse("cuda:0").unwrap()),
            }],
            request_diagnostics: Vec::new(),
        }
    }
}
