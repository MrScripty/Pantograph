use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};
use uuid::Uuid;

use crate::workflow::WorkflowSchedulerInferenceTaskProjections;
use crate::workflow::WorkflowServiceError;

use super::super::executable_validation_snapshot_source::{
    CurrentExecutableValidationSnapshotSource, CurrentExecutableValidationSnapshotSourceRequest,
};
use super::super::inference_interface_publication::WorkflowGraphInferenceValidationPublication;
use super::super::inference_interface_validation::WorkflowGraphInferenceValidationSession;
use super::super::inference_validation_lifecycle::WorkflowGraphValidationLifecycleEventSnapshot;
use super::super::inference_validation_publisher::{
    publish_workflow_graph_validation_attempt, WorkflowGraphValidationPublishAttempt,
    WorkflowGraphValidationPublishAttemptOutcome,
};
use super::super::inference_validation_state::{
    CurrentInferenceSchedulerProjectionRequest, WorkflowGraphCurrentValidationRefreshRequest,
    WorkflowGraphCurrentValidationRefreshResponse, WorkflowGraphCurrentValidationSummaryRequest,
    WorkflowGraphCurrentValidationSummaryResponse,
    WorkflowGraphCurrentValidationSummaryStateRequest,
};
use super::super::types::WorkflowGraph;
use super::GraphSessionStore;

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
            &self.validation_lifecycle,
            &self.validation_state,
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
            &self.validation_lifecycle,
            &self.validation_state,
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
