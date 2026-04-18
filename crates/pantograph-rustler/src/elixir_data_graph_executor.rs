use std::collections::HashMap;
use std::sync::Arc;

use node_engine::{EventSink, OrchestrationStore, TaskExecutor, WorkflowExecutor, WorkflowGraph};

use crate::BeamEventSink;

/// DataGraphExecutor that executes data graphs using the Elixir callback bridge.
pub(crate) struct ElixirDataGraphExecutor {
    store: Arc<tokio::sync::RwLock<OrchestrationStore>>,
    task_executor: Arc<dyn TaskExecutor>,
    event_sink_pid: rustler::LocalPid,
}

impl ElixirDataGraphExecutor {
    pub(crate) fn new(
        store: Arc<tokio::sync::RwLock<OrchestrationStore>>,
        task_executor: Arc<dyn TaskExecutor>,
        event_sink_pid: rustler::LocalPid,
    ) -> Self {
        Self {
            store,
            task_executor,
            event_sink_pid,
        }
    }
}

#[async_trait::async_trait]
impl node_engine::DataGraphExecutor for ElixirDataGraphExecutor {
    async fn execute_data_graph(
        &self,
        graph_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _event_sink: &dyn EventSink,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let graph = {
            let store = self.store.read().await;
            store.get_data_graph(graph_id).cloned().ok_or_else(|| {
                node_engine::NodeEngineError::ExecutionFailed(format!(
                    "Data graph '{}' not found in store",
                    graph_id
                ))
            })?
        };

        let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(self.event_sink_pid));
        let exec_id = format!("data-graph-{}", graph_id);
        let executor = WorkflowExecutor::new(&exec_id, graph.clone(), event_sink);

        for (port, value) in &inputs {
            for node in &graph.nodes {
                let key = node_engine::ContextKeys::input(&node.id, port);
                executor.set_context_value(&key, value.clone()).await;
            }
        }

        let terminal_nodes: Vec<String> = graph
            .nodes
            .iter()
            .filter(|node| !graph.edges.iter().any(|edge| edge.source == node.id))
            .map(|node| node.id.clone())
            .collect();

        let demand_nodes = if terminal_nodes.is_empty() {
            graph.nodes.iter().map(|node| node.id.clone()).collect()
        } else {
            terminal_nodes
        };

        let results = executor
            .demand_multiple(&demand_nodes, self.task_executor.as_ref())
            .await?;

        let mut outputs = HashMap::new();
        for (node_id, node_outputs) in results {
            for (port, value) in node_outputs {
                outputs.insert(format!("{}.{}", node_id, port), value);
            }
        }

        Ok(outputs)
    }

    fn get_data_graph(&self, graph_id: &str) -> Option<WorkflowGraph> {
        let store = self.store.blocking_read();
        store.get_data_graph(graph_id).cloned()
    }
}
