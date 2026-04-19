use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::task_executor;
use crate::{
    EmbeddedWorkflowHost, RuntimeExtensionsSnapshot, apply_runtime_extensions_for_execution,
};
use node_engine::{CoreTaskExecutor, NullEventSink, WorkflowExecutor};
use pantograph_workflow_service::{
    WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunHandle,
    WorkflowServiceError, graph_memory_impact_from_node_engine_graph_change,
};

struct WorkflowSessionExecutionEntry {
    workflow_id: String,
    graph_fingerprint: String,
    executor: Arc<tokio::sync::Mutex<WorkflowExecutor>>,
    carried_inputs: Vec<WorkflowPortBinding>,
}

#[derive(Clone)]
struct WorkflowSessionExecutionHandle {
    workflow_id: String,
    graph_fingerprint: String,
    executor: Arc<tokio::sync::Mutex<WorkflowExecutor>>,
    carried_inputs: Vec<WorkflowPortBinding>,
}

#[derive(Default)]
pub(crate) struct WorkflowSessionExecutionStore {
    entries: Mutex<HashMap<String, WorkflowSessionExecutionEntry>>,
}

impl WorkflowSessionExecutionStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn get(
        &self,
        session_id: &str,
    ) -> Result<Option<WorkflowSessionExecutionHandle>, WorkflowServiceError> {
        let entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;

        Ok(entries
            .get(session_id)
            .map(|entry| WorkflowSessionExecutionHandle {
                workflow_id: entry.workflow_id.clone(),
                graph_fingerprint: entry.graph_fingerprint.clone(),
                executor: entry.executor.clone(),
                carried_inputs: entry.carried_inputs.clone(),
            }))
    }

    pub(crate) fn upsert(
        &self,
        session_id: &str,
        workflow_id: &str,
        graph_fingerprint: &str,
        executor: Arc<tokio::sync::Mutex<WorkflowExecutor>>,
        carried_inputs: Vec<WorkflowPortBinding>,
    ) -> Result<(), WorkflowServiceError> {
        let mut entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;
        entries.insert(
            session_id.to_string(),
            WorkflowSessionExecutionEntry {
                workflow_id: workflow_id.to_string(),
                graph_fingerprint: graph_fingerprint.to_string(),
                executor,
                carried_inputs,
            },
        );
        Ok(())
    }

    pub(crate) fn remember_explicit_inputs(
        &self,
        session_id: &str,
        inputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        if inputs.is_empty() {
            return Ok(());
        }

        let mut entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;
        let Some(entry) = entries.get_mut(session_id) else {
            return Ok(());
        };

        for input in inputs {
            if let Some(existing) = entry.carried_inputs.iter_mut().find(|binding| {
                binding.node_id == input.node_id && binding.port_id == input.port_id
            }) {
                existing.value = input.value.clone();
            } else {
                entry.carried_inputs.push(input.clone());
            }
        }

        entry.carried_inputs.sort_by(|left, right| {
            (&left.node_id, &left.port_id).cmp(&(&right.node_id, &right.port_id))
        });
        Ok(())
    }

    pub(crate) fn set_carried_inputs(
        &self,
        session_id: &str,
        carried_inputs: Vec<WorkflowPortBinding>,
    ) -> Result<(), WorkflowServiceError> {
        let mut entries = self.entries.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "workflow session execution store lock poisoned".to_string(),
            )
        })?;
        if let Some(entry) = entries.get_mut(session_id) {
            entry.carried_inputs = carried_inputs;
        }
        Ok(())
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
    let existing = host.session_executions.get(workflow_session_id)?;
    let (session_executor, replayed_inputs) = match existing {
        Some(entry)
            if entry.workflow_id == workflow_id && entry.graph_fingerprint == graph_fingerprint =>
        {
            (entry.executor, false)
        }
        Some(entry) if entry.workflow_id == workflow_id => {
            reconcile_session_graph_change(&entry.executor, workflow_session_id, &graph).await?;
            host.session_executions.upsert(
                workflow_session_id,
                workflow_id,
                &graph_fingerprint,
                entry.executor.clone(),
                entry.carried_inputs,
            )?;
            (entry.executor, true)
        }
        Some(entry) => {
            let executor = Arc::new(tokio::sync::Mutex::new(
                build_session_executor(
                    graph.clone(),
                    workflow_session_id,
                    &runtime_ext,
                    python_runtime_execution_recorder.clone(),
                )
                .await,
            ));
            host.session_executions.upsert(
                workflow_session_id,
                workflow_id,
                &graph_fingerprint,
                executor.clone(),
                Vec::new(),
            )?;
            drop(entry);
            (executor, false)
        }
        None => {
            let executor = Arc::new(tokio::sync::Mutex::new(
                build_session_executor(
                    graph.clone(),
                    workflow_session_id,
                    &runtime_ext,
                    python_runtime_execution_recorder.clone(),
                )
                .await,
            ));
            host.session_executions.upsert(
                workflow_session_id,
                workflow_id,
                &graph_fingerprint,
                executor.clone(),
                Vec::new(),
            )?;
            (executor, false)
        }
    };

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
    if replayed_inputs {
        let carried_inputs = replay_carried_inputs(&executor, workflow_session_id, host).await?;
        host.session_executions
            .set_carried_inputs(workflow_session_id, carried_inputs)?;
    }
    apply_incremental_input_bindings(&executor, inputs).await?;
    host.session_executions
        .remember_explicit_inputs(workflow_session_id, inputs)?;

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

async fn reconcile_session_graph_change(
    executor: &Arc<tokio::sync::Mutex<WorkflowExecutor>>,
    workflow_session_id: &str,
    graph: &node_engine::WorkflowGraph,
) -> Result<(), WorkflowServiceError> {
    let executor = executor.lock().await;
    let previous_graph = executor.get_graph_snapshot().await;
    executor.restore_graph_snapshot(graph.clone()).await;
    if let Some(memory_impact) =
        graph_memory_impact_from_node_engine_graph_change(&previous_graph, graph)
    {
        executor
            .reconcile_workflow_session_node_memory(workflow_session_id, &memory_impact)
            .await;
    }
    Ok(())
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

async fn replay_carried_inputs(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
    host: &EmbeddedWorkflowHost,
) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
    let Some(entry) = host.session_executions.get(workflow_session_id)? else {
        return Ok(Vec::new());
    };

    let mut retained_inputs = Vec::new();
    for binding in entry.carried_inputs {
        if apply_input_binding_if_present(executor, &binding).await? {
            retained_inputs.push(binding);
        }
    }

    Ok(retained_inputs)
}

async fn apply_incremental_input_bindings(
    executor: &WorkflowExecutor,
    inputs: &[WorkflowPortBinding],
) -> Result<(), WorkflowServiceError> {
    for binding in inputs {
        let Some(updated_data) = update_input_binding_payload(executor, binding, true).await?
        else {
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
    strict: bool,
) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
    let graph = executor.graph().read().await;
    let Some(node) = graph.find_node(&binding.node_id) else {
        if strict {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "input binding references unknown node_id '{}'",
                binding.node_id
            )));
        }
        return Ok(None);
    };

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

async fn apply_input_binding_if_present(
    executor: &WorkflowExecutor,
    binding: &WorkflowPortBinding,
) -> Result<bool, WorkflowServiceError> {
    let Some(updated_data) = update_input_binding_payload(executor, binding, false).await? else {
        return Ok(executor
            .graph()
            .read()
            .await
            .find_node(&binding.node_id)
            .is_some());
    };
    executor
        .update_node_data(&binding.node_id, updated_data)
        .await
        .map_err(node_engine_error_to_workflow_service_error)?;
    Ok(true)
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
