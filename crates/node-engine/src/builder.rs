//! Fluent builders for workflow and orchestration graphs
//!
//! Provides a type-safe, fluent API for constructing graphs programmatically.

use crate::orchestration::{
    ConditionConfig, DataGraphConfig, LoopConfig, OrchestrationEdge, OrchestrationGraph,
    OrchestrationNode, OrchestrationNodeType,
};
use crate::types::{GraphEdge, GraphNode, WorkflowGraph};

/// Fluent builder for constructing workflow (data) graphs
///
/// # Example
///
/// ```ignore
/// let graph = WorkflowBuilder::new("wf-1", "My Workflow")
///     .add_node("input-1", "text-input", (0.0, 0.0))
///     .with_data(serde_json::json!({"text": "Hello"}))
///     .add_node("output-1", "text-output", (200.0, 0.0))
///     .add_edge("input-1", "text", "output-1", "text")
///     .build();
/// ```
pub struct WorkflowBuilder {
    id: String,
    name: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    edge_counter: usize,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            edge_counter: 0,
        }
    }

    /// Add a node to the graph
    pub fn add_node(
        mut self,
        id: impl Into<String>,
        node_type: impl Into<String>,
        position: (f64, f64),
    ) -> Self {
        self.nodes.push(GraphNode {
            id: id.into(),
            node_type: node_type.into(),
            data: serde_json::Value::Null,
            position,
        });
        self
    }

    /// Set data on the most recently added node
    ///
    /// Must be called immediately after `add_node`.
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        if let Some(node) = self.nodes.last_mut() {
            node.data = data;
        }
        self
    }

    /// Add an edge between two nodes (auto-generates edge ID)
    pub fn add_edge(
        mut self,
        source: impl Into<String>,
        source_port: impl Into<String>,
        target: impl Into<String>,
        target_port: impl Into<String>,
    ) -> Self {
        self.edge_counter += 1;
        self.edges.push(GraphEdge {
            id: format!("edge-{}", self.edge_counter),
            source: source.into(),
            source_handle: source_port.into(),
            target: target.into(),
            target_handle: target_port.into(),
        });
        self
    }

    /// Add an edge with an explicit ID
    pub fn add_edge_with_id(
        mut self,
        edge_id: impl Into<String>,
        source: impl Into<String>,
        source_port: impl Into<String>,
        target: impl Into<String>,
        target_port: impl Into<String>,
    ) -> Self {
        self.edges.push(GraphEdge {
            id: edge_id.into(),
            source: source.into(),
            source_handle: source_port.into(),
            target: target.into(),
            target_handle: target_port.into(),
        });
        self
    }

    /// Build the graph without validation
    pub fn build(self) -> WorkflowGraph {
        let mut graph = WorkflowGraph::new(self.id, self.name);
        graph.nodes = self.nodes;
        graph.edges = self.edges;
        graph
    }
}

/// Fluent builder for orchestration graphs
///
/// # Example
///
/// ```ignore
/// let graph = OrchestrationBuilder::new("orch-1", "My Orchestration")
///     .add_start("start", (0.0, 0.0))
///     .add_data_graph("compile", (100.0, 0.0), "wf-compile")
///     .add_end("done", (200.0, 0.0))
///     .connect("start", "next", "compile", "input")
///     .connect("compile", "next", "done", "input")
///     .build();
/// ```
pub struct OrchestrationBuilder {
    id: String,
    name: String,
    description: String,
    nodes: Vec<OrchestrationNode>,
    edges: Vec<OrchestrationEdge>,
    data_graphs: std::collections::HashMap<String, String>,
    edge_counter: usize,
}

impl OrchestrationBuilder {
    /// Create a new orchestration builder
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            data_graphs: std::collections::HashMap::new(),
            edge_counter: 0,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a Start node
    pub fn add_start(mut self, id: impl Into<String>, position: (f64, f64)) -> Self {
        self.nodes
            .push(OrchestrationNode::new(id, OrchestrationNodeType::Start, position));
        self
    }

    /// Add an End node
    pub fn add_end(mut self, id: impl Into<String>, position: (f64, f64)) -> Self {
        self.nodes
            .push(OrchestrationNode::new(id, OrchestrationNodeType::End, position));
        self
    }

    /// Add a Condition node
    pub fn add_condition(
        mut self,
        id: impl Into<String>,
        position: (f64, f64),
        config: ConditionConfig,
    ) -> Self {
        self.nodes.push(OrchestrationNode::with_config(
            id,
            OrchestrationNodeType::Condition,
            position,
            serde_json::to_value(config).unwrap_or_default(),
        ));
        self
    }

    /// Add a Loop node
    pub fn add_loop(
        mut self,
        id: impl Into<String>,
        position: (f64, f64),
        config: LoopConfig,
    ) -> Self {
        self.nodes.push(OrchestrationNode::with_config(
            id,
            OrchestrationNodeType::Loop,
            position,
            serde_json::to_value(config).unwrap_or_default(),
        ));
        self
    }

    /// Add a DataGraph node with an associated data graph ID
    pub fn add_data_graph(
        mut self,
        id: impl Into<String>,
        position: (f64, f64),
        data_graph_id: impl Into<String>,
    ) -> Self {
        let id = id.into();
        let dg_id = data_graph_id.into();
        let config = DataGraphConfig {
            data_graph_id: dg_id.clone(),
            input_mappings: std::collections::HashMap::new(),
            output_mappings: std::collections::HashMap::new(),
        };
        self.nodes.push(OrchestrationNode::with_config(
            &id,
            OrchestrationNodeType::DataGraph,
            position,
            serde_json::to_value(config).unwrap_or_default(),
        ));
        self.data_graphs.insert(id, dg_id);
        self
    }

    /// Add a Merge node
    pub fn add_merge(mut self, id: impl Into<String>, position: (f64, f64)) -> Self {
        self.nodes
            .push(OrchestrationNode::new(id, OrchestrationNodeType::Merge, position));
        self
    }

    /// Connect two orchestration nodes
    pub fn connect(
        mut self,
        source: impl Into<String>,
        source_handle: impl Into<String>,
        target: impl Into<String>,
        target_handle: impl Into<String>,
    ) -> Self {
        self.edge_counter += 1;
        self.edges.push(OrchestrationEdge {
            id: format!("orch-edge-{}", self.edge_counter),
            source: source.into(),
            source_handle: source_handle.into(),
            target: target.into(),
            target_handle: target_handle.into(),
        });
        self
    }

    /// Build the orchestration graph
    pub fn build(self) -> OrchestrationGraph {
        let mut graph = OrchestrationGraph::new(self.id, self.name);
        graph.description = self.description;
        graph.nodes = self.nodes;
        graph.edges = self.edges;
        graph.data_graphs = self.data_graphs;
        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder_basic() {
        let graph = WorkflowBuilder::new("wf-1", "Test Workflow")
            .add_node("input-1", "text-input", (0.0, 0.0))
            .with_data(serde_json::json!({"text": "Hello"}))
            .add_node("output-1", "text-output", (200.0, 0.0))
            .add_edge("input-1", "text", "output-1", "text")
            .build();

        assert_eq!(graph.id, "wf-1");
        assert_eq!(graph.name, "Test Workflow");
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.nodes[0].data, serde_json::json!({"text": "Hello"}));
    }

    #[test]
    fn test_workflow_builder_auto_edge_ids() {
        let graph = WorkflowBuilder::new("wf", "Test")
            .add_node("a", "input", (0.0, 0.0))
            .add_node("b", "process", (100.0, 0.0))
            .add_node("c", "output", (200.0, 0.0))
            .add_edge("a", "out", "b", "in")
            .add_edge("b", "out", "c", "in")
            .build();

        assert_eq!(graph.edges[0].id, "edge-1");
        assert_eq!(graph.edges[1].id, "edge-2");
    }

    #[test]
    fn test_orchestration_builder_linear() {
        let graph = OrchestrationBuilder::new("orch-1", "Linear Flow")
            .with_description("A simple linear flow")
            .add_start("start", (0.0, 0.0))
            .add_data_graph("step1", (100.0, 0.0), "wf-step1")
            .add_end("end", (200.0, 0.0))
            .connect("start", "next", "step1", "input")
            .connect("step1", "next", "end", "input")
            .build();

        assert_eq!(graph.id, "orch-1");
        assert_eq!(graph.description, "A simple linear flow");
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.data_graphs.contains_key("step1"));
    }

    #[test]
    fn test_orchestration_builder_with_condition() {
        let graph = OrchestrationBuilder::new("orch-2", "Conditional Flow")
            .add_start("start", (0.0, 0.0))
            .add_condition(
                "check",
                (100.0, 0.0),
                ConditionConfig {
                    condition_key: "success".to_string(),
                    expected_value: Some(serde_json::json!(true)),
                },
            )
            .add_data_graph("pass", (200.0, -50.0), "wf-pass")
            .add_data_graph("fail", (200.0, 50.0), "wf-fail")
            .add_end("end", (300.0, 0.0))
            .connect("start", "next", "check", "input")
            .connect("check", "true", "pass", "input")
            .connect("check", "false", "fail", "input")
            .connect("pass", "next", "end", "input")
            .connect("fail", "next", "end", "input")
            .build();

        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.edges.len(), 5);
    }

    #[test]
    fn test_workflow_builder_serde_roundtrip() {
        let graph = WorkflowBuilder::new("wf-rt", "Roundtrip Test")
            .add_node("a", "input", (0.0, 0.0))
            .add_node("b", "output", (100.0, 0.0))
            .add_edge("a", "out", "b", "in")
            .build();

        let json = serde_json::to_string(&graph).unwrap();
        let restored: WorkflowGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "wf-rt");
        assert_eq!(restored.nodes.len(), 2);
        assert_eq!(restored.edges.len(), 1);
    }
}
