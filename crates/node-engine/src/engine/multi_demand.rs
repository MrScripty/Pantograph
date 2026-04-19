use std::collections::{HashMap, HashSet};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;

use graph_flow::Context;

use super::{DemandEngine, TaskExecutor, WorkflowExecutor};
use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::types::{NodeId, WorkflowGraph};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemandMultiplePlan {
    requested_targets: Vec<NodeId>,
    execution_batches: Vec<Vec<NodeId>>,
}

#[derive(Debug, Default)]
struct DemandMultipleResults {
    outputs: HashMap<NodeId, HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Default)]
struct DemandBatchExecutionOutcome {
    completed_targets: Vec<NodeId>,
}

#[derive(Debug, Default)]
struct DemandDispatchWindowOutcome {
    completed_targets: Vec<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DemandDispatchWindowExecutionMode {
    Sequential,
    BoundedParallel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemandDispatchWindowPlan {
    targets: Vec<NodeId>,
    execution_mode: DemandDispatchWindowExecutionMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemandBatchDispatchPlan {
    execution_windows: Vec<DemandDispatchWindowPlan>,
}

#[derive(Debug)]
enum DemandBatchExecutionResult {
    Completed(DemandBatchExecutionOutcome),
    Interrupted(NodeEngineError),
}

#[derive(Debug)]
enum DemandDispatchWindowResult {
    Completed(DemandDispatchWindowOutcome),
    Interrupted(NodeEngineError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DemandExecutionBudget {
    max_in_flight: usize,
}

struct DemandMultipleCoordinator<'a> {
    budget: DemandExecutionBudget,
    plan: &'a DemandMultiplePlan,
    window_runner: DemandWindowRunner<'a>,
    results: DemandMultipleResults,
}

struct DemandWindowRunner<'a> {
    engine: &'a mut DemandEngine,
    graph: &'a WorkflowGraph,
    executor: &'a dyn TaskExecutor,
    context: &'a Context,
    event_sink: &'a dyn EventSink,
    extensions: &'a ExecutorExtensions,
    node_memories: Option<&'a HashMap<NodeId, super::NodeMemorySnapshot>>,
}

struct DemandIsolatedTargetRun {
    node_id: NodeId,
    outputs: HashMap<String, serde_json::Value>,
    engine: DemandEngine,
}

type DemandIsolatedTargetRunFuture<'a> =
    Pin<Box<dyn Future<Output = Result<DemandIsolatedTargetRun>> + Send + 'a>>;

impl DemandMultiplePlan {
    fn from_requested_targets(node_ids: &[NodeId], graph: &WorkflowGraph) -> Self {
        let mut requested_targets = Vec::new();
        let mut requested_target_set = HashSet::new();

        for node_id in node_ids {
            requested_targets.push(node_id.clone());
            requested_target_set.insert(node_id.clone());
        }

        let execution_targets = requested_targets
            .iter()
            .filter(|node_id| !is_redundant_requested_target(graph, node_id, &requested_target_set))
            .fold(Vec::new(), |mut deduped_targets, node_id| {
                if !deduped_targets.contains(node_id) {
                    deduped_targets.push(node_id.clone());
                }
                deduped_targets
            });

        Self {
            requested_targets,
            execution_batches: into_execution_batches(graph, execution_targets),
        }
    }

    fn requested_targets(&self) -> &[NodeId] {
        &self.requested_targets
    }

    fn execution_batches(&self) -> &[Vec<NodeId>] {
        &self.execution_batches
    }
}

fn into_execution_batches(
    graph: &WorkflowGraph,
    execution_targets: Vec<NodeId>,
) -> Vec<Vec<NodeId>> {
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    let mut current_batch_nodes = HashSet::new();

    for node_id in execution_targets {
        let dependency_closure = collect_dependency_closure(graph, &node_id, &mut HashSet::new());
        let overlaps_current_batch = dependency_closure
            .iter()
            .any(|dependency_id| current_batch_nodes.contains(dependency_id));

        if overlaps_current_batch && !current_batch.is_empty() {
            batches.push(current_batch);
            current_batch = Vec::new();
            current_batch_nodes = HashSet::new();
        }

        current_batch_nodes.extend(dependency_closure);
        current_batch.push(node_id);
    }

    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

fn collect_dependency_closure(
    graph: &WorkflowGraph,
    node_id: &NodeId,
    visited: &mut HashSet<NodeId>,
) -> HashSet<NodeId> {
    if !visited.insert(node_id.clone()) {
        return HashSet::new();
    }

    let mut closure = HashSet::from([node_id.clone()]);
    for dependency_id in graph.get_dependencies(node_id) {
        closure.extend(collect_dependency_closure(graph, &dependency_id, visited));
    }
    closure
}

fn is_redundant_requested_target(
    graph: &WorkflowGraph,
    node_id: &NodeId,
    requested_target_set: &HashSet<NodeId>,
) -> bool {
    has_requested_dependent(graph, node_id, requested_target_set, &mut HashSet::new())
}

fn has_requested_dependent(
    graph: &WorkflowGraph,
    node_id: &NodeId,
    requested_target_set: &HashSet<NodeId>,
    visited: &mut HashSet<NodeId>,
) -> bool {
    if !visited.insert(node_id.clone()) {
        return false;
    }

    graph
        .get_dependents(node_id)
        .into_iter()
        .any(|dependent_id| {
            requested_target_set.contains(&dependent_id)
                || has_requested_dependent(graph, &dependent_id, requested_target_set, visited)
        })
}

impl DemandMultipleResults {
    fn record_success(&mut self, node_id: &NodeId, outputs: HashMap<String, serde_json::Value>) {
        self.outputs.insert(node_id.clone(), outputs);
    }

    fn into_outputs(self) -> HashMap<NodeId, HashMap<String, serde_json::Value>> {
        self.outputs
    }
}

impl DemandExecutionBudget {
    fn new(max_in_flight: usize) -> Self {
        Self {
            max_in_flight: max_in_flight.max(1),
        }
    }

    fn default_parallel() -> Self {
        Self::new(2)
    }

    fn sequential() -> Self {
        Self::new(1)
    }

    fn max_in_flight(self) -> usize {
        self.max_in_flight
    }
}

impl DemandBatchDispatchPlan {
    fn from_batch(batch: &[NodeId], budget: DemandExecutionBudget) -> Self {
        let max_in_flight = budget.max_in_flight().max(1);
        let execution_windows = batch
            .chunks(max_in_flight)
            .map(|window| {
                DemandDispatchWindowPlan::new(
                    window.to_vec(),
                    DemandDispatchWindowExecutionMode::for_window(
                        budget.max_in_flight(),
                        window.len(),
                    ),
                )
            })
            .collect();

        Self { execution_windows }
    }

    fn execution_windows(&self) -> &[DemandDispatchWindowPlan] {
        &self.execution_windows
    }
}

impl DemandDispatchWindowExecutionMode {
    fn for_window(max_in_flight: usize, window_len: usize) -> Self {
        if max_in_flight > 1 && window_len > 1 {
            Self::BoundedParallel
        } else {
            Self::Sequential
        }
    }
}

impl DemandDispatchWindowPlan {
    fn new(targets: Vec<NodeId>, execution_mode: DemandDispatchWindowExecutionMode) -> Self {
        Self {
            targets,
            execution_mode,
        }
    }

    fn targets(&self) -> &[NodeId] {
        &self.targets
    }

    fn execution_mode(&self) -> DemandDispatchWindowExecutionMode {
        self.execution_mode
    }
}

impl DemandBatchExecutionOutcome {
    fn record_completed_target(&mut self, node_id: &NodeId) {
        self.completed_targets.push(node_id.clone());
    }

    fn completed_targets(&self) -> &[NodeId] {
        &self.completed_targets
    }
}

impl DemandDispatchWindowOutcome {
    fn record_completed_target(&mut self, node_id: &NodeId) {
        self.completed_targets.push(node_id.clone());
    }

    fn completed_targets(&self) -> &[NodeId] {
        &self.completed_targets
    }
}

impl<'a> DemandMultipleCoordinator<'a> {
    fn new(
        engine: &'a mut DemandEngine,
        plan: &'a DemandMultiplePlan,
        budget: DemandExecutionBudget,
        graph: &'a WorkflowGraph,
        executor: &'a dyn TaskExecutor,
        context: &'a Context,
        event_sink: &'a dyn EventSink,
        extensions: &'a ExecutorExtensions,
        node_memories: Option<&'a HashMap<NodeId, super::NodeMemorySnapshot>>,
    ) -> Self {
        Self {
            budget,
            plan,
            window_runner: DemandWindowRunner::new(
                engine,
                graph,
                executor,
                context,
                event_sink,
                extensions,
                node_memories,
            ),
            results: DemandMultipleResults::default(),
        }
    }

    async fn run(self) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        self.run_dispatch_schedule().await
    }

    async fn run_dispatch_schedule(
        mut self,
    ) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        for batch in self.plan.execution_batches() {
            match self.execute_batch(batch).await {
                DemandBatchExecutionResult::Completed(_outcome) => {}
                DemandBatchExecutionResult::Interrupted(error) => return Err(error),
            }
        }

        self.collect_requested_outputs().await?;

        Ok(self.results.into_outputs())
    }

    async fn demand_target(&mut self, node_id: &NodeId) -> Result<()> {
        let output = self.window_runner.demand_target(node_id).await?;
        self.results.record_success(node_id, output);
        Ok(())
    }

    async fn execute_batch(&mut self, batch: &[NodeId]) -> DemandBatchExecutionResult {
        let mut outcome = DemandBatchExecutionOutcome::default();
        let dispatch_plan = DemandBatchDispatchPlan::from_batch(batch, self.budget);

        for window_plan in dispatch_plan.execution_windows() {
            let window_result = match window_plan.execution_mode() {
                DemandDispatchWindowExecutionMode::Sequential => {
                    self.execute_sequential_window(window_plan).await
                }
                DemandDispatchWindowExecutionMode::BoundedParallel => {
                    self.execute_bounded_parallel_window(window_plan).await
                }
            };

            match window_result {
                DemandDispatchWindowResult::Completed(window_outcome) => {
                    for node_id in window_outcome.completed_targets() {
                        outcome.record_completed_target(node_id);
                    }
                }
                DemandDispatchWindowResult::Interrupted(error) => {
                    return DemandBatchExecutionResult::Interrupted(error);
                }
            }
        }

        DemandBatchExecutionResult::Completed(outcome)
    }

    async fn execute_bounded_parallel_window(
        &mut self,
        window_plan: &DemandDispatchWindowPlan,
    ) -> DemandDispatchWindowResult {
        let mut outcome = DemandDispatchWindowOutcome::default();
        let base_engine = self.window_runner.clone_engine();
        let isolated_runs = match self
            .window_runner
            .demand_targets_in_isolation_concurrently(&base_engine, window_plan.targets())
            .await
        {
            Ok(isolated_runs) => isolated_runs,
            Err(error) => return DemandDispatchWindowResult::Interrupted(error),
        };
        let mut isolated_runs_by_target = isolated_runs
            .into_iter()
            .map(|isolated_run| (isolated_run.node_id.clone(), isolated_run))
            .collect::<HashMap<_, _>>();

        for node_id in window_plan.targets() {
            let DemandIsolatedTargetRun {
                node_id,
                outputs,
                engine,
            } = isolated_runs_by_target
                .remove(node_id)
                .expect("bounded window target run should exist");

            self.window_runner
                .reconcile_isolated_target_engine(&base_engine, &engine);
            self.results.record_success(&node_id, outputs);
            outcome.record_completed_target(&node_id);
        }

        DemandDispatchWindowResult::Completed(outcome)
    }

    async fn execute_sequential_window(
        &mut self,
        window_plan: &DemandDispatchWindowPlan,
    ) -> DemandDispatchWindowResult {
        let mut outcome = DemandDispatchWindowOutcome::default();

        for node_id in window_plan.targets() {
            if let Err(error) = self.demand_target(node_id).await {
                return DemandDispatchWindowResult::Interrupted(error);
            }
            outcome.record_completed_target(node_id);
        }

        DemandDispatchWindowResult::Completed(outcome)
    }

    async fn collect_requested_outputs(&mut self) -> Result<()> {
        for node_id in self.plan.requested_targets() {
            let outputs = self.window_runner.load_requested_outputs(node_id).await?;
            self.results.record_success(node_id, outputs);
        }

        Ok(())
    }
}

impl<'a> DemandWindowRunner<'a> {
    fn new(
        engine: &'a mut DemandEngine,
        graph: &'a WorkflowGraph,
        executor: &'a dyn TaskExecutor,
        context: &'a Context,
        event_sink: &'a dyn EventSink,
        extensions: &'a ExecutorExtensions,
        node_memories: Option<&'a HashMap<NodeId, super::NodeMemorySnapshot>>,
    ) -> Self {
        Self {
            engine,
            graph,
            executor,
            context,
            event_sink,
            extensions,
            node_memories,
        }
    }

    async fn demand_target(
        &mut self,
        node_id: &NodeId,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let graph = self.graph;
        let executor = self.executor;
        let context = self.context;
        let event_sink = self.event_sink;
        let extensions = self.extensions;
        let engine = &mut *self.engine;

        Self::demand_target_with_engine(
            engine,
            node_id,
            graph,
            executor,
            context,
            event_sink,
            extensions,
            self.node_memories,
        )
        .await
    }

    fn clone_engine(&self) -> DemandEngine {
        self.engine.clone()
    }

    fn reconcile_isolated_target_engine(
        &mut self,
        base_engine: &DemandEngine,
        isolated_engine: &DemandEngine,
    ) {
        self.engine
            .reconcile_isolated_run(base_engine, isolated_engine);
    }

    async fn demand_target_in_isolation(
        &self,
        base_engine: &DemandEngine,
        node_id: &NodeId,
    ) -> Result<DemandIsolatedTargetRun> {
        let mut isolated_engine = base_engine.clone();
        let outputs = Self::demand_target_with_engine(
            &mut isolated_engine,
            node_id,
            self.graph,
            self.executor,
            self.context,
            self.event_sink,
            self.extensions,
            self.node_memories,
        )
        .await?;

        Ok(DemandIsolatedTargetRun {
            node_id: node_id.clone(),
            outputs,
            engine: isolated_engine,
        })
    }

    async fn demand_targets_in_isolation_concurrently(
        &self,
        base_engine: &DemandEngine,
        node_ids: &[NodeId],
    ) -> Result<Vec<DemandIsolatedTargetRun>> {
        let futures = node_ids
            .iter()
            .cloned()
            .map(|node_id| self.demand_target_in_isolation_future(base_engine, node_id))
            .collect::<Vec<_>>();

        await_isolated_target_runs(futures).await
    }

    fn demand_target_in_isolation_future<'b>(
        &'b self,
        base_engine: &'b DemandEngine,
        node_id: NodeId,
    ) -> DemandIsolatedTargetRunFuture<'b> {
        Box::pin(async move { self.demand_target_in_isolation(base_engine, &node_id).await })
    }

    async fn demand_target_with_engine(
        engine: &mut DemandEngine,
        node_id: &NodeId,
        graph: &WorkflowGraph,
        executor: &dyn TaskExecutor,
        context: &Context,
        event_sink: &dyn EventSink,
        extensions: &ExecutorExtensions,
        node_memories: Option<&HashMap<NodeId, super::NodeMemorySnapshot>>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        engine
            .demand_with_node_memory(
                node_id,
                graph,
                executor,
                context,
                event_sink,
                extensions,
                node_memories,
            )
            .await
    }

    async fn load_requested_outputs(
        &mut self,
        node_id: &NodeId,
    ) -> Result<HashMap<String, serde_json::Value>> {
        if let Some(outputs) = self.engine.get_cached(node_id, self.graph) {
            serde_json::from_value(outputs.clone()).map_err(Into::into)
        } else {
            self.demand_target(node_id).await
        }
    }
}

async fn await_isolated_target_runs<'a>(
    pending_runs: Vec<DemandIsolatedTargetRunFuture<'a>>,
) -> Result<Vec<DemandIsolatedTargetRun>> {
    let mut pending_runs = pending_runs.into_iter().map(Some).collect::<Vec<_>>();
    let mut completed_runs = Vec::new();

    poll_fn(|cx| {
        let mut pending_count = 0usize;

        for run_future_slot in pending_runs.iter_mut() {
            let Some(run_future) = run_future_slot.as_mut() else {
                continue;
            };

            match run_future.as_mut().poll(cx) {
                Poll::Ready(Ok(run)) => {
                    completed_runs.push(run);
                    *run_future_slot = None;
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => {
                    pending_count += 1;
                }
            }
        }

        if pending_count == 0 {
            Poll::Ready(Ok(std::mem::take(&mut completed_runs)))
        } else {
            Poll::Pending
        }
    })
    .await
}

pub(super) async fn demand_multiple_with_executor(
    workflow_executor: &WorkflowExecutor,
    node_ids: &[NodeId],
    executor: &dyn TaskExecutor,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    demand_multiple_with_budget(
        workflow_executor,
        node_ids,
        executor,
        DemandExecutionBudget::default_parallel(),
    )
    .await
}

async fn demand_multiple_with_budget(
    workflow_executor: &WorkflowExecutor,
    node_ids: &[NodeId],
    executor: &dyn TaskExecutor,
    budget: DemandExecutionBudget,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let node_memories =
        super::workflow_session::bound_workflow_session_node_memory_view(workflow_executor).await;
    let graph = workflow_executor.graph.read().await;
    let plan = DemandMultiplePlan::from_requested_targets(node_ids, &graph);
    workflow_executor
        .emit_incremental_execution_started(graph.id.clone(), plan.requested_targets().to_vec());
    let mut demand_engine = workflow_executor.demand_engine.write().await;

    let outputs = execute_plan_with_budget(
        &mut demand_engine,
        &plan,
        budget,
        &graph,
        executor,
        &workflow_executor.context,
        workflow_executor.event_sink.as_ref(),
        &workflow_executor.extensions,
        node_memories.as_ref(),
    )
    .await?;
    drop(demand_engine);
    drop(graph);

    super::workflow_session::sync_bound_session_node_memory_from_cache(workflow_executor).await;
    Ok(outputs)
}

pub(super) async fn demand_multiple_with_default_budget(
    engine: &mut DemandEngine,
    node_ids: &[NodeId],
    graph: &WorkflowGraph,
    executor: &dyn TaskExecutor,
    context: &Context,
    event_sink: &dyn EventSink,
    extensions: &ExecutorExtensions,
    node_memories: Option<&HashMap<NodeId, super::NodeMemorySnapshot>>,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    demand_multiple_with_explicit_budget(
        engine,
        node_ids,
        DemandExecutionBudget::default_parallel(),
        graph,
        executor,
        context,
        event_sink,
        extensions,
        node_memories,
    )
    .await
}

async fn demand_multiple_with_explicit_budget(
    engine: &mut DemandEngine,
    node_ids: &[NodeId],
    budget: DemandExecutionBudget,
    graph: &WorkflowGraph,
    executor: &dyn TaskExecutor,
    context: &Context,
    event_sink: &dyn EventSink,
    extensions: &ExecutorExtensions,
    node_memories: Option<&HashMap<NodeId, super::NodeMemorySnapshot>>,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let plan = DemandMultiplePlan::from_requested_targets(node_ids, graph);

    execute_plan_with_budget(
        engine,
        &plan,
        budget,
        graph,
        executor,
        context,
        event_sink,
        extensions,
        node_memories,
    )
    .await
}

async fn execute_plan_with_budget(
    engine: &mut DemandEngine,
    plan: &DemandMultiplePlan,
    budget: DemandExecutionBudget,
    graph: &WorkflowGraph,
    executor: &dyn TaskExecutor,
    context: &Context,
    event_sink: &dyn EventSink,
    extensions: &ExecutorExtensions,
    node_memories: Option<&HashMap<NodeId, super::NodeMemorySnapshot>>,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    DemandMultipleCoordinator::new(
        engine,
        plan,
        budget,
        graph,
        executor,
        context,
        event_sink,
        extensions,
        node_memories,
    )
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::future::Future;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use async_trait::async_trait;
    use graph_flow::Context;

    use crate::engine::WorkflowExecutor;
    use crate::error::NodeEngineError;
    use crate::events::NullEventSink;
    use crate::extensions::ExecutorExtensions;
    use crate::types::{GraphEdge, GraphNode, WorkflowGraph};

    use super::{
        DemandBatchDispatchPlan, DemandBatchExecutionOutcome, DemandBatchExecutionResult,
        DemandDispatchWindowExecutionMode, DemandDispatchWindowOutcome, DemandDispatchWindowPlan,
        DemandDispatchWindowResult, DemandEngine, DemandExecutionBudget, DemandMultiplePlan,
        DemandMultipleResults, DemandWindowRunner, TaskExecutor,
        demand_multiple_with_default_budget, demand_multiple_with_explicit_budget,
    };

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
        let outputs = demand_multiple_with_explicit_budget(
            &mut engine,
            &["left".to_string(), "right".to_string()],
            DemandExecutionBudget::new(budget),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
            None,
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
        let runner = DemandWindowRunner::new(
            &mut engine,
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
            None,
        );
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

        let outputs = demand_multiple_with_explicit_budget(
            &mut engine,
            &["left".to_string(), "right".to_string()],
            DemandExecutionBudget::new(2),
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
            None,
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

        let outputs = demand_multiple_with_default_budget(
            &mut engine,
            &["left".to_string(), "right".to_string()],
            &graph,
            &executor,
            &context,
            &event_sink,
            &extensions,
            None,
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
}
