use std::collections::HashMap;

use graph_flow::Context;

use super::{DemandEngine, TaskExecutor, WorkflowExecutor};
use crate::error::Result;
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::types::{NodeId, WorkflowGraph};

pub(super) async fn demand_multiple_with_executor(
    workflow_executor: &WorkflowExecutor,
    node_ids: &[NodeId],
    executor: &dyn TaskExecutor,
) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
    let graph = workflow_executor.graph.read().await;
    workflow_executor.emit_incremental_execution_started(graph.id.clone(), node_ids.to_vec());
    let mut demand_engine = workflow_executor.demand_engine.write().await;

    demand_multiple_sequential(
        &mut demand_engine,
        node_ids,
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
    let mut results = HashMap::new();

    for node_id in node_ids {
        let output = engine
            .demand(node_id, graph, executor, context, event_sink, extensions)
            .await?;
        results.insert(node_id.clone(), output);
    }

    Ok(results)
}
