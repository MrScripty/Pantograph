use std::{collections::BTreeMap, future::Future};

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};

use crate::workflow::WorkflowServiceError;

use super::inference_interface_facts::InferenceInterfaceFactsProvider;
use super::inference_interface_publication::{
    publish_inference_validation_for_resolution_inputs, WorkflowGraphInferenceValidationPublication,
};
use super::inference_interface_request::inference_interface_resolution_inputs_from_graph;
use super::inference_interface_request::InferenceInterfaceGraphResolutionInput;
use super::inference_interface_resolver::InferenceInterfaceResolverFacts;
use super::inference_validation_lifecycle::{
    WorkflowGraphValidationCancellationReason, WorkflowGraphValidationLifecycleError,
    WorkflowGraphValidationLifecycleOwner,
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
    Cancelled {
        current_graph_revision: WorkflowGraphRevision,
        reason: WorkflowGraphValidationCancellationReason,
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
    let cancellation = validation_lifecycle
        .begin_validation(
            request.graph_session_id.clone(),
            request.graph_revision.clone(),
            request.validation_session_id.clone(),
        )
        .await
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;

    let facts_by_node_id = match facts_for_resolution_inputs_until_cancelled(
        facts_provider,
        &resolution_inputs.requests,
        cancellation,
        current_graph_revision_after_facts,
    )
    .await?
    {
        WorkflowGraphValidationFactsLookupOutcome::FactsAvailable(facts) => facts,
        WorkflowGraphValidationFactsLookupOutcome::Cancelled {
            current_graph_revision,
            reason,
        } => {
            return Ok(WorkflowGraphValidationPublishAttemptOutcome::Cancelled {
                current_graph_revision,
                reason,
            });
        }
    };

    let current_graph_revision = facts_by_node_id.current_graph_revision;

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
        facts_by_node_id.facts_by_node_id,
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

struct WorkflowGraphValidationFactsLookup {
    current_graph_revision: WorkflowGraphRevision,
    facts_by_node_id: BTreeMap<String, InferenceInterfaceResolverFacts>,
}

enum WorkflowGraphValidationFactsLookupOutcome {
    FactsAvailable(WorkflowGraphValidationFactsLookup),
    Cancelled {
        current_graph_revision: WorkflowGraphRevision,
        reason: WorkflowGraphValidationCancellationReason,
    },
}

async fn facts_for_resolution_inputs_until_cancelled<F, Fut>(
    facts_provider: &dyn InferenceInterfaceFactsProvider,
    resolution_inputs: &[InferenceInterfaceGraphResolutionInput],
    mut cancellation: tokio::sync::watch::Receiver<
        Option<WorkflowGraphValidationCancellationReason>,
    >,
    current_graph_revision_after_facts: F,
) -> Result<WorkflowGraphValidationFactsLookupOutcome, WorkflowServiceError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<WorkflowGraphRevision, WorkflowServiceError>>,
{
    let facts_lookup = facts_provider.facts_for_resolution_inputs(resolution_inputs);
    tokio::pin!(facts_lookup);

    let facts_by_node_id = loop {
        tokio::select! {
            facts = &mut facts_lookup => {
                break facts.map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
            }
            changed = cancellation.changed() => {
                if changed.is_err() {
                    continue;
                }
                let reason = *cancellation.borrow_and_update();
                if let Some(reason) = reason {
                    let current_graph_revision = current_graph_revision_after_facts().await?;
                    return Ok(WorkflowGraphValidationFactsLookupOutcome::Cancelled {
                        current_graph_revision,
                        reason,
                    });
                }
            }
        }
    };

    let current_graph_revision = current_graph_revision_after_facts().await?;
    if let Some(reason) = *cancellation.borrow() {
        return Ok(WorkflowGraphValidationFactsLookupOutcome::Cancelled {
            current_graph_revision,
            reason,
        });
    }

    Ok(WorkflowGraphValidationFactsLookupOutcome::FactsAvailable(
        WorkflowGraphValidationFactsLookup {
            current_graph_revision,
            facts_by_node_id,
        },
    ))
}
