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
use super::GraphSessionStore;

const INFERENCE_INTERFACE_SNAPSHOT_FIELD: &str = "inference_interface_snapshot";

impl GraphSessionStore {
    pub async fn current_validation_summary(
        &self,
        request: WorkflowGraphCurrentValidationSummaryRequest,
    ) -> Result<WorkflowGraphCurrentValidationSummaryResponse, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(&request.graph_session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let handle = self.get_session_handle(&request.graph_session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let current_graph_revision =
            WorkflowGraphRevision::parse(&state.graph.compute_fingerprint())
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        Ok(self
            .validation_state
            .current_validation_summary(WorkflowGraphCurrentValidationSummaryStateRequest {
                graph_session_id,
                requested_graph_revision: request.graph_revision,
                current_graph_revision,
            })
            .await)
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
        let current_graph_revision = WorkflowGraphRevision::parse(&graph.compute_fingerprint())
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
        let publication = publish_workflow_graph_validation_attempt(
            WorkflowGraphValidationPublishAttempt {
                graph_session_id: graph_session_id.clone(),
                graph_revision: current_graph_revision.clone(),
                validation_session_id,
                graph,
            },
            self.inference_interface_facts_provider.as_ref(),
            self.validation_lifecycle.as_ref(),
            self.validation_state.as_ref(),
            || self.current_graph_revision_for_validation(&request.graph_session_id),
        )
        .await?;

        let (summary_revision, node_projections) = match publication {
            WorkflowGraphValidationPublishAttemptOutcome::Published(publication) => (
                current_graph_revision.clone(),
                publication.node_projections.clone(),
            ),
            WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision {
                current_graph_revision,
            } => (current_graph_revision, Vec::new()),
            WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected {
                current_graph_revision,
                ..
            } => (current_graph_revision, Vec::new()),
            WorkflowGraphValidationPublishAttemptOutcome::Cancelled {
                current_graph_revision,
                ..
            } => (current_graph_revision, Vec::new()),
        };

        let summary = self
            .validation_state
            .current_validation_summary(WorkflowGraphCurrentValidationSummaryStateRequest {
                graph_session_id,
                requested_graph_revision: request.graph_revision,
                current_graph_revision: summary_revision,
            })
            .await;

        Ok(WorkflowGraphCurrentValidationRefreshResponse {
            summary,
            node_projections,
        })
    }

    pub async fn scheduler_inference_task_projections_for_session(
        &self,
        session_id: &str,
        validation_session_id: Option<DraftGraphValidationSessionId>,
    ) -> Result<WorkflowSchedulerInferenceTaskProjections, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph_revision = WorkflowGraphRevision::parse(&state.graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        self.validation_state
            .scheduler_inference_task_projections(CurrentInferenceSchedulerProjectionRequest {
                graph_session_id,
                graph_revision,
                validation_session_id,
            })
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
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
                WorkflowGraphRevision::parse(&state.graph.compute_fingerprint())
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
        self.cancel_active_validation_after_graph_mutation(&session_id)
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
        let handle = self.get_session_handle(&request.graph_session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph = state.graph.clone();
        let current_graph_revision = WorkflowGraphRevision::parse(&graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        if current_graph_revision != request.graph_revision {
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
                handle,
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
        validation_session: WorkflowGraphInferenceValidationSession,
    ) -> Result<(), WorkflowServiceError> {
        validation_session
            .validate()
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let current_graph_revision =
            WorkflowGraphRevision::parse(&state.graph.compute_fingerprint())
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        if current_graph_revision != validation_session.graph_revision {
            return Err(WorkflowServiceError::InvalidRequest(
                "validation session graph revision does not match current graph revision"
                    .to_string(),
            ));
        }
        drop(state);

        self.validation_state
            .record_validation_session(graph_session_id, validation_session)
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
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
        let graph_revision = WorkflowGraphRevision::parse(&graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        match publish_workflow_graph_validation_attempt(
            WorkflowGraphValidationPublishAttempt {
                graph_session_id: graph_session_id.clone(),
                graph_revision: graph_revision.clone(),
                validation_session_id,
                graph,
            },
            self.inference_interface_facts_provider.as_ref(),
            self.validation_lifecycle.as_ref(),
            self.validation_state.as_ref(),
            || self.current_graph_revision_for_validation(session_id),
        )
        .await?
        {
            WorkflowGraphValidationPublishAttemptOutcome::Published(publication) => Ok(publication),
            WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision { .. } => {
                Err(WorkflowServiceError::InvalidRequest(
                    "validation graph revision changed before publication".to_string(),
                ))
            }
            WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected {
                reason, ..
            } => Err(WorkflowServiceError::InvalidRequest(reason.to_string())),
            WorkflowGraphValidationPublishAttemptOutcome::Cancelled { reason, .. } => {
                Err(WorkflowServiceError::InvalidRequest(format!(
                    "validation publication cancelled: {reason}"
                )))
            }
        }
    }

    async fn current_graph_revision_for_validation(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraphRevision, WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        WorkflowGraphRevision::parse(&state.graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
    }

    pub(crate) async fn executable_validation_snapshot_source_for_session(
        &self,
        session_id: &str,
        validation_session_id: Option<DraftGraphValidationSessionId>,
    ) -> Result<(WorkflowGraph, CurrentExecutableValidationSnapshotSource), WorkflowServiceError>
    {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let graph = state.graph.clone();
        let graph_revision = WorkflowGraphRevision::parse(&graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        drop(state);

        let source = self
            .validation_state
            .current_executable_validation_snapshot_source(
                CurrentExecutableValidationSnapshotSourceRequest {
                    graph_session_id,
                    graph_revision,
                    validation_session_id,
                },
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

        Ok((graph, source))
    }
}
