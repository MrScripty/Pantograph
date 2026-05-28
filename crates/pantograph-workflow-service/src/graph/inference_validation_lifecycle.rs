use std::collections::{HashMap, HashSet, VecDeque};

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};
use thiserror::Error;
use tokio::sync::{watch, RwLock};

#[derive(Debug)]
pub(crate) struct WorkflowGraphValidationLifecycleOwner {
    active: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationLifecycleRecord>>,
    closed_sessions: RwLock<HashSet<WorkflowGraphSessionId>>,
    events: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationLifecycleEventLog>>,
    max_events_per_session: usize,
}

impl WorkflowGraphValidationLifecycleOwner {
    pub(crate) fn new() -> Self {
        Self {
            active: RwLock::new(HashMap::new()),
            closed_sessions: RwLock::new(HashSet::new()),
            events: RwLock::new(HashMap::new()),
            max_events_per_session: DEFAULT_MAX_LIFECYCLE_EVENTS_PER_SESSION,
        }
    }

    pub(crate) async fn begin_validation(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
    ) -> Result<
        watch::Receiver<Option<WorkflowGraphValidationCancellationReason>>,
        WorkflowGraphValidationLifecycleError,
    > {
        if self
            .closed_sessions
            .read()
            .await
            .contains(&graph_session_id)
        {
            return Err(WorkflowGraphValidationLifecycleError::GraphSessionClosed);
        }

        let (cancellation_tx, cancellation) = watch::channel(None);
        let record = WorkflowGraphValidationLifecycleRecord {
            graph_revision: graph_revision.clone(),
            validation_session_id: validation_session_id.clone(),
            cancellation_tx,
        };
        let previous = self
            .active
            .write()
            .await
            .insert(graph_session_id.clone(), record);
        if let Some(previous) = previous.as_ref() {
            let _ = previous
                .cancellation_tx
                .send(Some(WorkflowGraphValidationCancellationReason::Superseded));
        }
        let kind = match previous.as_ref() {
            Some(previous) => WorkflowGraphValidationLifecycleEventKind::ValidationSuperseded {
                superseded_validation_session_id: previous.validation_session_id.clone(),
            },
            None => WorkflowGraphValidationLifecycleEventKind::ValidationPending,
        };
        self.push_event(
            graph_session_id.clone(),
            graph_revision,
            validation_session_id,
            kind,
        )
        .await;
        Ok(cancellation)
    }

    pub(crate) async fn accept_publication(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
        graph_revision: &WorkflowGraphRevision,
        validation_session_id: &DraftGraphValidationSessionId,
    ) -> Result<(), WorkflowGraphValidationLifecycleError> {
        if self.closed_sessions.read().await.contains(graph_session_id) {
            return Err(WorkflowGraphValidationLifecycleError::GraphSessionClosed);
        }

        let result = {
            let active = self.active.read().await;
            match active.get(graph_session_id) {
                Some(record) if &record.graph_revision != graph_revision => {
                    Err(WorkflowGraphValidationLifecycleError::GraphRevisionChanged)
                }
                Some(record) if &record.validation_session_id != validation_session_id => {
                    Err(WorkflowGraphValidationLifecycleError::ValidationSessionSuperseded)
                }
                Some(_) => Ok(()),
                None => Err(WorkflowGraphValidationLifecycleError::ValidationSessionMissing),
            }
        };

        if let Err(error) = result {
            return self
                .record_publication_rejection(
                    graph_session_id.clone(),
                    graph_revision.clone(),
                    validation_session_id.clone(),
                    error,
                )
                .await;
        }
        self.push_event(
            graph_session_id.clone(),
            graph_revision.clone(),
            validation_session_id.clone(),
            WorkflowGraphValidationLifecycleEventKind::PublicationAccepted,
        )
        .await;
        Ok(())
    }

    async fn record_publication_rejection(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
        reason: WorkflowGraphValidationLifecycleError,
    ) -> Result<(), WorkflowGraphValidationLifecycleError> {
        self.push_event(
            graph_session_id,
            graph_revision,
            validation_session_id,
            WorkflowGraphValidationLifecycleEventKind::PublicationRejected { reason },
        )
        .await;
        Err(reason)
    }

    pub(crate) async fn close_graph_session(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> Option<DraftGraphValidationSessionId> {
        self.closed_sessions
            .write()
            .await
            .insert(graph_session_id.clone());
        let closed = self.active.write().await.remove(graph_session_id);
        if let Some(record) = closed.as_ref() {
            let _ = record.cancellation_tx.send(Some(
                WorkflowGraphValidationCancellationReason::GraphSessionClosed,
            ));
        }
        self.events.write().await.remove(graph_session_id);
        closed.map(|record| record.validation_session_id)
    }

    async fn push_event(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
        kind: WorkflowGraphValidationLifecycleEventKind,
    ) {
        let mut events = self.events.write().await;
        let event_log = events.entry(graph_session_id.clone()).or_default();
        if event_log.events.len() == self.max_events_per_session {
            event_log.events.pop_front();
            event_log.dropped_event_count += 1;
        }
        let event = WorkflowGraphValidationLifecycleEvent {
            graph_session_id,
            graph_revision,
            validation_session_id,
            sequence: event_log.next_sequence,
            kind,
        };
        event_log.next_sequence += 1;
        event_log.events.push_back(event);
    }

    #[cfg(test)]
    fn with_event_limit(max_events_per_session: usize) -> Self {
        Self {
            max_events_per_session,
            ..Self::new()
        }
    }

    #[cfg(test)]
    async fn event_snapshot(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> WorkflowGraphValidationLifecycleEventSnapshot {
        let events = self.events.read().await;
        let Some(event_log) = events.get(graph_session_id) else {
            return WorkflowGraphValidationLifecycleEventSnapshot::default();
        };
        WorkflowGraphValidationLifecycleEventSnapshot {
            events: event_log.events.iter().cloned().collect(),
            dropped_event_count: event_log.dropped_event_count,
        }
    }
}

const DEFAULT_MAX_LIFECYCLE_EVENTS_PER_SESSION: usize = 128;

impl Default for WorkflowGraphValidationLifecycleOwner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct WorkflowGraphValidationLifecycleRecord {
    graph_revision: WorkflowGraphRevision,
    validation_session_id: DraftGraphValidationSessionId,
    cancellation_tx: watch::Sender<Option<WorkflowGraphValidationCancellationReason>>,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowGraphValidationCancellationReason {
    #[error("validation session was superseded")]
    Superseded,
    #[error("graph validation session is closed")]
    GraphSessionClosed,
}

#[derive(Debug, Default)]
struct WorkflowGraphValidationLifecycleEventLog {
    events: VecDeque<WorkflowGraphValidationLifecycleEvent>,
    next_sequence: u64,
    dropped_event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowGraphValidationLifecycleEvent {
    graph_session_id: WorkflowGraphSessionId,
    graph_revision: WorkflowGraphRevision,
    validation_session_id: DraftGraphValidationSessionId,
    sequence: u64,
    kind: WorkflowGraphValidationLifecycleEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkflowGraphValidationLifecycleEventKind {
    ValidationPending,
    ValidationSuperseded {
        superseded_validation_session_id: DraftGraphValidationSessionId,
    },
    PublicationAccepted,
    PublicationRejected {
        reason: WorkflowGraphValidationLifecycleError,
    },
}

#[cfg(test)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct WorkflowGraphValidationLifecycleEventSnapshot {
    events: Vec<WorkflowGraphValidationLifecycleEvent>,
    dropped_event_count: u64,
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum WorkflowGraphValidationLifecycleError {
    #[error("graph validation session is closed")]
    GraphSessionClosed,
    #[error("graph validation session is missing")]
    ValidationSessionMissing,
    #[error("graph revision changed before validation publication")]
    GraphRevisionChanged,
    #[error("validation session was superseded")]
    ValidationSessionSuperseded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn begin_validation_supersedes_active_session() {
        let owner = WorkflowGraphValidationLifecycleOwner::new();
        let graph_session_id: WorkflowGraphSessionId =
            "graph-session-1".parse().expect("valid graph session id");
        let graph_revision: WorkflowGraphRevision =
            "aaaaaaaaaaaaaaaa".parse().expect("valid graph revision");
        let first_session: DraftGraphValidationSessionId = "validation.session.1"
            .parse()
            .expect("valid validation session id");
        let second_session: DraftGraphValidationSessionId = "validation.session.2"
            .parse()
            .expect("valid validation session id");

        let first_cancellation = owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                first_session.clone(),
            )
            .await
            .expect("begin first validation");
        assert!(first_cancellation.borrow().is_none());

        let second_cancellation = owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                second_session.clone(),
            )
            .await
            .expect("begin second validation");

        assert_eq!(
            *first_cancellation.borrow(),
            Some(WorkflowGraphValidationCancellationReason::Superseded)
        );
        assert!(second_cancellation.borrow().is_none());
        assert_eq!(
            owner
                .accept_publication(&graph_session_id, &graph_revision, &first_session)
                .await,
            Err(WorkflowGraphValidationLifecycleError::ValidationSessionSuperseded)
        );
        owner
            .accept_publication(&graph_session_id, &graph_revision, &second_session)
            .await
            .expect("latest validation session can publish");
        let events = owner.event_snapshot(&graph_session_id).await;
        assert_eq!(
            events
                .events
                .iter()
                .map(|event| &event.kind)
                .collect::<Vec<_>>(),
            vec![
                &WorkflowGraphValidationLifecycleEventKind::ValidationPending,
                &WorkflowGraphValidationLifecycleEventKind::ValidationSuperseded {
                    superseded_validation_session_id: first_session
                },
                &WorkflowGraphValidationLifecycleEventKind::PublicationRejected {
                    reason: WorkflowGraphValidationLifecycleError::ValidationSessionSuperseded,
                },
                &WorkflowGraphValidationLifecycleEventKind::PublicationAccepted,
            ]
        );
    }

    #[tokio::test]
    async fn close_graph_session_rejects_later_publication() {
        let owner = WorkflowGraphValidationLifecycleOwner::new();
        let graph_session_id: WorkflowGraphSessionId =
            "graph-session-1".parse().expect("valid graph session id");
        let graph_revision: WorkflowGraphRevision =
            "aaaaaaaaaaaaaaaa".parse().expect("valid graph revision");
        let validation_session_id: DraftGraphValidationSessionId = "validation.session.1"
            .parse()
            .expect("valid validation session id");
        let cancellation = owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                validation_session_id.clone(),
            )
            .await
            .expect("begin validation");

        let closed = owner.close_graph_session(&graph_session_id).await;

        assert_eq!(closed, Some(validation_session_id.clone()));
        assert_eq!(
            *cancellation.borrow(),
            Some(WorkflowGraphValidationCancellationReason::GraphSessionClosed)
        );
        assert_eq!(
            owner
                .accept_publication(&graph_session_id, &graph_revision, &validation_session_id)
                .await,
            Err(WorkflowGraphValidationLifecycleError::GraphSessionClosed)
        );
        assert_eq!(
            owner
                .begin_validation(graph_session_id, graph_revision, validation_session_id)
                .await
                .map(|_| ()),
            Err(WorkflowGraphValidationLifecycleError::GraphSessionClosed)
        );
        let events = owner
            .event_snapshot(&"graph-session-1".parse().unwrap())
            .await;
        assert!(events.events.is_empty());
        assert_eq!(events.dropped_event_count, 0);
    }

    #[tokio::test]
    async fn lifecycle_event_log_is_bounded_and_records_dropped_events() {
        let owner = WorkflowGraphValidationLifecycleOwner::with_event_limit(2);
        let graph_session_id: WorkflowGraphSessionId =
            "graph-session-1".parse().expect("valid graph session id");
        let graph_revision: WorkflowGraphRevision =
            "aaaaaaaaaaaaaaaa".parse().expect("valid graph revision");
        let first_session: DraftGraphValidationSessionId = "validation.session.1"
            .parse()
            .expect("valid validation session id");
        let second_session: DraftGraphValidationSessionId = "validation.session.2"
            .parse()
            .expect("valid validation session id");

        owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                first_session,
            )
            .await
            .expect("begin first validation");
        owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                second_session.clone(),
            )
            .await
            .expect("begin second validation");
        owner
            .accept_publication(&graph_session_id, &graph_revision, &second_session)
            .await
            .expect("latest validation publishes");

        let events = owner.event_snapshot(&graph_session_id).await;

        assert_eq!(events.dropped_event_count, 1);
        assert_eq!(events.events.len(), 2);
        assert_eq!(events.events[0].sequence, 1);
        assert_eq!(events.events[1].sequence, 2);
    }
}
