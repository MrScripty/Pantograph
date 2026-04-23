use std::sync::Arc;

use node_engine::{CoreTaskExecutor, EventSink, WorkflowExecutor};
use pantograph_workflow_service::{
    convert_graph_to_node_engine, WorkflowGraph, WorkflowTraceRuntimeMetrics,
};

use crate::{
    apply_runtime_extensions_for_execution, embedding_workflow, runtime_registry, task_executor,
    workflow_runtime, EmbeddedRuntime, HostRuntimeModeSnapshot, RuntimeExtensionsSnapshot,
};

#[derive(Debug, Clone)]
pub struct EditSessionGraphExecutionOutcome {
    pub runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub runtime_model_target: Option<String>,
    pub waiting_for_input: bool,
    pub error: Option<String>,
}

impl EmbeddedRuntime {
    pub async fn execute_edit_session_graph(
        &self,
        session_id: &str,
        session_graph: &WorkflowGraph,
        embedding_request: inference::EmbeddingStartRequest,
        event_sink: Arc<dyn EventSink>,
    ) -> Result<EditSessionGraphExecutionOutcome, String> {
        let runtime_ext = RuntimeExtensionsSnapshot::from_shared(&self.extensions).await;
        let restore_config = embedding_workflow::prepare_embedding_runtime_for_workflow(
            self.gateway.as_ref(),
            runtime_ext.pumas_api.as_deref(),
            embedding_request,
            embedding_workflow::resolve_embedding_model_id_from_workflow_graph(session_graph)?,
            embedding_workflow::workflow_graph_has_embedding_node(session_graph),
            embedding_workflow::workflow_graph_has_llamacpp_inference_node(session_graph),
        )
        .await?;
        self.reconcile_runtime_registry_from_gateway().await;

        let core = Arc::new(
            CoreTaskExecutor::new()
                .with_project_root(self.config.project_root.clone())
                .with_gateway(self.gateway.clone())
                .with_event_sink(event_sink.clone())
                .with_execution_id(session_id.to_string()),
        );
        let host = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
            self.rag_backend.clone(),
            self.python_runtime.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );

        let terminal_nodes: Vec<String> = session_graph
            .nodes
            .iter()
            .filter(|node| {
                !session_graph
                    .edges
                    .iter()
                    .any(|edge| edge.source == node.id)
            })
            .map(|node| node.id.clone())
            .collect();

        let python_runtime_execution_recorder =
            Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());
        let mut executor = WorkflowExecutor::new(
            session_id,
            convert_graph_to_node_engine(session_graph),
            event_sink.clone(),
        );
        apply_runtime_extensions_for_execution(
            &mut executor,
            &runtime_ext,
            Some(event_sink.clone()),
            Some(session_id.to_string()),
            Some(python_runtime_execution_recorder.clone()),
        );
        executor.set_event_sink(event_sink.clone());
        workflow_runtime::sync_embedding_emit_metadata_flags(&mut executor)
            .await
            .map_err(|error| error.to_string())?;

        self.workflow_service
            .workflow_graph_mark_edit_session_running(session_id)
            .await
            .map_err(|error| error.to_envelope_json())?;

        let _ = event_sink.send(
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: session_id.to_string(),
                execution_id: session_id.to_string(),
                occurred_at_ms: None,
            }
            .now(),
        );

        let mut workflow_result: Result<(), node_engine::NodeEngineError> = Ok(());
        let mut waiting_for_input = false;
        for node_id in &terminal_nodes {
            match executor.demand(node_id, &task_executor).await {
                Ok(_outputs) => {
                    log::debug!("Demanded outputs from node: {}", node_id);
                }
                Err(node_engine::NodeEngineError::WaitingForInput { task_id, prompt }) => {
                    log::info!(
                        "Workflow session '{}' is waiting for input at node '{}' (prompt: {:?})",
                        session_id,
                        task_id,
                        prompt
                    );
                    waiting_for_input = true;
                    break;
                }
                Err(error) => {
                    log::error!("Error demanding from node {}: {}", node_id, error);
                    workflow_result = Err(error);
                    break;
                }
            }
        }

        self.workflow_service
            .workflow_graph_mark_edit_session_finished(session_id)
            .await
            .map_err(|error| error.to_envelope_json())?;

        if waiting_for_input {
            log::debug!(
                "Workflow session '{}' paused in waiting-for-input state",
                session_id
            );
        } else if workflow_result.is_ok() {
            let _ = event_sink.send(
                node_engine::WorkflowEvent::WorkflowCompleted {
                    workflow_id: session_id.to_string(),
                    execution_id: session_id.to_string(),
                    occurred_at_ms: None,
                }
                .now(),
            );
        } else if let Err(error) = &workflow_result {
            let terminal_event = match error {
                node_engine::NodeEngineError::Cancelled => {
                    node_engine::WorkflowEvent::WorkflowCancelled {
                        workflow_id: session_id.to_string(),
                        execution_id: session_id.to_string(),
                        error: error.to_string(),
                        occurred_at_ms: None,
                    }
                }
                _ => node_engine::WorkflowEvent::WorkflowFailed {
                    workflow_id: session_id.to_string(),
                    execution_id: session_id.to_string(),
                    error: error.to_string(),
                    occurred_at_ms: None,
                },
            };
            let _ = event_sink.send(terminal_event.now());
        }

        let execution_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&self.gateway.mode_info().await);
        let recorded_python_runtimes = python_runtime_execution_recorder.snapshots();
        let recorded_python_runtime = recorded_python_runtimes.last();
        let runtime_snapshot = if let Some(metadata) = recorded_python_runtime.as_ref() {
            metadata.snapshot.clone()
        } else {
            self.gateway.runtime_lifecycle_snapshot().await
        };
        let runtime_model_target = recorded_python_runtime
            .and_then(|metadata| metadata.model_target.clone())
            .or_else(|| {
                workflow_runtime::resolve_runtime_model_target(
                    &execution_mode_info,
                    &runtime_snapshot,
                )
            });
        let observed_runtime_ids = recorded_python_runtimes
            .iter()
            .filter_map(|metadata| metadata.snapshot.runtime_id.clone())
            .collect::<Vec<_>>();
        let trace_runtime_metrics =
            workflow_runtime::trace_runtime_metrics_with_observed_runtime_ids(
                &runtime_snapshot,
                runtime_model_target.as_deref(),
                &observed_runtime_ids,
            );
        let restore_result = if let Some(runtime_registry) = self.runtime_registry.as_ref() {
            runtime_registry::restore_runtime_and_reconcile_runtime_registry(
                self.gateway.as_ref(),
                runtime_registry.as_ref(),
                restore_config,
            )
            .await
        } else {
            self.gateway.restore_inference_runtime(restore_config).await
        };
        if let Err(error) = restore_result {
            log::warn!(
                "Workflow executed in embedding mode but failed to restore previous inference mode: {}",
                error
            );
        }

        Ok(EditSessionGraphExecutionOutcome {
            runtime_snapshot,
            trace_runtime_metrics,
            runtime_model_target,
            waiting_for_input,
            error: workflow_result.err().map(|error| error.to_string()),
        })
    }
}
