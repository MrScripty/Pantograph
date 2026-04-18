use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Backend-owned workflow-session runtime residency states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSessionResidencyState {
    Active,
    Warm,
    CheckpointedButUnloaded,
    Restored,
}

/// Backend-owned node-memory compatibility decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeMemoryCompatibility {
    PreserveAsIs,
    PreserveWithInputRefresh,
    DropOnIdentityChange,
    DropOnSchemaIncompatibility,
    FallbackFullInvalidation,
}

/// Backend-owned logical node-memory status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeMemoryStatus {
    Empty,
    Ready,
    Invalidated,
}

/// Stable node-memory identity for one node in one workflow session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeMemoryIdentity {
    pub session_id: String,
    pub node_id: String,
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
}

/// Read-only backend-owned node-memory snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NodeMemorySnapshot {
    pub identity: NodeMemoryIdentity,
    pub status: NodeMemoryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_snapshot: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_state: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspection_metadata: Option<serde_json::Value>,
}

/// Per-node compatibility decision used by graph mutation and input reinjection
/// diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeMemoryCompatibilitySnapshot {
    pub node_id: String,
    pub compatibility: NodeMemoryCompatibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Backend-owned graph mutation impact summary for current node-memory rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GraphMemoryImpactSummary {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_decisions: Vec<NodeMemoryCompatibilitySnapshot>,
    pub fallback_to_full_invalidation: bool,
}

impl GraphMemoryImpactSummary {
    pub fn empty() -> Self {
        Self {
            node_decisions: Vec::new(),
            fallback_to_full_invalidation: false,
        }
    }

    pub fn fallback_full_invalidation<I, S>(node_ids: I, reason: impl Into<String>) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let reason = reason.into();
        Self {
            node_decisions: node_ids
                .into_iter()
                .map(|node_id| NodeMemoryCompatibilitySnapshot {
                    node_id: node_id.into(),
                    compatibility: NodeMemoryCompatibility::FallbackFullInvalidation,
                    reason: Some(reason.clone()),
                })
                .collect(),
            fallback_to_full_invalidation: true,
        }
    }
}

/// Bounded session-checkpoint summary for workflow-session continuity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowSessionCheckpointSummary {
    pub session_id: String,
    pub graph_revision: String,
    pub residency: WorkflowSessionResidencyState,
    pub checkpoint_available: bool,
    pub preserved_node_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpointed_at_ms: Option<u64>,
}

impl WorkflowSessionCheckpointSummary {
    pub fn unavailable(
        session_id: impl Into<String>,
        graph_revision: impl Into<String>,
        residency: WorkflowSessionResidencyState,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            graph_revision: graph_revision.into(),
            residency,
            checkpoint_available: false,
            preserved_node_count: 0,
            checkpointed_at_ms: None,
        }
    }
}

/// Private executor-owned Phase 6 session-state scaffold.
#[derive(Debug)]
pub(crate) struct WorkflowExecutorSessionState {
    residency: RwLock<WorkflowSessionResidencyState>,
}

impl WorkflowExecutorSessionState {
    pub(crate) fn new() -> Self {
        Self {
            residency: RwLock::new(WorkflowSessionResidencyState::Active),
        }
    }

    pub(crate) async fn residency(&self) -> WorkflowSessionResidencyState {
        self.residency.read().await.clone()
    }

    pub(crate) async fn set_residency(&self, state: WorkflowSessionResidencyState) {
        *self.residency.write().await = state;
    }

    pub(crate) async fn checkpoint_summary(
        &self,
        workflow_session_id: &str,
        graph_revision: &str,
    ) -> WorkflowSessionCheckpointSummary {
        WorkflowSessionCheckpointSummary::unavailable(
            workflow_session_id,
            graph_revision,
            self.residency().await,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GraphMemoryImpactSummary, NodeMemoryCompatibility, WorkflowExecutorSessionState,
        WorkflowSessionCheckpointSummary, WorkflowSessionResidencyState,
    };

    #[test]
    fn fallback_full_invalidation_marks_every_node() {
        let impact = GraphMemoryImpactSummary::fallback_full_invalidation(
            ["input", "merge", "output"],
            "phase6_contract_pending_reconciliation",
        );

        assert!(impact.fallback_to_full_invalidation);
        assert_eq!(impact.node_decisions.len(), 3);
        assert!(
            impact
                .node_decisions
                .iter()
                .all(|decision| decision.compatibility
                    == NodeMemoryCompatibility::FallbackFullInvalidation)
        );
    }

    #[test]
    fn unavailable_checkpoint_summary_has_no_timestamp() {
        let summary = WorkflowSessionCheckpointSummary::unavailable(
            "session-1",
            "graph-rev-1",
            WorkflowSessionResidencyState::Warm,
        );

        assert_eq!(summary.session_id, "session-1");
        assert_eq!(summary.graph_revision, "graph-rev-1");
        assert_eq!(summary.residency, WorkflowSessionResidencyState::Warm);
        assert!(!summary.checkpoint_available);
        assert_eq!(summary.preserved_node_count, 0);
        assert_eq!(summary.checkpointed_at_ms, None);
    }

    #[tokio::test]
    async fn executor_session_state_tracks_residency() {
        let state = WorkflowExecutorSessionState::new();

        assert_eq!(
            state.residency().await,
            WorkflowSessionResidencyState::Active
        );
        state
            .set_residency(WorkflowSessionResidencyState::CheckpointedButUnloaded)
            .await;
        assert_eq!(
            state.residency().await,
            WorkflowSessionResidencyState::CheckpointedButUnloaded
        );
    }

    #[tokio::test]
    async fn executor_session_state_builds_checkpoint_summary() {
        let state = WorkflowExecutorSessionState::new();
        let summary = state.checkpoint_summary("session-1", "graph-1").await;

        assert_eq!(summary.session_id, "session-1");
        assert_eq!(summary.graph_revision, "graph-1");
        assert_eq!(summary.residency, WorkflowSessionResidencyState::Active);
        assert!(!summary.checkpoint_available);
    }
}
