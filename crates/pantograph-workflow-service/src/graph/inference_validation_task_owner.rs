use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::workflow::WorkflowServiceError;

use super::inference_interface_facts::InferenceInterfaceFactsProvider;
use super::inference_validation_lifecycle::WorkflowGraphValidationCancellationReason;
use super::inference_validation_lifecycle::WorkflowGraphValidationLifecycleOwner;
use super::inference_validation_publisher::{
    publish_workflow_graph_validation_attempt, WorkflowGraphValidationPublishAttempt,
    WorkflowGraphValidationPublishAttemptOutcome,
};
use super::inference_validation_state::CurrentInferenceValidationStateStore;
use super::session::GraphSessionHandle;
use super::types::WorkflowGraph;

pub(crate) struct WorkflowGraphValidationTaskOwner {
    active: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationTaskRecord>>,
    events: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationTaskEventLog>>,
    max_events_per_session: usize,
}

impl WorkflowGraphValidationTaskOwner {
    pub(crate) fn new() -> Self {
        Self {
            active: RwLock::new(HashMap::new()),
            events: RwLock::new(HashMap::new()),
            max_events_per_session: DEFAULT_MAX_VALIDATION_TASK_EVENTS_PER_SESSION,
        }
    }

    pub(crate) async fn start_validation_task(
        &self,
        request: WorkflowGraphValidationTaskStartRequest,
        facts_provider: Arc<dyn InferenceInterfaceFactsProvider>,
        validation_lifecycle: Arc<WorkflowGraphValidationLifecycleOwner>,
        validation_state: Arc<CurrentInferenceValidationStateStore>,
        session_handle: GraphSessionHandle,
    ) {
        self.drain_finished_tasks().await;
        self.abort_active_task(
            &request.graph_session_id,
            WorkflowGraphValidationTaskTerminalState::Cancelled {
                reason: WorkflowGraphValidationCancellationReason::Superseded,
            },
        )
        .await;

        let graph_session_id = request.graph_session_id.clone();
        let graph_revision = request.graph_revision.clone();
        let validation_session_id = request.validation_session_id.clone();
        let handle = tokio::spawn(async move {
            publish_workflow_graph_validation_attempt(
                WorkflowGraphValidationPublishAttempt {
                    graph_session_id,
                    graph_revision,
                    validation_session_id,
                    graph: request.graph,
                },
                facts_provider.as_ref(),
                validation_lifecycle.as_ref(),
                validation_state.as_ref(),
                || async {
                    let mut state = session_handle.lock().await;
                    state.touch();
                    state.canonicalize_graph();
                    WorkflowGraphRevision::parse(&state.graph.compute_fingerprint())
                        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))
                },
            )
            .await
        });

        self.active.write().await.insert(
            request.graph_session_id,
            WorkflowGraphValidationTaskRecord {
                graph_revision: request.graph_revision,
                validation_session_id: request.validation_session_id,
                handle,
            },
        );
    }

    pub(crate) async fn close_graph_session(&self, graph_session_id: &WorkflowGraphSessionId) {
        self.drain_finished_tasks().await;
        self.abort_active_task(
            graph_session_id,
            WorkflowGraphValidationTaskTerminalState::Cancelled {
                reason: WorkflowGraphValidationCancellationReason::GraphSessionClosed,
            },
        )
        .await;
        self.events.write().await.remove(graph_session_id);
    }

    async fn drain_finished_tasks(&self) {
        let finished = {
            let mut active = self.active.write().await;
            let finished_keys = active
                .iter()
                .filter_map(|(graph_session_id, record)| {
                    record
                        .handle
                        .is_finished()
                        .then(|| graph_session_id.clone())
                })
                .collect::<Vec<_>>();
            finished_keys
                .into_iter()
                .filter_map(|graph_session_id| {
                    active
                        .remove(&graph_session_id)
                        .map(|record| (graph_session_id, record))
                })
                .collect::<Vec<_>>()
        };

        for (graph_session_id, record) in finished {
            let terminal_state = terminal_state_from_join_result(record.handle.await);
            self.push_event(
                graph_session_id,
                record.graph_revision,
                record.validation_session_id,
                terminal_state,
            )
            .await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn await_all_tasks(&self) {
        let active = {
            let mut active = self.active.write().await;
            active.drain().collect::<Vec<_>>()
        };

        for (graph_session_id, record) in active {
            let terminal_state = terminal_state_from_join_result(record.handle.await);
            self.push_event(
                graph_session_id,
                record.graph_revision,
                record.validation_session_id,
                terminal_state,
            )
            .await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn event_snapshot(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> Vec<WorkflowGraphValidationTaskEvent> {
        self.events
            .read()
            .await
            .get(graph_session_id)
            .map(|log| log.events.iter().cloned().collect())
            .unwrap_or_default()
    }

    async fn abort_active_task(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
        terminal_state: WorkflowGraphValidationTaskTerminalState,
    ) {
        let active = self.active.write().await.remove(graph_session_id);
        if let Some(record) = active {
            record.handle.abort();
            let _ = record.handle.await;
            self.push_event(
                graph_session_id.clone(),
                record.graph_revision,
                record.validation_session_id,
                terminal_state,
            )
            .await;
        }
    }

    async fn push_event(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
        terminal_state: WorkflowGraphValidationTaskTerminalState,
    ) {
        let mut events = self.events.write().await;
        let event_log = events.entry(graph_session_id.clone()).or_default();
        if event_log.events.len() == self.max_events_per_session {
            event_log.events.pop_front();
            event_log.dropped_event_count += 1;
        }
        event_log
            .events
            .push_back(WorkflowGraphValidationTaskEvent {
                graph_session_id,
                graph_revision,
                validation_session_id,
                terminal_state,
            });
    }
}

impl Default for WorkflowGraphValidationTaskOwner {
    fn default() -> Self {
        Self::new()
    }
}

const DEFAULT_MAX_VALIDATION_TASK_EVENTS_PER_SESSION: usize = 128;
const MAX_TASK_ERROR_MESSAGE_LEN: usize = 512;

pub(crate) struct WorkflowGraphValidationTaskStartRequest {
    pub(crate) graph_session_id: WorkflowGraphSessionId,
    pub(crate) graph_revision: WorkflowGraphRevision,
    pub(crate) validation_session_id: DraftGraphValidationSessionId,
    pub(crate) graph: WorkflowGraph,
}

struct WorkflowGraphValidationTaskRecord {
    graph_revision: WorkflowGraphRevision,
    validation_session_id: DraftGraphValidationSessionId,
    handle: JoinHandle<Result<WorkflowGraphValidationPublishAttemptOutcome, WorkflowServiceError>>,
}

#[derive(Debug, Default)]
struct WorkflowGraphValidationTaskEventLog {
    events: VecDeque<WorkflowGraphValidationTaskEvent>,
    dropped_event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowGraphValidationTaskEvent {
    pub(crate) graph_session_id: WorkflowGraphSessionId,
    pub(crate) graph_revision: WorkflowGraphRevision,
    pub(crate) validation_session_id: DraftGraphValidationSessionId,
    pub(crate) terminal_state: WorkflowGraphValidationTaskTerminalState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkflowGraphValidationTaskTerminalState {
    Completed,
    Cancelled {
        reason: WorkflowGraphValidationCancellationReason,
    },
    Rejected {
        reason: WorkflowGraphValidationTaskRejectionReason,
    },
    Failed {
        message: String,
    },
    Panicked,
    Aborted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowGraphValidationTaskRejectionReason {
    StaleGraphRevision,
    PublicationRejected,
}

fn bounded_task_error_message(message: String) -> String {
    if message.len() <= MAX_TASK_ERROR_MESSAGE_LEN {
        return message;
    }
    let mut truncated = message;
    truncated.truncate(MAX_TASK_ERROR_MESSAGE_LEN);
    truncated
}

fn terminal_state_from_join_result(
    result: Result<
        Result<WorkflowGraphValidationPublishAttemptOutcome, WorkflowServiceError>,
        tokio::task::JoinError,
    >,
) -> WorkflowGraphValidationTaskTerminalState {
    match result {
        Ok(Ok(WorkflowGraphValidationPublishAttemptOutcome::Published(_))) => {
            WorkflowGraphValidationTaskTerminalState::Completed
        }
        Ok(Ok(WorkflowGraphValidationPublishAttemptOutcome::StaleGraphRevision { .. })) => {
            WorkflowGraphValidationTaskTerminalState::Rejected {
                reason: WorkflowGraphValidationTaskRejectionReason::StaleGraphRevision,
            }
        }
        Ok(Ok(WorkflowGraphValidationPublishAttemptOutcome::PublicationRejected { .. })) => {
            WorkflowGraphValidationTaskTerminalState::Rejected {
                reason: WorkflowGraphValidationTaskRejectionReason::PublicationRejected,
            }
        }
        Ok(Ok(WorkflowGraphValidationPublishAttemptOutcome::Cancelled { reason, .. })) => {
            WorkflowGraphValidationTaskTerminalState::Cancelled { reason }
        }
        Ok(Err(error)) => WorkflowGraphValidationTaskTerminalState::Failed {
            message: bounded_task_error_message(error.to_string()),
        },
        Err(error) if error.is_cancelled() => WorkflowGraphValidationTaskTerminalState::Aborted,
        Err(error) if error.is_panic() => WorkflowGraphValidationTaskTerminalState::Panicked,
        Err(error) => WorkflowGraphValidationTaskTerminalState::Failed {
            message: bounded_task_error_message(error.to_string()),
        },
    }
}
