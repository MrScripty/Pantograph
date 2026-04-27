use serde::{Deserialize, Serialize};

use super::types::WorkflowGraph;
use crate::workflow::WorkflowServiceError;

const RUN_SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphRunSettings {
    pub schema_version: u32,
    pub nodes: Vec<WorkflowGraphRunSettingsNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphRunSettingsNode {
    pub node_id: String,
    pub node_type: String,
    pub data: serde_json::Value,
}

pub fn workflow_graph_run_settings(graph: &WorkflowGraph) -> WorkflowGraphRunSettings {
    let mut nodes = graph
        .nodes
        .iter()
        .map(|node| WorkflowGraphRunSettingsNode {
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            data: node.data.clone(),
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    WorkflowGraphRunSettings {
        schema_version: RUN_SETTINGS_SCHEMA_VERSION,
        nodes,
    }
}

pub fn workflow_graph_run_settings_json(
    settings: &WorkflowGraphRunSettings,
) -> Result<String, WorkflowServiceError> {
    serde_json::to_string(settings).map_err(|error| {
        WorkflowServiceError::CapabilityViolation(format!(
            "failed to encode workflow graph run settings: {error}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, Position};

    fn graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "node-b".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 200.0, y: 50.0 },
                    data: serde_json::json!({"temperature": 0.7}),
                },
                GraphNode {
                    id: "node-a".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 10.0, y: 20.0 },
                    data: serde_json::json!({"context_length": 4096}),
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
    fn graph_run_settings_are_sorted_and_exclude_display_metadata() {
        let mut left = graph();
        let mut right = graph();
        right.nodes.reverse();
        right.nodes[0].position = Position { x: 999.0, y: 999.0 };
        right.edges.clear();

        assert_eq!(
            workflow_graph_run_settings(&left),
            workflow_graph_run_settings(&right)
        );

        left.nodes[0].data = serde_json::json!({"temperature": 0.2});
        assert_ne!(
            workflow_graph_run_settings(&left),
            workflow_graph_run_settings(&right)
        );
    }
}
