use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};

use crate::workflow::WorkflowSchedulerInferenceTaskProjections;
use crate::workflow::WorkflowServiceError;

use super::super::executable_validation_snapshot_source::{
    CurrentExecutableValidationSnapshotSource, CurrentExecutableValidationSnapshotSourceRequest,
};
use super::super::inference_interface_publication::{
    publish_inference_validation_for_resolution_inputs, WorkflowGraphInferenceValidationPublication,
};
use super::super::inference_interface_request::inference_interface_resolution_inputs_from_graph;
use super::super::inference_interface_validation::WorkflowGraphInferenceValidationSession;
use super::super::inference_validation_state::CurrentInferenceSchedulerProjectionRequest;
use super::super::types::WorkflowGraph;
use super::GraphSessionStore;

impl GraphSessionStore {
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

        let resolution_inputs = inference_interface_resolution_inputs_from_graph(&graph);
        let facts_by_node_id = self
            .inference_interface_facts_provider
            .facts_for_resolution_inputs(&resolution_inputs.requests)
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

        let publication = publish_inference_validation_for_resolution_inputs(
            validation_session_id,
            graph_revision,
            resolution_inputs,
            facts_by_node_id,
        )
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

        self.validation_state
            .record_validation_publication(
                graph_session_id,
                publication.validation_session.clone(),
                publication.node_projections.clone(),
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

        Ok(publication)
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
