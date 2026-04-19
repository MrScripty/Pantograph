use std::collections::HashMap;

use super::{TaskExecutor, WorkflowExecutor};
use crate::error::Result;
use crate::types::NodeId;

pub(super) async fn demand_with_executor(
    workflow_executor: &WorkflowExecutor,
    node_id: &NodeId,
    executor: &dyn TaskExecutor,
) -> Result<HashMap<String, serde_json::Value>> {
    let graph = workflow_executor.graph.read().await;
    let mut demand_engine = workflow_executor.demand_engine.write().await;

    let outputs = demand_engine
        .demand(
            node_id,
            &graph,
            executor,
            &workflow_executor.context,
            workflow_executor.event_sink.as_ref(),
            &workflow_executor.extensions,
        )
        .await?;
    drop(demand_engine);
    drop(graph);

    super::workflow_session::sync_bound_session_node_memory_from_cache(workflow_executor).await;
    Ok(outputs)
}
