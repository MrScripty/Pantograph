use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::task_executor;
use crate::{
    EmbeddedWorkflowHost, RuntimeExtensionsSnapshot, apply_runtime_extensions_for_execution,
};
use node_engine::{CoreTaskExecutor, NullEventSink, WorkflowExecutor};
use pantograph_workflow_service::{
    WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunHandle,
    WorkflowServiceError,
};

struct WorkflowSessionExecutionEntry {
    workflow_id: String,
    graph_fingerprint: String,
    executor: Arc<tokio::sync::Mutex<WorkflowExecutor>>,
}

#[derive(Default)]
pub(crate) struct WorkflowSessionExecutionStore {
    entries: Mutex<HashMap<String, WorkflowSessionExecutionEntry>>,
}

impl WorkflowSessionExecutionStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn retain_or_replace(
        &self,
        session_id: &str,
        workflow_id: &str,
        graph_fingerprint: &str,
        executor: WorkflowExecutor,
    ) -> Result<Arc<tokio::sync::Mutex<WorkflowExecutor>>, WorkflowServiceError> {
        let mut entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;

        if let Some(entry) = entries.get(session_id) {
            if entry.workflow_id == workflow_id && entry.graph_fingerprint == graph_fingerprint {
                return Ok(entry.executor.clone());
            }
        }

        let executor = Arc::new(tokio::sync::Mutex::new(executor));
        entries.insert(
            session_id.to_string(),
            WorkflowSessionExecutionEntry {
                workflow_id: workflow_id.to_string(),
                graph_fingerprint: graph_fingerprint.to_string(),
                executor: executor.clone(),
            },
        );
        Ok(executor)
    }

    pub(crate) fn remove(&self, session_id: &str) -> Result<(), WorkflowServiceError> {
        let mut entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;
        entries.remove(session_id);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn handle(
        &self,
        session_id: &str,
    ) -> Result<Option<Arc<tokio::sync::Mutex<WorkflowExecutor>>>, WorkflowServiceError> {
        let entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;
        Ok(entries.get(session_id).map(|entry| entry.executor.clone()))
    }
}

pub(crate) async fn run_session_workflow(
    host: &EmbeddedWorkflowHost,
    workflow_id: &str,
    workflow_session_id: &str,
    inputs: &[WorkflowPortBinding],
    output_targets: Option<&[WorkflowOutputTarget]>,
    run_handle: WorkflowRunHandle,
) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
    if run_handle.is_cancelled() {
        return Err(WorkflowServiceError::Cancelled(
            "workflow run cancelled before execution started".to_string(),
        ));
    }

    let stored = pantograph_workflow_service::capabilities::load_and_validate_workflow(
        workflow_id,
        &host.workflow_roots,
    )?;
    let graph = stored.to_workflow_graph(workflow_id);
    let output_node_ids = EmbeddedWorkflowHost::resolve_output_node_ids(&graph, output_targets)?;
    let graph_fingerprint = WorkflowHost::workflow_graph_fingerprint(host, workflow_id).await?;
    let runtime_ext = RuntimeExtensionsSnapshot::from_shared(&host.extensions).await;

    let python_runtime_execution_recorder =
        Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());
    let session_executor = host.session_executions.retain_or_replace(
        workflow_session_id,
        workflow_id,
        &graph_fingerprint,
        build_session_executor(
            graph,
            workflow_session_id,
            &runtime_ext,
            python_runtime_execution_recorder.clone(),
        )
        .await,
    )?;

    let core = Arc::new(
        CoreTaskExecutor::new()
            .with_project_root(host.project_root.clone())
            .with_gateway(host.gateway.clone())
            .with_execution_id(workflow_session_id.to_string()),
    );
    let tauri_executor = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
        host.rag_backend.clone(),
        host.python_runtime.clone(),
    ));
    let task_executor = node_engine::CompositeTaskExecutor::new(
        Some(tauri_executor as Arc<dyn node_engine::TaskExecutor>),
        core,
    );

    let mut executor = session_executor.lock().await;
    apply_runtime_extensions_for_execution(
        &mut executor,
        &runtime_ext,
        None,
        Some(workflow_session_id.to_string()),
        Some(python_runtime_execution_recorder.clone()),
    );
    apply_incremental_input_bindings(&executor, inputs).await?;

    let mut node_outputs = HashMap::new();
    for node_id in &output_node_ids {
        if run_handle.is_cancelled() {
            return Err(WorkflowServiceError::Cancelled(
                "workflow run cancelled during execution".to_string(),
            ));
        }
        let outputs = executor
            .demand(node_id, &task_executor)
            .await
            .map_err(node_engine_error_to_workflow_service_error)?;
        node_outputs.insert(node_id.clone(), outputs);
    }
    drop(executor);

    let python_runtime_execution_metadata = python_runtime_execution_recorder.snapshots();
    host.observe_python_runtime_execution_metadata(&python_runtime_execution_metadata)?;

    EmbeddedWorkflowHost::collect_run_outputs(&node_outputs, &output_node_ids, output_targets)
}

async fn build_session_executor(
    graph: node_engine::WorkflowGraph,
    workflow_session_id: &str,
    runtime_ext: &RuntimeExtensionsSnapshot,
    python_runtime_execution_recorder: Arc<task_executor::PythonRuntimeExecutionRecorder>,
) -> WorkflowExecutor {
    let mut executor = WorkflowExecutor::new(
        workflow_session_id.to_string(),
        graph,
        Arc::new(NullEventSink),
    );
    executor
        .bind_workflow_session(workflow_session_id.to_string())
        .await;
    apply_runtime_extensions_for_execution(
        &mut executor,
        runtime_ext,
        None,
        Some(workflow_session_id.to_string()),
        Some(python_runtime_execution_recorder),
    );
    executor
}

async fn apply_incremental_input_bindings(
    executor: &WorkflowExecutor,
    inputs: &[WorkflowPortBinding],
) -> Result<(), WorkflowServiceError> {
    for binding in inputs {
        let Some(updated_data) = update_input_binding_payload(executor, binding).await? else {
            continue;
        };
        executor
            .update_node_data(&binding.node_id, updated_data)
            .await
            .map_err(node_engine_error_to_workflow_service_error)?;
    }

    Ok(())
}

async fn update_input_binding_payload(
    executor: &WorkflowExecutor,
    binding: &WorkflowPortBinding,
) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
    let graph = executor.graph().read().await;
    let node = graph.find_node(&binding.node_id).ok_or_else(|| {
        WorkflowServiceError::InvalidRequest(format!(
            "input binding references unknown node_id '{}'",
            binding.node_id
        ))
    })?;

    let mut updated_data = if node.data.is_null() {
        serde_json::json!({})
    } else {
        node.data.clone()
    };
    let map = updated_data.as_object_mut().ok_or_else(|| {
        WorkflowServiceError::InvalidRequest(format!(
            "input node '{}' has non-object data payload",
            binding.node_id
        ))
    })?;

    if map.get(&binding.port_id) == Some(&binding.value) {
        return Ok(None);
    }

    map.insert(binding.port_id.clone(), binding.value.clone());
    Ok(Some(updated_data))
}

fn node_engine_error_to_workflow_service_error(
    error: node_engine::NodeEngineError,
) -> WorkflowServiceError {
    match error {
        node_engine::NodeEngineError::WaitingForInput { task_id, .. } => {
            WorkflowServiceError::InvalidRequest(format!(
                "workflow requires interactive input at node '{}'",
                task_id
            ))
        }
        other => WorkflowServiceError::Internal(other.to_string()),
    }
}
