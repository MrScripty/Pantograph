use std::collections::HashMap;
use std::sync::Arc;

use node_engine::{CoreTaskExecutor, EventSink, WorkflowExecutor, WorkflowGraph};
use uuid::Uuid;

use crate::{
    apply_runtime_extensions_for_execution, task_executor, EmbeddedRuntime, EmbeddedWorkflowHost,
    InferenceLifecycleWorkflowLedgerSink, NodeExecutionWorkflowLedgerSink,
    RuntimeExtensionsSnapshot,
};

impl EmbeddedRuntime {
    pub async fn execute_data_graph(
        &self,
        graph_id: &str,
        graph: &WorkflowGraph,
        inputs: &HashMap<String, serde_json::Value>,
        event_sink: Arc<dyn EventSink>,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let runtime_ext = RuntimeExtensionsSnapshot::from_shared_with_workflow_service(
            &self.extensions,
            self.workflow_service.clone(),
        )
        .await;
        let execution_id = format!("data-graph-{}-{}", graph_id, Uuid::new_v4());
        let workflow_event_sink = NodeExecutionWorkflowLedgerSink::try_new(
            self.workflow_service.clone(),
            graph_id.to_string(),
            execution_id.clone(),
            execution_id.clone(),
            graph,
            Some(event_sink.clone()),
        )
        .ok()
        .map(|sink| Arc::new(sink) as Arc<dyn EventSink>)
        .unwrap_or(event_sink);
        let core = Arc::new(
            CoreTaskExecutor::new()
                .with_project_root(self.config.project_root.clone())
                .with_gateway(self.gateway.clone())
                .with_event_sink(workflow_event_sink.clone())
                .with_execution_id(execution_id.clone()),
        );
        let host = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
            self.rag_backend.clone(),
            self.python_runtime.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );
        let python_runtime_execution_recorder =
            Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());
        let mut graph = graph.clone();
        let terminal_nodes = EmbeddedWorkflowHost::terminal_data_graph_node_ids(&graph);
        EmbeddedWorkflowHost::apply_data_graph_inputs(&mut graph, inputs);

        let inference_lifecycle_graph = graph.clone();
        let mut executor =
            WorkflowExecutor::new(execution_id.clone(), graph, workflow_event_sink.clone());
        let inference_lifecycle_sink = InferenceLifecycleWorkflowLedgerSink::try_new(
            self.workflow_service.clone(),
            graph_id.to_string(),
            execution_id.clone(),
            execution_id.clone(),
            &inference_lifecycle_graph,
        )
        .ok()
        .map(|sink| Arc::new(sink) as Arc<dyn inference::InferenceRequestLifecycleEventSink>);
        apply_runtime_extensions_for_execution(
            &mut executor,
            &runtime_ext,
            Some(workflow_event_sink),
            Some(execution_id.clone()),
            Some(python_runtime_execution_recorder.clone()),
            inference_lifecycle_sink,
        );

        let mut node_outputs = HashMap::new();
        let mut terminal_errors = HashMap::new();
        let mut terminal_outcome_error = None;
        for terminal_id in &terminal_nodes {
            match executor.demand(terminal_id, &task_executor).await {
                Ok(outputs) => {
                    node_outputs.insert(terminal_id.clone(), outputs);
                }
                Err(error) => {
                    if matches!(
                        error,
                        node_engine::NodeEngineError::WaitingForInput { .. }
                            | node_engine::NodeEngineError::Cancelled
                    ) {
                        terminal_outcome_error = Some(error);
                        break;
                    }
                    log::error!(
                        "Error executing terminal node '{}' in data graph '{}': {}",
                        terminal_id,
                        graph_id,
                        error
                    );
                    terminal_errors.insert(
                        format!("{}.error", terminal_id),
                        serde_json::Value::String(error.to_string()),
                    );
                }
            }
        }

        let python_runtime_execution_metadata = python_runtime_execution_recorder.snapshots();
        self.host()
            .observe_python_runtime_execution_metadata(&python_runtime_execution_metadata)
            .map_err(|error| node_engine::NodeEngineError::failed(error.to_string()))?;

        if let Some(error) = terminal_outcome_error {
            return Err(error);
        }

        let mut outputs = EmbeddedWorkflowHost::collect_data_graph_outputs(
            graph_id,
            &terminal_nodes,
            &node_outputs,
        );
        outputs.extend(terminal_errors);
        Ok(outputs)
    }
}
