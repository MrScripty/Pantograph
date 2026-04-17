use std::collections::HashMap;

use graph_flow::Context;

use super::{DemandEngine, TaskExecutor};
use crate::error::Result;
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::types::{NodeId, WorkflowGraph};

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
