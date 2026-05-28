use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    WorkflowGraphRevision, WorkflowGraphSessionId, WorkflowNodeId,
};
use thiserror::Error;

use super::inference_interface_publication::InferenceInterfaceNodeProjectionRecord;
use super::inference_validation_state::CurrentDependencyRequirementsProof;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CurrentExecutableValidationSnapshotSourceRequest {
    pub graph_session_id: WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: Option<DraftGraphValidationSessionId>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CurrentExecutableValidationSnapshotSource {
    pub graph_session_id: WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub validation_summary: DraftGraphValidationSummary,
    pub nodes: Vec<CurrentExecutableValidationSnapshotNodeSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CurrentExecutableValidationSnapshotNodeSource {
    pub projection: InferenceInterfaceNodeProjectionRecord,
    pub dependency_requirements_proof: CurrentDependencyRequirementsProof,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum CurrentExecutableValidationSnapshotSourceError {
    #[error("inference validation summary is missing for the current graph revision")]
    ValidationSummaryMissing,
    #[error("inference validation session does not match the current graph validation state")]
    ValidationSessionMismatch,
    #[error("inference validation summary is not executable: {status:?}")]
    ValidationSummaryNotExecutable { status: DraftGraphValidationStatus },
    #[error("inference validation node state is incomplete for node {node_id}: {message}")]
    IncompleteNodeState {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("dependency requirements proof is missing for executable node {node_id}")]
    DependencyProofMissing { node_id: WorkflowNodeId },
    #[error("dependency requirements proof is stale for executable node {node_id}")]
    DependencyProofStale { node_id: WorkflowNodeId },
    #[error("dependency requirements proof is unavailable for executable node {node_id}")]
    DependencyProofUnavailable { node_id: WorkflowNodeId },
    #[error("dependency requirements proof is invalid for executable node {node_id}")]
    DependencyProofInvalid { node_id: WorkflowNodeId },
}
