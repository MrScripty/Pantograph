use serde::{Deserialize, Serialize};

use super::types::{GraphEdge, Position, WorkflowGraph};
use crate::workflow::WorkflowServiceError;

const PRESENTATION_METADATA_SCHEMA_VERSION: u32 = 1;
const WORKFLOW_PRESENTATION_FINGERPRINT_PREFIX: &str = "workflow-presentation-blake3:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPresentationMetadata {
    pub schema_version: u32,
    pub nodes: Vec<WorkflowPresentationNode>,
    pub edges: Vec<WorkflowPresentationEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPresentationNode {
    pub node_id: String,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowPresentationEdge {
    pub edge_id: String,
    pub source_node_id: String,
    pub source_port_id: String,
    pub target_node_id: String,
    pub target_port_id: String,
}

pub fn workflow_presentation_metadata(graph: &WorkflowGraph) -> WorkflowPresentationMetadata {
    let mut nodes = graph
        .nodes
        .iter()
        .map(|node| WorkflowPresentationNode {
            node_id: node.id.clone(),
            position: node.position.clone(),
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    let mut edges = graph
        .edges
        .iter()
        .map(presentation_edge)
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        left.source_node_id
            .cmp(&right.source_node_id)
            .then_with(|| left.source_port_id.cmp(&right.source_port_id))
            .then_with(|| left.target_node_id.cmp(&right.target_node_id))
            .then_with(|| left.target_port_id.cmp(&right.target_port_id))
            .then_with(|| left.edge_id.cmp(&right.edge_id))
    });

    WorkflowPresentationMetadata {
        schema_version: PRESENTATION_METADATA_SCHEMA_VERSION,
        nodes,
        edges,
    }
}

pub fn workflow_presentation_metadata_json(
    metadata: &WorkflowPresentationMetadata,
) -> Result<String, WorkflowServiceError> {
    serde_json::to_string(metadata).map_err(|error| {
        WorkflowServiceError::CapabilityViolation(format!(
            "failed to encode workflow presentation metadata: {error}"
        ))
    })
}

pub fn workflow_presentation_fingerprint(
    graph: &WorkflowGraph,
) -> Result<String, WorkflowServiceError> {
    let metadata = workflow_presentation_metadata(graph);
    workflow_presentation_fingerprint_for_metadata(&metadata)
}

pub fn workflow_presentation_fingerprint_for_metadata(
    metadata: &WorkflowPresentationMetadata,
) -> Result<String, WorkflowServiceError> {
    let bytes = serde_json::to_vec(metadata).map_err(|error| {
        WorkflowServiceError::CapabilityViolation(format!(
            "failed to encode workflow presentation metadata: {error}"
        ))
    })?;
    Ok(format!(
        "{WORKFLOW_PRESENTATION_FINGERPRINT_PREFIX}{}",
        blake3::hash(&bytes)
    ))
}

fn presentation_edge(edge: &GraphEdge) -> WorkflowPresentationEdge {
    WorkflowPresentationEdge {
        edge_id: edge.id.clone(),
        source_node_id: edge.source.clone(),
        source_port_id: edge.source_handle.clone(),
        target_node_id: edge.target.clone(),
        target_port_id: edge.target_handle.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphNode, Position};

    fn graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "node-b".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 200.0, y: 50.0 },
                    data: serde_json::json!({"value": "ignored"}),
                },
                GraphNode {
                    id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 10.0, y: 20.0 },
                    data: serde_json::json!({"value": "ignored"}),
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

    #[test]
    fn presentation_metadata_is_sorted_and_excludes_node_data() {
        let mut left = graph();
        let mut right = graph();
        right.nodes.reverse();
        right.nodes[0].data = serde_json::json!({"value": "changed"});
        left.derived_graph = Some(crate::graph::WorkflowDerivedGraph {
            schema_version: 1,
            graph_fingerprint: "cache".to_string(),
            consumer_count_map: Default::default(),
        });

        let left_metadata = workflow_presentation_metadata(&left);
        let right_metadata = workflow_presentation_metadata(&right);

        assert_eq!(left_metadata, right_metadata);
        assert_eq!(left_metadata.nodes[0].node_id, "node-a");
    }

    #[test]
    fn presentation_fingerprint_changes_when_display_metadata_changes() {
        let left = graph();
        let mut right = graph();
        right.nodes[0].position = Position { x: 999.0, y: 50.0 };

        let left_fingerprint = workflow_presentation_fingerprint(&left).expect("left");
        let right_fingerprint = workflow_presentation_fingerprint(&right).expect("right");

        assert_ne!(left_fingerprint, right_fingerprint);
    }
}
