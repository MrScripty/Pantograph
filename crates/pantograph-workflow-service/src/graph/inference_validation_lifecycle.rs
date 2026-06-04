use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{watch, RwLock};

pub(crate) struct WorkflowGraphValidationLifecycleOwner {
    active: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationLifecycleRecord>>,
    closed_sessions: RwLock<HashSet<WorkflowGraphSessionId>>,
    events: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationLifecycleEventLog>>,
    event_sink: RwLock<Option<Arc<dyn WorkflowGraphValidationLifecycleEventSink>>>,
    max_events_per_session: usize,
}

impl WorkflowGraphValidationLifecycleOwner {
    pub(crate) fn new() -> Self {
        Self {
            active: RwLock::new(HashMap::new()),
            closed_sessions: RwLock::new(HashSet::new()),
            events: RwLock::new(HashMap::new()),
            event_sink: RwLock::new(None),
            max_events_per_session: DEFAULT_MAX_LIFECYCLE_EVENTS_PER_SESSION,
        }
    }

    pub(crate) async fn set_event_sink(
        &self,
        event_sink: Option<Arc<dyn WorkflowGraphValidationLifecycleEventSink>>,
    ) {
        *self.event_sink.write().await = event_sink;
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

    pub(crate) async fn cancel_active_validation_for_graph_change(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> Option<DraftGraphValidationSessionId> {
        self.cancel_active_validation(
            graph_session_id,
            WorkflowGraphValidationCancellationReason::GraphRevisionChanged,
        )
        .await
    }

    pub(crate) async fn cancel_active_validation(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
        reason: WorkflowGraphValidationCancellationReason,
    ) -> Option<DraftGraphValidationSessionId> {
        let cancelled = self.active.write().await.remove(graph_session_id);
        if let Some(record) = cancelled.as_ref() {
            let _ = record.cancellation_tx.send(Some(reason));
            self.push_event(
                graph_session_id.clone(),
                record.graph_revision.clone(),
                record.validation_session_id.clone(),
                WorkflowGraphValidationLifecycleEventKind::ValidationCancelled { reason },
            )
            .await;
        }
        cancelled.map(|record| record.validation_session_id)
    }

    async fn push_event(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
        kind: WorkflowGraphValidationLifecycleEventKind,
    ) {
        let event = {
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
            event_log.events.push_back(event.clone());
            event
        };
        let event_sink = self.event_sink.read().await.clone();
        if let Some(event_sink) = event_sink {
            event_sink.publish_validation_lifecycle_event(event);
        }
    }

    #[cfg(test)]
    fn with_event_limit(max_events_per_session: usize) -> Self {
        Self {
            max_events_per_session,
            ..Self::new()
        }
    }

    pub(crate) async fn event_snapshot(
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

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowGraphValidationCancellationReason {
    #[error("validation session was superseded")]
    Superseded,
    #[error("graph revision changed")]
    GraphRevisionChanged,
    #[error("graph validation session is closed")]
    GraphSessionClosed,
    #[error("workflow graph validation task owner is shutting down")]
    Shutdown,
}

#[derive(Debug, Default)]
struct WorkflowGraphValidationLifecycleEventLog {
    events: VecDeque<WorkflowGraphValidationLifecycleEvent>,
    next_sequence: u64,
    dropped_event_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowGraphValidationLifecycleEvent {
    pub graph_session_id: WorkflowGraphSessionId,
    pub graph_revision: WorkflowGraphRevision,
    pub validation_session_id: DraftGraphValidationSessionId,
    pub sequence: u64,
    pub kind: WorkflowGraphValidationLifecycleEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum WorkflowGraphValidationLifecycleEventKind {
    ValidationPending,
    ValidationSuperseded {
        superseded_validation_session_id: DraftGraphValidationSessionId,
    },
    ValidationCancelled {
        reason: WorkflowGraphValidationCancellationReason,
    },
    PublicationAccepted,
    PublicationRejected {
        reason: WorkflowGraphValidationLifecycleError,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowGraphValidationLifecycleEventSnapshot {
    pub events: Vec<WorkflowGraphValidationLifecycleEvent>,
    pub dropped_event_count: u64,
}

pub trait WorkflowGraphValidationLifecycleEventSink: Send + Sync {
    fn publish_validation_lifecycle_event(&self, event: WorkflowGraphValidationLifecycleEvent);
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowGraphValidationLifecycleError {
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
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingLifecycleEventSink {
        events: Mutex<Vec<WorkflowGraphValidationLifecycleEvent>>,
    }

    impl WorkflowGraphValidationLifecycleEventSink for RecordingLifecycleEventSink {
        fn publish_validation_lifecycle_event(&self, event: WorkflowGraphValidationLifecycleEvent) {
            self.events.lock().expect("events lock").push(event);
        }
    }

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
    async fn graph_revision_change_cancels_active_session() {
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

        let cancelled = owner
            .cancel_active_validation_for_graph_change(&graph_session_id)
            .await;

        assert_eq!(cancelled, Some(validation_session_id.clone()));
        assert_eq!(
            *cancellation.borrow(),
            Some(WorkflowGraphValidationCancellationReason::GraphRevisionChanged)
        );
        assert_eq!(
            owner
                .accept_publication(&graph_session_id, &graph_revision, &validation_session_id)
                .await,
            Err(WorkflowGraphValidationLifecycleError::ValidationSessionMissing)
        );
        let events = owner.event_snapshot(&graph_session_id).await.events;
        assert!(events.iter().any(|event| matches!(
            event.kind,
            WorkflowGraphValidationLifecycleEventKind::ValidationCancelled {
                reason: WorkflowGraphValidationCancellationReason::GraphRevisionChanged,
            }
        )));
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

    #[tokio::test]
    async fn lifecycle_event_snapshot_serializes_typed_event_identity() {
        let owner = WorkflowGraphValidationLifecycleOwner::new();
        let graph_session_id: WorkflowGraphSessionId =
            "graph-session-1".parse().expect("valid graph session id");
        let graph_revision: WorkflowGraphRevision =
            "aaaaaaaaaaaaaaaa".parse().expect("valid graph revision");
        let validation_session_id: DraftGraphValidationSessionId = "validation.session.1"
            .parse()
            .expect("valid validation session id");

        owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision,
                validation_session_id,
            )
            .await
            .expect("begin validation");

        let snapshot = owner.event_snapshot(&graph_session_id).await;
        let encoded = serde_json::to_value(&snapshot).expect("serialize snapshot");

        assert_eq!(encoded["dropped_event_count"], 0);
        assert_eq!(encoded["events"][0]["graph_session_id"], "graph-session-1");
        assert_eq!(encoded["events"][0]["sequence"], 0);
        assert_eq!(encoded["events"][0]["kind"]["kind"], "validation_pending");
    }

    #[tokio::test]
    async fn lifecycle_event_sink_receives_typed_events_after_state_recording() {
        let owner = WorkflowGraphValidationLifecycleOwner::new();
        let sink = Arc::new(RecordingLifecycleEventSink::default());
        owner.set_event_sink(Some(sink.clone())).await;
        let graph_session_id: WorkflowGraphSessionId =
            "graph-session-1".parse().expect("valid graph session id");
        let graph_revision: WorkflowGraphRevision =
            "aaaaaaaaaaaaaaaa".parse().expect("valid graph revision");
        let validation_session_id: DraftGraphValidationSessionId = "validation.session.1"
            .parse()
            .expect("valid validation session id");

        owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                validation_session_id.clone(),
            )
            .await
            .expect("begin validation");
        owner
            .accept_publication(&graph_session_id, &graph_revision, &validation_session_id)
            .await
            .expect("accept publication");

        let snapshot = owner.event_snapshot(&graph_session_id).await;
        let received = sink.events.lock().expect("events lock").clone();

        assert_eq!(received, snapshot.events);
        assert_eq!(
            received.iter().map(|event| &event.kind).collect::<Vec<_>>(),
            vec![
                &WorkflowGraphValidationLifecycleEventKind::ValidationPending,
                &WorkflowGraphValidationLifecycleEventKind::PublicationAccepted,
            ]
        );
    }
}
