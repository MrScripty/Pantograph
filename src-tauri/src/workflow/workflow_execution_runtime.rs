use std::sync::Arc;

use tauri::{ipc::Channel, AppHandle, Manager, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::startup::build_resolved_embedding_request;
use crate::llm::{SharedAppConfig, SharedGateway, SharedRuntimeRegistry};
use node_engine::EventSink;
pub(crate) use pantograph_embedded_runtime::workflow_runtime::unix_timestamp_ms;
use pantograph_embedded_runtime::{
    list_managed_runtime_manager_runtimes,
    workflow_runtime::{
        build_workflow_execution_diagnostics_snapshot_with_registry_sync,
        WorkflowExecutionDiagnosticsSyncInput,
    },
};
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowGraph, WorkflowGraphEditSessionCreateRequest,
};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::event_adapter::TauriEventAdapter;
use super::events::{
    WorkflowEvent, WorkflowRuntimeSnapshotEventInput, WorkflowSchedulerSnapshotEventInput,
};

pub struct WorkflowExecutionRuntimeState<'a> {
    pub gateway: State<'a, SharedGateway>,
    pub runtime_registry: State<'a, SharedRuntimeRegistry>,
    pub config: State<'a, SharedAppConfig>,
    pub rag_manager: State<'a, SharedRagManager>,
    pub extensions: State<'a, SharedExtensions>,
    pub workflow_service: State<'a, SharedWorkflowService>,
    pub diagnostics_store: State<'a, SharedWorkflowDiagnosticsStore>,
}

pub struct ExecuteWorkflowV2Input<'a> {
    pub app: AppHandle,
    pub graph: WorkflowGraph,
    pub state: WorkflowExecutionRuntimeState<'a>,
    pub channel: Channel<WorkflowEvent>,
}

pub struct RunWorkflowExecutionSessionInput<'a> {
    pub app: AppHandle,
    pub session_id: String,
    pub state: WorkflowExecutionRuntimeState<'a>,
    pub channel: Channel<WorkflowEvent>,
}

struct DiagnosticsEmissionInput<'a> {
    app: &'a AppHandle,
    session_id: &'a str,
    gateway: &'a SharedGateway,
    runtime_registry: &'a SharedRuntimeRegistry,
    extensions: &'a SharedExtensions,
    workflow_service: &'a SharedWorkflowService,
    diagnostics_store: &'a SharedWorkflowDiagnosticsStore,
    channel: &'a Channel<WorkflowEvent>,
    runtime_snapshot_override: Option<inference::RuntimeLifecycleSnapshot>,
    trace_runtime_metrics_override:
        Option<pantograph_workflow_service::WorkflowTraceRuntimeMetrics>,
    runtime_model_target_override: Option<String>,
}

struct SessionGraphSnapshotInput<'a> {
    app: AppHandle,
    session_id: String,
    session_graph: WorkflowGraph,
    state: WorkflowExecutionRuntimeState<'a>,
    channel: Channel<WorkflowEvent>,
}

fn managed_runtime_diagnostics_views(
    app: &AppHandle,
) -> Vec<pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView> {
    let Ok(app_data_dir) = app.path().app_data_dir() else {
        return Vec::new();
    };
    list_managed_runtime_manager_runtimes(&app_data_dir).unwrap_or_default()
}

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

fn finalize_edit_session_execution(
    waiting_for_input: bool,
    error: Option<String>,
) -> Result<(), String> {
    if waiting_for_input {
        return Ok(());
    }
    if let Some(error) = error {
        return Err(error);
    }
    Ok(())
}

fn persisted_workflow_id_for_runtime_capabilities<'a>(
    session_id: &str,
    workflow_id: &'a str,
) -> Option<&'a str> {
    (workflow_id != session_id).then_some(workflow_id)
}

async fn emit_diagnostics_snapshots(input: DiagnosticsEmissionInput<'_>) {
    let scheduler_snapshot = match input
        .workflow_service
        .workflow_get_scheduler_snapshot(
            pantograph_workflow_service::WorkflowSchedulerSnapshotRequest {
                session_id: input.session_id.to_string(),
            },
        )
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::debug!(
                "Skipping diagnostics snapshots for session '{}' because scheduler snapshot is unavailable: {}",
                input.session_id,
                error
            );
            return;
        }
    };

    let captured_at_ms = unix_timestamp_ms();
    let runtime = super::headless_runtime::build_runtime(
        input.app,
        input.gateway,
        input.runtime_registry,
        input.extensions,
        input.workflow_service,
        None,
    )
    .await;

    let runtime_workflow_id = scheduler_snapshot
        .workflow_id
        .clone()
        .unwrap_or_else(|| scheduler_snapshot.session.workflow_id.clone());
    let (runtime_capabilities, runtime_error) = if let Some(persisted_workflow_id) =
        persisted_workflow_id_for_runtime_capabilities(input.session_id, &runtime_workflow_id)
    {
        match runtime {
            Ok(runtime) => match runtime
                .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                    workflow_id: persisted_workflow_id.to_string(),
                })
                .await
            {
                Ok(response) => (Some(response), None),
                Err(error) => {
                    log::warn!(
                        "Failed to collect runtime snapshot for workflow '{}' in session '{}': {}",
                        persisted_workflow_id,
                        input.session_id,
                        error
                    );
                    (None, Some(error.to_envelope_json()))
                }
            },
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
    };

    let snapshot = build_workflow_execution_diagnostics_snapshot_with_registry_sync(
        input.gateway.as_ref(),
        WorkflowExecutionDiagnosticsSyncInput {
            runtime_registry: Some(input.runtime_registry.as_ref()),
            scheduler_snapshot: &scheduler_snapshot,
            captured_at_ms,
            runtime_capabilities,
            runtime_error,
            trace_runtime_metrics_override: input.trace_runtime_metrics_override,
            runtime_snapshot_override: input.runtime_snapshot_override.as_ref(),
            runtime_model_target_override: input.runtime_model_target_override.as_deref(),
        },
    )
    .await;
    let managed_runtimes = managed_runtime_diagnostics_views(input.app);

    let scheduler_event = WorkflowEvent::scheduler_snapshot(WorkflowSchedulerSnapshotEventInput {
        workflow_id: snapshot.scheduler.workflow_id,
        execution_id: snapshot.scheduler.trace_execution_id.clone(),
        session_id: snapshot.scheduler.session_id,
        captured_at_ms: snapshot.scheduler.captured_at_ms,
        session: Some(snapshot.scheduler.session),
        items: snapshot.scheduler.items,
        diagnostics: snapshot.scheduler.diagnostics,
        error: None,
    });
    input
        .diagnostics_store
        .record_workflow_event(&scheduler_event, captured_at_ms);
    let _ = input.channel.send(scheduler_event);
    send_diagnostics_projection(
        input.channel,
        input.diagnostics_store,
        &snapshot.scheduler.trace_execution_id,
    );

    let runtime_event = WorkflowEvent::runtime_snapshot(WorkflowRuntimeSnapshotEventInput {
        workflow_id: snapshot.runtime.workflow_id,
        execution_id: snapshot.runtime.trace_execution_id.clone(),
        captured_at_ms: snapshot.runtime.captured_at_ms,
        capabilities: snapshot.runtime.capabilities,
        trace_runtime_metrics: snapshot.runtime.trace_runtime_metrics,
        active_model_target: snapshot.runtime.active_model_target,
        embedding_model_target: snapshot.runtime.embedding_model_target,
        active_runtime_snapshot: Some(snapshot.runtime.active_runtime_snapshot),
        embedding_runtime_snapshot: snapshot.runtime.embedding_runtime_snapshot,
        managed_runtimes,
        error: snapshot.runtime.error,
    });
    input
        .diagnostics_store
        .record_workflow_event(&runtime_event, captured_at_ms);
    let _ = input.channel.send(runtime_event);
    send_diagnostics_projection(
        input.channel,
        input.diagnostics_store,
        &snapshot.runtime.trace_execution_id,
    );
}

async fn run_session_graph_snapshot(input: SessionGraphSnapshotInput<'_>) -> Result<(), String> {
    let SessionGraphSnapshotInput {
        app,
        session_id,
        session_graph,
        state,
        channel,
    } = input;
    let WorkflowExecutionRuntimeState {
        gateway,
        runtime_registry,
        config,
        rag_manager,
        extensions,
        workflow_service,
        diagnostics_store,
    } = state;
    let diagnostics_channel = channel.clone();

    workflow_service
        .workflow_graph_mark_edit_session_running(&session_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    emit_diagnostics_snapshots(DiagnosticsEmissionInput {
        app: &app,
        session_id: &session_id,
        gateway: gateway.inner(),
        runtime_registry: runtime_registry.inner(),
        extensions: extensions.inner(),
        workflow_service: workflow_service.inner(),
        diagnostics_store: diagnostics_store.inner(),
        channel: &diagnostics_channel,
        runtime_snapshot_override: None,
        trace_runtime_metrics_override: None,
        runtime_model_target_override: None,
    })
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
    let runtime = match super::headless_runtime::build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await
    {
        Ok(runtime) => runtime,
        Err(error) => {
            if let Err(finish_error) = workflow_service
                .workflow_graph_mark_edit_session_finished(&session_id)
                .await
            {
                log::warn!(
                    "Failed to finish scheduler state for edit session '{}': {}",
                    session_id,
                    finish_error
                );
            }
            return Err(error);
        }
    };
    let outcome = match runtime
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
    {
        Ok(outcome) => outcome,
        Err(error) => {
            if let Err(finish_error) = workflow_service
                .workflow_graph_mark_edit_session_finished(&session_id)
                .await
            {
                log::warn!(
                    "Failed to finish scheduler state for edit session '{}': {}",
                    session_id,
                    finish_error
                );
            }
            return Err(error.to_string());
        }
    };
    emit_diagnostics_snapshots(DiagnosticsEmissionInput {
        app: &app,
        session_id: &session_id,
        gateway: gateway.inner(),
        runtime_registry: runtime_registry.inner(),
        extensions: extensions.inner(),
        workflow_service: workflow_service.inner(),
        diagnostics_store: diagnostics_store.inner(),
        channel: &diagnostics_channel,
        runtime_snapshot_override: Some(outcome.runtime_snapshot),
        trace_runtime_metrics_override: Some(outcome.trace_runtime_metrics),
        runtime_model_target_override: outcome.runtime_model_target,
    })
    .await;
    finalize_edit_session_execution(outcome.waiting_for_input, outcome.error)
}

pub async fn execute_workflow_v2(input: ExecuteWorkflowV2Input<'_>) -> Result<String, String> {
    let ExecuteWorkflowV2Input {
        app,
        graph,
        state,
        channel,
    } = input;
    let session = state
        .workflow_service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: None,
        })
        .await
        .map_err(|e| e.to_envelope_json())?;
    let execution_id = session.session_id.clone();
    let session_graph = state
        .workflow_service
        .workflow_graph_get_runtime_snapshot(&execution_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    run_session_graph_snapshot(SessionGraphSnapshotInput {
        app,
        session_id: execution_id.clone(),
        session_graph,
        state,
        channel,
    })
    .await?;
    Ok(execution_id)
}

pub async fn run_workflow_execution_session(
    input: RunWorkflowExecutionSessionInput<'_>,
) -> Result<(), String> {
    let RunWorkflowExecutionSessionInput {
        app,
        session_id,
        state,
        channel,
    } = input;
    let session_graph = state
        .workflow_service
        .workflow_graph_get_runtime_snapshot(&session_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    run_session_graph_snapshot(SessionGraphSnapshotInput {
        app,
        session_id,
        session_graph,
        state,
        channel,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{finalize_edit_session_execution, persisted_workflow_id_for_runtime_capabilities};

    #[test]
    fn finalize_edit_session_execution_treats_waiting_as_non_error() {
        assert!(finalize_edit_session_execution(true, None).is_ok());
    }

    #[test]
    fn finalize_edit_session_execution_propagates_terminal_error() {
        let error =
            finalize_edit_session_execution(false, Some("workflow execution failed".to_string()))
                .expect_err("terminal error should be returned");

        assert_eq!(error, "workflow execution failed");
    }

    #[test]
    fn finalize_edit_session_execution_accepts_success_without_error() {
        assert!(finalize_edit_session_execution(false, None).is_ok());
    }

    #[test]
    fn persisted_workflow_id_for_runtime_capabilities_skips_transient_edit_session_ids() {
        assert_eq!(
            persisted_workflow_id_for_runtime_capabilities("session-1", "session-1"),
            None
        );
        assert_eq!(
            persisted_workflow_id_for_runtime_capabilities("session-1", "saved-flow"),
            Some("saved-flow")
        );
    }
}
