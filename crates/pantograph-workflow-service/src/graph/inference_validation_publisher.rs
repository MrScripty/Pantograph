use std::future::Future;

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};

use crate::workflow::WorkflowServiceError;

use super::inference_interface_facts::InferenceInterfaceFactsProvider;
use super::inference_interface_publication::{
    publish_inference_validation_for_resolution_inputs, WorkflowGraphInferenceValidationPublication,
};
use super::inference_interface_request::inference_interface_resolution_inputs_from_graph;
use super::inference_validation_lifecycle::{
    WorkflowGraphValidationLifecycleError, WorkflowGraphValidationLifecycleOwner,
};
use super::inference_validation_state::CurrentInferenceValidationStateStore;
use super::types::WorkflowGraph;

pub(crate) struct WorkflowGraphValidationPublishAttempt {
    pub(crate) graph_session_id: WorkflowGraphSessionId,
    pub(crate) graph_revision: WorkflowGraphRevision,
    pub(crate) validation_session_id: DraftGraphValidationSessionId,
    pub(crate) graph: WorkflowGraph,
}

pub(crate) enum WorkflowGraphValidationPublishAttemptOutcome {
    Published(WorkflowGraphInferenceValidationPublication),
    StaleGraphRevision {
        current_graph_revision: WorkflowGraphRevision,
    },
    PublicationRejected {
        current_graph_revision: WorkflowGraphRevision,
        reason: WorkflowGraphValidationLifecycleError,
    },
}

pub(crate) async fn publish_workflow_graph_validation_attempt<F, Fut>(
    request: WorkflowGraphValidationPublishAttempt,
    facts_provider: &dyn InferenceInterfaceFactsProvider,
    validation_lifecycle: &WorkflowGraphValidationLifecycleOwner,
    validation_state: &CurrentInferenceValidationStateStore,
    current_graph_revision_after_facts: F,
) -> Result<WorkflowGraphValidationPublishAttemptOutcome, WorkflowServiceError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<WorkflowGraphRevision, WorkflowServiceError>>,
{
    let resolution_inputs = inference_interface_resolution_inputs_from_graph(&request.graph);
    validation_lifecycle
        .begin_validation(
            request.graph_session_id.clone(),
            request.graph_revision.clone(),
            request.validation_session_id.clone(),
        )
        .await
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

    let facts_by_node_id = facts_provider
        .facts_for_resolution_inputs(&resolution_inputs.requests)
        .await
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

    let current_graph_revision = current_graph_revision_after_facts().await?;
    if current_graph_revision != request.graph_revision {
        return Ok(
            WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision {
                current_graph_revision,
            },
        );
    }

    if let Err(reason) = validation_lifecycle
        .accept_publication(
            &request.graph_session_id,
            &request.graph_revision,
            &request.validation_session_id,
        )
        .await
    {
        return Ok(
            WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected {
                current_graph_revision,
                reason,
            },
        );
    }

    let publication = publish_inference_validation_for_resolution_inputs(
        request.validation_session_id,
        request.graph_revision,
        resolution_inputs,
        facts_by_node_id,
    )
    .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

    validation_state
        .record_validation_publication(
            request.graph_session_id,
            publication.validation_session.clone(),
            publication.node_projections.clone(),
        )
        .await
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

    Ok(WorkflowGraphValidationPublishAttemptOutcome::Published(
        publication,
    ))
}
