use std::collections::{BTreeMap, HashMap};

use pantograph_dependency_planning::{
    produce_dependency_requirements_proof, DependencyBindingId, DependencyEnvironmentRequest,
    DependencyNodeTypeId, DependencyOverrideFingerprint, DependencyOverridePatchV1,
    DependencyPlanningCallerContext, DependencyPlanningDiagnostic, DependencyPlanningIdentityKey,
    DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyReadinessDescriptorFingerprint, DependencyReadinessGraphRevision,
    DependencyReadinessValidationSessionId, DependencyRequirementsId, DependencyRequirementsProof,
    DependencyRequirementsProofStatus, DependencyTaskId, DeviceIntentId, PumasModelRef,
    RuntimeIntentId, SchedulerIntent, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyPlanningRequest,
};
use pantograph_inference_interface_contracts::{
    DependencyEnvironmentAction, DependencyEnvironmentActionIntent,
    DependencyEnvironmentActionIntentResult, DependencyEnvironmentActionIntentStatus,
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceAvailabilityStatus, InferenceDiagnosticCode, InferenceDiagnosticSeverity,
    InferenceInterfaceDiagnostic, InferenceInterfaceFingerprint, InferenceTaskKind,
    ValidatedDependencyEnvironmentActionIntent, WorkflowGraphRevision, WorkflowGraphSessionId,
    WorkflowNodeId,
};
use pantograph_scheduler::SchedulerTraitSetting;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::dependency_environment_subject::DependencyEnvironmentActionSubjectResolution;
use super::executable_validation_snapshot_source::{
    CurrentExecutableValidationSnapshotNodeSource, CurrentExecutableValidationSnapshotSource,
    CurrentExecutableValidationSnapshotSourceError,
    CurrentExecutableValidationSnapshotSourceRequest,
};
use super::inference_interface_validation::{
    InferenceInterfaceValidationSessionError, WorkflowGraphInferenceValidationSession,
};
use super::InferenceInterfaceNodeProjectionRecord;
use crate::workflow::{
    WorkflowSchedulerBlockedInferenceTaskProjection,
    WorkflowSchedulerBlockedInferenceTaskProjectionReason,
    WorkflowSchedulerDependencyReadinessSource, WorkflowSchedulerInferenceTaskProjection,
    WorkflowSchedulerInferenceTaskProjections, WorkflowSchedulerReadyInferenceTaskProjection,
};

#[allow(dead_code)]
pub const CURRENT_DEPENDENCY_REQUIREMENTS_PROOF_MAX_DIAGNOSTICS: usize = 16;
pub const CURRENT_VALIDATION_SUMMARY_MAX_DIAGNOSTICS: usize = 16;

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

    pub(crate) async fn clear_graph_session(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> usize {
        let mut summaries = self.summaries.write().await;
        let before = summaries.len();
        summaries.retain(|key, _record| &key.graph_session_id != graph_session_id);
        before - summaries.len()
    }

    #[cfg(test)]
    pub(crate) async fn record_count_for_graph_session(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> usize {
        self.summaries
            .read()
            .await
            .keys()
            .filter(|key| &key.graph_session_id == graph_session_id)
            .count()
    }

    pub(crate) async fn current_validation_summary(
        &self,
        request: WorkflowGraphCurrentValidationSummaryStateRequest,
    ) -> WorkflowGraphCurrentValidationSummaryResponse {
        if request.requested_graph_revision != request.current_graph_revision {
            return current_validation_summary_response(
                request.graph_session_id,
                request.requested_graph_revision,
                request.current_graph_revision,
                None,
                WorkflowGraphCurrentValidationSummaryState::Stale,
                None,
                vec![summary_diagnostic(
                    InferenceDiagnosticCode::GraphRevisionMismatch,
                    "Graph validation summary was requested for a stale graph revision.",
                    Some("Refresh validation for the current graph revision before submitting."),
                )],
            );
        }

        let summaries = self.summaries.read().await;
        let key = CurrentInferenceValidationStateKey {
            graph_session_id: request.graph_session_id.clone(),
            graph_revision: request.current_graph_revision.clone(),
        };
        let Some(record) = summaries.get(&key) else {
            return current_validation_summary_response(
                request.graph_session_id,
                request.requested_graph_revision,
                request.current_graph_revision,
                None,
                WorkflowGraphCurrentValidationSummaryState::Missing,
                None,
                vec![summary_diagnostic(
                    InferenceDiagnosticCode::ValidationSummaryMissing,
                    "Inference validation has not completed for this graph revision.",
                    Some("Run graph validation before submitting this workflow."),
                )],
            );
        };

        current_validation_summary_response(
            request.graph_session_id,
            request.requested_graph_revision,
            request.current_graph_revision,
            Some(record.validation_session_id.clone()),
            current_validation_summary_state(record.summary.status),
            Some(record.summary.clone()),
            bounded_current_validation_diagnostics(record),
        )
    }

    #[allow(dead_code)]
    pub async fn record_dependency_requirements_proof(
        &self,
        request: CurrentDependencyRequirementsProofRequest,
    ) -> Result<CurrentDependencyRequirementsProof, CurrentDependencyRequirementsProofError> {
        if request.diagnostics.len() > CURRENT_DEPENDENCY_REQUIREMENTS_PROOF_MAX_DIAGNOSTICS {
            return Err(
                CurrentDependencyRequirementsProofError::TooManyDiagnostics {
                    count: request.diagnostics.len(),
                    max: CURRENT_DEPENDENCY_REQUIREMENTS_PROOF_MAX_DIAGNOSTICS,
                },
            );
        }
        for diagnostic in &request.diagnostics {
            diagnostic.validate().map_err(|error| {
                CurrentDependencyRequirementsProofError::InvalidDiagnostic {
                    message: error.to_string(),
                }
            })?;
        }

        let key = CurrentInferenceValidationStateKey {
            graph_session_id: request.graph_session_id,
            graph_revision: request.graph_revision.clone(),
        };
        let mut summaries = self.summaries.write().await;
        let record = summaries
            .get_mut(&key)
            .ok_or(CurrentDependencyRequirementsProofError::ValidationSummaryMissing)?;

        if record.validation_session_id != request.validation_session_id {
            return Err(CurrentDependencyRequirementsProofError::ValidationSessionMismatch);
        }

        if !record.summary.executable {
            return Err(
                CurrentDependencyRequirementsProofError::ValidationSummaryNotExecutable {
                    status: record.summary.status,
                },
            );
        }

        let node = record.nodes.get_mut(&request.inference_node_id).ok_or(
            CurrentDependencyRequirementsProofError::InferenceNodeMissing {
                node_id: request.inference_node_id.clone(),
            },
        )?;
        if !node.has_dependency_basis() {
            return Err(CurrentDependencyRequirementsProofError::IncompleteNodeState {
                node_id: node.node_id.clone(),
                message: "node record is missing descriptor, task, model, availability, or constraint data"
                    .to_string(),
            });
        }

        let proof = CurrentDependencyRequirementsProof {
            inference_node_id: node.node_id.clone(),
            graph_revision: request.graph_revision,
            validation_session_id: request.validation_session_id,
            descriptor_fingerprint: node.descriptor_fingerprint.clone(),
            pumas_model_ref: path_free_model_ref(&node.pumas_model_ref),
            task_kind: node.task_kind.clone(),
            runtime_constraint: node.runtime_constraint.clone(),
            device_constraint: node.device_constraint.clone(),
            trait_constraints: Vec::new(),
            dependency_requirements_id: request.dependency_requirements_id,
            selected_binding_ids: request.selected_binding_ids,
            dependency_override_fingerprint: request.dependency_override_fingerprint,
            status: request.status,
            diagnostics: request.diagnostics,
        };
        node.dependency_requirements_proof = Some(proof.clone());
        Ok(proof)
    }

    #[cfg(test)]
    pub(crate) async fn resolve_dependency_environment_action_intent(
        &self,
        request: DependencyEnvironmentActionIntentStateRequest,
    ) -> DependencyEnvironmentActionIntentResult {
        match self
            .resolve_dependency_environment_action_request(request)
            .await
        {
            DependencyEnvironmentActionIntentStateResolution::Blocked(result) => result,
            DependencyEnvironmentActionIntentStateResolution::RequestReady { intent, .. } => {
                request_ready_dependency_environment_action_result(&intent)
            }
        }
    }

    pub(crate) async fn resolve_dependency_environment_action_request(
        &self,
        request: DependencyEnvironmentActionIntentStateRequest,
    ) -> DependencyEnvironmentActionIntentStateResolution {
        let intent = request.intent.into_inner();
        if request.current_graph_revision != intent.graph_revision {
            return blocked_dependency_environment_action_resolution(
                &intent,
                InferenceDiagnosticCode::GraphRevisionMismatch,
                "Dependency environment action was requested for a stale graph revision.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }
        if let Some(diagnostic) = request.sidecar_diagnostic {
            return blocked_dependency_environment_action_resolution_from_diagnostic(
                &intent, diagnostic,
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
            } => {
                return blocked_dependency_environment_action_resolution(
                    &intent, code, message, hint,
                );
            }
        };

        let key = CurrentInferenceValidationStateKey {
            graph_session_id: intent.graph_session_id.clone(),
            graph_revision: intent.graph_revision.clone(),
        };
        let mut summaries = self.summaries.write().await;
        let Some(record) = summaries.get_mut(&key) else {
            return blocked_dependency_environment_action_resolution(
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
            return blocked_dependency_environment_action_resolution(
                &intent,
                InferenceDiagnosticCode::DescriptorStale,
                "Dependency environment action was requested for a stale validation session.",
                Some(
                    "Refresh graph validation for the current graph before resolving dependencies.",
                ),
            );
        }

        if !record.summary.executable {
            return blocked_dependency_environment_action_resolution(
                &intent,
                diagnostic_code_for_summary_status(record.summary.status),
                "Inference validation summary is not executable for this graph revision.",
                Some("Resolve blocking inference validation diagnostics before resolving dependencies."),
            );
        }

        if !record.nodes.contains_key(&inference_node_id) {
            return blocked_dependency_environment_action_resolution(
                &intent,
                InferenceDiagnosticCode::DependencySidecarDescriptorUnavailable,
                "Associated inference node is missing current descriptor validation state.",
                Some("Refresh descriptor validation before resolving dependency environments."),
            );
        }

        if let Some(node_record) = record.nodes.get_mut(&inference_node_id) {
            if !node_record.has_dependency_basis() {
                return blocked_dependency_environment_action_resolution(
                    &intent,
                    InferenceDiagnosticCode::DependencySidecarDescriptorInvalid,
                    "Associated inference validation state is incomplete for dependency derivation.",
                    Some("Refresh descriptor validation before resolving dependency environments."),
                );
            }
            match node_record.current_dependency_requirements_proof(
                &intent.graph_revision,
                &record.validation_session_id,
            ) {
                Ok(proof) => {
                    let current_dependency_requirements_id =
                        Some(proof.dependency_requirements_id.clone());
                    return match node_record.derive_current_dependency_environment_request(
                        intent.action,
                        &intent.graph_revision,
                        &record.validation_session_id,
                        request.selected_binding_ids,
                        request.dependency_override_patches,
                        current_dependency_requirements_id,
                    ) {
                        Ok(environment_request) => {
                            request_ready_dependency_environment_action_resolution(
                                intent,
                                environment_request,
                            )
                        }
                        Err(error) => blocked_dependency_environment_action_resolution(
                            &intent,
                            error.diagnostic_code(),
                            error.message(),
                            error.hint().as_deref(),
                        ),
                    };
                }
                Err(CurrentDependencyRequirementsProofStateError::Missing)
                    if matches!(intent.action, DependencyEnvironmentAction::Resolve) =>
                {
                    return match node_record.derive_current_dependency_environment_request(
                        intent.action,
                        &intent.graph_revision,
                        &record.validation_session_id,
                        request.selected_binding_ids,
                        request.dependency_override_patches,
                        None,
                    ) {
                        Ok(environment_request) => {
                            request_ready_dependency_environment_action_resolution(
                                intent,
                                environment_request,
                            )
                        }
                        Err(error) => blocked_dependency_environment_action_resolution(
                            &intent,
                            error.diagnostic_code(),
                            error.message(),
                            error.hint().as_deref(),
                        ),
                    };
                }
                Err(CurrentDependencyRequirementsProofStateError::Missing) => {
                    return blocked_dependency_environment_action_resolution(
                        &intent,
                        InferenceDiagnosticCode::DependencyRequirementsMissing,
                        "Dependency environment request derivation requires dependency requirements for this graph revision.",
                        Some("Resolve dependency requirements from the current validation state before checking or installing environments."),
                    );
                }
                Err(CurrentDependencyRequirementsProofStateError::Stale) => {
                    return blocked_dependency_environment_action_resolution(
                        &intent,
                        InferenceDiagnosticCode::DescriptorStale,
                        "Dependency requirements proof is stale for this graph revision or validation session.",
                        Some("Refresh descriptor validation and dependency requirements before resolving environments."),
                    );
                }
                Err(CurrentDependencyRequirementsProofStateError::Unavailable) => {
                    return blocked_dependency_environment_action_resolution(
                        &intent,
                        InferenceDiagnosticCode::DependencySidecarDescriptorUnavailable,
                        "Dependency requirements proof is unavailable for this graph revision.",
                        Some("Resolve dependency requirements before checking or installing environments."),
                    );
                }
                Err(CurrentDependencyRequirementsProofStateError::Invalid) => {
                    return blocked_dependency_environment_action_resolution(
                        &intent,
                        InferenceDiagnosticCode::DependencySidecarDescriptorInvalid,
                        "Dependency requirements proof is invalid for dependency request derivation.",
                        Some("Refresh descriptor validation and dependency requirements before resolving environments."),
                    );
                }
            }
        }

        blocked_dependency_environment_action_resolution(
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
            projections.push(node.scheduler_inference_task_projection(
                &key.graph_revision,
                &record.validation_session_id,
            )?);
        }
        WorkflowSchedulerInferenceTaskProjections::from_records(projections).map_err(|error| {
            CurrentInferenceSchedulerProjectionError::InvalidProjection {
                message: error.to_string(),
            }
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn current_executable_validation_snapshot_source(
        &self,
        request: CurrentExecutableValidationSnapshotSourceRequest,
    ) -> Result<
        CurrentExecutableValidationSnapshotSource,
        CurrentExecutableValidationSnapshotSourceError,
    > {
        let summaries = self.summaries.read().await;
        let key = CurrentInferenceValidationStateKey {
            graph_session_id: request.graph_session_id.clone(),
            graph_revision: request.graph_revision.clone(),
        };
        let record = summaries
            .get(&key)
            .ok_or(CurrentExecutableValidationSnapshotSourceError::ValidationSummaryMissing)?;

        if request
            .validation_session_id
            .as_ref()
            .is_some_and(|validation_session_id| {
                validation_session_id != &record.validation_session_id
            })
        {
            return Err(CurrentExecutableValidationSnapshotSourceError::ValidationSessionMismatch);
        }

        if !record.summary.executable {
            return Err(
                CurrentExecutableValidationSnapshotSourceError::ValidationSummaryNotExecutable {
                    status: record.summary.status,
                },
            );
        }

        let mut nodes = Vec::with_capacity(record.nodes.len());
        for node in record.nodes.values() {
            if !node.has_dependency_basis() {
                return Err(
                    CurrentExecutableValidationSnapshotSourceError::IncompleteNodeState {
                        node_id: node.node_id.clone(),
                        message: "node record is missing descriptor, task, model, availability, or constraint data"
                            .to_string(),
                    },
                );
            }
            let dependency_requirements_proof = node
                .current_dependency_requirements_proof(
                    &request.graph_revision,
                    &record.validation_session_id,
                )
                .map_err(|error| executable_snapshot_source_proof_error(error, &node.node_id))?
                .clone();
            nodes.push(CurrentExecutableValidationSnapshotNodeSource {
                projection: node.projection.clone(),
                dependency_requirements_proof,
            });
        }

        Ok(CurrentExecutableValidationSnapshotSource {
            graph_session_id: request.graph_session_id,
            graph_revision: request.graph_revision,
            validation_session_id: record.validation_session_id.clone(),
            validation_summary: record.summary.clone(),
            nodes,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowGraphCurrentValidationSummaryStateRequest {
    pub graph_session_id: WorkflowGraphSessionId,
    pub requested_graph_revision: WorkflowGraphRevision,
    pub current_graph_revision: WorkflowGraphRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphCurrentValidationSummaryRequest {
    pub graph_session_id: String,
    pub graph_revision: WorkflowGraphRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphCurrentValidationRefreshRequest {
    pub graph_session_id: String,
    pub graph_revision: WorkflowGraphRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphCurrentValidationRefreshResponse {
    pub summary: WorkflowGraphCurrentValidationSummaryResponse,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_projections: Vec<InferenceInterfaceNodeProjectionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphCurrentValidationSummaryResponse {
    pub graph_session_id: WorkflowGraphSessionId,
    pub requested_graph_revision: WorkflowGraphRevision,
    pub current_graph_revision: WorkflowGraphRevision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_session_id: Option<DraftGraphValidationSessionId>,
    pub state: WorkflowGraphCurrentValidationSummaryState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<DraftGraphValidationSummary>,
    pub submit_gate: WorkflowGraphValidationSubmitGate,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowGraphCurrentValidationSummaryState {
    Current,
    Pending,
    Missing,
    Stale,
    Unavailable,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphValidationSubmitGate {
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<WorkflowGraphValidationSubmitGateReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowGraphValidationSubmitGateReason {
    ValidationSummaryMissing,
    GraphRevisionStale,
    ValidationPending,
    ValidationStale,
    DescriptorUnresolved,
    DescriptorUnavailable,
    BlockingDiagnostics,
    MissingRequiredInput,
    InvalidPortBinding,
    InvalidRuntimeConstraint,
    InvalidDeviceConstraint,
    DriftRequiresReview,
    ValidationNotExecutable,
}

fn current_validation_summary_response(
    graph_session_id: WorkflowGraphSessionId,
    requested_graph_revision: WorkflowGraphRevision,
    current_graph_revision: WorkflowGraphRevision,
    validation_session_id: Option<DraftGraphValidationSessionId>,
    state: WorkflowGraphCurrentValidationSummaryState,
    summary: Option<DraftGraphValidationSummary>,
    diagnostics: Vec<InferenceInterfaceDiagnostic>,
) -> WorkflowGraphCurrentValidationSummaryResponse {
    let submit_gate = submit_gate_for_current_validation_summary(state, summary.as_ref());
    WorkflowGraphCurrentValidationSummaryResponse {
        graph_session_id,
        requested_graph_revision,
        current_graph_revision,
        validation_session_id,
        state,
        summary,
        submit_gate,
        diagnostics,
    }
}

fn current_validation_summary_state(
    status: DraftGraphValidationStatus,
) -> WorkflowGraphCurrentValidationSummaryState {
    match status {
        DraftGraphValidationStatus::Executable => {
            WorkflowGraphCurrentValidationSummaryState::Current
        }
        DraftGraphValidationStatus::Pending => WorkflowGraphCurrentValidationSummaryState::Pending,
        DraftGraphValidationStatus::Stale => WorkflowGraphCurrentValidationSummaryState::Stale,
        DraftGraphValidationStatus::Unresolved | DraftGraphValidationStatus::Unavailable => {
            WorkflowGraphCurrentValidationSummaryState::Unavailable
        }
        DraftGraphValidationStatus::Blocked => WorkflowGraphCurrentValidationSummaryState::Invalid,
        _ => WorkflowGraphCurrentValidationSummaryState::Invalid,
    }
}

fn submit_gate_for_current_validation_summary(
    state: WorkflowGraphCurrentValidationSummaryState,
    summary: Option<&DraftGraphValidationSummary>,
) -> WorkflowGraphValidationSubmitGate {
    if summary.is_some_and(|summary| summary.executable) {
        return WorkflowGraphValidationSubmitGate {
            allowed: true,
            reason_code: None,
            message: None,
        };
    }

    let reason_code = submit_gate_reason(state, summary);
    WorkflowGraphValidationSubmitGate {
        allowed: false,
        reason_code: Some(reason_code),
        message: Some(submit_gate_message(reason_code).to_string()),
    }
}

fn submit_gate_reason(
    state: WorkflowGraphCurrentValidationSummaryState,
    summary: Option<&DraftGraphValidationSummary>,
) -> WorkflowGraphValidationSubmitGateReason {
    match state {
        WorkflowGraphCurrentValidationSummaryState::Missing => {
            return WorkflowGraphValidationSubmitGateReason::ValidationSummaryMissing;
        }
        WorkflowGraphCurrentValidationSummaryState::Stale => {
            return WorkflowGraphValidationSubmitGateReason::GraphRevisionStale;
        }
        WorkflowGraphCurrentValidationSummaryState::Pending => {
            return WorkflowGraphValidationSubmitGateReason::ValidationPending;
        }
        WorkflowGraphCurrentValidationSummaryState::Unavailable => {
            return WorkflowGraphValidationSubmitGateReason::DescriptorUnavailable;
        }
        WorkflowGraphCurrentValidationSummaryState::Invalid
        | WorkflowGraphCurrentValidationSummaryState::Current => {}
    }

    summary
        .and_then(|summary| summary.enqueue_disabled_reasons.first().copied())
        .map(submit_gate_reason_from_disabled_reason)
        .unwrap_or(WorkflowGraphValidationSubmitGateReason::ValidationNotExecutable)
}

fn submit_gate_reason_from_disabled_reason(
    reason: pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason,
) -> WorkflowGraphValidationSubmitGateReason {
    match reason {
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::ValidationPending => {
            WorkflowGraphValidationSubmitGateReason::ValidationPending
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::ValidationStale => {
            WorkflowGraphValidationSubmitGateReason::ValidationStale
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::DescriptorUnresolved => {
            WorkflowGraphValidationSubmitGateReason::DescriptorUnresolved
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::DescriptorUnavailable => {
            WorkflowGraphValidationSubmitGateReason::DescriptorUnavailable
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::BlockingDiagnostics => {
            WorkflowGraphValidationSubmitGateReason::BlockingDiagnostics
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::MissingRequiredInput => {
            WorkflowGraphValidationSubmitGateReason::MissingRequiredInput
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::InvalidPortBinding => {
            WorkflowGraphValidationSubmitGateReason::InvalidPortBinding
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::InvalidRuntimeConstraint => {
            WorkflowGraphValidationSubmitGateReason::InvalidRuntimeConstraint
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::InvalidDeviceConstraint => {
            WorkflowGraphValidationSubmitGateReason::InvalidDeviceConstraint
        }
        pantograph_inference_interface_contracts::DraftGraphEnqueueDisabledReason::DriftRequiresReview => {
            WorkflowGraphValidationSubmitGateReason::DriftRequiresReview
        }
        _ => WorkflowGraphValidationSubmitGateReason::ValidationNotExecutable,
    }
}

fn submit_gate_message(reason: WorkflowGraphValidationSubmitGateReason) -> &'static str {
    match reason {
        WorkflowGraphValidationSubmitGateReason::ValidationSummaryMissing => {
            "Inference validation has not completed for this graph revision"
        }
        WorkflowGraphValidationSubmitGateReason::GraphRevisionStale => {
            "Inference validation is stale for the current graph"
        }
        WorkflowGraphValidationSubmitGateReason::ValidationPending => {
            "Inference validation is still pending"
        }
        WorkflowGraphValidationSubmitGateReason::ValidationStale => {
            "Inference validation must be refreshed before submitting"
        }
        WorkflowGraphValidationSubmitGateReason::DescriptorUnresolved => {
            "Inference descriptor could not be resolved"
        }
        WorkflowGraphValidationSubmitGateReason::DescriptorUnavailable => {
            "Inference descriptor is unavailable"
        }
        WorkflowGraphValidationSubmitGateReason::BlockingDiagnostics => {
            "Inference validation has blocking diagnostics"
        }
        WorkflowGraphValidationSubmitGateReason::MissingRequiredInput => {
            "Inference validation is missing required inputs"
        }
        WorkflowGraphValidationSubmitGateReason::InvalidPortBinding => {
            "Inference validation has invalid port bindings"
        }
        WorkflowGraphValidationSubmitGateReason::InvalidRuntimeConstraint => {
            "Inference validation has an invalid runtime constraint"
        }
        WorkflowGraphValidationSubmitGateReason::InvalidDeviceConstraint => {
            "Inference validation has an invalid device constraint"
        }
        WorkflowGraphValidationSubmitGateReason::DriftRequiresReview => {
            "Inference interface drift requires review"
        }
        WorkflowGraphValidationSubmitGateReason::ValidationNotExecutable => {
            "Inference validation is not executable"
        }
    }
}

fn bounded_current_validation_diagnostics(
    record: &CurrentInferenceValidationStateRecord,
) -> Vec<InferenceInterfaceDiagnostic> {
    record
        .nodes
        .values()
        .flat_map(|node| node.projection.descriptor.diagnostics.iter().cloned())
        .take(CURRENT_VALIDATION_SUMMARY_MAX_DIAGNOSTICS)
        .collect()
}

fn summary_diagnostic(
    code: InferenceDiagnosticCode,
    message: &'static str,
    hint: Option<&'static str>,
) -> InferenceInterfaceDiagnostic {
    InferenceInterfaceDiagnostic {
        severity: InferenceDiagnosticSeverity::Error,
        code,
        message: message.to_string(),
        hint: hint.map(str::to_string),
        port_id: None,
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
    pub selected_binding_ids: Vec<DependencyBindingId>,
    pub dependency_override_patches: Vec<DependencyOverridePatchV1>,
    pub sidecar_diagnostic: Option<InferenceInterfaceDiagnostic>,
}

#[derive(Debug, Clone)]
pub(crate) enum DependencyEnvironmentActionIntentStateResolution {
    Blocked(DependencyEnvironmentActionIntentResult),
    RequestReady {
        intent: DependencyEnvironmentActionIntent,
        environment_request: ValidatedDependencyEnvironmentRequest,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct CurrentInferenceSchedulerProjectionRequest {
    pub graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: Option<DraftGraphValidationSessionId>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CurrentDependencyRequirementsProofRequest {
    pub graph_session_id: pantograph_inference_interface_contracts::WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub inference_node_id: WorkflowNodeId,
    pub dependency_requirements_id: DependencyRequirementsId,
    pub selected_binding_ids: Vec<DependencyBindingId>,
    pub dependency_override_fingerprint: DependencyOverrideFingerprint,
    pub status: CurrentDependencyRequirementsProofStatus,
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CurrentDependencyRequirementsProof {
    pub inference_node_id: WorkflowNodeId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub pumas_model_ref: PumasModelRef,
    pub task_kind: InferenceTaskKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_constraint: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_constraint: Option<DeviceIntentId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_constraints: Vec<SchedulerTraitSetting>,
    pub dependency_requirements_id: DependencyRequirementsId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
    pub dependency_override_fingerprint: DependencyOverrideFingerprint,
    pub status: CurrentDependencyRequirementsProofStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrentDependencyRequirementsProofStatus {
    Current,
    Stale,
    Unavailable,
    Invalid,
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum CurrentDependencyRequirementsProofError {
    #[error("inference validation summary is missing for the current graph revision")]
    ValidationSummaryMissing,
    #[error("inference validation session does not match the current graph validation state")]
    ValidationSessionMismatch,
    #[error("inference validation summary is not executable: {status:?}")]
    ValidationSummaryNotExecutable { status: DraftGraphValidationStatus },
    #[error("associated inference node is missing from current validation state: {node_id}")]
    InferenceNodeMissing { node_id: WorkflowNodeId },
    #[error("inference validation node state is incomplete for node {node_id}: {message}")]
    IncompleteNodeState {
        node_id: WorkflowNodeId,
        message: String,
    },
    #[error("dependency requirements proof has too many diagnostics: {count} > {max}")]
    TooManyDiagnostics { count: usize, max: usize },
    #[error("dependency requirements proof diagnostic is invalid: {message}")]
    InvalidDiagnostic { message: String },
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
    #[allow(dead_code)]
    pub projection: InferenceInterfaceNodeProjectionRecord,
    pub dependency_requirements_proof: Option<CurrentDependencyRequirementsProof>,
}

impl From<InferenceInterfaceNodeProjectionRecord> for CurrentInferenceValidationNodeRecord {
    fn from(record: InferenceInterfaceNodeProjectionRecord) -> Self {
        let node_id = record.node_id.clone();
        let descriptor_fingerprint = record.descriptor.descriptor_fingerprint.clone();
        let task_kind = record.descriptor.task_kind.clone();
        let availability_status = record.descriptor.availability.status;
        let validation_status = record.validation_summary.status;
        let pumas_model_ref = record.descriptor.model_ref.clone();
        let runtime_constraint = record.runtime_constraint.clone();
        let device_constraint = record.device_constraint.clone();
        Self {
            node_id,
            descriptor_fingerprint,
            task_kind,
            availability_status,
            validation_status,
            pumas_model_ref,
            runtime_constraint,
            device_constraint,
            projection: record,
            dependency_requirements_proof: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentDependencyRequirementsProofStateError {
    Missing,
    Stale,
    Unavailable,
    Invalid,
}

#[allow(dead_code)]
fn executable_snapshot_source_proof_error(
    error: CurrentDependencyRequirementsProofStateError,
    node_id: &WorkflowNodeId,
) -> CurrentExecutableValidationSnapshotSourceError {
    match error {
        CurrentDependencyRequirementsProofStateError::Missing => {
            CurrentExecutableValidationSnapshotSourceError::DependencyProofMissing {
                node_id: node_id.clone(),
            }
        }
        CurrentDependencyRequirementsProofStateError::Stale => {
            CurrentExecutableValidationSnapshotSourceError::DependencyProofStale {
                node_id: node_id.clone(),
            }
        }
        CurrentDependencyRequirementsProofStateError::Unavailable => {
            CurrentExecutableValidationSnapshotSourceError::DependencyProofUnavailable {
                node_id: node_id.clone(),
            }
        }
        CurrentDependencyRequirementsProofStateError::Invalid => {
            CurrentExecutableValidationSnapshotSourceError::DependencyProofInvalid {
                node_id: node_id.clone(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentDependencyEnvironmentRequestError {
    RequirementsStale,
    RequirementsUnavailable,
    RequirementsInvalid,
    Contract(String),
}

impl CurrentDependencyEnvironmentRequestError {
    fn diagnostic_code(&self) -> InferenceDiagnosticCode {
        match self {
            Self::RequirementsStale => InferenceDiagnosticCode::DescriptorStale,
            Self::RequirementsUnavailable => {
                InferenceDiagnosticCode::DependencySidecarDescriptorUnavailable
            }
            Self::RequirementsInvalid | Self::Contract(_) => {
                InferenceDiagnosticCode::DependencySidecarDescriptorInvalid
            }
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::RequirementsStale => {
                "Dependency requirements proof is stale for the current sidecar choices."
            }
            Self::RequirementsUnavailable => {
                "Dependency requirements proof is unavailable for dependency request derivation."
            }
            Self::RequirementsInvalid | Self::Contract(_) => {
                "Dependency environment request could not be derived from current validation state."
            }
        }
    }

    fn hint(&self) -> Option<String> {
        match self {
            Self::RequirementsStale => Some(
                "Resolve dependency requirements again after changing sidecar bindings or overrides."
                    .to_string(),
            ),
            Self::RequirementsUnavailable => Some(
                "Resolve dependency requirements before checking or installing environments."
                    .to_string(),
            ),
            Self::RequirementsInvalid => Some(
                "Refresh descriptor validation and dependency requirements before resolving environments."
                    .to_string(),
            ),
            Self::Contract(message) => Some(message.clone()),
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
        graph_revision: &WorkflowGraphRevision,
        validation_session_id: &DraftGraphValidationSessionId,
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
            let dependency_requirements_proof = self
                .current_dependency_requirements_proof(graph_revision, validation_session_id)
                .map_err(
                    |error| CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                        node_id: self.node_id.clone(),
                        message: format!("dependency requirements proof is not current: {error:?}"),
                    },
                )?;
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
                    dependency_readiness_source: WorkflowSchedulerDependencyReadinessSource {
                        graph_revision: DependencyReadinessGraphRevision::parse(
                            graph_revision.as_str(),
                        )
                        .map_err(|error| {
                            CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                                node_id: self.node_id.clone(),
                                message: format!("graph revision is invalid: {error}"),
                            }
                        })?,
                        validation_session_id: Some(
                            DependencyReadinessValidationSessionId::parse(
                                validation_session_id.as_str(),
                            )
                            .map_err(|error| {
                                CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                                    node_id: self.node_id.clone(),
                                    message: format!("validation session id is invalid: {error}"),
                                }
                            })?,
                        ),
                        validation_snapshot_id: None,
                        descriptor_fingerprint: DependencyReadinessDescriptorFingerprint::parse(
                            self.descriptor_fingerprint.as_str(),
                        )
                        .map_err(|error| {
                            CurrentInferenceSchedulerProjectionError::IncompleteNodeState {
                                node_id: self.node_id.clone(),
                                message: format!("descriptor fingerprint is invalid: {error}"),
                            }
                        })?,
                        dependency_requirements_id: dependency_requirements_proof
                            .dependency_requirements_id
                            .clone(),
                        selected_binding_ids: dependency_requirements_proof
                            .selected_binding_ids
                            .clone(),
                        dependency_override_fingerprint: dependency_requirements_proof
                            .dependency_override_fingerprint
                            .clone(),
                    },
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

    fn current_dependency_requirements_proof_from_producer(
        &self,
        graph_revision: &WorkflowGraphRevision,
        validation_session_id: &DraftGraphValidationSessionId,
        producer_proof: DependencyRequirementsProof,
    ) -> CurrentDependencyRequirementsProof {
        CurrentDependencyRequirementsProof {
            inference_node_id: self.node_id.clone(),
            graph_revision: graph_revision.clone(),
            validation_session_id: validation_session_id.clone(),
            descriptor_fingerprint: self.descriptor_fingerprint.clone(),
            pumas_model_ref: path_free_model_ref(&self.pumas_model_ref),
            task_kind: self.task_kind.clone(),
            runtime_constraint: self.runtime_constraint.clone(),
            device_constraint: self.device_constraint.clone(),
            trait_constraints: Vec::new(),
            dependency_requirements_id: producer_proof.dependency_requirements_id,
            selected_binding_ids: producer_proof.identity_key.selected_binding_ids,
            dependency_override_fingerprint: producer_proof.dependency_override_fingerprint,
            status: current_status_from_producer_status(producer_proof.status),
            diagnostics: producer_proof.diagnostics,
        }
    }

    fn dependency_planning_request(
        &self,
        selected_binding_ids: Vec<DependencyBindingId>,
        dependency_override_patches: Vec<DependencyOverridePatchV1>,
    ) -> Result<
        DependencyPlanningRequest,
        pantograph_dependency_planning::DependencyPlanningContractError,
    > {
        Ok(DependencyPlanningRequest {
            model_ref: path_free_model_ref(&self.pumas_model_ref),
            task_id: DependencyTaskId::parse(self.task_kind.as_str())?,
            task_type: None,
            expected_artifact_kind: None,
            scheduler_intent: SchedulerIntent {
                requested_runtime_id: self.runtime_constraint.clone(),
                requested_device_id: self.device_constraint.clone(),
            },
            platform_context: Some(DependencyPlanningPlatformContext::from_os_arch(
                std::env::consts::OS,
                std::env::consts::ARCH,
            )?),
            selected_binding_ids,
            dependency_override_patches,
            trait_intents: Vec::new(),
            caller_context: DependencyPlanningCallerContext {
                source_node_type: Some(DependencyNodeTypeId::parse("llm-inference")?),
                node_id: Some(self.node_id.as_str().to_string()),
                ..Default::default()
            },
        })
    }

    fn derive_current_dependency_environment_request(
        &mut self,
        action: DependencyEnvironmentAction,
        graph_revision: &WorkflowGraphRevision,
        validation_session_id: &DraftGraphValidationSessionId,
        selected_binding_ids: Vec<DependencyBindingId>,
        dependency_override_patches: Vec<DependencyOverridePatchV1>,
        current_dependency_requirements_id: Option<DependencyRequirementsId>,
    ) -> Result<ValidatedDependencyEnvironmentRequest, CurrentDependencyEnvironmentRequestError>
    {
        let planning_request = self
            .dependency_planning_request(selected_binding_ids, dependency_override_patches)
            .map_err(|error| {
                CurrentDependencyEnvironmentRequestError::Contract(error.to_string())
            })?;
        let validated_request = ValidatedDependencyPlanningRequest::try_from(
            planning_request.clone(),
        )
        .map_err(|error| CurrentDependencyEnvironmentRequestError::Contract(error.to_string()))?;
        let producer_proof = produce_dependency_requirements_proof(&validated_request, None)
            .map_err(|error| {
                CurrentDependencyEnvironmentRequestError::Contract(error.to_string())
            })?;
        let proof_status = current_status_from_producer_status(producer_proof.status);
        match proof_status {
            CurrentDependencyRequirementsProofStatus::Current => {}
            CurrentDependencyRequirementsProofStatus::Stale => {
                return Err(CurrentDependencyEnvironmentRequestError::RequirementsStale);
            }
            CurrentDependencyRequirementsProofStatus::Unavailable => {
                return Err(CurrentDependencyEnvironmentRequestError::RequirementsUnavailable);
            }
            CurrentDependencyRequirementsProofStatus::Invalid => {
                return Err(CurrentDependencyEnvironmentRequestError::RequirementsInvalid);
            }
        }

        let dependency_requirements_id = producer_proof.dependency_requirements_id.clone();
        if matches!(action, DependencyEnvironmentAction::Resolve) {
            let proof = self.current_dependency_requirements_proof_from_producer(
                graph_revision,
                validation_session_id,
                producer_proof,
            );
            self.dependency_requirements_proof = Some(proof);
        } else if current_dependency_requirements_id
            .map(|current_id| current_id != dependency_requirements_id)
            .unwrap_or(true)
        {
            return Err(CurrentDependencyEnvironmentRequestError::RequirementsStale);
        }

        let identity_key = DependencyPlanningIdentityKey::from_planning_request(&planning_request)
            .map_err(|error| {
                CurrentDependencyEnvironmentRequestError::Contract(error.to_string())
            })?;
        let request = DependencyEnvironmentRequest {
            contract_version: 1,
            action,
            identity_key,
            planning_request,
            dependency_requirements_id: Some(dependency_requirements_id),
            environment_ref: None,
        };
        ValidatedDependencyEnvironmentRequest::try_from(request)
            .map_err(|error| CurrentDependencyEnvironmentRequestError::Contract(error.to_string()))
    }

    fn current_dependency_requirements_proof(
        &self,
        graph_revision: &WorkflowGraphRevision,
        validation_session_id: &DraftGraphValidationSessionId,
    ) -> Result<&CurrentDependencyRequirementsProof, CurrentDependencyRequirementsProofStateError>
    {
        let Some(proof) = &self.dependency_requirements_proof else {
            return Err(CurrentDependencyRequirementsProofStateError::Missing);
        };
        if proof.graph_revision != *graph_revision
            || proof.validation_session_id != *validation_session_id
            || proof.inference_node_id != self.node_id
            || proof.descriptor_fingerprint != self.descriptor_fingerprint
            || proof.task_kind != self.task_kind
            || proof.pumas_model_ref != path_free_model_ref(&self.pumas_model_ref)
            || proof.runtime_constraint != self.runtime_constraint
            || proof.device_constraint != self.device_constraint
        {
            return Err(CurrentDependencyRequirementsProofStateError::Stale);
        }
        match proof.status {
            CurrentDependencyRequirementsProofStatus::Current => Ok(proof),
            CurrentDependencyRequirementsProofStatus::Stale => {
                Err(CurrentDependencyRequirementsProofStateError::Stale)
            }
            CurrentDependencyRequirementsProofStatus::Unavailable => {
                Err(CurrentDependencyRequirementsProofStateError::Unavailable)
            }
            CurrentDependencyRequirementsProofStatus::Invalid => {
                Err(CurrentDependencyRequirementsProofStateError::Invalid)
            }
        }
    }
}

fn current_status_from_producer_status(
    status: DependencyRequirementsProofStatus,
) -> CurrentDependencyRequirementsProofStatus {
    match status {
        DependencyRequirementsProofStatus::Current => {
            CurrentDependencyRequirementsProofStatus::Current
        }
        DependencyRequirementsProofStatus::Invalid => {
            CurrentDependencyRequirementsProofStatus::Invalid
        }
        DependencyRequirementsProofStatus::Stale => CurrentDependencyRequirementsProofStatus::Stale,
        DependencyRequirementsProofStatus::Unavailable
        | DependencyRequirementsProofStatus::Ambiguous
        | DependencyRequirementsProofStatus::NeedsDetail
        | DependencyRequirementsProofStatus::Missing
        | DependencyRequirementsProofStatus::NotImplemented => {
            CurrentDependencyRequirementsProofStatus::Unavailable
        }
        _ => CurrentDependencyRequirementsProofStatus::Unavailable,
    }
}

fn path_free_model_ref(model_ref: &PumasModelRef) -> PumasModelRef {
    let mut model_ref = model_ref.clone();
    model_ref.selected_artifact_path = None;
    model_ref
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

fn blocked_dependency_environment_action_result_from_diagnostic(
    intent: &DependencyEnvironmentActionIntent,
    diagnostic: InferenceInterfaceDiagnostic,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: vec![diagnostic],
    }
}

fn blocked_dependency_environment_action_resolution(
    intent: &DependencyEnvironmentActionIntent,
    code: InferenceDiagnosticCode,
    message: &str,
    hint: Option<&str>,
) -> DependencyEnvironmentActionIntentStateResolution {
    DependencyEnvironmentActionIntentStateResolution::Blocked(
        blocked_dependency_environment_action_result(intent, code, message, hint),
    )
}

fn blocked_dependency_environment_action_resolution_from_diagnostic(
    intent: &DependencyEnvironmentActionIntent,
    diagnostic: InferenceInterfaceDiagnostic,
) -> DependencyEnvironmentActionIntentStateResolution {
    DependencyEnvironmentActionIntentStateResolution::Blocked(
        blocked_dependency_environment_action_result_from_diagnostic(intent, diagnostic),
    )
}

fn request_ready_dependency_environment_action_resolution(
    intent: DependencyEnvironmentActionIntent,
    environment_request: ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentActionIntentStateResolution {
    DependencyEnvironmentActionIntentStateResolution::RequestReady {
        intent,
        environment_request,
    }
}

#[cfg(test)]
fn request_ready_dependency_environment_action_result(
    intent: &DependencyEnvironmentActionIntent,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::RequestReady,
        diagnostics: Vec::new(),
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
    async fn clear_graph_session_removes_only_matching_validation_state() {
        let store = CurrentInferenceValidationStateStore::new();
        let target_session_id: WorkflowGraphSessionId =
            "graph-session-1".parse().expect("valid graph session id");
        let other_session_id: WorkflowGraphSessionId =
            "graph-session-2".parse().expect("valid graph session id");
        store
            .record_validation_session(
                target_session_id.clone(),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
            )
            .await
            .expect("valid validation session");
        store
            .record_validation_session(
                other_session_id.clone(),
                validation_session(
                    "bbbbbbbbbbbbbbbb",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
            )
            .await
            .expect("valid validation session");

        assert_eq!(
            store
                .record_count_for_graph_session(&target_session_id)
                .await,
            1
        );

        let removed = store.clear_graph_session(&target_session_id).await;

        assert_eq!(removed, 1);
        assert_eq!(
            store
                .record_count_for_graph_session(&target_session_id)
                .await,
            0
        );
        assert_eq!(
            store
                .record_count_for_graph_session(&other_session_id)
                .await,
            1
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
    async fn resolve_action_intent_state_produces_current_dependency_requirements_proof() {
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
            result.status,
            DependencyEnvironmentActionIntentStatus::RequestReady
        );
    }

    #[tokio::test]
    async fn dependency_requirements_proof_recording_creates_path_free_bounded_proof() {
        let store = CurrentInferenceValidationStateStore::new();
        let mut projection = node_projection(DraftGraphValidationStatus::Executable);
        projection.descriptor.model_ref.selected_artifact_path =
            Some("/tmp/legacy-selected-artifact".to_string());
        store
            .record_validation_publication(
                "graph-session-1".parse().expect("valid graph session id"),
                validation_session(
                    "aaaaaaaaaaaaaaaa",
                    DraftGraphValidationStatus::Executable,
                    true,
                ),
                vec![projection],
            )
            .await
            .expect("valid validation session");

        let proof = store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Current,
            ))
            .await
            .expect("record proof");

        assert_eq!(proof.inference_node_id.as_str(), "infer");
        assert_eq!(
            proof.dependency_requirements_id.as_str(),
            "requirements.image_generation.cuda0"
        );
        assert_eq!(proof.selected_binding_ids.len(), 1);
        assert_eq!(proof.selected_binding_ids[0].as_str(), "torch-diffusers");
        assert_eq!(
            proof.dependency_override_fingerprint.as_str(),
            "override.none"
        );
        assert_eq!(
            proof.status,
            CurrentDependencyRequirementsProofStatus::Current
        );
        assert_eq!(proof.pumas_model_ref.selected_artifact_path, None);
        let encoded = serde_json::to_string(&proof).expect("encode proof");
        assert!(!encoded.contains("selected_artifact_path"));
        assert!(!encoded.contains("model_path"));
        assert!(!encoded.contains("package_facts"));
        assert!(!encoded.contains("runtime_load_target"));
        assert!(!encoded.contains("/tmp/legacy-selected-artifact"));
    }

    #[tokio::test]
    async fn action_intent_state_returns_ready_with_current_dependency_requirements_proof() {
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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Current,
            ))
            .await
            .expect("record proof");

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
            result.status,
            DependencyEnvironmentActionIntentStatus::RequestReady
        );
        assert!(result.diagnostics.is_empty());
    }

    #[tokio::test]
    async fn check_and_install_actions_fail_closed_without_current_dependency_requirements_proof() {
        for action in [
            DependencyEnvironmentAction::Check,
            DependencyEnvironmentAction::Install,
        ] {
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
                .resolve_dependency_environment_action_intent(
                    state_request_with_validation_session_and_action(
                        "graph-session-1",
                        "aaaaaaaaaaaaaaaa",
                        "aaaaaaaaaaaaaaaa",
                        "validation.session.1",
                        "dependency-node-1",
                        true,
                        action,
                    ),
                )
                .await;

            assert_eq!(
                result.diagnostics[0].code,
                InferenceDiagnosticCode::DependencyRequirementsMissing
            );
        }
    }

    #[tokio::test]
    async fn check_action_rejects_sidecar_choices_that_do_not_match_current_proof() {
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

        let mut resolve_request = state_request_with_validation_session(
            "graph-session-1",
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaa",
            "validation.session.1",
            "dependency-node-1",
            true,
        );
        resolve_request.selected_binding_ids =
            vec![DependencyBindingId::parse("binding-a").expect("valid binding id")];
        let resolve_result = store
            .resolve_dependency_environment_action_intent(resolve_request)
            .await;
        assert_eq!(
            resolve_result.status,
            DependencyEnvironmentActionIntentStatus::RequestReady
        );

        let mut check_request = state_request_with_validation_session_and_action(
            "graph-session-1",
            "aaaaaaaaaaaaaaaa",
            "aaaaaaaaaaaaaaaa",
            "validation.session.1",
            "dependency-node-1",
            true,
            DependencyEnvironmentAction::Check,
        );
        check_request.selected_binding_ids =
            vec![DependencyBindingId::parse("binding-b").expect("valid binding id")];
        let check_result = store
            .resolve_dependency_environment_action_intent(check_request)
            .await;

        assert_eq!(
            check_result.status,
            DependencyEnvironmentActionIntentStatus::Blocked
        );
        assert_eq!(
            check_result.diagnostics[0].code,
            InferenceDiagnosticCode::DescriptorStale
        );
    }

    #[tokio::test]
    async fn action_intent_state_rejects_stale_dependency_requirements_proof() {
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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Stale,
            ))
            .await
            .expect("record proof");

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
            InferenceDiagnosticCode::DescriptorStale
        );
    }

    #[tokio::test]
    async fn validation_publication_refresh_clears_dependency_requirements_proof() {
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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Current,
            ))
            .await
            .expect("record proof");
        let mut refreshed_session = validation_session(
            "aaaaaaaaaaaaaaaa",
            DraftGraphValidationStatus::Executable,
            true,
        );
        refreshed_session.validation_session_id = "validation.session.2"
            .parse()
            .expect("valid validation session id");
        store
            .record_validation_publication(
                "graph-session-1".parse().expect("valid graph session id"),
                refreshed_session,
                vec![node_projection(DraftGraphValidationStatus::Executable)],
            )
            .await
            .expect("refresh validation session");

        let result = store
            .resolve_dependency_environment_action_intent(
                state_request_with_validation_session_and_action(
                    "graph-session-1",
                    "aaaaaaaaaaaaaaaa",
                    "aaaaaaaaaaaaaaaa",
                    "validation.session.2",
                    "dependency-node-1",
                    true,
                    DependencyEnvironmentAction::Check,
                ),
            )
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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Current,
            ))
            .await
            .expect("record proof");

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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Current,
            ))
            .await
            .expect("record proof");

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
        assert_eq!(
            projection
                .dependency_readiness_source
                .dependency_requirements_id
                .as_str(),
            "requirements.image_generation.cuda0"
        );
        assert_eq!(
            projection
                .dependency_readiness_source
                .selected_binding_ids
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn executable_snapshot_source_returns_projection_with_current_dependency_proof() {
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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Current,
            ))
            .await
            .expect("record proof");

        let source = store
            .current_executable_validation_snapshot_source(snapshot_source_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                Some("validation.session.1"),
            ))
            .await
            .expect("snapshot source");

        assert_eq!(source.nodes.len(), 1);
        assert_eq!(
            source.validation_session_id.as_str(),
            "validation.session.1"
        );
        let node = &source.nodes[0];
        assert_eq!(node.projection.node_id.as_str(), "infer");
        assert_eq!(
            node.projection.descriptor.descriptor_fingerprint.as_str(),
            "iface.scheduler.v1"
        );
        assert_eq!(
            node.dependency_requirements_proof
                .dependency_requirements_id
                .as_str(),
            "requirements.image_generation.cuda0"
        );
        assert_eq!(
            node.dependency_requirements_proof.selected_binding_ids[0].as_str(),
            "torch-diffusers"
        );
        assert_eq!(
            node.dependency_requirements_proof
                .dependency_override_fingerprint
                .as_str(),
            "override.none"
        );
    }

    #[tokio::test]
    async fn executable_snapshot_source_fails_closed_without_current_dependency_proof() {
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
            .current_executable_validation_snapshot_source(snapshot_source_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                Some("validation.session.1"),
            ))
            .await
            .expect_err("missing proof");

        assert!(matches!(
            error,
            CurrentExecutableValidationSnapshotSourceError::DependencyProofMissing { .. }
        ));
    }

    #[tokio::test]
    async fn executable_snapshot_source_rejects_stale_dependency_proof() {
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
        store
            .record_dependency_requirements_proof(proof_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                "validation.session.1",
                "infer",
                "requirements.image_generation.cuda0",
                CurrentDependencyRequirementsProofStatus::Stale,
            ))
            .await
            .expect("record proof");

        let error = store
            .current_executable_validation_snapshot_source(snapshot_source_request(
                "graph-session-1",
                "aaaaaaaaaaaaaaaa",
                Some("validation.session.1"),
            ))
            .await
            .expect_err("stale proof");

        assert!(matches!(
            error,
            CurrentExecutableValidationSnapshotSourceError::DependencyProofStale { .. }
        ));
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
            selected_binding_ids: Vec::new(),
            dependency_override_patches: Vec::new(),
            sidecar_diagnostic: None,
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

    fn state_request_with_validation_session_and_action(
        graph_session_id: &str,
        intent_revision: &str,
        current_revision: &str,
        validation_session_id: &str,
        target_node_id: &str,
        target_node_exists: bool,
        action: DependencyEnvironmentAction,
    ) -> DependencyEnvironmentActionIntentStateRequest {
        let mut request = state_request_with_validation_session(
            graph_session_id,
            intent_revision,
            current_revision,
            validation_session_id,
            target_node_id,
            target_node_exists,
        );
        let mut intent = request.intent.into_inner();
        intent.action = action;
        request.intent = ValidatedDependencyEnvironmentActionIntent::try_from(intent)
            .expect("valid action intent");
        request
    }

    fn proof_request(
        graph_session_id: &str,
        graph_revision: &str,
        validation_session_id: &str,
        inference_node_id: &str,
        dependency_requirements_id: &str,
        status: CurrentDependencyRequirementsProofStatus,
    ) -> CurrentDependencyRequirementsProofRequest {
        CurrentDependencyRequirementsProofRequest {
            graph_session_id: graph_session_id.parse().expect("valid graph session id"),
            graph_revision: graph_revision.parse().expect("valid graph revision"),
            validation_session_id: validation_session_id
                .parse()
                .expect("valid validation session id"),
            inference_node_id: inference_node_id.parse().expect("valid inference node id"),
            dependency_requirements_id: dependency_requirements_id
                .parse()
                .expect("valid dependency requirements id"),
            selected_binding_ids: vec![
                DependencyBindingId::parse("torch-diffusers").expect("valid binding id")
            ],
            dependency_override_fingerprint: "override.none"
                .parse()
                .expect("valid override fingerprint"),
            status,
            diagnostics: Vec::new(),
        }
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

    fn snapshot_source_request(
        graph_session_id: &str,
        graph_revision: &str,
        validation_session_id: Option<&str>,
    ) -> CurrentExecutableValidationSnapshotSourceRequest {
        CurrentExecutableValidationSnapshotSourceRequest {
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
