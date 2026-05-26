use std::collections::BTreeMap;

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};

use crate::workflow::WorkflowServiceError;

use super::super::inference_interface_publication::{
    publish_inference_validation_for_graph, WorkflowGraphInferenceValidationPublication,
};
use super::super::inference_interface_resolver::InferenceInterfaceResolverFacts;
use super::super::inference_interface_validation::WorkflowGraphInferenceValidationSession;
use super::GraphSessionStore;

impl GraphSessionStore {
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
        facts_by_node_id: BTreeMap<String, InferenceInterfaceResolverFacts>,
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

        let publication = publish_inference_validation_for_graph(
            validation_session_id,
            graph_revision,
            &graph,
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
}
