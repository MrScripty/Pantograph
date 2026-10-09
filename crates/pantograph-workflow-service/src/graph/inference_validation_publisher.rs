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
use super::session::GraphSessionMap;
use super::types::WorkflowGraph;
use super::WorkflowGroupPreflight;

pub(crate) struct WorkflowGraphValidationPublishAttempt {
    pub(crate) graph_session_id: WorkflowGraphSessionId,
    pub(crate) graph_revision: WorkflowGraphRevision,
    pub(crate) validation_session_id: DraftGraphValidationSessionId,
    pub(crate) graph: WorkflowGraph,
    pub(crate) cancellation:
        Option<tokio::sync::watch::Receiver<Option<WorkflowGraphValidationCancellationReason>>>,
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

pub(crate) async fn publish_workflow_graph_validation_attempt(
    request: WorkflowGraphValidationPublishAttempt,
    facts_provider: &dyn InferenceInterfaceFactsProvider,
    validation_lifecycle: &WorkflowGraphValidationLifecycleOwner,
    validation_state: &CurrentInferenceValidationStateStore,
    sessions: &GraphSessionMap,
) -> Result<WorkflowGraphValidationPublishAttemptOutcome, WorkflowServiceError> {
    if request.graph.compute_fingerprint() != request.graph_revision.as_str() {
        return Err(WorkflowServiceError::InvalidRequest(
            "captured authored graph does not match validation revision".into(),
        ));
    }
    let cancellation = match request.cancellation {
        Some(cancellation) => cancellation,
        None => validation_lifecycle
            .begin_validation_with_invalidation(
                request.graph_session_id.clone(),
                request.graph_revision.clone(),
                request.validation_session_id.clone(),
                || validation_state.clear_graph_session(&request.graph_session_id),
            )
            .await
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?,
    };
    let group_preflight = WorkflowGroupPreflight::capture(
        &request.graph,
        request.graph_session_id.clone(),
        request.graph_revision.clone(),
        request.validation_session_id.clone(),
    );
    let resolution_inputs = inference_interface_resolution_inputs_from_graph(&request.graph);
    let facts = if group_preflight
        .as_ref()
        .is_some_and(|facts| !facts.failures.is_empty())
    {
        // Invalid groups need no external descriptor lookup or resource acquisition.
        BTreeMap::new()
    } else {
        match facts_for_resolution_inputs_until_cancelled(
            facts_provider,
            &resolution_inputs.requests,
            cancellation,
            || async {
                let membership = sessions.read().await;
                let handle = membership
                    .get(request.graph_session_id.as_str())
                    .ok_or_else(|| {
                        WorkflowServiceError::SessionNotFound(request.graph_session_id.to_string())
                    })?;
                let mut state = handle.lock().await;
                state.touch();
                state.canonicalize_graph();
                WorkflowGraphRevision::parse(state.graph.compute_fingerprint())
                    .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
            },
        )
        .await?
        {
            WorkflowGraphValidationFactsLookupOutcome::FactsAvailable(facts) => {
                if facts.current_graph_revision != request.graph_revision {
                    return Ok(
                        WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision {
                            current_graph_revision: facts.current_graph_revision,
                        },
                    );
                }
                facts.facts_by_node_id
            }
            WorkflowGraphValidationFactsLookupOutcome::Cancelled {
                current_graph_revision,
                reason,
            } => {
                return Ok(WorkflowGraphValidationPublishAttemptOutcome::Cancelled {
                    current_graph_revision,
                    reason,
                })
            }
        }
    };
    let mut publication = publish_inference_validation_for_resolution_inputs(
        request.validation_session_id.clone(),
        request.graph_revision.clone(),
        resolution_inputs,
        facts,
    )
    .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
    if let Some(facts) = &group_preflight {
        facts.apply_to_session(&mut publication.validation_session);
    }
    // Membership -> authored graph -> generation -> validation state.
    // The same guards cover the final semantic and pure-group recheck and store.
    let membership = sessions.read().await;
    let handle = membership
        .get(request.graph_session_id.as_str())
        .ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(request.graph_session_id.to_string())
        })?;
    let mut state = handle.lock().await;
    state.touch();
    state.canonicalize_graph();
    let current_graph_revision = WorkflowGraphRevision::parse(state.graph.compute_fingerprint())
        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
    if current_graph_revision != request.graph_revision {
        return Ok(
            WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision {
                current_graph_revision,
            },
        );
    }
    let rechecked = WorkflowGroupPreflight::capture(
        &state.graph,
        request.graph_session_id.clone(),
        current_graph_revision.clone(),
        request.validation_session_id.clone(),
    );
    if rechecked != group_preflight {
        return Err(WorkflowServiceError::InvalidRequest(
            "authored group preflight changed before publication".into(),
        ));
    }
    let stored = validation_lifecycle
        .with_current_generation(
            &request.graph_session_id,
            &request.graph_revision,
            Some(&request.validation_session_id),
            |_| async {
                validation_state
                    .record_validation_publication_with_group_preflight(
                        request.graph_session_id.clone(),
                        publication.validation_session.clone(),
                        publication.node_projections.clone(),
                        group_preflight,
                    )
                    .await
            },
        )
        .await;
    drop(state);
    drop(membership);
    match stored {
        Err(reason) => {
            let _ = validation_lifecycle
                .record_publication_rejection(
                    request.graph_session_id.clone(),
                    request.graph_revision,
                    request.validation_session_id,
                    reason,
                )
                .await;
            Ok(
                WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected {
                    current_graph_revision,
                    reason,
                },
            )
        }
        Ok(result) => {
            result.map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
            validation_lifecycle
                .publication_accepted_event(
                    request.graph_session_id,
                    request.graph_revision,
                    request.validation_session_id,
                )
                .await;
            Ok(WorkflowGraphValidationPublishAttemptOutcome::Published(
                publication,
            ))
        }
    }
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
