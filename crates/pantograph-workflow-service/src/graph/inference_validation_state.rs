use std::collections::{BTreeMap, HashMap};

use pantograph_dependency_planning::{
    DependencyTaskId, DeviceIntentId, PumasModelRef, RuntimeIntentId,
};
use pantograph_inference_interface_contracts::{
    DependencyEnvironmentActionIntent, DependencyEnvironmentActionIntentResult,
    DependencyEnvironmentActionIntentStatus, DraftGraphValidationSessionId,
    DraftGraphValidationStatus, DraftGraphValidationSummary, InferenceAvailabilityStatus,
    InferenceDiagnosticCode, InferenceDiagnosticSeverity, InferenceInterfaceDiagnostic,
    InferenceInterfaceFingerprint, InferenceTaskKind, ValidatedDependencyEnvironmentActionIntent,
    WorkflowGraphRevision, WorkflowNodeId,
};
use tokio::sync::RwLock;

use super::dependency_environment_subject::DependencyEnvironmentActionSubjectResolution;
use super::inference_interface_validation::{
    InferenceInterfaceValidationSessionError, WorkflowGraphInferenceValidationSession,
};
use super::InferenceInterfaceNodeProjectionRecord;
use crate::workflow::{
    WorkflowSchedulerBlockedInferenceTaskProjection,
    WorkflowSchedulerBlockedInferenceTaskProjectionReason,
    WorkflowSchedulerInferenceTaskProjection, WorkflowSchedulerInferenceTaskProjections,
    WorkflowSchedulerReadyInferenceTaskProjection,
};

#[derive(Debug)]
pub(crate) struct CurrentInferenceValidationStateStore {
    summaries:
        RwLock<HashMap<CurrentInferenceValidationStateKey, CurrentInferenceValidationStateRecord>>,
}

impl Default for CurrentInferenceValidationStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentInferenceValidationStateStore {
    pub(crate) fn new() -> Self {
        Self {
            summaries: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) async fn record_validation_session(
        &self,
        graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
        session: WorkflowGraphInferenceValidationSession,
    ) -> Result<(), InferenceInterfaceValidationSessionError> {
        self.record_validation_publication(graph_session_id, session, Vec::new())
            .await
    }

    pub(crate) async fn record_validation_publication(
        &self,
        graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
        session: WorkflowGraphInferenceValidationSession,
        node_projections: Vec<InferenceInterfaceNodeProjectionRecord>,
    ) -> Result<(), InferenceInterfaceValidationSessionError> {
        session.validate()?;
        let key = CurrentInferenceValidationStateKey {
            graph_session_id,
            graph_revision: session.graph_revision.clone(),
        };
        let record = CurrentInferenceValidationStateRecord {
            validation_session_id: session.validation_session_id,
            summary: session.summary,
            nodes: node_projections
                .into_iter()
                .map(|projection| {
                    (
                        projection.node_id.clone(),
                        CurrentInferenceValidationNodeRecord::from(projection),
                    )
                })
                .collect(),
        };
        self.summaries.write().await.insert(key, record);
        Ok(())
    }

    pub(crate) async fn resolve_dependency_environment_action_intent(
        &self,
        request: DependencyEnvironmentActionIntentStateRequest,
    ) -> DependencyEnvironmentActionIntentResult {
        let intent = request.intent.into_inner();
        if request.current_graph_revision != intent.graph_revision {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::GraphRevisionMismatch,
                "Dependency environment action was requested for a stale graph revision.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }

        let inference_node_id = match request.subject {
            DependencyEnvironmentActionSubjectResolution::Resolved { inference_node_id } => {
                inference_node_id
            }
            DependencyEnvironmentActionSubjectResolution::Blocked {
                code,
                message,
                hint,
            } => return blocked_dependency_environment_action_result(&intent, code, message, hint),
        };

        let key = CurrentInferenceValidationStateKey {
            graph_session_id: intent.graph_session_id.clone(),
            graph_revision: intent.graph_revision.clone(),
        };
        let summaries = self.summaries.read().await;
        let Some(record) = summaries.get(&key) else {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::ValidationSummaryMissing,
                "Inference validation has not completed for this graph revision.",
                Some("Run descriptor validation before resolving dependency environments."),
            );
        };

        if intent
            .validation_session_id
            .as_ref()
            .is_some_and(|validation_session_id| {
                validation_session_id != &record.validation_session_id
            })
        {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::DescriptorStale,
                "Dependency environment action was requested for a stale validation session.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }

        if !record.summary.executable {
            return blocked_dependency_environment_action_result(
                &intent,
                diagnostic_code_for_summary_status(record.summary.status),
                "Inference validation summary is not executable for this graph revision.",
                Some("Resolve blocking inference validation diagnostics before resolving dependencies."),
            );
        }

        if !record.nodes.contains_key(&inference_node_id) {
            return blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::DependencySidecarDescriptorUnavailable,
                "Associated inference node is missing current descriptor validation state.",
                Some("Refresh descriptor validation before resolving dependency environments."),
            );
        }

        if let Some(node_record) = record.nodes.get(&inference_node_id) {
            if !node_record.has_dependency_basis() {
                return blocked_dependency_environment_action_result(
                    &intent,
                    InferenceDiagnosticCode::DependencySidecarDescriptorInvalid,
                    "Associated inference validation state is incomplete for dependency derivation.",
                    Some("Refresh descriptor validation before resolving dependency environments."),
                );
            }
        }

        blocked_dependency_environment_action_result(
            &intent,
            InferenceDiagnosticCode::DependencyRequirementsMissing,
            "Dependency environment request derivation requires dependency requirements for this graph revision.",
            Some("Resolve dependency requirements from the current validation state before checking or installing environments."),
        )
    }

    pub(crate) async fn scheduler_inference_task_projections(
        &self,
        request: CurrentInferenceSchedulerProjectionRequest,
    ) -> Result<WorkflowSchedulerInferenceTaskProjections, CurrentInferenceSchedulerProjectionError>
    {
        let summaries = self.summaries.read().await;
        let key = CurrentInferenceValidationStateKey {
            graph_session_id: request.graph_session_id,
            graph_revision: request.graph_revision,
        };
        let record = summaries
            .get(&key)
            .ok_or(CurrentInferenceSchedulerProjectionError::ValidationSummaryMissing)?;

        if request
            .validation_session_id
            .as_ref()
            .is_some_and(|validation_session_id| {
                validation_session_id != &record.validation_session_id
            })
        {
            return Err(CurrentInferenceSchedulerProjectionError::ValidationSessionMismatch);
        }

        if !record.summary.executable {
            return Err(
                CurrentInferenceSchedulerProjectionError::ValidationSummaryNotExecutable {
                    status: record.summary.status,
                },
            );
        }

        let mut projections = Vec::with_capacity(record.nodes.len());
        for node in record.nodes.values() {
            projections.push(node.scheduler_inference_task_projection()?);
        }
        WorkflowSchedulerInferenceTaskProjections::from_records(projections).map_err(|error| {
            CurrentInferenceSchedulerProjectionError::InvalidProjection {
                message: error.to_string(),
            }
        })
    }
}

fn diagnostic_code_for_summary_status(
    status: DraftGraphValidationStatus,
) -> InferenceDiagnosticCode {
    match status {
        DraftGraphValidationStatus::Pending => InferenceDiagnosticCode::GraphValidationPending,
        DraftGraphValidationStatus::Stale => InferenceDiagnosticCode::DescriptorStale,
        DraftGraphValidationStatus::Unresolved | DraftGraphValidationStatus::Unavailable => {
            InferenceDiagnosticCode::DescriptorUnavailable
        }
        DraftGraphValidationStatus::Blocked => InferenceDiagnosticCode::DriftDetected,
        DraftGraphValidationStatus::Executable => {
            InferenceDiagnosticCode::DependencyRequirementsMissing
        }
        _ => InferenceDiagnosticCode::DescriptorUnavailable,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyEnvironmentActionIntentStateRequest {
    pub intent: ValidatedDependencyEnvironmentActionIntent,
    pub current_graph_revision: WorkflowGraphRevision,
    pub subject: DependencyEnvironmentActionSubjectResolution,
}

#[derive(Debug, Clone)]
pub(crate) struct CurrentInferenceSchedulerProjectionRequest {
    pub graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: Option<DraftGraphValidationSessionId>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CurrentInferenceSchedulerProjectionError {
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
    #[error("inference scheduler projection is invalid: {message}")]
    InvalidProjection { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CurrentInferenceValidationStateKey {
    graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
    graph_revision: WorkflowGraphRevision,
}

#[derive(Debug, Clone)]
struct CurrentInferenceValidationStateRecord {
    validation_session_id: DraftGraphValidationSessionId,
    summary: DraftGraphValidationSummary,
    nodes: BTreeMap<WorkflowNodeId, CurrentInferenceValidationNodeRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct CurrentInferenceValidationNodeRecord {
    pub node_id: WorkflowNodeId,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub task_kind: InferenceTaskKind,
    pub availability_status: InferenceAvailabilityStatus,
    pub validation_status: DraftGraphValidationStatus,
    pub pumas_model_ref: PumasModelRef,
    pub runtime_constraint: Option<RuntimeIntentId>,
    pub device_constraint: Option<DeviceIntentId>,
}

impl From<InferenceInterfaceNodeProjectionRecord> for CurrentInferenceValidationNodeRecord {
    fn from(record: InferenceInterfaceNodeProjectionRecord) -> Self {
        Self {
            node_id: record.node_id,
            descriptor_fingerprint: record.descriptor.descriptor_fingerprint,
            task_kind: record.descriptor.task_kind,
            availability_status: record.descriptor.availability.status,
            validation_status: record.validation_summary.status,
            pumas_model_ref: record.descriptor.model_ref,
            runtime_constraint: record.runtime_constraint,
            device_constraint: record.device_constraint,
        }
    }
}

impl CurrentInferenceValidationNodeRecord {
    fn has_dependency_basis(&self) -> bool {
        !self.node_id.as_str().is_empty()
            && !self.descriptor_fingerprint.as_str().is_empty()
            && !self.task_kind.as_str().is_empty()
            && self.pumas_model_ref.validate().is_ok()
            && matches!(
                self.availability_status,
                InferenceAvailabilityStatus::Available
                    | InferenceAvailabilityStatus::Pending
                    | InferenceAvailabilityStatus::Stale
                    | InferenceAvailabilityStatus::Unavailable
                    | InferenceAvailabilityStatus::Unsupported
                    | InferenceAvailabilityStatus::NotImplemented
            )
            && matches!(
                self.validation_status,
                DraftGraphValidationStatus::Pending
                    | DraftGraphValidationStatus::Stale
                    | DraftGraphValidationStatus::Unresolved
                    | DraftGraphValidationStatus::Unavailable
                    | DraftGraphValidationStatus::Blocked
                    | DraftGraphValidationStatus::Executable
            )
            && self
                .runtime_constraint
                .as_ref()
                .map(|runtime| !runtime.as_str().is_empty())
                .unwrap_or(true)
            && self
                .device_constraint
                .as_ref()
                .map(|device| !device.as_str().is_empty())
                .unwrap_or(true)
    }

    fn scheduler_inference_task_projection(
        &self,
    ) -> Result<WorkflowSchedulerInferenceTaskProjection, CurrentInferenceSchedulerProjectionError>
    {
        if !self.has_dependency_basis() {
            return Err(
                CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                    node_id: self.node_id.clone(),
                    message: "node record is missing descriptor, task, model, availability, or constraint data"
                        .to_string(),
                },
            );
        }

        if self.validation_status == DraftGraphValidationStatus::Executable
            && self.availability_status == InferenceAvailabilityStatus::Available
        {
            return Ok(WorkflowSchedulerInferenceTaskProjection::Ready(
                WorkflowSchedulerReadyInferenceTaskProjection {
                    node_id: pantograph_scheduler::SchedulerNodeId::parse(self.node_id.as_str())
                        .map_err(|error| {
                            CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                                node_id: self.node_id.clone(),
                                message: format!("scheduler node id is invalid: {error}"),
                            }
                        })?,
                    descriptor_fingerprint: self.descriptor_fingerprint.clone(),
                    task_type: DependencyTaskId::parse(self.task_kind.as_str()).map_err(
                        |error| CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                            node_id: self.node_id.clone(),
                            message: format!("scheduler task kind is invalid: {error}"),
                        },
                    )?,
                    model_ref: self.pumas_model_ref.clone(),
                    constraints: pantograph_scheduler::SchedulerRuntimeDeviceConstraints {
                        requested_runtime_id: self.runtime_constraint.clone(),
                        requested_device_id: self.device_constraint.clone(),
                    },
                    trait_settings: Vec::new(),
                    estimate_hints: Vec::new(),
                },
            ));
        }

        Ok(WorkflowSchedulerInferenceTaskProjection::Blocked(
            WorkflowSchedulerBlockedInferenceTaskProjection {
                node_id: pantograph_scheduler::SchedulerNodeId::parse(self.node_id.as_str())
                    .map_err(|error| {
                        CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                            node_id: self.node_id.clone(),
                            message: format!("scheduler node id is invalid: {error}"),
                        }
                    })?,
                descriptor_fingerprint: Some(self.descriptor_fingerprint.clone()),
                reason: blocked_projection_reason(self.validation_status, self.availability_status),
                message: format!(
                    "inference descriptor is not executable: validation={:?}, availability={:?}",
                    self.validation_status, self.availability_status
                ),
            },
        ))
    }
}

fn blocked_projection_reason(
    validation_status: DraftGraphValidationStatus,
    availability_status: InferenceAvailabilityStatus,
) -> WorkflowSchedulerBlockedInferenceTaskProjectionReason {
    match validation_status {
        DraftGraphValidationStatus::Stale => {
            WorkflowSchedulerBlockedInferenceTaskProjectionReason::Stale
        }
        DraftGraphValidationStatus::Unavailable | DraftGraphValidationStatus::Unresolved => {
            WorkflowSchedulerBlockedInferenceTaskProjectionReason::Unavailable
        }
        DraftGraphValidationStatus::Pending => {
            WorkflowSchedulerBlockedInferenceTaskProjectionReason::Missing
        }
        DraftGraphValidationStatus::Blocked => {
            WorkflowSchedulerBlockedInferenceTaskProjectionReason::Invalid
        }
        DraftGraphValidationStatus::Executable => match availability_status {
            InferenceAvailabilityStatus::Available => {
                WorkflowSchedulerBlockedInferenceTaskProjectionReason::Invalid
            }
            InferenceAvailabilityStatus::Pending => {
                WorkflowSchedulerBlockedInferenceTaskProjectionReason::Missing
            }
            InferenceAvailabilityStatus::Stale => {
                WorkflowSchedulerBlockedInferenceTaskProjectionReason::Stale
            }
            InferenceAvailabilityStatus::Unavailable
            | InferenceAvailabilityStatus::Unsupported
            | InferenceAvailabilityStatus::NotImplemented => {
                WorkflowSchedulerBlockedInferenceTaskProjectionReason::Unavailable
            }
            _ => WorkflowSchedulerBlockedInferenceTaskProjectionReason::Invalid,
        },
        _ => WorkflowSchedulerBlockedInferenceTaskProjectionReason::Invalid,
    }
}

fn blocked_dependency_environment_action_result(
    intent: &DependencyEnvironmentActionIntent,
    code: InferenceDiagnosticCode,
    message: &str,
    hint: Option<&str>,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: vec![InferenceInterfaceDiagnostic {
            severity: InferenceDiagnosticSeverity::Error,
            code,
            message: message.to_string(),
            hint: hint.map(str::to_string),
            port_id: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use pantograph_inference_interface_contracts::{
        AuthoredInferenceInterfaceSnapshot, DependencyEnvironmentAction,
        DraftGraphValidationStatus, InferenceAvailability, InferenceDiagnosticCode,
        InferenceInterfaceDescriptor,
    };

    use super::*;

    #[tokio::test]
    async fn action_intent_state_rejects_stale_graph_revision() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbb",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::GraphRevisionMismatch
        );
    }

    #[tokio::test]
    async fn action_intent_state_rejects_missing_target_node() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                false,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::TargetNodeMissing
        );
    }

    #[tokio::test]
    async fn action_intent_state_requires_current_summary() {
        let store = CurrentInferenceValidationStateStore::new();

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::ValidationSummaryMissing
        );
    }

    #[tokio::test]
    async fn action_intent_state_blocks_pending_summary() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_session(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Pending,
                    false,
                ),
            )
            .await
            .expect("valid validation session");

        let result = store
            .resolve_dependency_environment_action_intent(state_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::GraphValidationPending
        );
    }

    #[tokio::test]
    async fn action_intent_state_rejects_stale_validation_session() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_session(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
            )
            .await
            .expect("valid validation session");

        let result = store
            .resolve_dependency_environment_action_intent(state_request_with_validation_session(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "validation.session.old",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::DescriptorStale
        );
    }

    #[tokio::test]
    async fn action_intent_state_accepts_executable_summary_until_requirements_derivation() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_publication(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
                vec![node_projection(DraftGraphValidationStatus::Executable)],
            )
            .await
            .expect("valid validation session");

        let result = store
            .resolve_dependency_environment_action_intent(state_request_with_validation_session(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "dependency-node-1",
                true,
            ))
            .await;

        assert_eq!(
            result.diagnostics[0].code,
            InferenceDiagnosticCode::DependencyRequirementsMissing
        );
    }

    #[tokio::test]
    async fn scheduler_projection_state_requires_current_summary() {
        let store = CurrentInferenceValidationStateStore::new();

        let error = store
            .scheduler_inference_task_projections(scheduler_projection_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                None,
            ))
            .await
            .expect_err("missing summary");

        assert!(matches!(
            error,
            CurrentInferenceSchedulerProjectionError::ValidationSummaryMissing
        ));
    }

    #[tokio::test]
    async fn scheduler_projection_state_rejects_stale_validation_session() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_publication(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
                vec![node_projection(DraftGraphValidationStatus::Executable)],
            )
            .await
            .expect("record publication");

        let error = store
            .scheduler_inference_task_projections(scheduler_projection_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                Some("validation.session.old"),
            ))
            .await
            .expect_err("stale validation session");

        assert!(matches!(
            error,
            CurrentInferenceSchedulerProjectionError::ValidationSessionMismatch
        ));
    }

    #[tokio::test]
    async fn scheduler_projection_state_requires_executable_summary() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_publication(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Unavailable,
                    false,
                ),
                vec![node_projection(DraftGraphValidationStatus::Unavailable)],
            )
            .await
            .expect("record publication");

        let error = store
            .scheduler_inference_task_projections(scheduler_projection_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                Some("validation.session.1"),
            ))
            .await
            .expect_err("non-executable summary");

        assert!(matches!(
            error,
            CurrentInferenceSchedulerProjectionError::ValidationSummaryNotExecutable {
                status: DraftGraphValidationStatus::Unavailable
            }
        ));
    }

    #[tokio::test]
    async fn scheduler_projection_state_projects_executable_node_records() {
        let store = CurrentInferenceValidationStateStore::new();
        store
            .record_validation_publication(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
                vec![node_projection(DraftGraphValidationStatus::Executable)],
            )
            .await
            .expect("record publication");

        let projections = store
            .scheduler_inference_task_projections(scheduler_projection_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                Some("validation.session.1"),
            ))
            .await
            .expect("scheduler projections");
        let projection = projections
            .get(&pantograph_scheduler::SchedulerNodeId::parse("infer").expect("node id"))
            .expect("projection for inference node");

        let WorkflowSchedulerInferenceTaskProjection::Ready(projection) = projection else {
            panic!("expected ready projection");
        };
        assert_eq!(projection.node_id.as_str(), "infer");
        assert_eq!(projection.task_type.as_str(), "image_generation");
        assert_eq!(
            projection.model_ref.model_id,
            "image/example/tiny-diffusion"
        );
        assert_eq!(
            projection
                .constraints
                .requested_runtime_id
                .as_ref()
                .map(|runtime| runtime.as_str()),
            Some("pytorch")
        );
    }

    fn state_request(
        graph_session_id: &str,
        intent_revision: &str,
        current_revision: &str,
        target_node_id: &str,
        target_node_exists: bool,
    ) -> DependencyEnvironmentActionIntentStateRequest {
        DependencyEnvironmentActionIntentStateRequest {
            intent: ValidatedDependencyEnvironmentActionIntent::try_from(
                DependencyEnvironmentActionIntent {
                    contract_version: 1,
                    graph_session_id: graph_session_id.parse().expect("valid graph session id"),
                    graph_revision: intent_revision.parse().expect("valid graph revision"),
                    validation_session_id: None,
                    target_node_id: target_node_id.parse().expect("valid target node id"),
                    action: DependencyEnvironmentAction::Resolve,
                },
            )
            .expect("valid action intent"),
            current_graph_revision: current_revision.parse().expect("valid current revision"),
            subject: if target_node_exists {
                DependencyEnvironmentActionSubjectResolution::resolved(
                    "infer".parse().expect("valid inference node id"),
                )
            } else {
                DependencyEnvironmentActionSubjectResolution::Blocked {
                    code: InferenceDiagnosticCode::TargetNodeMissing,
                    message:
                        "Dependency environment action target node does not exist in the current graph.",
                    hint: None,
                }
            },
        }
    }

    fn state_request_with_validation_session(
        graph_session_id: &str,
        intent_revision: &str,
        current_revision: &str,
        validation_session_id: &str,
        target_node_id: &str,
        target_node_exists: bool,
    ) -> DependencyEnvironmentActionIntentStateRequest {
        let mut request = state_request(
            graph_session_id,
            intent_revision,
            current_revision,
            target_node_id,
            target_node_exists,
        );
        let mut intent = request.intent.into_inner();
        intent.validation_session_id = Some(
            validation_session_id
                .parse()
                .expect("valid validation session id"),
        );
        request.intent = ValidatedDependencyEnvironmentActionIntent::try_from(intent)
            .expect("valid action intent");
        request
    }

    fn validation_session(
        graph_revision: &str,
        status: DraftGraphValidationStatus,
        executable: bool,
    ) -> WorkflowGraphInferenceValidationSession {
        WorkflowGraphInferenceValidationSession {
            contract_version:
                pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION,
            validation_session_id: "validation.session.1"
                .parse()
                .expect("valid validation session id"),
            graph_revision: graph_revision.parse().expect("valid graph revision"),
            latest_sequence: 0,
            summary: DraftGraphValidationSummary {
                status,
                executable,
                enqueue_disabled_reasons: Vec::new(),
                diagnostics_count: 0,
                blocking_diagnostics_count: 0,
            },
            events: Vec::new(),
        }
    }

    fn scheduler_projection_request(
        graph_session_id: &str,
        graph_revision: &str,
        validation_session_id: Option<&str>,
    ) -> CurrentInferenceSchedulerProjectionRequest {
        CurrentInferenceSchedulerProjectionRequest {
            graph_session_id: graph_session_id.parse().expect("valid graph session id"),
            graph_revision: graph_revision.parse().expect("valid graph revision"),
            validation_session_id: validation_session_id
                .map(|value| value.parse().expect("valid validation session id")),
        }
    }

    fn node_projection(
        validation_status: DraftGraphValidationStatus,
    ) -> InferenceInterfaceNodeProjectionRecord {
        InferenceInterfaceNodeProjectionRecord {
            node_id: "infer".parse().expect("valid node id"),
            descriptor: InferenceInterfaceDescriptor {
                contract_version:
                    pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION,
                model_ref: PumasModelRef {
                    model_id: "image/example/tiny-diffusion".to_string(),
                    revision: Some("main".to_string()),
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: None,
                    migration_diagnostics: Vec::new(),
                },
                task_kind: "image_generation".parse().expect("valid task kind"),
                descriptor_fingerprint: "iface.scheduler.v1".parse().expect("fingerprint"),
                runtime_conditions: Vec::new(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                availability: InferenceAvailability::available(),
                diagnostics: Vec::new(),
            },
            authored_snapshot: AuthoredInferenceInterfaceSnapshot {
                contract_version:
                    pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION,
                descriptor_fingerprint: "iface.scheduler.v1".parse().expect("fingerprint"),
                task_kind: "image_generation".parse().expect("valid task kind"),
                inputs: Vec::new(),
                outputs: Vec::new(),
            },
            validation_summary: DraftGraphValidationSummary {
                status: validation_status,
                executable: validation_status == DraftGraphValidationStatus::Executable,
                enqueue_disabled_reasons: Vec::new(),
                diagnostics_count: 0,
                blocking_diagnostics_count: 0,
            },
            runtime_constraint: Some("pytorch".parse().expect("runtime id")),
            device_constraint: Some("cuda.0".parse().expect("device id")),
        }
    }
}
