use std::collections::{BTreeSet, HashMap};

use node_engine::{
    GraphMemoryImpactSummary, NodeMemoryCompatibility, NodeMemoryCompatibilitySnapshot,
};

use super::types::WorkflowGraph;
use super::{GraphEdge, GraphNode, Position};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeGraphChangeKind {
    Added,
    Removed,
    TypeChanged,
    SchemaChanged,
    DataChanged,
    TopologyChanged,
    Unchanged,
}

pub(crate) fn graph_memory_impact_from_graph_change(
    before: &WorkflowGraph,
    after: &WorkflowGraph,
    candidate_node_ids: &[String],
) -> Option<GraphMemoryImpactSummary> {
    let candidate_node_ids = candidate_node_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if candidate_node_ids.is_empty() {
        return None;
    }

    let node_change_kinds = candidate_node_ids
        .iter()
        .map(|node_id| {
            (
                node_id.clone(),
                node_graph_change_kind(before, after, node_id.as_str()),
            )
        })
        .collect::<HashMap<_, _>>();

    let node_decisions = candidate_node_ids
        .into_iter()
        .map(|node_id| {
            compatibility_snapshot_for_node(before, after, &node_change_kinds, node_id.as_str())
        })
        .collect::<Vec<_>>();

    Some(GraphMemoryImpactSummary {
        fallback_to_full_invalidation: node_decisions.iter().any(|decision| {
            decision.compatibility == NodeMemoryCompatibility::FallbackFullInvalidation
        }),
        node_decisions,
    })
}

pub fn graph_memory_impact_from_node_engine_graph_change(
    before: &node_engine::WorkflowGraph,
    after: &node_engine::WorkflowGraph,
) -> Option<GraphMemoryImpactSummary> {
    let before_graph = workflow_graph_from_node_engine(before);
    let after_graph = workflow_graph_from_node_engine(after);
    let mut candidate_node_ids = before
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .chain(after.nodes.iter().map(|node| node.id.clone()))
        .collect::<Vec<_>>();
    candidate_node_ids.sort();
    candidate_node_ids.dedup();

    graph_memory_impact_from_graph_change(&before_graph, &after_graph, &candidate_node_ids)
}

fn workflow_graph_from_node_engine(graph: &node_engine::WorkflowGraph) -> WorkflowGraph {
    WorkflowGraph {
        nodes: graph
            .nodes
            .iter()
            .map(|node| GraphNode {
                id: node.id.clone(),
                node_type: node.node_type.clone(),
                position: Position {
                    x: node.position.0,
                    y: node.position.1,
                },
                data: node.data.clone(),
            })
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|edge| GraphEdge {
                id: edge.id.clone(),
                source: edge.source.clone(),
                source_handle: edge.source_handle.clone(),
                target: edge.target.clone(),
                target_handle: edge.target_handle.clone(),
            })
            .collect(),
        derived_graph: None,
    }
}

fn compatibility_snapshot_for_node(
    before: &WorkflowGraph,
    after: &WorkflowGraph,
    node_change_kinds: &HashMap<String, NodeGraphChangeKind>,
    node_id: &str,
) -> NodeMemoryCompatibilitySnapshot {
    let (compatibility, reason) = match node_change_kinds
        .get(node_id)
        .copied()
        .unwrap_or(NodeGraphChangeKind::Unchanged)
    {
        NodeGraphChangeKind::Added => (
            NodeMemoryCompatibility::DropOnIdentityChange,
            "node_added".to_string(),
        ),
        NodeGraphChangeKind::Removed => (
            NodeMemoryCompatibility::DropOnIdentityChange,
            "node_removed".to_string(),
        ),
        NodeGraphChangeKind::TypeChanged => (
            NodeMemoryCompatibility::DropOnIdentityChange,
            "node_type_changed".to_string(),
        ),
        NodeGraphChangeKind::SchemaChanged => (
            NodeMemoryCompatibility::DropOnSchemaIncompatibility,
            "schema_version_changed".to_string(),
        ),
        NodeGraphChangeKind::DataChanged => (
            NodeMemoryCompatibility::PreserveWithInputRefresh,
            "node_data_changed".to_string(),
        ),
        NodeGraphChangeKind::TopologyChanged => (
            NodeMemoryCompatibility::PreserveWithInputRefresh,
            "edge_topology_changed".to_string(),
        ),
        NodeGraphChangeKind::Unchanged => {
            if node_has_changed_dependency(after, node_id, node_change_kinds) {
                (
                    NodeMemoryCompatibility::PreserveWithInputRefresh,
                    "upstream_dependency_changed".to_string(),
                )
            } else if before.find_node(node_id).is_some() || after.find_node(node_id).is_some() {
                (
                    NodeMemoryCompatibility::PreserveAsIs,
                    "graph_edit_preserves_node_identity".to_string(),
                )
            } else {
                (
                    NodeMemoryCompatibility::FallbackFullInvalidation,
                    "node_missing_from_graph_change".to_string(),
                )
            }
        }
    };

    NodeMemoryCompatibilitySnapshot {
        node_id: node_id.to_string(),
        compatibility,
        reason: Some(reason),
    }
}

fn node_has_changed_dependency(
    graph: &WorkflowGraph,
    node_id: &str,
    node_change_kinds: &HashMap<String, NodeGraphChangeKind>,
) -> bool {
    incoming_dependency_ids(graph, node_id)
        .into_iter()
        .any(|dependency_id| {
            node_change_kinds
                .get(&dependency_id)
                .map(|change_kind| *change_kind != NodeGraphChangeKind::Unchanged)
                .unwrap_or(false)
        })
}

fn node_graph_change_kind(
    before: &WorkflowGraph,
    after: &WorkflowGraph,
    node_id: &str,
) -> NodeGraphChangeKind {
    match (before.find_node(node_id), after.find_node(node_id)) {
        (None, Some(_)) => NodeGraphChangeKind::Added,
        (Some(_), None) => NodeGraphChangeKind::Removed,
        (None, None) => NodeGraphChangeKind::Unchanged,
        (Some(before_node), Some(after_node)) => {
            if before_node.node_type != after_node.node_type {
                return NodeGraphChangeKind::TypeChanged;
            }

            if node_schema_version(&before_node.data) != node_schema_version(&after_node.data) {
                return NodeGraphChangeKind::SchemaChanged;
            }

            if incoming_edge_signatures(before, node_id) != incoming_edge_signatures(after, node_id)
            {
                return NodeGraphChangeKind::TopologyChanged;
            }

            if before_node.data != after_node.data {
                return NodeGraphChangeKind::DataChanged;
            }

            NodeGraphChangeKind::Unchanged
        }
    }
}

fn incoming_dependency_ids(graph: &WorkflowGraph, node_id: &str) -> BTreeSet<String> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.target == node_id)
        .map(|edge| edge.source.clone())
        .collect()
}

fn incoming_edge_signatures(
    graph: &WorkflowGraph,
    node_id: &str,
) -> BTreeSet<(String, String, String)> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.target == node_id)
        .map(|edge| {
            (
                edge.source.clone(),
                edge.source_handle.clone(),
                edge.target_handle.clone(),
            )
        })
        .collect()
}

fn node_schema_version(node_data: &serde_json::Value) -> Option<String> {
    node_data
        .get("definition")
        .and_then(|definition| {
            definition
                .get("schema_version")
                .and_then(|value| value.as_str())
                .or_else(|| {
                    definition
                        .get("schemaVersion")
                        .and_then(|value| value.as_str())
                })
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use node_engine::NodeMemoryCompatibility;

    use super::*;
    use crate::graph::types::{GraphEdge, GraphNode, Position};

    fn sample_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "input".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({
                        "text": "hello",
                        "definition": { "schema_version": "v1" }
                    }),
                },
                GraphNode {
                    id: "output".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 120.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "input".to_string(),
                source_handle: "text".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            }],
            derived_graph: None,
        }
    }

    #[test]
    fn data_change_preserves_node_and_refreshes_dependents() {
        let before = sample_graph();
        let mut after = sample_graph();
        after.find_node_mut("input").expect("input").data["text"] = serde_json::json!("updated");

        let impact = graph_memory_impact_from_graph_change(
            &before,
            &after,
            &["input".to_string(), "output".to_string()],
        )
        .expect("impact");

        assert_eq!(impact.node_decisions.len(), 2);
        assert_eq!(
            impact.node_decisions[0].compatibility,
            NodeMemoryCompatibility::PreserveWithInputRefresh
        );
        assert_eq!(
            impact.node_decisions[0].reason.as_deref(),
            Some("node_data_changed")
        );
        assert_eq!(
            impact.node_decisions[1].compatibility,
            NodeMemoryCompatibility::PreserveWithInputRefresh
        );
        assert_eq!(
            impact.node_decisions[1].reason.as_deref(),
            Some("upstream_dependency_changed")
        );
        assert!(!impact.fallback_to_full_invalidation);
    }

    #[test]
    fn removed_node_drops_identity() {
        let before = sample_graph();
        let mut after = sample_graph();
        after.nodes.retain(|node| node.id != "output");
        after.edges.clear();

        let impact =
            graph_memory_impact_from_graph_change(&before, &after, &["output".to_string()])
                .expect("impact");

        assert_eq!(impact.node_decisions.len(), 1);
        assert_eq!(
            impact.node_decisions[0].compatibility,
            NodeMemoryCompatibility::DropOnIdentityChange
        );
        assert_eq!(
            impact.node_decisions[0].reason.as_deref(),
            Some("node_removed")
        );
    }

    #[test]
    fn schema_change_drops_memory_for_incompatible_node() {
        let before = sample_graph();
        let mut after = sample_graph();
        after.find_node_mut("input").expect("input").data["definition"]["schema_version"] =
            serde_json::json!("v2");

        let impact = graph_memory_impact_from_graph_change(
            &before,
            &after,
            &["input".to_string(), "output".to_string()],
        )
        .expect("impact");

        assert_eq!(
            impact.node_decisions[0].compatibility,
            NodeMemoryCompatibility::DropOnSchemaIncompatibility
        );
        assert_eq!(
            impact.node_decisions[1].compatibility,
            NodeMemoryCompatibility::PreserveWithInputRefresh
        );
    }

    #[test]
    fn edge_change_refreshes_target() {
        let before = sample_graph();
        let mut after = sample_graph();
        after.edges.push(GraphEdge {
            id: "extra-edge".to_string(),
            source: "input".to_string(),
            source_handle: "text".to_string(),
            target: "output".to_string(),
            target_handle: "alt".to_string(),
        });

        let impact =
            graph_memory_impact_from_graph_change(&before, &after, &["output".to_string()])
                .expect("impact");

        assert_eq!(
            impact.node_decisions[0].compatibility,
            NodeMemoryCompatibility::PreserveWithInputRefresh
        );
        assert_eq!(
            impact.node_decisions[0].reason.as_deref(),
            Some("edge_topology_changed")
        );
    }

    #[test]
    fn node_engine_graph_change_reuses_the_same_compatibility_rules() {
        let before = node_engine::WorkflowGraph {
            id: "wf".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                node_engine::GraphNode {
                    id: "input".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({
                        "text": "alpha",
                        "definition": { "schema_version": "v1" }
                    }),
                    position: (0.0, 0.0),
                },
                node_engine::GraphNode {
                    id: "output".to_string(),
                    node_type: "text-output".to_string(),
                    data: serde_json::json!({
                        "definition": { "schema_version": "v1" }
                    }),
                    position: (1.0, 0.0),
                },
            ],
            edges: vec![node_engine::GraphEdge {
                id: "edge".to_string(),
                source: "input".to_string(),
                source_handle: "text".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            }],
            groups: Vec::new(),
        };
        let after = node_engine::WorkflowGraph {
            id: "wf".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                node_engine::GraphNode {
                    id: "input".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({
                        "text": "beta",
                        "definition": { "schema_version": "v1" }
                    }),
                    position: (0.0, 0.0),
                },
                node_engine::GraphNode {
                    id: "output".to_string(),
                    node_type: "text-output".to_string(),
                    data: serde_json::json!({
                        "definition": { "schema_version": "v1" }
                    }),
                    position: (1.0, 0.0),
                },
            ],
            edges: vec![node_engine::GraphEdge {
                id: "edge".to_string(),
                source: "input".to_string(),
                source_handle: "text".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            }],
            groups: Vec::new(),
        };

        let impact = graph_memory_impact_from_node_engine_graph_change(&before, &after)
            .expect("memory impact");

        assert_eq!(impact.node_decisions.len(), 2);
        assert_eq!(
            impact.node_decisions[0],
            NodeMemoryCompatibilitySnapshot {
                node_id: "input".to_string(),
                compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                reason: Some("node_data_changed".to_string()),
            }
        );
        assert_eq!(
            impact.node_decisions[1],
            NodeMemoryCompatibilitySnapshot {
                node_id: "output".to_string(),
                compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                reason: Some("upstream_dependency_changed".to_string()),
            }
        );
        assert!(!impact.fallback_to_full_invalidation);
    }
}
