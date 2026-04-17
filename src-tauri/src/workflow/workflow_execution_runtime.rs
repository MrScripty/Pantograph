use std::sync::Arc;

use tauri::{ipc::Channel, AppHandle, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::startup::build_resolved_embedding_request;
use crate::llm::{SharedAppConfig, SharedGateway, SharedRuntimeRegistry};
use node_engine::EventSink;
use pantograph_embedded_runtime::workflow_runtime::build_workflow_execution_diagnostics_snapshot_with_registry_sync;
pub(crate) use pantograph_embedded_runtime::workflow_runtime::unix_timestamp_ms;
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowGraph, WorkflowGraphEditSessionCreateRequest,
};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::event_adapter::TauriEventAdapter;
use super::events::WorkflowEvent;

fn send_diagnostics_projection(
    channel: &Channel<WorkflowEvent>,
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    execution_id: &str,
) {
    let _ = channel.send(WorkflowEvent::diagnostics_snapshot(
        execution_id.to_string(),
        diagnostics_store.snapshot(),
    ));
}

async fn emit_diagnostics_snapshots(
    app: &AppHandle,
    session_id: &str,
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    extensions: &SharedExtensions,
    workflow_service: &SharedWorkflowService,
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    channel: &Channel<WorkflowEvent>,
    runtime_snapshot_override: Option<inference::RuntimeLifecycleSnapshot>,
    trace_runtime_metrics_override: Option<
        pantograph_workflow_service::WorkflowTraceRuntimeMetrics,
    >,
    runtime_model_target_override: Option<String>,
) {
    let scheduler_snapshot = match workflow_service
        .workflow_get_scheduler_snapshot(
            pantograph_workflow_service::WorkflowSchedulerSnapshotRequest {
                session_id: session_id.to_string(),
            },
        )
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::debug!(
                "Skipping diagnostics snapshots for session '{}' because scheduler snapshot is unavailable: {}",
                session_id,
                error
            );
            return;
        }
    };

    let captured_at_ms = unix_timestamp_ms();
    let runtime = super::headless_runtime::build_runtime(
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
        None,
    )
    .await;

    let runtime_workflow_id = scheduler_snapshot
        .workflow_id
        .clone()
        .unwrap_or_else(|| scheduler_snapshot.session.workflow_id.clone());
    let (runtime_capabilities, runtime_error) = match runtime {
        Ok(runtime) => match runtime
            .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                workflow_id: runtime_workflow_id.clone(),
            })
            .await
        {
            Ok(response) => (Some(response), None),
            Err(error) => {
                log::warn!(
                    "Failed to collect runtime snapshot for workflow '{}' in session '{}': {}",
                    runtime_workflow_id,
                    session_id,
                    error
                );
                (None, Some(error.to_envelope_json()))
            }
        },
        Err(error) => (None, Some(error)),
    };

    let snapshot = build_workflow_execution_diagnostics_snapshot_with_registry_sync(
        gateway.as_ref(),
        Some(runtime_registry.as_ref()),
        &scheduler_snapshot,
        captured_at_ms,
        runtime_capabilities,
        runtime_error,
        trace_runtime_metrics_override,
        runtime_snapshot_override.as_ref(),
        runtime_model_target_override.as_deref(),
    )
    .await;

    let scheduler_event = WorkflowEvent::scheduler_snapshot(
        snapshot.scheduler.workflow_id,
        snapshot.scheduler.trace_execution_id.clone(),
        snapshot.scheduler.session_id,
        snapshot.scheduler.captured_at_ms,
        Some(snapshot.scheduler.session),
        snapshot.scheduler.items,
        snapshot.scheduler.diagnostics,
        None,
    );
    diagnostics_store.record_workflow_event(&scheduler_event, captured_at_ms);
    let _ = channel.send(scheduler_event);
    send_diagnostics_projection(
        channel,
        diagnostics_store,
        &snapshot.scheduler.trace_execution_id,
    );

    let runtime_event = WorkflowEvent::runtime_snapshot(
        snapshot.runtime.workflow_id,
        snapshot.runtime.trace_execution_id.clone(),
        snapshot.runtime.captured_at_ms,
        snapshot.runtime.capabilities,
        snapshot.runtime.trace_runtime_metrics,
        snapshot.runtime.active_model_target,
        snapshot.runtime.embedding_model_target,
        Some(snapshot.runtime.active_runtime_snapshot),
        snapshot.runtime.embedding_runtime_snapshot,
        snapshot.runtime.error,
    );
    diagnostics_store.record_workflow_event(&runtime_event, captured_at_ms);
    let _ = channel.send(runtime_event);
    send_diagnostics_projection(
        channel,
        diagnostics_store,
        &snapshot.runtime.trace_execution_id,
    );
}

async fn run_session_graph_snapshot(
    app: AppHandle,
    session_id: String,
    session_graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    let diagnostics_channel = channel.clone();

    emit_diagnostics_snapshots(
        &app,
        &session_id,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        diagnostics_store.inner(),
        &diagnostics_channel,
        None,
        None,
        None,
    )
    .await;

    diagnostics_store.set_execution_graph(&session_id, &session_graph);

    let event_adapter = Arc::new(TauriEventAdapter::new(
        channel,
        &session_id,
        diagnostics_store.inner().clone(),
    ));
    let guard = config.read().await;
    let device = guard.device.clone();
    drop(guard);
    let runtime = super::headless_runtime::build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await?;
    let outcome = runtime
        .execute_edit_session_graph(
            &session_id,
            &session_graph,
            build_resolved_embedding_request(
                None,
                None,
                &device,
                Some("nomic-embed-text".to_string()),
            ),
            event_adapter.clone() as Arc<dyn EventSink>,
        )
        .await
        .map_err(|error| error.to_string())?;
    emit_diagnostics_snapshots(
        &app,
        &session_id,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        diagnostics_store.inner(),
        &diagnostics_channel,
        Some(outcome.runtime_snapshot),
        Some(outcome.trace_runtime_metrics),
        outcome.runtime_model_target,
    )
    .await;
    if outcome.waiting_for_input {
        return Ok(());
    }
    if let Some(error) = outcome.error {
        return Err(error);
    }
    Ok(())
}

pub async fn execute_workflow_v2(
    app: AppHandle,
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<String, String> {
    let session = workflow_service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest { graph })
        .await
        .map_err(|e| e.to_envelope_json())?;
    let execution_id = session.session_id.clone();
    let session_graph = workflow_service
        .workflow_graph_get_runtime_snapshot(&execution_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    run_session_graph_snapshot(
        app,
        execution_id.clone(),
        session_graph,
        gateway,
        runtime_registry,
        config,
        rag_manager,
        extensions,
        workflow_service,
        diagnostics_store,
        channel,
    )
    .await?;
    Ok(execution_id)
}

pub async fn run_workflow_session(
    app: AppHandle,
    session_id: String,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    let session_graph = workflow_service
        .workflow_graph_get_runtime_snapshot(&session_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    run_session_graph_snapshot(
        app,
        session_id,
        session_graph,
        gateway,
        runtime_registry,
        config,
        rag_manager,
        extensions,
        workflow_service,
        diagnostics_store,
        channel,
    )
    .await
}
