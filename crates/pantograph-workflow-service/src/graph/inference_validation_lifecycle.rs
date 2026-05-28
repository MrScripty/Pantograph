use std::collections::{HashMap, HashSet};

use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, WorkflowGraphRevision, WorkflowGraphSessionId,
};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub(crate) struct WorkflowGraphValidationLifecycleOwner {
    active: RwLock<HashMap<WorkflowGraphSessionId, WorkflowGraphValidationLifecycleRecord>>,
    closed_sessions: RwLock<HashSet<WorkflowGraphSessionId>>,
}

impl WorkflowGraphValidationLifecycleOwner {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn begin_validation(
        &self,
        graph_session_id: WorkflowGraphSessionId,
        graph_revision: WorkflowGraphRevision,
        validation_session_id: DraftGraphValidationSessionId,
    ) -> Result<WorkflowGraphValidationLifecycleBegin, WorkflowGraphValidationLifecycleError> {
        if self
            .closed_sessions
            .read()
            .await
            .contains(&graph_session_id)
        {
            return Err(WorkflowGraphValidationLifecycleError::GraphSessionClosed);
        }

        let record = WorkflowGraphValidationLifecycleRecord {
            graph_revision,
            validation_session_id,
        };
        let previous = self.active.write().await.insert(graph_session_id, record);
        Ok(WorkflowGraphValidationLifecycleBegin {
            superseded_validation_session_id: previous.map(|record| record.validation_session_id),
        })
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

        let active = self.active.read().await;
        let Some(record) = active.get(graph_session_id) else {
            return Err(WorkflowGraphValidationLifecycleError::ValidationSessionMissing);
        };
        if &record.graph_revision != graph_revision {
            return Err(WorkflowGraphValidationLifecycleError::GraphRevisionChanged);
        }
        if &record.validation_session_id != validation_session_id {
            return Err(WorkflowGraphValidationLifecycleError::ValidationSessionSuperseded);
        }
        Ok(())
    }

    pub(crate) async fn close_graph_session(
        &self,
        graph_session_id: &WorkflowGraphSessionId,
    ) -> Option<DraftGraphValidationSessionId> {
        self.closed_sessions
            .write()
            .await
            .insert(graph_session_id.clone());
        self.active
            .write()
            .await
            .remove(graph_session_id)
            .map(|record| record.validation_session_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowGraphValidationLifecycleBegin {
    pub(crate) superseded_validation_session_id: Option<DraftGraphValidationSessionId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowGraphValidationLifecycleRecord {
    graph_revision: WorkflowGraphRevision,
    validation_session_id: DraftGraphValidationSessionId,
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

        let first = owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                first_session.clone(),
            )
            .await
            .expect("begin first validation");
        let second = owner
            .begin_validation(
                graph_session_id.clone(),
                graph_revision.clone(),
                second_session.clone(),
            )
            .await
            .expect("begin second validation");

        assert!(first.superseded_validation_session_id.is_none());
        assert_eq!(
            second.superseded_validation_session_id,
            Some(first_session.clone())
        );
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
        owner
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
    }
}
