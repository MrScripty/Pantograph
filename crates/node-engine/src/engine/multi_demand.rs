use std::collections::{HashMap, HashSet};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;

use super::{DemandEngine, TaskExecutor, WorkflowExecutor};
use crate::error::{NodeEngineError, Result};
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
    runtime: super::DemandRuntimeContext<'a>,
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

    #[cfg(test)]
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

    #[cfg(test)]
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
        runtime: super::DemandRuntimeContext<'a>,
    ) -> Self {
        Self {
            budget,
            plan,
            window_runner: DemandWindowRunner::new(engine, runtime),
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
    fn new(engine: &'a mut DemandEngine, runtime: super::DemandRuntimeContext<'a>) -> Self {
        Self { engine, runtime }
    }

    async fn demand_target(
        &mut self,
        node_id: &NodeId,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let runtime = self.runtime;
        let engine = &mut *self.engine;

        Self::demand_target_with_engine(engine, node_id, runtime).await
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
        let outputs =
            Self::demand_target_with_engine(&mut isolated_engine, node_id, self.runtime).await?;

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
        runtime: super::DemandRuntimeContext<'_>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        engine.demand_with_context(runtime, node_id).await
    }

    async fn load_requested_outputs(
        &mut self,
        node_id: &NodeId,
    ) -> Result<HashMap<String, serde_json::Value>> {
        if let Some(outputs) = self.engine.get_cached(node_id, self.runtime.graph) {
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
    let runtime = super::DemandRuntimeContext::new(
        &graph,
        executor,
        &workflow_executor.context,
        workflow_executor.event_sink.as_ref(),
        &workflow_executor.extensions,
        node_memories.as_ref(),
    );

    let outputs = execute_plan_with_budget(&mut demand_engine, &plan, budget, runtime).await?;
    drop(demand_engine);
    drop(graph);

    super::workflow_session::sync_bound_session_node_memory_from_cache(workflow_executor).await;
    Ok(outputs)
}

pub(super) async fn demand_multiple_with_default_budget(
    engine: &mut DemandEngine,
    node_ids: &[NodeId],
    runtime: super::DemandRuntimeContext<'_>,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    demand_multiple_with_explicit_budget(
        engine,
        node_ids,
        DemandExecutionBudget::default_parallel(),
        runtime,
    )
    .await
}

async fn demand_multiple_with_explicit_budget(
    engine: &mut DemandEngine,
    node_ids: &[NodeId],
    budget: DemandExecutionBudget,
    runtime: super::DemandRuntimeContext<'_>,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let plan = DemandMultiplePlan::from_requested_targets(node_ids, runtime.graph);

    execute_plan_with_budget(engine, &plan, budget, runtime).await
}

async fn execute_plan_with_budget(
    engine: &mut DemandEngine,
    plan: &DemandMultiplePlan,
    budget: DemandExecutionBudget,
    runtime: super::DemandRuntimeContext<'_>,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    DemandMultipleCoordinator::new(engine, plan, budget, runtime)
        .run()
        .await
}

#[cfg(test)]
#[path = "multi_demand_tests.rs"]
mod tests;
