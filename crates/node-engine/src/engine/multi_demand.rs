use std::collections::{HashMap, HashSet};

use graph_flow::Context;

use super::{DemandEngine, TaskExecutor, WorkflowExecutor};
use crate::error::Result;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DemandExecutionBudget {
    max_in_flight: usize,
}

struct DemandMultipleCoordinator<'a> {
    budget: DemandExecutionBudget,
    engine: &'a mut DemandEngine,
    plan: &'a DemandMultiplePlan,
    graph: &'a WorkflowGraph,
    executor: &'a dyn TaskExecutor,
    context: &'a Context,
    event_sink: &'a dyn EventSink,
    extensions: &'a ExecutorExtensions,
    results: DemandMultipleResults,
}

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

    graph.get_dependents(node_id).into_iter().any(|dependent_id| {
        requested_target_set.contains(&dependent_id)
            || has_requested_dependent(graph, &dependent_id, requested_target_set, visited)
    })
}

impl DemandMultipleResults {
    fn record_success(
        &mut self,
        node_id: &NodeId,
        outputs: HashMap<String, serde_json::Value>,
    ) {
        self.outputs.insert(node_id.clone(), outputs);
    }

    fn into_outputs(self) -> HashMap<NodeId, HashMap<String, serde_json::Value>> {
        self.outputs
    }
}

impl DemandExecutionBudget {
    fn sequential() -> Self {
        Self { max_in_flight: 1 }
    }

    fn max_in_flight(self) -> usize {
        self.max_in_flight
    }
}

impl<'a> DemandMultipleCoordinator<'a> {
    fn new(
        engine: &'a mut DemandEngine,
        plan: &'a DemandMultiplePlan,
        graph: &'a WorkflowGraph,
        executor: &'a dyn TaskExecutor,
        context: &'a Context,
        event_sink: &'a dyn EventSink,
        extensions: &'a ExecutorExtensions,
    ) -> Self {
        Self {
            budget: DemandExecutionBudget::sequential(),
            engine,
            plan,
            graph,
            executor,
            context,
            event_sink,
            extensions,
            results: DemandMultipleResults::default(),
        }
    }

    async fn run(self) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        match self.budget.max_in_flight() {
            1 => self.run_sequential().await,
            _ => unreachable!("bounded parallel execution is not implemented yet"),
        }
    }

    async fn run_sequential(
        mut self,
    ) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        for batch in self.plan.execution_batches() {
            for node_id in batch {
                self.demand_target(node_id).await?;
            }
        }

        self.collect_requested_outputs().await?;

        Ok(self.results.into_outputs())
    }

    async fn demand_target(&mut self, node_id: &NodeId) -> Result<()> {
        let output = self
            .engine
            .demand(
                node_id,
                self.graph,
                self.executor,
                self.context,
                self.event_sink,
                self.extensions,
            )
            .await?;
        self.results.record_success(node_id, output);
        Ok(())
    }

    async fn collect_requested_outputs(&mut self) -> Result<()> {
        for node_id in self.plan.requested_targets() {
            let outputs = if let Some(outputs) = self.engine.get_cached(node_id, self.graph) {
                serde_json::from_value(outputs.clone())?
            } else {
                self.engine.demand(
                    node_id,
                    self.graph,
                    self.executor,
                    self.context,
                    self.event_sink,
                    self.extensions,
                )
                .await?
            };
            self.results.record_success(node_id, outputs);
        }

        Ok(())
    }
}

pub(super) async fn demand_multiple_with_executor(
    workflow_executor: &WorkflowExecutor,
    node_ids: &[NodeId],
    executor: &dyn TaskExecutor,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let graph = workflow_executor.graph.read().await;
    let plan = DemandMultiplePlan::from_requested_targets(node_ids, &graph);
    workflow_executor
        .emit_incremental_execution_started(graph.id.clone(), plan.requested_targets().to_vec());
    let mut demand_engine = workflow_executor.demand_engine.write().await;

    execute_sequential_plan(
        &mut demand_engine,
        &plan,
        &graph,
        executor,
        &workflow_executor.context,
        workflow_executor.event_sink.as_ref(),
        &workflow_executor.extensions,
    )
    .await
}

pub(super) async fn demand_multiple_sequential(
    engine: &mut DemandEngine,
    node_ids: &[NodeId],
    graph: &WorkflowGraph,
    executor: &dyn TaskExecutor,
    context: &Context,
    event_sink: &dyn EventSink,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let plan = DemandMultiplePlan::from_requested_targets(node_ids, graph);

    execute_sequential_plan(
        engine,
        &plan,
        graph,
        executor,
        context,
        event_sink,
        extensions,
    )
    .await
}

async fn execute_sequential_plan(
    engine: &mut DemandEngine,
    plan: &DemandMultiplePlan,
    graph: &WorkflowGraph,
    executor: &dyn TaskExecutor,
    context: &Context,
    event_sink: &dyn EventSink,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    DemandMultipleCoordinator::new(
        engine,
        plan,
        graph,
        executor,
        context,
        event_sink,
        extensions,
    )
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::types::{GraphEdge, GraphNode, WorkflowGraph};

    use super::{DemandExecutionBudget, DemandMultiplePlan, DemandMultipleResults};

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

    #[test]
    fn plan_preserves_requested_target_order_for_event_payloads() {
        let graph = make_linear_graph();
        let plan = DemandMultiplePlan::from_requested_targets(&[
            "node_b".to_string(),
            "node_a".to_string(),
            "node_c".to_string(),
        ], &graph);

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
    fn plan_prunes_requested_targets_covered_by_requested_dependents() {
        let graph = make_linear_graph();
        let plan =
            DemandMultiplePlan::from_requested_targets(&["b".to_string(), "c".to_string()], &graph);

        assert_eq!(plan.requested_targets(), &["b".to_string(), "c".to_string()]);
        assert_eq!(plan.execution_batches(), &[vec!["c".to_string()]]);
    }

    #[test]
    fn plan_dedupes_execution_targets_while_preserving_requested_duplicates() {
        let graph = make_linear_graph();
        let plan =
            DemandMultiplePlan::from_requested_targets(&["c".to_string(), "c".to_string()], &graph);

        assert_eq!(plan.requested_targets(), &["c".to_string(), "c".to_string()]);
        assert_eq!(plan.execution_batches(), &[vec!["c".to_string()]]);
    }

    #[test]
    fn plan_places_current_root_targets_into_one_batch() {
        let graph = make_linear_graph();
        let plan =
            DemandMultiplePlan::from_requested_targets(&["a".to_string(), "c".to_string()], &graph);

        assert_eq!(
            plan.execution_batches(),
            &[vec!["c".to_string()]]
        );
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
    fn results_keep_distinct_targets() {
        let mut results = DemandMultipleResults::default();
        let output_a = HashMap::from([(
            "value".to_string(),
            serde_json::json!("first"),
        )]);
        let output_b = HashMap::from([(
            "value".to_string(),
            serde_json::json!("second"),
        )]);

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
        let first_output = HashMap::from([(
            "value".to_string(),
            serde_json::json!("first"),
        )]);
        let second_output = HashMap::from([(
            "value".to_string(),
            serde_json::json!("second"),
        )]);

        results.record_success(&"node_a".to_string(), first_output);
        results.record_success(&"node_a".to_string(), second_output);

        let outputs = results.into_outputs();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs["node_a"]["value"], serde_json::json!("second"));
    }
}
