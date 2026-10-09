use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};
use uuid::Uuid;

use crate::workflow::WorkflowSchedulerInferenceTaskProjections;
use crate::workflow::WorkflowServiceError;

use super::super::executable_validation_snapshot_source::{
    CurrentExecutableValidationSnapshotSource, CurrentExecutableValidationSnapshotSourceRequest,
};
use super::super::inference_interface_patch::InferenceInterfaceApplyProposalRequest;
use super::super::inference_interface_publication::WorkflowGraphInferenceValidationPublication;
use super::super::inference_interface_validation::WorkflowGraphInferenceValidationSession;
use super::super::inference_validation_lifecycle::WorkflowGraphValidationLifecycleEventSnapshot;
use super::super::inference_validation_publisher::{
    publish_workflow_graph_validation_attempt, WorkflowGraphValidationPublishAttempt,
    WorkflowGraphValidationPublishAttemptOutcome,
};
use super::super::inference_validation_state::{
    CurrentInferenceInterfaceUpdateProposalStateRequest,
    CurrentInferenceSchedulerProjectionRequest, WorkflowGraphCurrentValidationRefreshRequest,
    WorkflowGraphCurrentValidationRefreshResponse, WorkflowGraphCurrentValidationSummaryRequest,
    WorkflowGraphCurrentValidationSummaryResponse,
    WorkflowGraphCurrentValidationSummaryStateRequest,
};
#[cfg(test)]
use super::super::inference_validation_task_owner::WorkflowGraphValidationTaskEvent;
use super::super::inference_validation_task_owner::WorkflowGraphValidationTaskStartRequest;
use super::super::memory_impact::graph_memory_impact_from_graph_change;
use super::super::session_contract::WorkflowGraphEditSessionGraphResponse;
use super::super::session_event::{dirty_tasks_from_seed_nodes, graph_modified_event};
use super::super::session_graph::sync_embedding_emit_metadata_flags;
use super::super::types::WorkflowGraph;
use super::super::{lower_groups, NodeRegistry, WorkflowGroupPreflight};
use super::GraphSessionStore;
use pantograph_inference_interface_contracts::InferenceDiagnosticCode;

const INFERENCE_INTERFACE_SNAPSHOT_FIELD: &str = "inference_interface_snapshot";

impl GraphSessionStore {
    pub async fn current_validation_summary(
        &self,
        request: WorkflowGraphCurrentValidationSummaryRequest,
    ) -> Result<WorkflowGraphCurrentValidationSummaryResponse, WorkflowServiceError> {
        Ok(self.current_validation_projection(request).await?.summary)
    }

    pub async fn current_validation_projection(
        &self,
        request: WorkflowGraphCurrentValidationSummaryRequest,
    ) -> Result<WorkflowGraphCurrentValidationRefreshResponse, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(&request.graph_session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let membership = self.sessions.read().await;
        let handle = membership.get(&request.graph_session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(request.graph_session_id.clone())
        })?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let current_graph_revision =
            WorkflowGraphRevision::parse(state.graph.compute_fingerprint())
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let key = WorkflowGraphCurrentValidationSummaryStateRequest {
            graph_session_id: graph_session_id.clone(),
            requested_graph_revision: request.graph_revision,
            current_graph_revision: current_graph_revision.clone(),
        };
        let result = self
            .validation_lifecycle
            .with_current_generation(
                &graph_session_id,
                &current_graph_revision,
                None,
                |generation| async {
                    let mut projection = self
                        .validation_state
                        .current_validation_projection(key.clone())
                        .await;
                    if projection
                        .summary
                        .validation_session_id
                        .as_ref()
                        .is_some_and(|id| id != &generation)
                    {
                        projection
                            .summary
                            .refuse_authority(InferenceDiagnosticCode::ValidationSessionSuperseded);
                        projection.node_projections.clear();
                    }
                    if projection.summary.submit_gate.allowed {
                        let expected = WorkflowGroupPreflight::capture(
                            &state.graph,
                            graph_session_id.clone(),
                            current_graph_revision.clone(),
                            generation,
                        );
                        if projection.summary.group_preflight != expected {
                            projection
                                .summary
                                .refuse_authority(InferenceDiagnosticCode::GroupContractBlocked);
                            projection.node_projections.clear();
                        }
                    }
                    projection
                },
            )
            .await;
        match result {
            Ok(projection) => Ok(projection),
            Err(_) => {
                let mut projection = self
                    .validation_state
                    .current_validation_projection(key)
                    .await;
                if projection.summary.validation_session_id.is_some()
                    || projection.summary.submit_gate.allowed
                {
                    projection
                        .summary
                        .refuse_authority(InferenceDiagnosticCode::ValidationSessionCancelled);
                    projection.node_projections.clear();
                }
                Ok(projection)
            }
        }
    }

    pub async fn refresh_current_validation_summary(
        &self,
        request: WorkflowGraphCurrentValidationRefreshRequest,
    ) -> Result<WorkflowGraphCurrentValidationRefreshResponse, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(&request.graph_session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let handle = self.get_session_handle(&request.graph_session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph = state.graph.clone();
        let current_graph_revision = WorkflowGraphRevision::parse(graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        if current_graph_revision != request.graph_revision {
            let summary = self
                .validation_state
                .current_validation_summary(WorkflowGraphCurrentValidationSummaryStateRequest {
                    graph_session_id,
                    requested_graph_revision: request.graph_revision,
                    current_graph_revision,
                })
                .await;
            return Ok(WorkflowGraphCurrentValidationRefreshResponse {
                summary,
                node_projections: Vec::new(),
            });
        }

        let validation_session_id =
            DraftGraphValidationSessionId::parse(format!("validation.session.{}", Uuid::new_v4()))
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let expected_generation = validation_session_id.clone();
        let publication = publish_workflow_graph_validation_attempt(
            WorkflowGraphValidationPublishAttempt {
                graph_session_id: graph_session_id.clone(),
                graph_revision: current_graph_revision.clone(),
                validation_session_id,
                graph,
                cancellation: None,
            },
            self.inference_interface_facts_provider.as_ref(),
            self.validation_lifecycle.as_ref(),
            self.validation_state.as_ref(),
            &self.sessions,
        )
        .await?;

        let rejected = match &publication {
            WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision { .. }
            | WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected { .. } => {
                Some(InferenceDiagnosticCode::ValidationSessionSuperseded)
            }
            WorkflowGraphValidationPublishAttemptOutcome::Cancelled { reason, .. } => Some(
                if *reason == super::super::inference_validation_lifecycle::WorkflowGraphValidationCancellationReason::Superseded {
                    InferenceDiagnosticCode::ValidationSessionSuperseded
                } else { InferenceDiagnosticCode::ValidationSessionCancelled }
            ),
            _ => None,
        };
        let mut response = self
            .current_validation_projection(WorkflowGraphCurrentValidationSummaryRequest {
                graph_session_id: request.graph_session_id,
                graph_revision: request.graph_revision,
            })
            .await?;
        if let Some(code) = rejected {
            // An old refresh must never borrow a newer generation's allowed gate.
            if response.summary.requested_graph_revision == response.summary.current_graph_revision
            {
                response.summary.refuse_authority(code);
            }
            response.node_projections.clear();
        } else if matches!(
            publication,
            WorkflowGraphValidationPublishAttemptOutcome::Published(_)
        ) && response.summary.validation_session_id.as_ref() != Some(&expected_generation)
        {
            response
                .summary
                .refuse_authority(InferenceDiagnosticCode::ValidationSessionSuperseded);
            response.node_projections.clear();
        }
        Ok(response)
    }

    pub async fn scheduler_inference_task_projections_for_session(
        &self,
        session_id: &str,
        validation_session_id: Option<DraftGraphValidationSessionId>,
    ) -> Result<WorkflowSchedulerInferenceTaskProjections, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let membership = self.sessions.read().await;
        let handle = membership
            .get(session_id)
            .ok_or_else(|| WorkflowServiceError::SessionNotFound(session_id.into()))?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph_revision = WorkflowGraphRevision::parse(state.graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        lower_groups(&state.graph, &NodeRegistry::new())?;
        self.validation_lifecycle
            .with_current_generation(
                &graph_session_id,
                &graph_revision,
                validation_session_id.as_ref(),
                |generation| async {
                    let summary = self
                        .validation_state
                        .current_validation_summary(
                            WorkflowGraphCurrentValidationSummaryStateRequest {
                                graph_session_id: graph_session_id.clone(),
                                requested_graph_revision: graph_revision.clone(),
                                current_graph_revision: graph_revision.clone(),
                            },
                        )
                        .await;
                    let expected = WorkflowGroupPreflight::capture(
                        &state.graph,
                        graph_session_id.clone(),
                        graph_revision.clone(),
                        generation.clone(),
                    );
                    if summary.group_preflight != expected {
                        return Err(WorkflowServiceError::InvalidRequest(
                            "current validation lacks matching authored group preflight".into(),
                        ));
                    }
                    self.validation_state
                        .scheduler_inference_task_projections(
                            CurrentInferenceSchedulerProjectionRequest {
                                graph_session_id: graph_session_id.clone(),
                                graph_revision: graph_revision.clone(),
                                validation_session_id: Some(generation),
                            },
                        )
                        .await
                        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
                },
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?
    }

    pub async fn apply_inference_interface_update_proposal(
        &self,
        request: InferenceInterfaceApplyProposalRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let session_id = request.graph_session_id.as_str().to_string();
        let node_id = request.node_id.as_str().to_string();
        let proposal = self
            .validation_state
            .current_update_proposal_for_apply(
                CurrentInferenceInterfaceUpdateProposalStateRequest {
                    graph_session_id: request.graph_session_id.clone(),
                    graph_revision: request.graph_revision.clone(),
                    validation_session_id: request.validation_session_id.clone(),
                    node_id: request.node_id.clone(),
                    proposal_id: request.proposal_id.clone(),
                    current_descriptor_fingerprint: request.current_descriptor_fingerprint.clone(),
                },
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let snapshot = request
            .replacement_snapshot(&proposal)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?
            .clone();

        let response = {
            let handle = self.get_session_handle(&session_id).await?;
            let mut state = handle.lock().await;
            state.touch();
            state.canonicalize_graph();
            let current_graph_revision =
                WorkflowGraphRevision::parse(state.graph.compute_fingerprint())
                    .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
            if current_graph_revision != request.graph_revision {
                return Err(WorkflowServiceError::InvalidRequest(
                    "proposal apply request graph revision is stale".to_string(),
                ));
            }

            let before_graph = state.graph.clone();
            if state.graph.find_node(&node_id).is_none() {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "node '{}' was not found",
                    node_id
                )));
            }
            state.push_undo_snapshot();
            let node = state.graph.find_node_mut(&node_id).ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!("node '{}' was not found", node_id))
            })?;
            let snapshot_value = serde_json::to_value(&snapshot)
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
            match &mut node.data {
                serde_json::Value::Object(map) => {
                    map.insert(
                        INFERENCE_INTERFACE_SNAPSHOT_FIELD.to_string(),
                        snapshot_value,
                    );
                }
                data => {
                    *data = serde_json::json!({
                        INFERENCE_INTERFACE_SNAPSHOT_FIELD: snapshot_value
                    });
                }
            }
            sync_embedding_emit_metadata_flags(&mut state.graph);
            let dirty_tasks =
                dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&node_id));
            let memory_impact =
                graph_memory_impact_from_graph_change(&before_graph, &state.graph, &dirty_tasks);
            let workflow_event =
                graph_modified_event(&session_id, &session_id, dirty_tasks, memory_impact.clone());
            let projection = super::phase6_memory_impact_projection(memory_impact);
            state.snapshot_response_with_state(&session_id, Some(workflow_event), projection)
        };
        self.start_validation_after_semantic_graph_mutation(&session_id)
            .await?;
        Ok(response)
    }

    pub async fn validation_lifecycle_event_snapshot(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraphValidationLifecycleEventSnapshot, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        Ok(self
            .validation_lifecycle
            .event_snapshot(&graph_session_id)
            .await)
    }

    pub async fn start_current_validation_task(
        &self,
        request: WorkflowGraphCurrentValidationRefreshRequest,
    ) -> Result<DraftGraphValidationSessionId, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(&request.graph_session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        self.start_validation_task_for_current_graph_revision(
            graph_session_id,
            Some(request.graph_revision),
        )
        .await
    }

    pub(super) async fn start_validation_task_for_current_graph_revision(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        requested_graph_revision: Option<WorkflowGraphRevision>,
    ) -> Result<DraftGraphValidationSessionId, WorkflowServiceError> {
        let handle = self.get_session_handle(graph_session_id.as_str()).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph = state.graph.clone();
        let current_graph_revision = WorkflowGraphRevision::parse(graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        if requested_graph_revision
            .as_ref()
            .is_some_and(|requested| requested != &current_graph_revision)
        {
            return Err(WorkflowServiceError::InvalidRequest(
                "validation task request graph revision is stale".to_string(),
            ));
        }

        let validation_session_id =
            DraftGraphValidationSessionId::parse(format!("validation.session.{}", Uuid::new_v4()))
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        self.validation_tasks
            .start_validation_task(
                WorkflowGraphValidationTaskStartRequest {
                    graph_session_id,
                    graph_revision: current_graph_revision,
                    validation_session_id: validation_session_id.clone(),
                    graph,
                },
                self.inference_interface_facts_provider.clone(),
                self.validation_lifecycle.clone(),
                self.validation_state.clone(),
                self.sessions.clone(),
            )
            .await?;
        Ok(validation_session_id)
    }

    #[cfg(test)]
    pub(crate) async fn drain_validation_tasks_for_tests(&self) {
        self.validation_tasks.await_all_tasks().await;
    }

    #[cfg(test)]
    pub(crate) async fn validation_task_events_for_tests(
        &self,
        session_id: &str,
    ) -> Result<Vec<WorkflowGraphValidationTaskEvent>, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        Ok(self
            .validation_tasks
            .event_snapshot(&graph_session_id)
            .await)
    }

    pub async fn record_inference_validation_session(
        &self,
        session_id: &str,
        mut validation_session: WorkflowGraphInferenceValidationSession,
    ) -> Result<(), WorkflowServiceError> {
        validation_session
            .validate()
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        {
            let membership = self.sessions.read().await;
            let handle = membership
                .get(session_id)
                .ok_or_else(|| WorkflowServiceError::SessionNotFound(session_id.into()))?;
            let state = handle.lock().await;
            if state.graph.compute_fingerprint() != validation_session.graph_revision.as_str() {
                return Err(WorkflowServiceError::InvalidRequest(
                    "validation session graph revision does not match current graph revision"
                        .into(),
                ));
            }
        }
        // Event sink callbacks occur before taking membership/graph guards.
        self.validation_lifecycle
            .begin_validation_with_invalidation(
                graph_session_id.clone(),
                validation_session.graph_revision.clone(),
                validation_session.validation_session_id.clone(),
                || self.validation_state.clear_graph_session(&graph_session_id),
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let membership = self.sessions.read().await;
        let handle = membership
            .get(session_id)
            .ok_or_else(|| WorkflowServiceError::SessionNotFound(session_id.into()))?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        if state.graph.compute_fingerprint() != validation_session.graph_revision.as_str() {
            return Err(WorkflowServiceError::InvalidRequest(
                "validation session graph revision does not match current graph revision".into(),
            ));
        }
        let facts = WorkflowGroupPreflight::capture(
            &state.graph,
            graph_session_id.clone(),
            validation_session.graph_revision.clone(),
            validation_session.validation_session_id.clone(),
        );
        if let Some(facts) = &facts {
            facts.apply_to_session(&mut validation_session);
        }
        self.validation_lifecycle
            .with_current_generation(
                &graph_session_id,
                &validation_session.graph_revision,
                Some(&validation_session.validation_session_id),
                |_| async {
                    self.validation_state
                        .record_validation_publication_with_group_preflight(
                            graph_session_id.clone(),
                            validation_session.clone(),
                            Vec::new(),
                            facts,
                        )
                        .await
                },
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);
        drop(membership);
        self.validation_lifecycle
            .publication_accepted_event(
                graph_session_id,
                validation_session.graph_revision,
                validation_session.validation_session_id,
            )
            .await;
        Ok(())
    }

    pub async fn publish_inference_validation_session(
        &self,
        session_id: &str,
        validation_session_id: DraftGraphValidationSessionId,
    ) -> Result<WorkflowGraphInferenceValidationPublication, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph = state.graph.clone();
        let graph_revision = WorkflowGraphRevision::parse(graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        match publish_workflow_graph_validation_attempt(
            WorkflowGraphValidationPublishAttempt {
                graph_session_id: graph_session_id.clone(),
                graph_revision: graph_revision.clone(),
                validation_session_id,
                graph,
                cancellation: None,
            },
            self.inference_interface_facts_provider.as_ref(),
            self.validation_lifecycle.as_ref(),
            self.validation_state.as_ref(),
            &self.sessions,
        )
        .await?
        {
            WorkflowGraphValidationPublishAttemptOutcome::Published(publication) => Ok(publication),
            WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision {
                current_graph_revision,
            } => Err(WorkflowServiceError::InvalidRequest(format!(
                "validation graph revision changed before publication: {current_graph_revision}",
            ))),
            WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected {
                reason,
                current_graph_revision,
            } => Err(WorkflowServiceError::InvalidRequest(format!(
                "{reason} (current graph revision: {current_graph_revision})"
            ))),
            WorkflowGraphValidationPublishAttemptOutcome::Cancelled {
                reason,
                current_graph_revision,
            } => Err(WorkflowServiceError::InvalidRequest(format!(
                "validation publication cancelled: {reason} (current graph revision: {current_graph_revision})"
            ))),
        }
    }

    pub(crate) async fn with_executable_validation_snapshot_source_for_session<R>(
        &self,
        session_id: &str,
        validation_session_id: Option<DraftGraphValidationSessionId>,
        handoff: impl FnOnce(
            WorkflowGraph,
            CurrentExecutableValidationSnapshotSource,
        ) -> Result<R, WorkflowServiceError>,
    ) -> Result<R, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let membership = self.sessions.read().await;
        let handle = membership
            .get(session_id)
            .ok_or_else(|| WorkflowServiceError::SessionNotFound(session_id.into()))?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph_revision = WorkflowGraphRevision::parse(state.graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        lower_groups(&state.graph, &NodeRegistry::new())?;
        self.validation_lifecycle
            .with_current_generation(
                &graph_session_id,
                &graph_revision,
                validation_session_id.as_ref(),
                |generation| async {
                    let summary = self
                        .validation_state
                        .current_validation_summary(
                            WorkflowGraphCurrentValidationSummaryStateRequest {
                                graph_session_id: graph_session_id.clone(),
                                requested_graph_revision: graph_revision.clone(),
                                current_graph_revision: graph_revision.clone(),
                            },
                        )
                        .await;
                    let expected = WorkflowGroupPreflight::capture(
                        &state.graph,
                        graph_session_id.clone(),
                        graph_revision.clone(),
                        generation.clone(),
                    );
                    if summary.group_preflight != expected {
                        return Err(WorkflowServiceError::InvalidRequest(
                            "current validation lacks matching authored group preflight".into(),
                        ));
                    }
                    let source = self
                        .validation_state
                        .current_executable_validation_snapshot_source(
                            CurrentExecutableValidationSnapshotSourceRequest {
                                graph_session_id: graph_session_id.clone(),
                                graph_revision: graph_revision.clone(),
                                validation_session_id: Some(generation),
                            },
                        )
                        .await
                        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
                    handoff(state.graph.clone(), source)
                },
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?
    }
}
