use node_engine::{
    GraphMemoryImpactSummary, NodeMemorySnapshot, WorkflowEvent, WorkflowSessionCheckpointSummary,
    WorkflowSessionResidencyState,
};

pub const PHASE6_SESSION_STATE_CONTRACT_VERSION: u32 = 1;
const PHASE6_FALLBACK_INVALIDATION_REASON: &str =
    "phase_6_graph_reconciliation_not_implemented_yet";

/// Backend-owned workflow-session state inputs that can be projected onto the
/// additive graph-session response contract.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkflowGraphSessionStateProjection {
    pub residency: WorkflowSessionResidencyState,
    pub node_memory: Vec<NodeMemorySnapshot>,
    pub memory_impact: Option<GraphMemoryImpactSummary>,
    pub checkpoint: Option<WorkflowSessionCheckpointSummary>,
}

impl Default for WorkflowGraphSessionStateProjection {
    fn default() -> Self {
        Self {
            residency: WorkflowSessionResidencyState::Active,
            node_memory: Vec::new(),
            memory_impact: None,
            checkpoint: None,
        }
    }
}

/// Additive Phase 6 workflow-session state view carried on graph edit-session
/// snapshots.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphSessionStateView {
    pub contract_version: u32,
    pub residency: WorkflowSessionResidencyState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_memory: Vec<NodeMemorySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_impact: Option<GraphMemoryImpactSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<WorkflowSessionCheckpointSummary>,
}

impl WorkflowGraphSessionStateView {
    pub fn new(
        residency: WorkflowSessionResidencyState,
        node_memory: Vec<NodeMemorySnapshot>,
        memory_impact: Option<GraphMemoryImpactSummary>,
        checkpoint: Option<WorkflowSessionCheckpointSummary>,
    ) -> Self {
        Self {
            contract_version: PHASE6_SESSION_STATE_CONTRACT_VERSION,
            residency,
            node_memory,
            memory_impact,
            checkpoint,
        }
    }
}

/// Canonical graph snapshot response for edit-session transport surfaces.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionGraphResponse {
    pub session_id: String,
    pub graph_revision: String,
    pub graph: super::types::WorkflowGraph,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_event: Option<WorkflowEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_session_state: Option<WorkflowGraphSessionStateView>,
}

#[cfg(test)]
fn build_graph_session_response(
    session_id: &str,
    graph: &super::types::WorkflowGraph,
    workflow_event: Option<WorkflowEvent>,
) -> WorkflowGraphEditSessionGraphResponse {
    build_graph_session_response_with_state(session_id, graph, workflow_event, None)
}

pub(crate) fn build_graph_session_response_with_state(
    session_id: &str,
    graph: &super::types::WorkflowGraph,
    workflow_event: Option<WorkflowEvent>,
    projection: Option<&WorkflowGraphSessionStateProjection>,
) -> WorkflowGraphEditSessionGraphResponse {
    let graph_revision = graph.compute_fingerprint();
    WorkflowGraphEditSessionGraphResponse {
        session_id: session_id.to_string(),
        graph_revision: graph_revision.clone(),
        graph: graph.clone(),
        workflow_event: workflow_event.clone(),
        workflow_session_state: Some(build_workflow_session_state_view(
            session_id,
            &graph_revision,
            workflow_event.as_ref(),
            projection,
        )),
    }
}

pub(crate) fn build_workflow_session_state_view(
    session_id: &str,
    graph_revision: &str,
    workflow_event: Option<&WorkflowEvent>,
    projection: Option<&WorkflowGraphSessionStateProjection>,
) -> WorkflowGraphSessionStateView {
    let projection = projection.cloned().unwrap_or_default();
    let memory_impact = resolve_workflow_session_memory_impact(workflow_event, Some(&projection));
    WorkflowGraphSessionStateView::new(
        projection.residency.clone(),
        projection.node_memory,
        memory_impact,
        projection.checkpoint.or_else(|| {
            Some(WorkflowSessionCheckpointSummary::unavailable(
                session_id,
                graph_revision,
                projection.residency,
            ))
        }),
    )
}

pub(crate) fn resolve_workflow_session_memory_impact(
    workflow_event: Option<&WorkflowEvent>,
    projection: Option<&WorkflowGraphSessionStateProjection>,
) -> Option<GraphMemoryImpactSummary> {
    projection
        .and_then(|projection| projection.memory_impact.clone())
        .or_else(|| graph_memory_impact_from_event(workflow_event))
}

fn graph_memory_impact_from_event(
    workflow_event: Option<&WorkflowEvent>,
) -> Option<GraphMemoryImpactSummary> {
    match workflow_event {
        Some(WorkflowEvent::GraphModified {
            memory_impact: Some(memory_impact),
            ..
        }) => Some(memory_impact.clone()),
        Some(WorkflowEvent::GraphModified { dirty_tasks, .. }) if !dirty_tasks.is_empty() => {
            Some(GraphMemoryImpactSummary::fallback_full_invalidation(
                dirty_tasks.iter().cloned(),
                PHASE6_FALLBACK_INVALIDATION_REASON,
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::WorkflowGraph;
    use super::{
        build_graph_session_response, build_graph_session_response_with_state,
        build_workflow_session_state_view, WorkflowGraphSessionStateProjection,
        WorkflowGraphSessionStateView, PHASE6_FALLBACK_INVALIDATION_REASON,
        PHASE6_SESSION_STATE_CONTRACT_VERSION,
    };
    use node_engine::{
        NodeMemoryCompatibility, NodeMemoryIdentity, NodeMemorySnapshot, NodeMemoryStatus,
        WorkflowEvent, WorkflowSessionCheckpointSummary, WorkflowSessionResidencyState,
    };

    #[test]
    fn state_view_defaults_to_active_without_memory() {
        let view = build_workflow_session_state_view("session-1", "graph-rev-1", None, None);

        assert_eq!(
            view,
            WorkflowGraphSessionStateView {
                contract_version: PHASE6_SESSION_STATE_CONTRACT_VERSION,
                residency: WorkflowSessionResidencyState::Active,
                node_memory: Vec::new(),
                memory_impact: None,
                checkpoint: Some(node_engine::WorkflowSessionCheckpointSummary::unavailable(
                    "session-1",
                    "graph-rev-1",
                    WorkflowSessionResidencyState::Active,
                )),
            }
        );
    }

    #[test]
    fn graph_modified_event_maps_to_fallback_full_invalidation() {
        let event = WorkflowEvent::GraphModified {
            workflow_id: "session-1".to_string(),
            execution_id: "session-1".to_string(),
            dirty_tasks: vec!["input".to_string(), "output".to_string()],
            memory_impact: None,
            occurred_at_ms: Some(123),
        };

        let view =
            build_workflow_session_state_view("session-1", "graph-rev-1", Some(&event), None);
        let impact = view.memory_impact.expect("memory impact");

        assert!(impact.fallback_to_full_invalidation);
        assert_eq!(impact.node_decisions.len(), 2);
        assert!(impact.node_decisions.iter().all(|decision| {
            decision.compatibility == NodeMemoryCompatibility::FallbackFullInvalidation
                && decision.reason.as_deref() == Some(PHASE6_FALLBACK_INVALIDATION_REASON)
        }));
    }

    #[test]
    fn graph_session_response_builds_revision_and_state_view() {
        let graph = WorkflowGraph::new();
        let response = build_graph_session_response("session-1", &graph, None);

        assert_eq!(response.session_id, "session-1");
        assert_eq!(response.graph, graph);
        assert!(response.workflow_session_state.is_some());
    }

    #[test]
    fn state_view_projects_explicit_backend_node_memory_and_checkpoint() {
        let projection = WorkflowGraphSessionStateProjection {
            residency: WorkflowSessionResidencyState::Warm,
            node_memory: vec![NodeMemorySnapshot {
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
                indirect_state_reference: None,
                inspection_metadata: Some(serde_json::json!({ "label": "Alpha" })),
            }],
            memory_impact: None,
            checkpoint: Some(WorkflowSessionCheckpointSummary {
                session_id: "session-1".to_string(),
                graph_revision: "graph-rev-1".to_string(),
                residency: WorkflowSessionResidencyState::Warm,
                checkpoint_available: false,
                preserved_node_count: 1,
                checkpointed_at_ms: None,
            }),
        };

        let graph = WorkflowGraph::new();
        let response =
            build_graph_session_response_with_state("session-1", &graph, None, Some(&projection));
        let view = response
            .workflow_session_state
            .expect("workflow session state");

        assert_eq!(view.residency, WorkflowSessionResidencyState::Warm);
        assert_eq!(view.node_memory.len(), 1);
        assert_eq!(view.node_memory[0].identity.node_id, "node-a");
        assert_eq!(view.checkpoint, projection.checkpoint);
    }
}
