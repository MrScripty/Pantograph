use std::collections::HashMap;

use graph_flow::Context;

use super::{DemandEngine, TaskExecutor, WorkflowExecutor};
use crate::error::Result;
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::types::{NodeId, WorkflowGraph};

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemandMultiplePlan {
    requested_targets: Vec<NodeId>,
    execution_targets: Vec<NodeId>,
}

impl DemandMultiplePlan {
    fn from_requested_targets(node_ids: &[NodeId]) -> Self {
        Self {
            requested_targets: node_ids.to_vec(),
            execution_targets: node_ids.to_vec(),
        }
    }

    fn requested_targets(&self) -> &[NodeId] {
        &self.requested_targets
    }

    fn execution_targets(&self) -> &[NodeId] {
        &self.execution_targets
    }
}

pub(super) async fn demand_multiple_with_executor(
    workflow_executor: &WorkflowExecutor,
    node_ids: &[NodeId],
    executor: &dyn TaskExecutor,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let plan = DemandMultiplePlan::from_requested_targets(node_ids);
    let graph = workflow_executor.graph.read().await;
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
    let plan = DemandMultiplePlan::from_requested_targets(node_ids);

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
    let mut results = HashMap::new();

    for node_id in plan.execution_targets() {
        let output = engine
            .demand(node_id, graph, executor, context, event_sink, extensions)
            .await?;
        results.insert(node_id.clone(), output);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::DemandMultiplePlan;

    #[test]
    fn plan_preserves_requested_target_order_for_event_payloads() {
        let plan = DemandMultiplePlan::from_requested_targets(&[
            "node_b".to_string(),
            "node_a".to_string(),
            "node_c".to_string(),
        ]);

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
        let plan = DemandMultiplePlan::from_requested_targets(&[
            "node_b".to_string(),
            "node_a".to_string(),
            "node_c".to_string(),
        ]);

        assert_eq!(plan.execution_targets(), plan.requested_targets());
    }

    #[test]
    fn plan_handles_empty_requests() {
        let plan = DemandMultiplePlan::from_requested_targets(&[]);

        assert!(plan.requested_targets().is_empty());
        assert!(plan.execution_targets().is_empty());
    }
}
