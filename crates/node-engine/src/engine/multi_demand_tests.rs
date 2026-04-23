use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use graph_flow::Context;

use crate::engine::{DemandRuntimeContext, WorkflowExecutor};
use crate::error::NodeEngineError;
use crate::events::{EventSink, NullEventSink};
use crate::extensions::ExecutorExtensions;
use crate::types::{GraphEdge, GraphNode, WorkflowGraph};

use super::{
    DemandBatchDispatchPlan, DemandBatchExecutionOutcome, DemandBatchExecutionResult,
    DemandDispatchWindowExecutionMode, DemandDispatchWindowOutcome, DemandDispatchWindowPlan,
    DemandDispatchWindowResult, DemandEngine, DemandExecutionBudget, DemandMultiplePlan,
    DemandMultipleResults, DemandWindowRunner, TaskExecutor, demand_multiple_with_default_budget,
    demand_multiple_with_explicit_budget,
};

fn demand_runtime<'a>(
    graph: &'a WorkflowGraph,
    executor: &'a dyn TaskExecutor,
    context: &'a Context,
    event_sink: &'a dyn EventSink,
    extensions: &'a ExecutorExtensions,
) -> DemandRuntimeContext<'a> {
    DemandRuntimeContext::new(graph, executor, context, event_sink, extensions, None)
}

fn make_linear_graph() -> WorkflowGraph {
    WorkflowGraph {
        id: "graph".to_string(),
        name: "Graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "a".to_string(),
                node_type: "input".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            },
            GraphNode {
                id: "b".to_string(),
                node_type: "middle".to_string(),
                data: serde_json::json!({}),
                position: (1.0, 0.0),
            },
            GraphNode {
                id: "c".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (2.0, 0.0),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "e1".to_string(),
                source: "a".to_string(),
                source_handle: "out".to_string(),
                target: "b".to_string(),
                target_handle: "in".to_string(),
            },
            GraphEdge {
                id: "e2".to_string(),
                source: "b".to_string(),
                source_handle: "out".to_string(),
                target: "c".to_string(),
                target_handle: "in".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

struct SnapshotTaskExecutor;

#[async_trait]
impl TaskExecutor for SnapshotTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        _inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> crate::error::Result<HashMap<String, serde_json::Value>> {
        Ok(HashMap::from([(
            "value".to_string(),
            serde_json::json!(task_id),
        )]))
    }
}

fn make_disjoint_branches_graph() -> WorkflowGraph {
    WorkflowGraph {
        id: "graph".to_string(),
        name: "Graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "a".to_string(),
                node_type: "input".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            },
            GraphNode {
                id: "b".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (1.0, 0.0),
            },
            GraphNode {
                id: "x".to_string(),
                node_type: "input".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 1.0),
            },
            GraphNode {
                id: "y".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (1.0, 1.0),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "e1".to_string(),
                source: "a".to_string(),
                source_handle: "out".to_string(),
                target: "b".to_string(),
                target_handle: "in".to_string(),
            },
            GraphEdge {
                id: "e2".to_string(),
                source: "x".to_string(),
                source_handle: "out".to_string(),
                target: "y".to_string(),
                target_handle: "in".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

fn make_shared_dependency_graph() -> WorkflowGraph {
    WorkflowGraph {
        id: "graph".to_string(),
        name: "Graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "a".to_string(),
                node_type: "input".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            },
            GraphNode {
                id: "b".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (1.0, 0.0),
            },
            GraphNode {
                id: "c".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (1.0, 1.0),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "e1".to_string(),
                source: "a".to_string(),
                source_handle: "out".to_string(),
                target: "b".to_string(),
                target_handle: "in".to_string(),
            },
            GraphEdge {
                id: "e2".to_string(),
                source: "a".to_string(),
                source_handle: "out".to_string(),
                target: "c".to_string(),
                target_handle: "in".to_string(),
            },
        ],
        groups: Vec::new(),
    }
}

fn make_parallel_roots_graph() -> WorkflowGraph {
    WorkflowGraph {
        id: "graph".to_string(),
        name: "Graph".to_string(),
        nodes: vec![
            GraphNode {
                id: "left".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            },
            GraphNode {
                id: "right".to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({}),
                position: (1.0, 0.0),
            },
        ],
        edges: Vec::new(),
        groups: Vec::new(),
    }
}

struct YieldingConcurrencyExecutor {
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingConcurrencyExecutor {
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

#[async_trait]
impl TaskExecutor for YieldingConcurrencyExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> crate::error::Result<HashMap<String, serde_json::Value>> {
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

struct TimedHarnessExecutor {
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    per_task_delay: Duration,
}

impl TimedHarnessExecutor {
    fn new(per_task_delay: Duration) -> Self {
        Self {
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            per_task_delay,
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

#[async_trait]
impl TaskExecutor for TimedHarnessExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> crate::error::Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);
        tokio::time::sleep(self.per_task_delay).await;
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

struct DemandHarnessObservation {
    elapsed: Duration,
    max_in_flight: usize,
    outputs: HashMap<String, HashMap<String, serde_json::Value>>,
}

async fn run_parallel_demand_harness(budget: usize) -> DemandHarnessObservation {
    let graph = make_parallel_roots_graph();
    let mut engine = DemandEngine::new("test");
    let executor = TimedHarnessExecutor::new(Duration::from_millis(30));
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    let started_at = Instant::now();
    let runtime = demand_runtime(&graph, &executor, &context, &event_sink, &extensions);
    let outputs = demand_multiple_with_explicit_budget(
        &mut engine,
        &["left".to_string(), "right".to_string()],
        DemandExecutionBudget::new(budget),
        runtime,
    )
    .await
    .expect("demand harness should succeed");

    DemandHarnessObservation {
        elapsed: started_at.elapsed(),
        max_in_flight: executor.max_in_flight(),
        outputs,
    }
}

fn assert_send_future<F>(_: &F)
where
    F: Future + Send,
{
}

#[test]
fn plan_preserves_requested_target_order_for_event_payloads() {
    let graph = make_linear_graph();
    let plan = DemandMultiplePlan::from_requested_targets(
        &[
            "node_b".to_string(),
            "node_a".to_string(),
            "node_c".to_string(),
        ],
        &graph,
    );

    assert_eq!(
        plan.requested_targets(),
        &[
            "node_b".to_string(),
            "node_a".to_string(),
            "node_c".to_string()
        ]
    );
}

#[test]
fn plan_starts_with_sequential_execution_order() {
    let graph = make_linear_graph();
    let plan = DemandMultiplePlan::from_requested_targets(&["a".to_string()], &graph);

    assert_eq!(plan.execution_batches(), &[vec!["a".to_string()]]);
}

#[test]
fn plan_handles_empty_requests() {
    let graph = make_linear_graph();
    let plan = DemandMultiplePlan::from_requested_targets(&[], &graph);

    assert!(plan.requested_targets().is_empty());
    assert!(plan.execution_batches().is_empty());
}

#[test]
fn isolated_target_future_satisfies_send_boundary() {
    let graph = make_parallel_roots_graph();
    let mut engine = DemandEngine::new("test");
    let executor = YieldingConcurrencyExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();
    let runtime = demand_runtime(&graph, &executor, &context, &event_sink, &extensions);
    let runner = DemandWindowRunner::new(&mut engine, runtime);
    let base_engine = runner.clone_engine();
    let future = runner.demand_target_in_isolation_future(&base_engine, "left".to_string());

    assert_send_future(&future);
}

#[test]
fn plan_prunes_requested_targets_covered_by_requested_dependents() {
    let graph = make_linear_graph();
    let plan =
        DemandMultiplePlan::from_requested_targets(&["b".to_string(), "c".to_string()], &graph);

    assert_eq!(
        plan.requested_targets(),
        &["b".to_string(), "c".to_string()]
    );
    assert_eq!(plan.execution_batches(), &[vec!["c".to_string()]]);
}

#[test]
fn plan_dedupes_execution_targets_while_preserving_requested_duplicates() {
    let graph = make_linear_graph();
    let plan =
        DemandMultiplePlan::from_requested_targets(&["c".to_string(), "c".to_string()], &graph);

    assert_eq!(
        plan.requested_targets(),
        &["c".to_string(), "c".to_string()]
    );
    assert_eq!(plan.execution_batches(), &[vec!["c".to_string()]]);
}

#[test]
fn plan_places_current_root_targets_into_one_batch() {
    let graph = make_linear_graph();
    let plan =
        DemandMultiplePlan::from_requested_targets(&["a".to_string(), "c".to_string()], &graph);

    assert_eq!(plan.execution_batches(), &[vec!["c".to_string()]]);
}

#[test]
fn plan_groups_independent_root_targets_into_one_batch() {
    let graph = make_disjoint_branches_graph();
    let plan =
        DemandMultiplePlan::from_requested_targets(&["b".to_string(), "y".to_string()], &graph);

    assert_eq!(
        plan.execution_batches(),
        &[vec!["b".to_string(), "y".to_string()]]
    );
}

#[test]
fn plan_separates_root_targets_with_shared_dependencies() {
    let graph = make_shared_dependency_graph();
    let plan =
        DemandMultiplePlan::from_requested_targets(&["b".to_string(), "c".to_string()], &graph);

    assert_eq!(
        plan.execution_batches(),
        &[vec!["b".to_string()], vec!["c".to_string()]]
    );
}

#[test]
fn execution_budget_defaults_to_one_in_flight() {
    assert_eq!(DemandExecutionBudget::sequential().max_in_flight(), 1);
}

#[test]
fn execution_budget_default_parallel_uses_two_in_flight() {
    assert_eq!(DemandExecutionBudget::default_parallel().max_in_flight(), 2);
}

#[test]
fn execution_budget_normalizes_zero_to_one_in_flight() {
    assert_eq!(DemandExecutionBudget::new(0).max_in_flight(), 1);
}

#[test]
fn batch_execution_outcome_tracks_completed_targets_in_order() {
    let mut outcome = DemandBatchExecutionOutcome::default();
    outcome.record_completed_target(&"node_a".to_string());
    outcome.record_completed_target(&"node_b".to_string());

    assert_eq!(
        outcome.completed_targets(),
        &["node_a".to_string(), "node_b".to_string()]
    );
}

#[test]
fn dispatch_window_outcome_tracks_completed_targets_in_order() {
    let mut outcome = DemandDispatchWindowOutcome::default();
    outcome.record_completed_target(&"node_a".to_string());
    outcome.record_completed_target(&"node_b".to_string());

    assert_eq!(
        outcome.completed_targets(),
        &["node_a".to_string(), "node_b".to_string()]
    );
}

#[test]
fn batch_execution_result_carries_interrupt_error() {
    let result = DemandBatchExecutionResult::Interrupted(NodeEngineError::waiting_for_input(
        "approval",
        Some("Approve deployment?".to_string()),
    ));

    match result {
        DemandBatchExecutionResult::Interrupted(NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt),
        }) => {
            assert_eq!(task_id, "approval");
            assert_eq!(prompt, "Approve deployment?");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn dispatch_window_result_carries_interrupt_error() {
    let result = DemandDispatchWindowResult::Interrupted(NodeEngineError::waiting_for_input(
        "approval",
        Some("Approve deployment?".to_string()),
    ));

    match result {
        DemandDispatchWindowResult::Interrupted(NodeEngineError::WaitingForInput {
            task_id,
            prompt: Some(prompt),
        }) => {
            assert_eq!(task_id, "approval");
            assert_eq!(prompt, "Approve deployment?");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn batch_dispatch_plan_splits_windows_by_budget() {
    let plan = DemandBatchDispatchPlan::from_batch(
        &[
            "node_a".to_string(),
            "node_b".to_string(),
            "node_c".to_string(),
        ],
        DemandExecutionBudget::new(2),
    );

    assert_eq!(
        plan.execution_windows(),
        &[
            DemandDispatchWindowPlan::new(
                vec!["node_a".to_string(), "node_b".to_string()],
                DemandDispatchWindowExecutionMode::BoundedParallel,
            ),
            DemandDispatchWindowPlan::new(
                vec!["node_c".to_string()],
                DemandDispatchWindowExecutionMode::Sequential,
            )
        ]
    );
}

#[test]
fn batch_dispatch_plan_normalizes_zero_budget_to_singleton_windows() {
    let plan = DemandBatchDispatchPlan::from_batch(
        &["node_a".to_string(), "node_b".to_string()],
        DemandExecutionBudget::new(0),
    );

    assert_eq!(
        plan.execution_windows(),
        &[
            DemandDispatchWindowPlan::new(
                vec!["node_a".to_string()],
                DemandDispatchWindowExecutionMode::Sequential,
            ),
            DemandDispatchWindowPlan::new(
                vec!["node_b".to_string()],
                DemandDispatchWindowExecutionMode::Sequential,
            )
        ]
    );
}

#[test]
fn batch_dispatch_plan_uses_singleton_windows_for_sequential_budget() {
    let plan = DemandBatchDispatchPlan::from_batch(
        &["node_a".to_string(), "node_b".to_string()],
        DemandExecutionBudget::sequential(),
    );

    assert_eq!(
        plan.execution_windows(),
        &[
            DemandDispatchWindowPlan::new(
                vec!["node_a".to_string()],
                DemandDispatchWindowExecutionMode::Sequential,
            ),
            DemandDispatchWindowPlan::new(
                vec!["node_b".to_string()],
                DemandDispatchWindowExecutionMode::Sequential,
            )
        ]
    );
}

#[test]
fn dispatch_window_plan_marks_execution_mode() {
    let window = DemandDispatchWindowPlan::new(
        vec!["node_a".to_string(), "node_b".to_string()],
        DemandDispatchWindowExecutionMode::BoundedParallel,
    );

    assert_eq!(
        window.targets(),
        &["node_a".to_string(), "node_b".to_string()]
    );
    assert_eq!(
        window.execution_mode(),
        DemandDispatchWindowExecutionMode::BoundedParallel
    );
}

#[test]
fn dispatch_window_execution_mode_falls_back_to_sequential() {
    assert_eq!(
        DemandDispatchWindowExecutionMode::for_window(1, 2),
        DemandDispatchWindowExecutionMode::Sequential
    );
    assert_eq!(
        DemandDispatchWindowExecutionMode::for_window(2, 1),
        DemandDispatchWindowExecutionMode::Sequential
    );
    assert_eq!(
        DemandDispatchWindowExecutionMode::for_window(2, 2),
        DemandDispatchWindowExecutionMode::BoundedParallel
    );
}

#[tokio::test]
async fn bounded_parallel_budget_runs_independent_targets_concurrently() {
    let graph = make_parallel_roots_graph();
    let mut engine = DemandEngine::new("test");
    let executor = YieldingConcurrencyExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    let runtime = demand_runtime(&graph, &executor, &context, &event_sink, &extensions);
    let outputs = demand_multiple_with_explicit_budget(
        &mut engine,
        &["left".to_string(), "right".to_string()],
        DemandExecutionBudget::new(2),
        runtime,
    )
    .await
    .expect("parallel demand should succeed");

    assert_eq!(executor.max_in_flight(), 2);
    assert_eq!(outputs.len(), 2);
    assert!(outputs.contains_key("left"));
    assert!(outputs.contains_key("right"));
}

#[tokio::test]
async fn default_budget_runs_independent_targets_concurrently() {
    let graph = make_parallel_roots_graph();
    let mut engine = DemandEngine::new("test");
    let executor = YieldingConcurrencyExecutor::new();
    let context = Context::new();
    let event_sink = NullEventSink;
    let extensions = ExecutorExtensions::new();

    let runtime = demand_runtime(&graph, &executor, &context, &event_sink, &extensions);
    let outputs = demand_multiple_with_default_budget(
        &mut engine,
        &["left".to_string(), "right".to_string()],
        runtime,
    )
    .await
    .expect("default parallel demand should succeed");

    assert_eq!(executor.max_in_flight(), 2);
    assert_eq!(outputs.len(), 2);
    assert!(outputs.contains_key("left"));
    assert!(outputs.contains_key("right"));
}

#[tokio::test]
async fn workflow_executor_multi_demand_records_bound_session_node_memory_from_cache() {
    let workflow_executor =
        WorkflowExecutor::new("exec-1", make_linear_graph(), Arc::new(NullEventSink));
    workflow_executor.bind_workflow_session("session-1").await;

    workflow_executor
        .demand_multiple(&["b".to_string(), "c".to_string()], &SnapshotTaskExecutor)
        .await
        .expect("multi-demand graph");

    let snapshots = workflow_executor
        .workflow_session_node_memory_snapshots("session-1")
        .await;
    assert_eq!(snapshots.len(), 3);
    assert_eq!(
        snapshots
            .iter()
            .map(|snapshot| snapshot.identity.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        snapshots[2].output_snapshot,
        Some(serde_json::json!({ "value": "c" }))
    );
}

#[tokio::test]
async fn workflow_executor_parallel_multi_demand_reconciles_input_snapshots() {
    let workflow_executor = WorkflowExecutor::new(
        "exec-1",
        make_parallel_roots_graph(),
        Arc::new(NullEventSink),
    );
    workflow_executor.bind_workflow_session("session-1").await;

    workflow_executor
        .demand_multiple(
            &["left".to_string(), "right".to_string()],
            &SnapshotTaskExecutor,
        )
        .await
        .expect("parallel multi-demand graph");

    let snapshots = workflow_executor
        .workflow_session_node_memory_snapshots("session-1")
        .await;
    assert_eq!(snapshots.len(), 2);
    assert_eq!(
        snapshots
            .iter()
            .map(|snapshot| snapshot.identity.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["left", "right"]
    );
    assert!(
        snapshots
            .iter()
            .all(|snapshot| snapshot.input_fingerprint.as_deref() == Some("{\"_data\":{}}"))
    );
    assert!(snapshots.iter().all(|snapshot| {
        snapshot.inspection_metadata
            == Some(serde_json::json!({
                "projection_source": "demand_engine_cache",
                "cache_version": 0,
                "input_snapshot": { "_data": {} }
            }))
    }));
}

#[tokio::test]
#[ignore = "benchmark-like harness for comparing sequential and bounded parallel demand"]
async fn demand_harness_compares_sequential_and_parallel_baselines() {
    let sequential = run_parallel_demand_harness(1).await;
    let bounded_parallel = run_parallel_demand_harness(2).await;

    assert_eq!(sequential.max_in_flight, 1);
    assert_eq!(bounded_parallel.max_in_flight, 2);
    assert_eq!(sequential.outputs, bounded_parallel.outputs);
    assert!(
        bounded_parallel.elapsed < sequential.elapsed,
        "expected bounded parallel harness to finish faster than sequential baseline: sequential={:?}, parallel={:?}",
        sequential.elapsed,
        bounded_parallel.elapsed
    );
}

#[test]
fn results_keep_distinct_targets() {
    let mut results = DemandMultipleResults::default();
    let output_a = HashMap::from([("value".to_string(), serde_json::json!("first"))]);
    let output_b = HashMap::from([("value".to_string(), serde_json::json!("second"))]);

    results.record_success(&"node_a".to_string(), output_a);
    results.record_success(&"node_b".to_string(), output_b);

    let outputs = results.into_outputs();
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs["node_a"]["value"], serde_json::json!("first"));
    assert_eq!(outputs["node_b"]["value"], serde_json::json!("second"));
}

#[test]
fn results_use_last_write_for_duplicate_targets() {
    let mut results = DemandMultipleResults::default();
    let first_output = HashMap::from([("value".to_string(), serde_json::json!("first"))]);
    let second_output = HashMap::from([("value".to_string(), serde_json::json!("second"))]);

    results.record_success(&"node_a".to_string(), first_output);
    results.record_success(&"node_a".to_string(), second_output);

    let outputs = results.into_outputs();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs["node_a"]["value"], serde_json::json!("second"));
}
