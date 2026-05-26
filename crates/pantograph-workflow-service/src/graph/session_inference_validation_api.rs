use pantograph_inference_interface_contracts::{WorkflowGraphRevision, WorkflowGraphSessionId};

use crate::workflow::WorkflowServiceError;

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
}
