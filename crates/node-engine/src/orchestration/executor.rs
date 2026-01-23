//! Orchestration graph executor.
//!
//! This module provides the executor for running orchestration graphs,
//! handling control flow between data graphs.

use super::nodes::{execute_node, prepare_data_graph_execution, NodeExecutionResult, OrchestrationContext};
use super::types::{OrchestrationGraph, OrchestrationNodeType, OrchestrationResult};
use crate::events::{EventSink, WorkflowEvent};
use crate::{NodeEngineError, Result, WorkflowGraph};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;

/// Trait for executing data graphs within an orchestration.
///
/// This trait abstracts the data graph execution, allowing the orchestration
/// executor to delegate data graph execution to an external implementation.
#[async_trait]
pub trait DataGraphExecutor: Send + Sync {
    /// Execute a data graph and return its outputs.
    ///
    /// # Arguments
    /// * `graph_id` - The ID of the data graph to execute
    /// * `inputs` - Input values mapped to port names
    /// * `event_sink` - Event sink for progress reporting
    ///
    /// # Returns
    /// A map of output port names to their values
    async fn execute_data_graph(
        &self,
        graph_id: &str,
        inputs: HashMap<String, Value>,
        event_sink: &dyn EventSink,
    ) -> Result<HashMap<String, Value>>;

    /// Get a data graph by its ID.
    fn get_data_graph(&self, graph_id: &str) -> Option<WorkflowGraph>;
}

/// Events emitted during orchestration execution.
#[derive(Debug, Clone)]
pub enum OrchestrationEvent {
    /// Orchestration execution started.
    Started {
        orchestration_id: String,
        node_count: usize,
    },
    /// An orchestration node started executing.
    NodeStarted {
        node_id: String,
        node_type: String,
    },
    /// An orchestration node completed.
    NodeCompleted {
        node_id: String,
        next_handle: String,
        message: Option<String>,
    },
    /// A data graph started executing.
    DataGraphStarted {
        node_id: String,
        data_graph_id: String,
    },
    /// A data graph completed.
    DataGraphCompleted {
        node_id: String,
        data_graph_id: String,
        outputs: HashMap<String, Value>,
    },
    /// A data graph failed.
    DataGraphFailed {
        node_id: String,
        data_graph_id: String,
        error: String,
    },
    /// Loop iteration started.
    LoopIteration {
        node_id: String,
        iteration: u32,
    },
    /// Condition evaluated.
    ConditionEvaluated {
        node_id: String,
        result: bool,
    },
    /// Orchestration completed successfully.
    Completed {
        outputs: HashMap<String, Value>,
        nodes_executed: u32,
        execution_time_ms: u64,
    },
    /// Orchestration failed.
    Failed {
        error: String,
        nodes_executed: u32,
        execution_time_ms: u64,
    },
}

/// Executor for orchestration graphs.
pub struct OrchestrationExecutor<E: DataGraphExecutor> {
    /// The data graph executor.
    data_executor: E,
    /// Maximum number of nodes to execute (for infinite loop protection).
    max_nodes: u32,
    /// Execution ID for this orchestration run.
    execution_id: String,
}

impl<E: DataGraphExecutor> OrchestrationExecutor<E> {
    /// Create a new orchestration executor.
    pub fn new(data_executor: E) -> Self {
        Self {
            data_executor,
            max_nodes: 1000, // Default limit
            execution_id: format!("orch-exec-{}", uuid::Uuid::new_v4()),
        }
    }

    /// Set the maximum number of nodes to execute.
    pub fn with_max_nodes(mut self, max_nodes: u32) -> Self {
        self.max_nodes = max_nodes;
        self
    }

    /// Set the execution ID.
    pub fn with_execution_id(mut self, execution_id: impl Into<String>) -> Self {
        self.execution_id = execution_id.into();
        self
    }

    /// Execute an orchestration graph.
    pub async fn execute(
        &self,
        graph: &OrchestrationGraph,
        initial_data: HashMap<String, Value>,
        event_sink: &dyn EventSink,
    ) -> Result<OrchestrationResult> {
        let start_time = Instant::now();
        let mut nodes_executed: u32 = 0;
        let mut context = OrchestrationContext::with_data(initial_data);

        // Emit started event
        self.emit_workflow_started(event_sink, &graph.id);

        // Find the start node
        let start_node = graph.find_start_node().ok_or_else(|| {
            NodeEngineError::failed("Orchestration graph has no Start node")
        })?;

        // Begin execution from the start node
        let mut current_node_id = start_node.id.clone();

        loop {
            // Check execution limit
            if nodes_executed >= self.max_nodes {
                let elapsed = start_time.elapsed().as_millis() as u64;
                let error = format!("Execution limit reached ({} nodes)", self.max_nodes);
                self.emit_workflow_failed(event_sink, &graph.id, &error);
                return Ok(OrchestrationResult::failure(error, nodes_executed, elapsed));
            }

            // Find the current node
            let node = graph.find_node(&current_node_id).ok_or_else(|| {
                NodeEngineError::failed(format!("Node '{}' not found in graph", current_node_id))
            })?;

            // Emit node started event
            self.emit_task_started(event_sink, &node.id);

            nodes_executed += 1;

            // Execute the node
            let result = match node.node_type {
                OrchestrationNodeType::DataGraph => {
                    // Special handling for data graph nodes
                    self.execute_data_graph_node(graph, node, &mut context, event_sink)
                        .await?
                }
                _ => {
                    // Regular node execution
                    execute_node(node, &mut context)?
                }
            };

            // Apply context updates
            for (key, value) in result.context_updates {
                context.set(key, value);
            }

            // Emit task completed event
            self.emit_task_completed(event_sink, &node.id, result.message.clone());

            // Emit specific events based on node type
            match node.node_type {
                OrchestrationNodeType::Condition => {
                    self.emit_task_progress(
                        event_sink,
                        &node.id,
                        1.0,
                        Some(format!("Condition: {}", if result.next_handle == "true" { "true" } else { "false" })),
                    );
                }
                OrchestrationNodeType::Loop => {
                    if result.next_handle == "iteration" {
                        let iteration = context.get_loop_iteration(&node.id);
                        self.emit_task_progress(
                            event_sink,
                            &node.id,
                            0.0,
                            Some(format!("Loop iteration: {}", iteration)),
                        );
                    }
                }
                _ => {}
            }

            // Check for End node (empty next handle)
            if result.next_handle.is_empty() {
                let elapsed = start_time.elapsed().as_millis() as u64;
                let outputs = context.into_data();

                self.emit_workflow_completed(event_sink, &graph.id);

                return Ok(OrchestrationResult::success(outputs, nodes_executed, elapsed));
            }

            // Find the next node by following the edge
            let next_node_id = self.find_next_node(graph, &node.id, &result.next_handle)?;
            current_node_id = next_node_id;
        }
    }

    /// Execute a DataGraph node by running the associated data graph.
    async fn execute_data_graph_node(
        &self,
        orchestration_graph: &OrchestrationGraph,
        node: &super::types::OrchestrationNode,
        context: &mut OrchestrationContext,
        event_sink: &dyn EventSink,
    ) -> Result<NodeExecutionResult> {
        // Get the data graph configuration
        let config = prepare_data_graph_execution(node)?;

        // Get the data graph ID (from config or from graph's data_graphs map)
        let data_graph_id = orchestration_graph
            .get_data_graph_id(&node.id)
            .map(|s| s.clone())
            .unwrap_or(config.data_graph_id.clone());

        // Emit data graph started progress
        self.emit_task_progress(
            event_sink,
            &node.id,
            0.0,
            Some(format!("Starting data graph: {}", data_graph_id)),
        );

        // Map orchestration context to data graph inputs
        let mut inputs = HashMap::new();
        for (context_key, port_name) in &config.input_mappings {
            if let Some(value) = context.get(context_key) {
                inputs.insert(port_name.clone(), value.clone());
            }
        }

        // Execute the data graph
        match self
            .data_executor
            .execute_data_graph(&data_graph_id, inputs, event_sink)
            .await
        {
            Ok(outputs) => {
                // Map data graph outputs back to orchestration context
                let mut context_updates = HashMap::new();
                for (port_name, context_key) in &config.output_mappings {
                    if let Some(value) = outputs.get(port_name) {
                        context_updates.insert(context_key.clone(), value.clone());
                    }
                }

                // Also store all outputs under the node ID prefix
                for (port_name, value) in &outputs {
                    context_updates.insert(format!("{}.{}", node.id, port_name), value.clone());
                }

                self.emit_task_progress(
                    event_sink,
                    &node.id,
                    1.0,
                    Some(format!("Completed data graph: {}", data_graph_id)),
                );

                Ok(NodeExecutionResult::handle("next")
                    .with_updates(context_updates)
                    .with_message(format!("Data graph '{}' completed", data_graph_id)))
            }
            Err(e) => {
                self.emit_task_failed(event_sink, &node.id, &e.to_string());

                // Route to error handle if available
                Ok(NodeExecutionResult::handle("error")
                    .with_update(format!("{}.error", node.id), Value::String(e.to_string()))
                    .with_message(format!("Data graph '{}' failed: {}", data_graph_id, e)))
            }
        }
    }

    /// Find the next node by following an edge from the given handle.
    fn find_next_node(
        &self,
        graph: &OrchestrationGraph,
        source_id: &str,
        source_handle: &str,
    ) -> Result<String> {
        let edges = graph.outgoing_edges(source_id);

        for edge in edges {
            if edge.source_handle == source_handle {
                return Ok(edge.target.clone());
            }
        }

        Err(NodeEngineError::failed(format!(
            "No edge found from node '{}' handle '{}'",
            source_id, source_handle
        )))
    }

    // Event emission helpers using the correct WorkflowEvent variants

    fn emit_workflow_started(&self, event_sink: &dyn EventSink, workflow_id: &str) {
        let _ = event_sink.send(WorkflowEvent::WorkflowStarted {
            workflow_id: workflow_id.to_string(),
            execution_id: self.execution_id.clone(),
        });
    }

    fn emit_workflow_completed(&self, event_sink: &dyn EventSink, workflow_id: &str) {
        let _ = event_sink.send(WorkflowEvent::WorkflowCompleted {
            workflow_id: workflow_id.to_string(),
            execution_id: self.execution_id.clone(),
        });
    }

    fn emit_workflow_failed(&self, event_sink: &dyn EventSink, workflow_id: &str, error: &str) {
        let _ = event_sink.send(WorkflowEvent::WorkflowFailed {
            workflow_id: workflow_id.to_string(),
            execution_id: self.execution_id.clone(),
            error: error.to_string(),
        });
    }

    fn emit_task_started(&self, event_sink: &dyn EventSink, task_id: &str) {
        let _ = event_sink.send(WorkflowEvent::TaskStarted {
            task_id: task_id.to_string(),
            execution_id: self.execution_id.clone(),
        });
    }

    fn emit_task_completed(&self, event_sink: &dyn EventSink, task_id: &str, output: Option<String>) {
        let _ = event_sink.send(WorkflowEvent::TaskCompleted {
            task_id: task_id.to_string(),
            execution_id: self.execution_id.clone(),
            output: output.map(|s| Value::String(s)),
        });
    }

    fn emit_task_failed(&self, event_sink: &dyn EventSink, task_id: &str, error: &str) {
        let _ = event_sink.send(WorkflowEvent::TaskFailed {
            task_id: task_id.to_string(),
            execution_id: self.execution_id.clone(),
            error: error.to_string(),
        });
    }

    fn emit_task_progress(&self, event_sink: &dyn EventSink, task_id: &str, progress: f32, message: Option<String>) {
        let _ = event_sink.send(WorkflowEvent::TaskProgress {
            task_id: task_id.to_string(),
            execution_id: self.execution_id.clone(),
            progress,
            message,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::NullEventSink;
    use crate::orchestration::types::{OrchestrationEdge, OrchestrationNode};

    /// Mock data graph executor for testing.
    struct MockDataGraphExecutor {
        outputs: HashMap<String, HashMap<String, Value>>,
    }

    impl MockDataGraphExecutor {
        fn new() -> Self {
            Self {
                outputs: HashMap::new(),
            }
        }

        fn with_output(mut self, graph_id: &str, outputs: HashMap<String, Value>) -> Self {
            self.outputs.insert(graph_id.to_string(), outputs);
            self
        }
    }

    #[async_trait]
    impl DataGraphExecutor for MockDataGraphExecutor {
        async fn execute_data_graph(
            &self,
            graph_id: &str,
            _inputs: HashMap<String, Value>,
            _event_sink: &dyn EventSink,
        ) -> Result<HashMap<String, Value>> {
            self.outputs
                .get(graph_id)
                .cloned()
                .ok_or_else(|| NodeEngineError::failed(format!("Unknown graph: {}", graph_id)))
        }

        fn get_data_graph(&self, _graph_id: &str) -> Option<WorkflowGraph> {
            None
        }
    }

    fn create_simple_graph() -> OrchestrationGraph {
        let mut graph = OrchestrationGraph::new("test", "Test Orchestration");

        // Add nodes: Start -> End
        graph.nodes.push(OrchestrationNode::new(
            "start",
            OrchestrationNodeType::Start,
            (0.0, 0.0),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end",
            OrchestrationNodeType::End,
            (200.0, 0.0),
        ));

        // Add edge
        graph.edges.push(OrchestrationEdge::new(
            "e1",
            "start",
            "next",
            "end",
            "input",
        ));

        graph
    }

    #[tokio::test]
    async fn test_simple_execution() {
        let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new());
        let graph = create_simple_graph();
        let event_sink = NullEventSink;

        let result = executor
            .execute(&graph, HashMap::new(), &event_sink)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.nodes_executed, 2); // Start + End
    }

    #[tokio::test]
    async fn test_condition_true_path() {
        let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new());
        let event_sink = NullEventSink;

        let mut graph = OrchestrationGraph::new("test", "Test");

        // Start -> Condition -> (true) -> End1
        //                    -> (false) -> End2
        graph.nodes.push(OrchestrationNode::new(
            "start",
            OrchestrationNodeType::Start,
            (0.0, 0.0),
        ));
        graph.nodes.push(OrchestrationNode::with_config(
            "cond",
            OrchestrationNodeType::Condition,
            (100.0, 0.0),
            serde_json::json!({"conditionKey": "isValid"}),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end_true",
            OrchestrationNodeType::End,
            (200.0, -50.0),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end_false",
            OrchestrationNodeType::End,
            (200.0, 50.0),
        ));

        graph.edges.push(OrchestrationEdge::new("e1", "start", "next", "cond", "input"));
        graph.edges.push(OrchestrationEdge::new("e2", "cond", "true", "end_true", "input"));
        graph.edges.push(OrchestrationEdge::new("e3", "cond", "false", "end_false", "input"));

        // Test true path
        let mut initial_data = HashMap::new();
        initial_data.insert("isValid".to_string(), Value::Bool(true));

        let result = executor.execute(&graph, initial_data, &event_sink).await.unwrap();
        assert!(result.success);
        assert_eq!(result.nodes_executed, 3); // Start + Condition + End
    }

    #[tokio::test]
    async fn test_loop_execution() {
        let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new());
        let event_sink = NullEventSink;

        let mut graph = OrchestrationGraph::new("test", "Test");

        // Start -> Loop (3 iterations) -> End
        graph.nodes.push(OrchestrationNode::new(
            "start",
            OrchestrationNodeType::Start,
            (0.0, 0.0),
        ));
        graph.nodes.push(OrchestrationNode::with_config(
            "loop",
            OrchestrationNodeType::Loop,
            (100.0, 0.0),
            serde_json::json!({"maxIterations": 3}),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end",
            OrchestrationNodeType::End,
            (200.0, 0.0),
        ));

        graph.edges.push(OrchestrationEdge::new("e1", "start", "next", "loop", "input"));
        graph.edges.push(OrchestrationEdge::new("e2", "loop", "iteration", "loop", "loop_back"));
        graph.edges.push(OrchestrationEdge::new("e3", "loop", "complete", "end", "input"));

        let result = executor.execute(&graph, HashMap::new(), &event_sink).await.unwrap();
        assert!(result.success);
        // Start + 3 iterations + 1 complete check + End = 6
        // Actually: Start(1) + Loop(4 times: 3 iterations + 1 for hitting max) + End(1) = 6
        assert_eq!(result.nodes_executed, 6);
    }

    #[tokio::test]
    async fn test_data_graph_execution() {
        let mut outputs = HashMap::new();
        outputs.insert("result".to_string(), Value::String("success".to_string()));

        let mock_executor = MockDataGraphExecutor::new().with_output("test_graph", outputs);
        let executor = OrchestrationExecutor::new(mock_executor);
        let event_sink = NullEventSink;

        let mut graph = OrchestrationGraph::new("test", "Test");

        graph.nodes.push(OrchestrationNode::new(
            "start",
            OrchestrationNodeType::Start,
            (0.0, 0.0),
        ));
        graph.nodes.push(OrchestrationNode::with_config(
            "data",
            OrchestrationNodeType::DataGraph,
            (100.0, 0.0),
            serde_json::json!({
                "dataGraphId": "test_graph",
                "inputMappings": {},
                "outputMappings": {"result": "output_value"}
            }),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end",
            OrchestrationNodeType::End,
            (200.0, 0.0),
        ));

        graph.edges.push(OrchestrationEdge::new("e1", "start", "next", "data", "input"));
        graph.edges.push(OrchestrationEdge::new("e2", "data", "next", "end", "input"));

        let result = executor.execute(&graph, HashMap::new(), &event_sink).await.unwrap();
        assert!(result.success);
        assert_eq!(
            result.outputs.get("output_value"),
            Some(&Value::String("success".to_string()))
        );
    }
}
