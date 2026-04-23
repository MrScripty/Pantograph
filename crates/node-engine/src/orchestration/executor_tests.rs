use super::*;
use crate::events::{NullEventSink, VecEventSink};
use crate::orchestration::types::{OrchestrationEdge, OrchestrationNode};

/// Mock data graph executor for testing.
#[derive(Clone)]
enum MockDataGraphError {
    WaitingForInput {
        task_id: String,
        prompt: Option<String>,
        emit_event: bool,
    },
    Cancelled,
}

struct MockDataGraphExecutor {
    outputs: HashMap<String, HashMap<String, Value>>,
    errors: HashMap<String, MockDataGraphError>,
}

impl MockDataGraphExecutor {
    fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    fn with_output(mut self, graph_id: &str, outputs: HashMap<String, Value>) -> Self {
        self.outputs.insert(graph_id.to_string(), outputs);
        self
    }

    fn with_error(mut self, graph_id: &str, error: MockDataGraphError) -> Self {
        self.errors.insert(graph_id.to_string(), error);
        self
    }
}

#[async_trait]
impl DataGraphExecutor for MockDataGraphExecutor {
    async fn execute_data_graph(
        &self,
        graph_id: &str,
        _inputs: HashMap<String, Value>,
        event_sink: &dyn EventSink,
    ) -> Result<HashMap<String, Value>> {
        if let Some(error) = self.errors.get(graph_id) {
            return Err(match error {
                MockDataGraphError::WaitingForInput {
                    task_id,
                    prompt,
                    emit_event,
                } => {
                    if *emit_event {
                        let _ = event_sink.send(WorkflowEvent::WaitingForInput {
                            workflow_id: graph_id.to_string(),
                            execution_id: "data-graph-exec".to_string(),
                            task_id: task_id.clone(),
                            prompt: prompt.clone(),
                            occurred_at_ms: None,
                        });
                    }
                    NodeEngineError::waiting_for_input(task_id.clone(), prompt.clone())
                }
                MockDataGraphError::Cancelled => NodeEngineError::Cancelled,
            });
        }

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
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "end", "input",
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
    assert_eq!(result.nodes_executed, 2);
}

#[tokio::test]
async fn test_condition_true_path() {
    let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new());
    let event_sink = NullEventSink;

    let mut graph = OrchestrationGraph::new("test", "Test");
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
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "cond", "input",
    ));
    graph.edges.push(OrchestrationEdge::new(
        "e2", "cond", "true", "end_true", "input",
    ));
    graph.edges.push(OrchestrationEdge::new(
        "e3",
        "cond",
        "false",
        "end_false",
        "input",
    ));

    let mut initial_data = HashMap::new();
    initial_data.insert("isValid".to_string(), Value::Bool(true));

    let result = executor
        .execute(&graph, initial_data, &event_sink)
        .await
        .unwrap();
    assert!(result.success);
    assert_eq!(result.nodes_executed, 3);
}

#[tokio::test]
async fn test_loop_execution() {
    let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new());
    let event_sink = NullEventSink;

    let mut graph = OrchestrationGraph::new("test", "Test");
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
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "loop", "input",
    ));
    graph.edges.push(OrchestrationEdge::new(
        "e2",
        "loop",
        "iteration",
        "loop",
        "loop_back",
    ));
    graph.edges.push(OrchestrationEdge::new(
        "e3", "loop", "complete", "end", "input",
    ));

    let result = executor
        .execute(&graph, HashMap::new(), &event_sink)
        .await
        .unwrap();
    assert!(result.success);
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
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "data", "input",
    ));
    graph
        .edges
        .push(OrchestrationEdge::new("e2", "data", "next", "end", "input"));

    let result = executor
        .execute(&graph, HashMap::new(), &event_sink)
        .await
        .unwrap();
    assert!(result.success);
    assert_eq!(
        result.outputs.get("output_value"),
        Some(&Value::String("success".to_string()))
    );
}

#[tokio::test]
async fn test_data_graph_waiting_for_input_propagates_without_terminal_failure() {
    let mock_executor = MockDataGraphExecutor::new().with_error(
        "test_graph",
        MockDataGraphError::WaitingForInput {
            task_id: "human-input-1".to_string(),
            prompt: Some("Approve deployment?".to_string()),
            emit_event: true,
        },
    );
    let executor = OrchestrationExecutor::new(mock_executor).with_execution_id("orch-exec-test");
    let event_sink = VecEventSink::new();

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
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "data", "input",
    ));
    graph
        .edges
        .push(OrchestrationEdge::new("e2", "data", "next", "end", "input"));

    let result = executor.execute(&graph, HashMap::new(), &event_sink).await;

    assert!(matches!(
        result,
        Err(NodeEngineError::WaitingForInput { task_id, prompt })
            if task_id == "human-input-1" && prompt.as_deref() == Some("Approve deployment?")
    ));

    let events = event_sink.events();
    assert!(events.iter().any(
        |event| matches!(event, WorkflowEvent::WaitingForInput { task_id, prompt, .. }
            if task_id == "human-input-1"
                && prompt.as_deref() == Some("Approve deployment?"))
    ));
    assert!(!events.iter().any(
        |event| matches!(event, WorkflowEvent::TaskCompleted { task_id, .. } if task_id == "data")
    ));
    assert!(!events.iter().any(
        |event| matches!(event, WorkflowEvent::TaskFailed { task_id, .. } if task_id == "data")
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::WorkflowFailed { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::WorkflowCompleted { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::WorkflowCancelled { .. }))
    );
}

#[tokio::test]
async fn test_data_graph_cancelled_propagates_without_task_failure() {
    let mock_executor =
        MockDataGraphExecutor::new().with_error("test_graph", MockDataGraphError::Cancelled);
    let executor = OrchestrationExecutor::new(mock_executor).with_execution_id("orch-exec-test");
    let event_sink = VecEventSink::new();

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
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "data", "input",
    ));
    graph
        .edges
        .push(OrchestrationEdge::new("e2", "data", "next", "end", "input"));

    let result = executor.execute(&graph, HashMap::new(), &event_sink).await;

    assert!(matches!(result, Err(NodeEngineError::Cancelled)));

    let events = event_sink.events();
    assert!(events.iter().any(|event| {
        matches!(
            event,
            WorkflowEvent::WorkflowCancelled {
                workflow_id,
                execution_id,
                error,
                ..
            } if workflow_id == "test"
                && execution_id == "orch-exec-test"
                && error == "Workflow cancelled"
        )
    }));
    assert!(!events.iter().any(
        |event| matches!(event, WorkflowEvent::TaskCompleted { task_id, .. } if task_id == "data")
    ));
    assert!(!events.iter().any(
        |event| matches!(event, WorkflowEvent::TaskFailed { task_id, .. } if task_id == "data")
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::WorkflowFailed { .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorkflowEvent::WorkflowCompleted { .. }))
    );
}

#[tokio::test]
async fn test_missing_start_node_emits_workflow_failed() {
    let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new())
        .with_execution_id("orch-exec-test");
    let graph = OrchestrationGraph::new("test", "Missing Start");
    let event_sink = VecEventSink::new();

    let result = executor.execute(&graph, HashMap::new(), &event_sink).await;

    assert!(matches!(result, Err(NodeEngineError::ExecutionFailed(_))));

    let events = event_sink.events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], WorkflowEvent::WorkflowStarted { .. }));
    assert!(matches!(
        &events[1],
        WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id,
            error,
            ..
        } if workflow_id == "test"
            && execution_id == "orch-exec-test"
            && error == "Task execution failed: Orchestration graph has no Start node"
    ));
}

#[test]
fn test_emit_terminal_workflow_error_uses_cancelled_variant() {
    let executor = OrchestrationExecutor::new(MockDataGraphExecutor::new())
        .with_execution_id("orch-exec-test");
    let event_sink = VecEventSink::new();

    executor.emit_terminal_workflow_error(&event_sink, "test", &NodeEngineError::Cancelled);

    let events = event_sink.events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0],
        WorkflowEvent::WorkflowCancelled {
            workflow_id,
            execution_id,
            error,
            ..
        } if workflow_id == "test"
            && execution_id == "orch-exec-test"
            && error == "Workflow cancelled"
    ));
}
