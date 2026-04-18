use node_engine::{
    GraphMemoryImpactSummary, NodeMemorySnapshot, WorkflowEvent, WorkflowSessionCheckpointSummary,
    WorkflowSessionResidencyState,
};

const PHASE6_SESSION_STATE_CONTRACT_VERSION: u32 = 1;
const PHASE6_FALLBACK_INVALIDATION_REASON: &str =
    "phase_6_graph_reconciliation_not_implemented_yet";

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

pub(crate) fn build_graph_session_response(
    session_id: &str,
    graph: &super::types::WorkflowGraph,
    workflow_event: Option<WorkflowEvent>,
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
        )),
    }
}

pub(crate) fn build_workflow_session_state_view(
    session_id: &str,
    graph_revision: &str,
    workflow_event: Option<&WorkflowEvent>,
) -> WorkflowGraphSessionStateView {
    WorkflowGraphSessionStateView {
        contract_version: PHASE6_SESSION_STATE_CONTRACT_VERSION,
        residency: WorkflowSessionResidencyState::Active,
        node_memory: Vec::new(),
        memory_impact: graph_memory_impact_from_event(workflow_event),
        checkpoint: Some(WorkflowSessionCheckpointSummary::unavailable(
            session_id,
            graph_revision,
            WorkflowSessionResidencyState::Active,
        )),
    }
}

fn graph_memory_impact_from_event(
    workflow_event: Option<&WorkflowEvent>,
) -> Option<GraphMemoryImpactSummary> {
    match workflow_event {
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
        PHASE6_FALLBACK_INVALIDATION_REASON, PHASE6_SESSION_STATE_CONTRACT_VERSION,
        WorkflowGraphSessionStateView, build_graph_session_response,
        build_workflow_session_state_view,
    };
    use node_engine::{NodeMemoryCompatibility, WorkflowEvent, WorkflowSessionResidencyState};

    #[test]
    fn state_view_defaults_to_active_without_memory() {
        let view = build_workflow_session_state_view("session-1", "graph-rev-1", None);

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
            occurred_at_ms: Some(123),
        };

        let view = build_workflow_session_state_view("session-1", "graph-rev-1", Some(&event));
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
}
