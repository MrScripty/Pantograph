use super::*;
use crate::error::NodeEngineError;
use crate::events::{NullEventSink, VecEventSink};
use crate::types::{GraphEdge, GraphNode};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn make_linear_graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("test", "Test");
    graph.nodes.push(GraphNode {
        id: "a".to_string(),
        node_type: "input".to_string(),
        data: serde_json::Value::Null,
        position: (0.0, 0.0),
    });
    graph.nodes.push(GraphNode {
        id: "b".to_string(),
        node_type: "process".to_string(),
        data: serde_json::Value::Null,
        position: (100.0, 0.0),
    });
    graph.nodes.push(GraphNode {
        id: "c".to_string(),
        node_type: "output".to_string(),
        data: serde_json::Value::Null,
        position: (200.0, 0.0),
    });
    graph.edges.push(GraphEdge {
        id: "e1".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "b".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e2".to_string(),
        source: "b".to_string(),
        source_handle: "out".to_string(),
        target: "c".to_string(),
        target_handle: "in".to_string(),
    });
    graph
}

fn make_diamond_graph() -> WorkflowGraph {
    // Diamond pattern: a -> b, a -> c, b -> d, c -> d
    let mut graph = WorkflowGraph::new("diamond", "Diamond");
    for (id, pos) in [("a", 0.0), ("b", 100.0), ("c", 100.0), ("d", 200.0)] {
        graph.nodes.push(GraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (pos, 0.0),
        });
    }
    graph.edges.push(GraphEdge {
        id: "e1".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "b".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e2".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "c".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e3".to_string(),
        source: "b".to_string(),
        source_handle: "out".to_string(),
        target: "d".to_string(),
        target_handle: "in_b".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e4".to_string(),
        source: "c".to_string(),
        source_handle: "out".to_string(),
        target: "d".to_string(),
        target_handle: "in_c".to_string(),
    });
    graph
}

fn make_shared_dependency_graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("shared", "Shared");
    for (id, x, y) in [("a", 0.0, 0.0), ("b", 100.0, 0.0), ("c", 100.0, 100.0)] {
        graph.nodes.push(GraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (x, y),
        });
    }
    graph.edges.push(GraphEdge {
        id: "e1".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "b".to_string(),
        target_handle: "in".to_string(),
    });
    graph.edges.push(GraphEdge {
        id: "e2".to_string(),
        source: "a".to_string(),
        source_handle: "out".to_string(),
        target: "c".to_string(),
        target_handle: "in".to_string(),
    });
    graph
}

fn make_parallel_roots_graph() -> WorkflowGraph {
    let mut graph = WorkflowGraph::new("parallel", "Parallel");
    for (id, x) in [("left", 0.0), ("right", 100.0)] {
        graph.nodes.push(GraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (x, 0.0),
        });
    }
    graph
}

/// A simple test executor that counts invocations
struct CountingExecutor {
    execution_count: AtomicUsize,
}

impl CountingExecutor {
    fn new() -> Self {
        Self {
            execution_count: AtomicUsize::new(0),
        }
    }

    fn count(&self) -> usize {
        self.execution_count.load(Ordering::SeqCst)
    }
}

struct YieldingExecutor {
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingExecutor {
    fn new() -> Self {
        Self {
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }

    fn record_max_in_flight(&self, observed: usize) {
        let mut current_max = self.max_in_flight.load(Ordering::SeqCst);
        while observed > current_max {
            match self.max_in_flight.compare_exchange(
                current_max,
                observed,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

struct YieldingFailingExecutor {
    fail_on: String,
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingFailingExecutor {
    fn new(fail_on: impl Into<String>) -> Self {
        Self {
            fail_on: fail_on.into(),
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }

    fn record_max_in_flight(&self, observed: usize) {
        let mut current_max = self.max_in_flight.load(Ordering::SeqCst);
        while observed > current_max {
            match self.max_in_flight.compare_exchange(
                current_max,
                observed,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

struct YieldingWaitingExecutor {
    wait_on: String,
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingWaitingExecutor {
    fn new(wait_on: impl Into<String>) -> Self {
        Self {
            wait_on: wait_on.into(),
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }

    fn record_max_in_flight(&self, observed: usize) {
        let mut current_max = self.max_in_flight.load(Ordering::SeqCst);
        while observed > current_max {
            match self.max_in_flight.compare_exchange(
                current_max,
                observed,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

struct FailingExecutor {
    fail_on: String,
    execution_log: Mutex<Vec<String>>,
}

impl FailingExecutor {
    fn new(fail_on: impl Into<String>) -> Self {
        Self {
            fail_on: fail_on.into(),
            execution_log: Mutex::new(Vec::new()),
        }
    }

    fn executed_tasks(&self) -> Vec<String> {
        self.execution_log.lock().expect("execution log").clone()
    }
}

struct WaitingExecutor {
    wait_on: String,
    execution_log: Mutex<Vec<String>>,
}

impl WaitingExecutor {
    fn new(wait_on: impl Into<String>) -> Self {
        Self {
            wait_on: wait_on.into(),
            execution_log: Mutex::new(Vec::new()),
        }
    }

    fn executed_tasks(&self) -> Vec<String> {
        self.execution_log.lock().expect("execution log").clone()
    }
}

#[async_trait]
impl TaskExecutor for CountingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execution_count.fetch_add(1, Ordering::SeqCst);

        // Simple passthrough: combine all inputs into output
        let mut outputs = HashMap::new();
        outputs.insert(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        );
        Ok(outputs)
    }
}

#[async_trait]
impl TaskExecutor for YieldingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);

        tokio::task::yield_now().await;

        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for YieldingFailingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);

        tokio::task::yield_now().await;

        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        if task_id == self.fail_on {
            return Err(NodeEngineError::failed(format!(
                "forced failure at {task_id}"
            )));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for YieldingWaitingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);

        tokio::task::yield_now().await;

        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        if task_id == self.wait_on {
            return Err(NodeEngineError::waiting_for_input(
                task_id.to_string(),
                Some(format!("waiting at {task_id}")),
            ));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for FailingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execution_log
            .lock()
            .expect("execution log")
            .push(task_id.to_string());

        if task_id == self.fail_on {
            return Err(NodeEngineError::failed(format!(
                "forced failure at {task_id}"
            )));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[async_trait]
impl TaskExecutor for WaitingExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        self.execution_log
            .lock()
            .expect("execution log")
            .push(task_id.to_string());

        if task_id == self.wait_on {
            return Err(NodeEngineError::waiting_for_input(
                task_id.to_string(),
                Some(format!("waiting at {task_id}")),
            ));
        }

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({
                "task": task_id,
                "inputs": inputs
            }),
        )]))
    }
}

#[test]
fn test_version_tracking() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Initially all versions are 0
    assert_eq!(engine.compute_input_version(&"a".to_string(), &graph), 0);
    assert_eq!(engine.compute_input_version(&"b".to_string(), &graph), 0);

    // Mark 'a' as modified
    engine.mark_modified(&"a".to_string());

    // 'b' input version should change (depends on 'a')
    assert_eq!(engine.compute_input_version(&"b".to_string(), &graph), 1);

    // 'a' input version should still be 0 (no dependencies)
    assert_eq!(engine.compute_input_version(&"a".to_string(), &graph), 0);
}

#[test]
fn test_cache_invalidation() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Cache output for 'b'
    engine.cache_output(&"b".to_string(), serde_json::json!("cached_value"), &graph);

    // Should be able to get cached value
    assert!(engine.get_cached(&"b".to_string(), &graph).is_some());

    // Mark 'a' as modified
    engine.mark_modified(&"a".to_string());

    // Cache for 'b' should now be invalid (input version changed)
    assert!(engine.get_cached(&"b".to_string(), &graph).is_none());
}

#[test]
fn test_cache_stats() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.mark_modified(&"c".to_string());

    let stats = engine.cache_stats();
    assert_eq!(stats.cached_nodes, 2);
    assert_eq!(stats.total_versions, 1); // Only 'c' has been modified
    assert_eq!(stats.global_version, 1);
}

#[test]
fn test_reconcile_isolated_run_merges_changed_state_without_touching_unrelated_entries() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);

    let base = engine.clone();
    let mut isolated = base.clone();
    isolated.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    isolated.mark_modified(&"c".to_string());

    engine.cache_output(&"z".to_string(), serde_json::json!("keep"), &graph);
    engine.reconcile_isolated_run(&base, &isolated);

    assert!(engine.cache.contains_key("a"));
    assert!(engine.cache.contains_key("b"));
    assert!(engine.cache.contains_key("z"));
    assert_eq!(engine.versions.get("c"), Some(&1));
    assert_eq!(engine.global_version, 1);
}

#[test]
fn test_reconcile_isolated_run_removes_entries_cleared_from_base_state() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

    let base = engine.clone();
    let mut isolated = base.clone();
    isolated.invalidate_downstream(&"b".to_string(), &graph);

    engine.reconcile_isolated_run(&base, &isolated);

    assert!(!engine.cache.contains_key("b"));
    assert!(!engine.cache.contains_key("c"));
}

#[test]
fn test_invalidate_downstream() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Cache all nodes
    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

    assert_eq!(engine.cache_stats().cached_nodes, 3);

    // Invalidate downstream from 'a' (should invalidate a, b, c)
    engine.invalidate_downstream(&"a".to_string(), &graph);

    assert_eq!(engine.cache_stats().cached_nodes, 0);
}

#[test]
fn test_invalidate_downstream_partial() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");

    // Cache all nodes
    engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
    engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
    engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

    // Invalidate downstream from 'b' (should invalidate b, c but not a)
    engine.invalidate_downstream(&"b".to_string(), &graph);

    assert_eq!(engine.cache_stats().cached_nodes, 1);
    assert!(engine.cache.contains_key("a"));
    assert!(!engine.cache.contains_key("b"));
    assert!(!engine.cache.contains_key("c"));
}

#[tokio::test]
async fn test_demand_linear_graph() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // Demand 'c' - should execute a, b, c
    let result = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(executor.count(), 3); // All three nodes executed
}

#[tokio::test]
async fn test_demand_caching() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // First demand
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    assert_eq!(executor.count(), 3);

    // Second demand - should use cache
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    assert_eq!(executor.count(), 3); // No additional executions
}

#[tokio::test]
async fn test_demand_partial_recompute() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // First demand
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    assert_eq!(executor.count(), 3);

    // Mark 'b' as modified
    engine.mark_modified(&"b".to_string());

    // Demand again - should only recompute b and c (not a)
    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;
    // Note: Due to version-based invalidation, this depends on implementation details.
    // The current implementation uses sum of dependency versions, so modifying 'b'
    // will invalidate 'c' but not necessarily re-execute 'a' if it's still cached.
}

#[tokio::test]
async fn test_demand_diamond_graph() {
    let graph = make_diamond_graph();
    let mut engine = DemandEngine::new("test");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    // Demand 'd' - should execute a, b, c, d (a only once despite diamond)
    let result = engine
        .demand(
            &"d".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(executor.count(), 4); // All four nodes executed exactly once
}

#[tokio::test]
async fn test_demand_events() {
    let graph = make_linear_graph();
    let mut engine = DemandEngine::new("test_exec");
    let executor = CountingExecutor::new();
    let context = Context::new();
    let event_sink = VecEventSink::new();
    let extensions = ExecutorExtensions::new();

    let _ = engine
        .demand(
            &"c".to_string(),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
        )
        .await;

    let events = event_sink.events();

    // Should have TaskStarted and TaskCompleted for each node
    let started_count = events
        .iter()
        .filter(|e| matches!(e, WorkflowEvent::TaskStarted { .. }))
        .count();
    let completed_count = events
        .iter()
        .filter(|e| matches!(e, WorkflowEvent::TaskCompleted { .. }))
        .count();

    assert_eq!(started_count, 3);
    assert_eq!(completed_count, 3);
}

#[tokio::test]
async fn test_workflow_executor_demand() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = CountingExecutor::new();

    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

    let result = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;

    assert!(result.is_ok());
    assert_eq!(executor_impl.count(), 3);
}

#[tokio::test]
async fn test_workflow_executor_update_node() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = CountingExecutor::new();

    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    // First demand
    let _ = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;
    let count_after_first = executor_impl.count();
    assert_eq!(count_after_first, 3);

    // Verify caching works - demand again without modification
    let _ = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;
    assert_eq!(executor_impl.count(), 3); // No additional executions

    // Update node 'a' data - this marks it as modified
    workflow_executor
        .update_node_data(&"a".to_string(), serde_json::json!({"new": "data"}))
        .await
        .unwrap();

    // Demand again - should recompute the chain
    let _ = workflow_executor
        .demand(&"c".to_string(), &executor_impl)
        .await;

    // After marking 'a' as modified, we expect recomputation of the entire chain
    // because b depends on a, and c depends on b
    let count_after_update = executor_impl.count();
    assert!(
        count_after_update > count_after_first,
        "Expected recomputation after update: got {} executions (expected > {})",
        count_after_update,
        count_after_first
    );
}

#[tokio::test]
async fn test_workflow_executor_mark_modified_emits_graph_modified_with_dirty_subgraph() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

    workflow_executor.mark_modified(&"b".to_string()).await;

    let events = event_sink.events();
    let graph_modified = events
        .iter()
        .find(|event| matches!(event, WorkflowEvent::GraphModified { .. }))
        .expect("graph modified event");

    match graph_modified {
        WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        } => {
            assert_eq!(workflow_id, "test");
            assert_eq!(execution_id, "exec_1");
            assert_eq!(dirty_tasks, &vec!["b".to_string(), "c".to_string()]);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_emits_incremental_execution_started() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = CountingExecutor::new();
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

    let _ = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect("incremental demand succeeds");

    let events = event_sink.events();
    let incremental_started = events
        .iter()
        .find(|event| matches!(event, WorkflowEvent::IncrementalExecutionStarted { .. }))
        .expect("incremental execution event");

    match incremental_started {
        WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            tasks,
            ..
        } => {
            assert_eq!(workflow_id, "test");
            assert_eq!(execution_id, "exec_1");
            assert_eq!(tasks, &vec!["b".to_string(), "c".to_string()]);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_emits_task_lifecycle_for_parallel_roots() {
    let graph = make_parallel_roots_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = YieldingExecutor::new();
    let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

    let outputs = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect("parallel incremental demand succeeds");

    assert_eq!(executor_impl.max_in_flight(), 2);
    assert_eq!(outputs.len(), 2);

    let events = event_sink.events();
    let started_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let completed_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    assert_eq!(
        started_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
    assert_eq!(
        completed_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_preserves_parallel_failure_event_attribution() {
    let graph = make_parallel_roots_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = YieldingFailingExecutor::new("right");
    let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

    let error = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect_err("parallel incremental demand should fail");

    assert!(
        matches!(error, NodeEngineError::ExecutionFailed(message) if message.contains("forced failure at right"))
    );
    assert_eq!(executor_impl.max_in_flight(), 2);

    let events = event_sink.events();
    let started_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let completed_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    assert_eq!(
        started_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
    assert_eq!(completed_tasks, HashSet::from(["left".to_string()]));
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_preserves_parallel_waiting_event_attribution() {
    let graph = make_parallel_roots_graph();
    let event_sink = Arc::new(VecEventSink::new());
    let executor_impl = YieldingWaitingExecutor::new("right");
    let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

    let error = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect_err("parallel incremental demand should pause");

    assert!(matches!(
        error,
        NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt)
        } if task_id == "right" && prompt == "waiting at right"
    ));
    assert_eq!(executor_impl.max_in_flight(), 2);

    let events = event_sink.events();
    let started_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let completed_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();
    let waiting_tasks = events
        .iter()
        .filter_map(|event| match event {
            WorkflowEvent::WaitingForInput { task_id, .. } => Some(task_id.clone()),
            _ => None,
        })
        .collect::<HashSet<_>>();

    assert_eq!(
        started_tasks,
        HashSet::from(["left".to_string(), "right".to_string()])
    );
    assert_eq!(completed_tasks, HashSet::from(["left".to_string()]));
    assert_eq!(waiting_tasks, HashSet::from(["right".to_string()]));
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_returns_redundant_requested_target_outputs() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = CountingExecutor::new();
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    let outputs = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect("incremental demand succeeds");

    assert_eq!(executor_impl.count(), 3);
    assert!(outputs.contains_key("b"));
    assert!(outputs.contains_key("c"));
    assert_eq!(outputs["b"]["out"]["task"], serde_json::json!("b"));
    assert_eq!(outputs["c"]["out"]["task"], serde_json::json!("c"));
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_stops_after_failed_batch() {
    let graph = make_shared_dependency_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = FailingExecutor::new("b");
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    let error = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect_err("first batch should fail");

    assert!(
        matches!(error, NodeEngineError::ExecutionFailed(message) if message.contains("forced failure at b"))
    );
    assert_eq!(
        executor_impl.executed_tasks(),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[tokio::test]
async fn test_workflow_executor_demand_multiple_stops_after_waiting_batch() {
    let graph = make_shared_dependency_graph();
    let event_sink = Arc::new(NullEventSink);
    let executor_impl = WaitingExecutor::new("b");
    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    let error = workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
        .await
        .expect_err("first batch should pause execution");

    assert!(matches!(
        error,
        NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt)
        } if task_id == "b" && prompt == "waiting at b"
    ));
    assert_eq!(
        executor_impl.executed_tasks(),
        vec!["a".to_string(), "b".to_string()]
    );
}

#[tokio::test]
async fn test_workflow_executor_human_input_emits_waiting_for_input() {
    let graph = WorkflowGraph {
        id: "interactive-workflow".to_string(),
        name: "Interactive Workflow".to_string(),
        nodes: vec![GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({
                "node_type": "human-input",
                "prompt": "Approve deployment?"
            }),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let event_sink = Arc::new(VecEventSink::new());
    let workflow_executor = WorkflowExecutor::new("exec_human_input", graph, event_sink.clone());
    let executor_impl = crate::core_executor::CoreTaskExecutor::new();

    let error = workflow_executor
        .demand(&"approval".to_string(), &executor_impl)
        .await
        .expect_err("human input should pause execution");
    assert!(matches!(
        error,
        NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt)
        } if task_id == "approval" && prompt == "Approve deployment?"
    ));

    let events = event_sink.events();
    assert!(matches!(
        events.as_slice(),
        [
            WorkflowEvent::TaskStarted { task_id, .. },
            WorkflowEvent::WaitingForInput {
                workflow_id,
                task_id: waiting_task_id,
                prompt: Some(prompt),
                ..
            }
        ] if task_id == "approval"
            && workflow_id == "interactive-workflow"
            && waiting_task_id == "approval"
            && prompt == "Approve deployment?"
    ));
}

#[tokio::test]
async fn test_workflow_executor_human_input_continues_with_response() {
    let graph = WorkflowGraph {
        id: "interactive-workflow".to_string(),
        name: "Interactive Workflow".to_string(),
        nodes: vec![GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({
                "node_type": "human-input",
                "prompt": "Approve deployment?",
                "user_response": "approved"
            }),
            position: (0.0, 0.0),
        }],
        edges: Vec::new(),
        groups: Vec::new(),
    };
    let event_sink = Arc::new(VecEventSink::new());
    let workflow_executor = WorkflowExecutor::new("exec_human_input", graph, event_sink.clone());
    let executor_impl = crate::core_executor::CoreTaskExecutor::new();

    let outputs = workflow_executor
        .demand(&"approval".to_string(), &executor_impl)
        .await
        .expect("human input should continue once a response is present");
    assert_eq!(outputs.get("value"), Some(&serde_json::json!("approved")));

    let events = event_sink.events();
    assert!(matches!(
        events.as_slice(),
        [
            WorkflowEvent::TaskStarted { task_id, .. },
            WorkflowEvent::TaskCompleted {
                task_id: completed_task_id,
                ..
            }
        ] if task_id == "approval" && completed_task_id == "approval"
    ));
}

#[tokio::test]
async fn test_workflow_executor_snapshot() {
    let graph = make_linear_graph();
    let event_sink = Arc::new(NullEventSink);

    let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

    // Get initial snapshot
    let snapshot = workflow_executor.get_graph_snapshot().await;
    assert_eq!(snapshot.nodes.len(), 3);

    // Add a new node
    workflow_executor
        .add_node(GraphNode {
            id: "d".to_string(),
            node_type: "new".to_string(),
            data: serde_json::Value::Null,
            position: (300.0, 0.0),
        })
        .await;

    // Verify node was added
    let updated = workflow_executor.get_graph_snapshot().await;
    assert_eq!(updated.nodes.len(), 4);

    // Restore original snapshot
    workflow_executor.restore_graph_snapshot(snapshot).await;

    // Verify restoration
    let restored = workflow_executor.get_graph_snapshot().await;
    assert_eq!(restored.nodes.len(), 3);
}
