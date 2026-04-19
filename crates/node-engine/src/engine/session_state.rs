use std::collections::HashMap;

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

/// Restore strategy for non-serializable runtime/process state referenced by a
/// node-memory snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeMemoryRestoreStrategy {
    RehydrateBeforeResume,
    RebindHostResource,
    DropIfUnavailable,
}

/// Indirect reference to non-serializable runtime/process state associated with
/// one node-memory snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct NodeMemoryIndirectStateReference {
    pub reference_kind: String,
    pub reference_id: String,
    pub restore_strategy: NodeMemoryRestoreStrategy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspection_metadata: Option<serde_json::Value>,
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
    pub indirect_state_reference: Option<NodeMemoryIndirectStateReference>,
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

    pub fn dirty_task_ids(&self) -> Vec<String> {
        let mut node_ids = self
            .node_decisions
            .iter()
            .filter(|decision| {
                decision.compatibility != NodeMemoryCompatibility::PreserveAsIs
            })
            .map(|decision| decision.node_id.clone())
            .collect::<Vec<_>>();
        node_ids.sort();
        node_ids.dedup();
        node_ids
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
    bound_workflow_session_id: RwLock<Option<String>>,
    node_memories: RwLock<HashMap<String, HashMap<String, NodeMemorySnapshot>>>,
    checkpoints: RwLock<HashMap<String, u64>>,
}

impl WorkflowExecutorSessionState {
    pub(crate) fn new() -> Self {
        Self {
            residency: RwLock::new(WorkflowSessionResidencyState::Active),
            bound_workflow_session_id: RwLock::new(None),
            node_memories: RwLock::new(HashMap::new()),
            checkpoints: RwLock::new(HashMap::new()),
        }
    }

    pub(crate) async fn residency(&self) -> WorkflowSessionResidencyState {
        self.residency.read().await.clone()
    }

    pub(crate) async fn set_residency(&self, state: WorkflowSessionResidencyState) {
        *self.residency.write().await = state;
    }

    pub(crate) async fn bind_workflow_session(&self, workflow_session_id: String) {
        *self.bound_workflow_session_id.write().await = Some(workflow_session_id);
    }

    pub(crate) async fn bound_workflow_session_id(&self) -> Option<String> {
        self.bound_workflow_session_id.read().await.clone()
    }

    pub(crate) async fn clear_bound_workflow_session(&self) {
        *self.bound_workflow_session_id.write().await = None;
    }

    pub(crate) async fn record_node_memory(&self, snapshot: NodeMemorySnapshot) {
        let session_id = snapshot.identity.session_id.clone();
        let node_id = snapshot.identity.node_id.clone();
        let mut node_memories = self.node_memories.write().await;
        node_memories
            .entry(session_id)
            .or_default()
            .insert(node_id, snapshot);
    }

    pub(crate) async fn node_memory_snapshots(
        &self,
        workflow_session_id: &str,
    ) -> Vec<NodeMemorySnapshot> {
        let mut snapshots = self
            .node_memories
            .read()
            .await
            .get(workflow_session_id)
            .map(|records| records.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        snapshots.sort_by(|left, right| left.identity.node_id.cmp(&right.identity.node_id));
        snapshots
    }

    pub(crate) async fn clear_node_memory(&self, workflow_session_id: &str) {
        self.node_memories.write().await.remove(workflow_session_id);
        self.checkpoints.write().await.remove(workflow_session_id);
    }

    pub(crate) async fn reconcile_node_memory(
        &self,
        workflow_session_id: &str,
        memory_impact: &GraphMemoryImpactSummary,
    ) {
        let mut node_memories = self.node_memories.write().await;
        let Some(session_memories) = node_memories.get_mut(workflow_session_id) else {
            return;
        };

        let mut removals = Vec::new();
        for decision in &memory_impact.node_decisions {
            match decision.compatibility {
                NodeMemoryCompatibility::PreserveAsIs => {}
                NodeMemoryCompatibility::PreserveWithInputRefresh
                | NodeMemoryCompatibility::FallbackFullInvalidation => {
                    if let Some(snapshot) = session_memories.get_mut(&decision.node_id) {
                        snapshot.status = NodeMemoryStatus::Invalidated;
                    }
                }
                NodeMemoryCompatibility::DropOnIdentityChange
                | NodeMemoryCompatibility::DropOnSchemaIncompatibility => {
                    removals.push(decision.node_id.clone());
                }
            }
        }

        for node_id in removals {
            session_memories.remove(&node_id);
        }

        let remove_checkpoint = session_memories.is_empty();
        if remove_checkpoint {
            node_memories.remove(workflow_session_id);
        }
        drop(node_memories);

        if remove_checkpoint {
            self.checkpoints.write().await.remove(workflow_session_id);
        }
    }

    pub(crate) async fn mark_checkpoint_available(&self, workflow_session_id: &str) {
        self.checkpoints.write().await.insert(
            workflow_session_id.to_string(),
            crate::events::unix_timestamp_ms(),
        );
    }

    pub(crate) async fn clear_checkpoint(&self, workflow_session_id: &str) {
        self.checkpoints.write().await.remove(workflow_session_id);
    }

    pub(crate) async fn checkpoint_summary(
        &self,
        workflow_session_id: &str,
        graph_revision: &str,
    ) -> WorkflowSessionCheckpointSummary {
        let preserved_node_count = self.node_memory_snapshots(workflow_session_id).await.len();
        let mut summary = WorkflowSessionCheckpointSummary::unavailable(
            workflow_session_id,
            graph_revision,
            self.residency().await,
        );
        summary.preserved_node_count = preserved_node_count;
        if let Some(checkpointed_at_ms) = self
            .checkpoints
            .read()
            .await
            .get(workflow_session_id)
            .copied()
        {
            summary.checkpoint_available = true;
            summary.checkpointed_at_ms = Some(checkpointed_at_ms);
        }
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GraphMemoryImpactSummary, NodeMemoryCompatibility, NodeMemoryCompatibilitySnapshot,
        NodeMemoryIdentity, NodeMemoryIndirectStateReference, NodeMemoryRestoreStrategy,
        NodeMemorySnapshot, NodeMemoryStatus, WorkflowExecutorSessionState,
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
    fn dirty_task_ids_only_reports_non_preserved_nodes() {
        let impact = GraphMemoryImpactSummary {
            node_decisions: vec![
                NodeMemoryCompatibilitySnapshot {
                    node_id: "input".to_string(),
                    compatibility: NodeMemoryCompatibility::PreserveAsIs,
                    reason: None,
                },
                NodeMemoryCompatibilitySnapshot {
                    node_id: "merge".to_string(),
                    compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                    reason: Some("input_changed".to_string()),
                },
                NodeMemoryCompatibilitySnapshot {
                    node_id: "output".to_string(),
                    compatibility: NodeMemoryCompatibility::DropOnSchemaIncompatibility,
                    reason: Some("schema_changed".to_string()),
                },
                NodeMemoryCompatibilitySnapshot {
                    node_id: "merge".to_string(),
                    compatibility: NodeMemoryCompatibility::FallbackFullInvalidation,
                    reason: Some("fallback".to_string()),
                },
            ],
            fallback_to_full_invalidation: true,
        };

        assert_eq!(
            impact.dirty_task_ids(),
            vec!["merge".to_string(), "output".to_string()]
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

    #[test]
    fn indirect_state_reference_serializes_restore_rule() {
        let snapshot = NodeMemorySnapshot {
            identity: NodeMemoryIdentity {
                session_id: "session-1".to_string(),
                node_id: "node-a".to_string(),
                node_type: "stream".to_string(),
                schema_version: Some("v1".to_string()),
            },
            status: NodeMemoryStatus::Ready,
            input_fingerprint: Some("fp-a".to_string()),
            output_snapshot: None,
            private_state: Some(serde_json::json!({ "cursor": 12 })),
            indirect_state_reference: Some(NodeMemoryIndirectStateReference {
                reference_kind: "kv-cache-segment".to_string(),
                reference_id: "cache-segment-1".to_string(),
                restore_strategy: NodeMemoryRestoreStrategy::RehydrateBeforeResume,
                inspection_metadata: Some(serde_json::json!({ "host": "gateway-1" })),
            }),
            inspection_metadata: Some(serde_json::json!({ "label": "Stream" })),
        };

        let serialized = serde_json::to_value(&snapshot).expect("serialize snapshot");
        assert_eq!(
            serialized,
            serde_json::json!({
                "identity": {
                    "session_id": "session-1",
                    "node_id": "node-a",
                    "node_type": "stream",
                    "schema_version": "v1"
                },
                "status": "ready",
                "input_fingerprint": "fp-a",
                "private_state": { "cursor": 12 },
                "indirect_state_reference": {
                    "reference_kind": "kv-cache-segment",
                    "reference_id": "cache-segment-1",
                    "restore_strategy": "rehydrate_before_resume",
                    "inspection_metadata": { "host": "gateway-1" }
                },
                "inspection_metadata": { "label": "Stream" }
            })
        );
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
    async fn executor_session_state_tracks_bound_workflow_session_identity() {
        let state = WorkflowExecutorSessionState::new();

        assert_eq!(state.bound_workflow_session_id().await, None);
        state.bind_workflow_session("session-1".to_string()).await;
        assert_eq!(
            state.bound_workflow_session_id().await,
            Some("session-1".to_string())
        );
        state.clear_bound_workflow_session().await;
        assert_eq!(state.bound_workflow_session_id().await, None);
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

    #[tokio::test]
    async fn executor_session_state_marks_checkpoint_availability() {
        let state = WorkflowExecutorSessionState::new();

        state.mark_checkpoint_available("session-1").await;
        let summary = state.checkpoint_summary("session-1", "graph-1").await;

        assert!(summary.checkpoint_available);
        assert!(summary.checkpointed_at_ms.is_some());

        state.clear_checkpoint("session-1").await;
        let cleared = state.checkpoint_summary("session-1", "graph-1").await;
        assert!(!cleared.checkpoint_available);
        assert_eq!(cleared.checkpointed_at_ms, None);
    }

    #[tokio::test]
    async fn executor_session_state_keeps_node_memory_isolated_per_session() {
        let state = WorkflowExecutorSessionState::new();
        state
            .record_node_memory(NodeMemorySnapshot {
                identity: NodeMemoryIdentity {
                    session_id: "session-1".to_string(),
                    node_id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    schema_version: Some("v1".to_string()),
                },
                status: NodeMemoryStatus::Ready,
                input_fingerprint: Some("fp-a".to_string()),
                output_snapshot: Some(serde_json::json!({ "text": "alpha" })),
                private_state: Some(serde_json::json!({ "cursor": 1 })),
                indirect_state_reference: Some(NodeMemoryIndirectStateReference {
                    reference_kind: "runtime-slot".to_string(),
                    reference_id: "slot-a".to_string(),
                    restore_strategy: NodeMemoryRestoreStrategy::RebindHostResource,
                    inspection_metadata: None,
                }),
                inspection_metadata: Some(serde_json::json!({ "label": "Alpha" })),
            })
            .await;
        state
            .record_node_memory(NodeMemorySnapshot {
                identity: NodeMemoryIdentity {
                    session_id: "session-2".to_string(),
                    node_id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    schema_version: Some("v1".to_string()),
                },
                status: NodeMemoryStatus::Ready,
                input_fingerprint: Some("fp-b".to_string()),
                output_snapshot: Some(serde_json::json!({ "text": "beta" })),
                private_state: None,
                indirect_state_reference: None,
                inspection_metadata: None,
            })
            .await;

        let session_1 = state.node_memory_snapshots("session-1").await;
        let session_2 = state.node_memory_snapshots("session-2").await;
        assert_eq!(session_1.len(), 1);
        assert_eq!(session_2.len(), 1);
        assert_eq!(session_1[0].identity.session_id, "session-1");
        assert_eq!(session_2[0].identity.session_id, "session-2");
        assert_eq!(
            session_1[0].output_snapshot,
            Some(serde_json::json!({ "text": "alpha" }))
        );
        assert_eq!(
            session_2[0].output_snapshot,
            Some(serde_json::json!({ "text": "beta" }))
        );
    }

    #[tokio::test]
    async fn executor_session_state_checkpoint_summary_counts_preserved_node_memory() {
        let state = WorkflowExecutorSessionState::new();
        state
            .record_node_memory(NodeMemorySnapshot {
                identity: NodeMemoryIdentity {
                    session_id: "session-1".to_string(),
                    node_id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    schema_version: None,
                },
                status: NodeMemoryStatus::Ready,
                input_fingerprint: None,
                output_snapshot: None,
                private_state: None,
                indirect_state_reference: None,
                inspection_metadata: None,
            })
            .await;
        state
            .record_node_memory(NodeMemorySnapshot {
                identity: NodeMemoryIdentity {
                    session_id: "session-1".to_string(),
                    node_id: "node-b".to_string(),
                    node_type: "text-output".to_string(),
                    schema_version: None,
                },
                status: NodeMemoryStatus::Ready,
                input_fingerprint: None,
                output_snapshot: None,
                private_state: None,
                indirect_state_reference: None,
                inspection_metadata: None,
            })
            .await;

        let summary = state.checkpoint_summary("session-1", "graph-1").await;
        assert_eq!(summary.preserved_node_count, 2);
        assert!(!summary.checkpoint_available);

        state.clear_node_memory("session-1").await;
        let cleared_summary = state.checkpoint_summary("session-1", "graph-1").await;
        assert_eq!(cleared_summary.preserved_node_count, 0);
    }

    #[tokio::test]
    async fn executor_session_state_reconciles_node_memory_status_and_removals() {
        let state = WorkflowExecutorSessionState::new();
        for node_id in ["node-a", "node-b", "node-c"] {
            state
                .record_node_memory(NodeMemorySnapshot {
                    identity: NodeMemoryIdentity {
                        session_id: "session-1".to_string(),
                        node_id: node_id.to_string(),
                        node_type: "text-node".to_string(),
                        schema_version: Some("v1".to_string()),
                    },
                    status: NodeMemoryStatus::Ready,
                    input_fingerprint: Some(format!("fp-{node_id}")),
                    output_snapshot: Some(serde_json::json!({ "node": node_id })),
                    private_state: None,
                    indirect_state_reference: None,
                    inspection_metadata: None,
                })
                .await;
        }

        state
            .reconcile_node_memory(
                "session-1",
                &GraphMemoryImpactSummary {
                    node_decisions: vec![
                        NodeMemoryCompatibilitySnapshot {
                            node_id: "node-a".to_string(),
                            compatibility: NodeMemoryCompatibility::PreserveAsIs,
                            reason: Some("unchanged".to_string()),
                        },
                        NodeMemoryCompatibilitySnapshot {
                            node_id: "node-b".to_string(),
                            compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                            reason: Some("upstream_input_changed".to_string()),
                        },
                        NodeMemoryCompatibilitySnapshot {
                            node_id: "node-c".to_string(),
                            compatibility: NodeMemoryCompatibility::DropOnIdentityChange,
                            reason: Some("node_removed".to_string()),
                        },
                    ],
                    fallback_to_full_invalidation: false,
                },
            )
            .await;

        let snapshots = state.node_memory_snapshots("session-1").await;
        assert_eq!(
            snapshots
                .iter()
                .map(|snapshot| snapshot.identity.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["node-a", "node-b"]
        );
        assert_eq!(snapshots[0].status, NodeMemoryStatus::Ready);
        assert_eq!(snapshots[1].status, NodeMemoryStatus::Invalidated);
        assert_eq!(
            snapshots[1].output_snapshot,
            Some(serde_json::json!({ "node": "node-b" }))
        );
    }

    #[tokio::test]
    async fn executor_session_state_fallback_reconciliation_invalidates_remaining_nodes() {
        let state = WorkflowExecutorSessionState::new();
        state
            .record_node_memory(NodeMemorySnapshot {
                identity: NodeMemoryIdentity {
                    session_id: "session-1".to_string(),
                    node_id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    schema_version: Some("v1".to_string()),
                },
                status: NodeMemoryStatus::Ready,
                input_fingerprint: Some("fp-a".to_string()),
                output_snapshot: Some(serde_json::json!({ "text": "alpha" })),
                private_state: None,
                indirect_state_reference: None,
                inspection_metadata: None,
            })
            .await;

        state
            .reconcile_node_memory(
                "session-1",
                &GraphMemoryImpactSummary::fallback_full_invalidation(
                    ["node-a", "node-missing"],
                    "graph_edit_not_proven",
                ),
            )
            .await;

        let snapshots = state.node_memory_snapshots("session-1").await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].identity.node_id, "node-a");
        assert_eq!(snapshots[0].status, NodeMemoryStatus::Invalidated);
    }

    #[tokio::test]
    async fn executor_session_state_reconciliation_drops_empty_session_bucket() {
        let state = WorkflowExecutorSessionState::new();
        state
            .record_node_memory(NodeMemorySnapshot {
                identity: NodeMemoryIdentity {
                    session_id: "session-1".to_string(),
                    node_id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    schema_version: Some("v1".to_string()),
                },
                status: NodeMemoryStatus::Ready,
                input_fingerprint: None,
                output_snapshot: None,
                private_state: None,
                indirect_state_reference: None,
                inspection_metadata: None,
            })
            .await;

        state
            .reconcile_node_memory(
                "session-1",
                &GraphMemoryImpactSummary {
                    node_decisions: vec![NodeMemoryCompatibilitySnapshot {
                        node_id: "node-a".to_string(),
                        compatibility: NodeMemoryCompatibility::DropOnSchemaIncompatibility,
                        reason: Some("schema_version_changed".to_string()),
                    }],
                    fallback_to_full_invalidation: false,
                },
            )
            .await;

        assert!(state.node_memory_snapshots("session-1").await.is_empty());
        assert_eq!(
            state
                .checkpoint_summary("session-1", "graph-1")
                .await
                .preserved_node_count,
            0
        );
    }
}
