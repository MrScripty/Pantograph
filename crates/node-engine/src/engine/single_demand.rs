use std::collections::HashMap;

use super::{TaskExecutor, WorkflowExecutor};
use crate::error::Result;
use crate::types::NodeId;

pub(super) async fn demand_with_executor(
    workflow_executor: &WorkflowExecutor,
    node_id: &NodeId,
    executor: &dyn TaskExecutor,
) -> Result<HashMap<String, serde_json::Value>> {
    let node_memories =
        super::workflow_session::bound_workflow_session_node_memory_view(workflow_executor).await;
    let graph = workflow_executor.graph.read().await;
    let mut demand_engine = workflow_executor.demand_engine.write().await;
    let runtime = super::DemandRuntimeContext::new(
        &graph,
        executor,
        &workflow_executor.context,
        workflow_executor.event_sink.as_ref(),
        &workflow_executor.extensions,
        node_memories.as_ref(),
    );

    let outputs = demand_engine.demand_with_context(runtime, node_id).await?;
    drop(demand_engine);
    drop(graph);

    super::workflow_session::sync_bound_session_node_memory_from_cache(workflow_executor).await;
    Ok(outputs)
}
