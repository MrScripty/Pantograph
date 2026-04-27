use std::collections::{BTreeMap, BTreeSet};

use pantograph_node_contracts::NodeBehaviorVersion;
use serde::{Deserialize, Serialize};

use super::types::{GraphEdge, WorkflowGraph};
use crate::workflow::WorkflowServiceError;

const EXECUTABLE_TOPOLOGY_SCHEMA_VERSION: u32 = 1;
const WORKFLOW_EXECUTION_FINGERPRINT_PREFIX: &str = "workflow-exec-blake3:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutableTopology {
    pub schema_version: u32,
    pub nodes: Vec<WorkflowExecutableTopologyNode>,
    pub edges: Vec<WorkflowExecutableTopologyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutableTopologyNode {
    pub node_id: String,
    pub node_type: String,
    pub contract_version: String,
    pub behavior_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowExecutableTopologyEdge {
    pub source_node_id: String,
    pub source_port_id: String,
    pub target_node_id: String,
    pub target_port_id: String,
}

pub fn workflow_executable_topology(
    graph: &WorkflowGraph,
) -> Result<WorkflowExecutableTopology, WorkflowServiceError> {
    let node_versions = workflow_nodes::builtin_node_contracts()
        .map_err(|error| {
            WorkflowServiceError::CapabilityViolation(format!(
                "failed to load built-in node behavior versions: {error}"
            ))
        })?
        .iter()
        .map(NodeBehaviorVersion::from_contract)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            WorkflowServiceError::CapabilityViolation(format!(
                "invalid built-in node behavior version: {error}"
            ))
        })?;
    workflow_executable_topology_with_node_versions(graph, node_versions)
}

pub fn workflow_executable_topology_with_node_versions(
    graph: &WorkflowGraph,
    node_versions: impl IntoIterator<Item = NodeBehaviorVersion>,
) -> Result<WorkflowExecutableTopology, WorkflowServiceError> {
    let node_version_by_type = node_versions
        .into_iter()
        .map(|version| (version.node_type.as_str().to_string(), version))
        .collect::<BTreeMap<_, _>>();
    let mut seen_node_ids = BTreeSet::new();
    let mut nodes = Vec::with_capacity(graph.nodes.len());

    for node in &graph.nodes {
        if !seen_node_ids.insert(node.id.clone()) {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "workflow executable topology has duplicate node id '{}'",
                node.id
            )));
        }
        let Some(version) = node_version_by_type.get(&node.node_type) else {
            return Err(WorkflowServiceError::CapabilityViolation(format!(
                "node '{}' of type '{}' does not provide behavior version facts",
                node.id, node.node_type
            )));
        };
        nodes.push(WorkflowExecutableTopologyNode {
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            contract_version: version.contract_version.clone(),
            behavior_digest: version.behavior_digest.clone(),
        });
    }
    nodes.sort_by(|left, right| {
        left.node_id
            .cmp(&right.node_id)
            .then_with(|| left.node_type.cmp(&right.node_type))
    });

    let mut edges = graph.edges.iter().map(executable_edge).collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.source_node_id
            .cmp(&right.source_node_id)
            .then_with(|| left.source_port_id.cmp(&right.source_port_id))
            .then_with(|| left.target_node_id.cmp(&right.target_node_id))
            .then_with(|| left.target_port_id.cmp(&right.target_port_id))
    });

    Ok(WorkflowExecutableTopology {
        schema_version: EXECUTABLE_TOPOLOGY_SCHEMA_VERSION,
        nodes,
        edges,
    })
}

pub fn workflow_execution_fingerprint(
    graph: &WorkflowGraph,
) -> Result<String, WorkflowServiceError> {
    let topology = workflow_executable_topology(graph)?;
    workflow_execution_fingerprint_for_topology(&topology)
}

pub fn workflow_execution_fingerprint_for_topology(
    topology: &WorkflowExecutableTopology,
) -> Result<String, WorkflowServiceError> {
    let bytes = serde_json::to_vec(topology).map_err(|error| {
        WorkflowServiceError::CapabilityViolation(format!(
            "failed to encode workflow executable topology: {error}"
        ))
    })?;
    Ok(format!(
        "{WORKFLOW_EXECUTION_FINGERPRINT_PREFIX}{}",
        blake3::hash(&bytes)
    ))
}

fn executable_edge(edge: &GraphEdge) -> WorkflowExecutableTopologyEdge {
    WorkflowExecutableTopologyEdge {
        source_node_id: edge.source.clone(),
        source_port_id: edge.source_handle.clone(),
        target_node_id: edge.target.clone(),
        target_port_id: edge.target_handle.clone(),
    }
}

#[cfg(test)]
mod tests {
    use pantograph_node_contracts::{NodeBehaviorVersion, NodeTypeId};

    use super::*;
    use crate::graph::{GraphEdge, GraphNode, Position};

    fn version(
        node_type: &str,
        contract_version: &str,
        behavior_digest: &str,
    ) -> NodeBehaviorVersion {
        NodeBehaviorVersion {
            node_type: NodeTypeId::try_from(node_type.to_string()).expect("node type"),
            contract_version: contract_version.to_string(),
            behavior_digest: behavior_digest.to_string(),
        }
    }

    fn graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "node-b".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 200.0, y: 50.0 },
                    data: serde_json::json!({"name": "Output"}),
                },
                GraphNode {
                    id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 10.0, y: 20.0 },
                    data: serde_json::json!({"value": "initial"}),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge-1".to_string(),
                source: "node-a".to_string(),
                source_handle: "text".to_string(),
                target: "node-b".to_string(),
                target_handle: "text".to_string(),
            }],
            derived_graph: None,
        }
    }

    fn versions() -> Vec<NodeBehaviorVersion> {
        vec![
            version("text-input", "1.0.0", "digest:input"),
            version("text-output", "1.0.0", "digest:output"),
        ]
    }

    #[test]
    fn executable_topology_is_sorted_and_excludes_display_metadata() {
        let mut left = graph();
        let mut right = graph();
        right.nodes.reverse();
        right.nodes[0].position = Position { x: 999.0, y: 888.0 };
        right.nodes[0].data = serde_json::json!({"value": "changed"});
        right.edges[0].id = "display-only-edge-id".to_string();

        let left_topology =
            workflow_executable_topology_with_node_versions(&left, versions()).expect("left");
        let right_topology =
            workflow_executable_topology_with_node_versions(&right, versions()).expect("right");

        assert_eq!(left_topology, right_topology);
        assert_eq!(left_topology.nodes[0].node_id, "node-a");
        assert_eq!(left_topology.edges[0].source_node_id, "node-a");
        left.derived_graph = Some(crate::graph::WorkflowDerivedGraph {
            schema_version: 1,
            graph_fingerprint: "display-cache".to_string(),
            consumer_count_map: Default::default(),
        });
        let viewport_topology =
            workflow_executable_topology_with_node_versions(&left, versions()).expect("viewport");
        assert_eq!(left_topology, viewport_topology);
    }

    #[test]
    fn execution_fingerprint_changes_when_node_behavior_changes() {
        let topology = workflow_executable_topology_with_node_versions(&graph(), versions())
            .expect("topology");
        let baseline =
            workflow_execution_fingerprint_for_topology(&topology).expect("baseline fingerprint");

        let changed_versions = vec![
            version("text-input", "1.0.1", "digest:input-v2"),
            version("text-output", "1.0.0", "digest:output"),
        ];
        let changed_topology =
            workflow_executable_topology_with_node_versions(&graph(), changed_versions)
                .expect("changed topology");
        let changed = workflow_execution_fingerprint_for_topology(&changed_topology)
            .expect("changed fingerprint");

        assert_ne!(baseline, changed);
    }

    #[test]
    fn executable_topology_rejects_missing_node_behavior_versions() {
        let result = workflow_executable_topology_with_node_versions(
            &graph(),
            vec![version("text-input", "1.0.0", "digest:input")],
        );

        assert!(matches!(
            result,
            Err(WorkflowServiceError::CapabilityViolation(message))
                if message.contains("does not provide behavior version facts")
        ));
    }
}
