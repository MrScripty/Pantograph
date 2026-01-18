//! Workflow execution engine
//!
//! The engine validates and executes workflow graphs, routing data
//! between nodes and streaming events to the frontend.

use std::collections::{HashMap, VecDeque};
use tauri::ipc::Channel;

use super::events::WorkflowEvent;
use super::node::{ExecutionContext, NodeError, NodeOutputs, PortValue};
use super::registry::NodeRegistry;
use super::types::WorkflowGraph;
use super::validation::{ValidationError, WorkflowValidator};

/// Errors that can occur during workflow execution
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Validation failed: {0}")]
    ValidationFailed(#[from] ValidationError),

    #[error("Node execution failed: {0}")]
    NodeFailed(#[from] NodeError),

    #[error("Failed to send event to frontend")]
    ChannelFailed,

    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

/// Result of executing a workflow
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowResult {
    /// Outputs from all executed nodes, keyed by node ID
    pub outputs: HashMap<String, NodeOutputs>,
}

/// The workflow execution engine
///
/// Validates workflow graphs and executes them in topological order,
/// streaming events to the frontend.
pub struct WorkflowEngine {
    registry: NodeRegistry,
}

impl WorkflowEngine {
    /// Create a new workflow engine with the default node registry
    pub fn new() -> Self {
        Self {
            registry: NodeRegistry::new(),
        }
    }

    /// Create a workflow engine with a custom registry
    pub fn with_registry(registry: NodeRegistry) -> Self {
        Self { registry }
    }

    /// Get a reference to the node registry
    pub fn registry(&self) -> &NodeRegistry {
        &self.registry
    }

    /// Execute a workflow graph
    ///
    /// This will:
    /// 1. Validate the graph (cycles, types, required inputs)
    /// 2. Compute topological execution order
    /// 3. Execute nodes in order, resolving inputs from upstream outputs
    /// 4. Stream events to the frontend via the channel
    pub async fn execute(
        &self,
        graph: WorkflowGraph,
        context: ExecutionContext,
        channel: Channel<WorkflowEvent>,
    ) -> Result<WorkflowResult, WorkflowError> {
        // 1. Validate the graph
        let validator = WorkflowValidator::new(&self.registry);
        validator.validate(&graph)?;

        // 2. Compute topological execution order
        let order = self.topological_sort(&graph);

        // 3. Send started event
        channel
            .send(WorkflowEvent::started(
                &context.execution_id,
                graph.nodes.len(),
            ))
            .map_err(|_| WorkflowError::ChannelFailed)?;

        // 4. Execute nodes in order
        let mut outputs: HashMap<String, NodeOutputs> = HashMap::new();

        for node_id in order {
            // Check abort signal
            if context.is_aborted() {
                return Err(WorkflowError::NodeFailed(NodeError::Cancelled));
            }

            let graph_node = graph
                .find_node(&node_id)
                .ok_or_else(|| WorkflowError::NodeNotFound(node_id.clone()))?;

            // Send node started event
            channel
                .send(WorkflowEvent::node_started(
                    &node_id,
                    &graph_node.node_type,
                ))
                .map_err(|_| WorkflowError::ChannelFailed)?;

            // Resolve inputs from upstream outputs and node data
            let inputs = self.resolve_inputs(&graph, &node_id, &outputs, &graph_node.data);

            // Create and execute the node
            let node = self.registry.create_node(&graph_node.node_type, &node_id)?;

            match node.execute(inputs, &context, &channel).await {
                Ok(result) => {
                    // Send completion event
                    channel
                        .send(WorkflowEvent::node_completed(&node_id, result.clone()))
                        .map_err(|_| WorkflowError::ChannelFailed)?;

                    outputs.insert(node_id, result);
                }
                Err(e) => {
                    // Send error event
                    channel
                        .send(WorkflowEvent::node_error(&node_id, e.to_string()))
                        .map_err(|_| WorkflowError::ChannelFailed)?;

                    channel
                        .send(WorkflowEvent::failed(e.to_string()))
                        .map_err(|_| WorkflowError::ChannelFailed)?;

                    return Err(WorkflowError::NodeFailed(e));
                }
            }
        }

        // 5. Send completion event
        channel
            .send(WorkflowEvent::completed(outputs.clone()))
            .map_err(|_| WorkflowError::ChannelFailed)?;

        Ok(WorkflowResult { outputs })
    }

    /// Compute topological execution order using Kahn's algorithm
    ///
    /// Returns node IDs in the order they should be executed.
    fn topological_sort(&self, graph: &WorkflowGraph) -> Vec<String> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize
        for node in &graph.nodes {
            in_degree.insert(&node.id, 0);
            adjacency.insert(&node.id, Vec::new());
        }

        // Build graph
        for edge in &graph.edges {
            if let Some(adj) = adjacency.get_mut(edge.source.as_str()) {
                adj.push(&edge.target);
            }
            if let Some(degree) = in_degree.get_mut(edge.target.as_str()) {
                *degree += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node.to_string());

            if let Some(neighbors) = adjacency.get(node) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        result
    }

    /// Resolve inputs for a node from upstream outputs and node data
    ///
    /// Priority:
    /// 1. Connected upstream outputs (from edges)
    /// 2. Values stored in node.data (for configuration)
    fn resolve_inputs(
        &self,
        graph: &WorkflowGraph,
        node_id: &str,
        outputs: &HashMap<String, NodeOutputs>,
        node_data: &serde_json::Value,
    ) -> HashMap<String, PortValue> {
        let mut inputs = HashMap::new();

        // First, include any data stored in the node itself
        if let Some(obj) = node_data.as_object() {
            for (key, value) in obj {
                inputs.insert(key.clone(), value.clone());
            }
        }

        // Then, resolve inputs from connected upstream nodes
        // (these override node data if there's a conflict)
        for edge in graph.incoming_edges(node_id) {
            if let Some(source_outputs) = outputs.get(&edge.source) {
                if let Some(value) = source_outputs.get(&edge.source_handle) {
                    inputs.insert(edge.target_handle.clone(), value.clone());
                }
            }
        }

        inputs
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::types::{GraphEdge, GraphNode, Position};

    #[test]
    fn test_topological_sort_simple() {
        let engine = WorkflowEngine::new();

        // A -> B -> C
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "a".into(),
                    node_type: "text-input".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
                GraphNode {
                    id: "b".into(),
                    node_type: "llm-inference".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
                GraphNode {
                    id: "c".into(),
                    node_type: "text-output".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "e1".into(),
                    source: "a".into(),
                    source_handle: "text".into(),
                    target: "b".into(),
                    target_handle: "prompt".into(),
                },
                GraphEdge {
                    id: "e2".into(),
                    source: "b".into(),
                    source_handle: "response".into(),
                    target: "c".into(),
                    target_handle: "text".into(),
                },
            ],
        };

        let order = engine.topological_sort(&graph);

        // A must come before B, B must come before C
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();

        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_topological_sort_parallel() {
        let engine = WorkflowEngine::new();

        // A and B both feed into C (A and B can run in any order)
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "a".into(),
                    node_type: "text-input".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
                GraphNode {
                    id: "b".into(),
                    node_type: "text-input".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
                GraphNode {
                    id: "c".into(),
                    node_type: "text-output".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "e1".into(),
                    source: "a".into(),
                    source_handle: "text".into(),
                    target: "c".into(),
                    target_handle: "text".into(),
                },
                GraphEdge {
                    id: "e2".into(),
                    source: "b".into(),
                    source_handle: "text".into(),
                    target: "c".into(),
                    target_handle: "context".into(),
                },
            ],
        };

        let order = engine.topological_sort(&graph);

        // Both A and B must come before C
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();

        assert!(a_pos < c_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_resolve_inputs_from_node_data() {
        let engine = WorkflowEngine::new();

        let graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "a".into(),
                node_type: "text-input".into(),
                position: Position::default(),
                data: serde_json::json!({"text": "hello world"}),
            }],
            edges: vec![],
        };

        let outputs = HashMap::new();
        let node_data = &graph.nodes[0].data;

        let inputs = engine.resolve_inputs(&graph, "a", &outputs, node_data);

        assert_eq!(inputs.get("text").unwrap(), "hello world");
    }

    #[test]
    fn test_resolve_inputs_from_upstream() {
        let engine = WorkflowEngine::new();

        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "a".into(),
                    node_type: "text-input".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
                GraphNode {
                    id: "b".into(),
                    node_type: "text-output".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
            ],
            edges: vec![GraphEdge {
                id: "e1".into(),
                source: "a".into(),
                source_handle: "text".into(),
                target: "b".into(),
                target_handle: "text".into(),
            }],
        };

        let mut outputs = HashMap::new();
        let mut a_outputs = HashMap::new();
        a_outputs.insert("text".to_string(), serde_json::json!("upstream value"));
        outputs.insert("a".to_string(), a_outputs);

        let inputs = engine.resolve_inputs(&graph, "b", &outputs, &serde_json::Value::Null);

        assert_eq!(inputs.get("text").unwrap(), "upstream value");
    }
}
