use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Manager, State, ipc::Channel};

use crate::agent::rag::SharedRagManager;
use crate::llm::startup::build_resolved_embedding_request;
use crate::llm::{SharedAppConfig, SharedGateway, SharedRuntimeRegistry};
use node_engine::EventSink;
pub(crate) use pantograph_embedded_runtime::workflow_runtime::unix_timestamp_ms;
use pantograph_embedded_runtime::{
    list_managed_runtime_manager_runtimes,
    workflow_runtime::{
        WorkflowExecutionDiagnosticsSyncInput,
        build_workflow_execution_diagnostics_snapshot_with_registry_sync,
    },
};
use pantograph_workflow_service::{WorkflowCapabilitiesRequest, WorkflowGraph};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::diagnostics::{
    SharedWorkflowDiagnosticsStore, WorkflowRuntimeSnapshotUpdate, WorkflowSchedulerSnapshotUpdate,
};
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

pub struct RunWorkflowExecutionSessionInput<'a> {
    pub app: AppHandle,
    pub session_id: String,
    pub state: WorkflowExecutionRuntimeState<'a>,
    pub channel: Channel<WorkflowEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowEditSessionRunResponse {
    pub workflow_run_id: String,
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
    workflow_run_id_override: Option<&'a str>,
}

struct SessionGraphSnapshotInput<'a> {
    app: AppHandle,
    session_id: String,
    workflow_run_id: String,
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

fn diagnostics_event_workflow_run_id(
    snapshot_workflow_run_id: Option<String>,
    workflow_run_id_override: Option<&str>,
) -> Option<String> {
    snapshot_workflow_run_id.or_else(|| workflow_run_id_override.map(ToOwned::to_owned))
}

async fn workflow_id_for_runtime_events(
    workflow_service: &SharedWorkflowService,
    session_id: &str,
) -> String {
    workflow_service
        .workflow_get_scheduler_snapshot(
            pantograph_workflow_service::WorkflowSchedulerSnapshotRequest {
                session_id: session_id.to_string(),
            },
        )
        .await
        .map(|snapshot| snapshot.workflow_id.unwrap_or(snapshot.session.workflow_id))
        .unwrap_or_else(|error| {
            log::debug!(
                "Falling back to session id for runtime events because scheduler snapshot is unavailable for session '{}': {}",
                session_id,
                error
            );
            session_id.to_string()
        })
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

    let scheduler_workflow_run_id = diagnostics_event_workflow_run_id(
        snapshot.scheduler.workflow_run_id.clone(),
        input.workflow_run_id_override,
    );
    if let Some(workflow_run_id) = scheduler_workflow_run_id {
        let scheduler_event =
            WorkflowEvent::scheduler_snapshot(WorkflowSchedulerSnapshotEventInput {
                workflow_id: snapshot.scheduler.workflow_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
                session_id: snapshot.scheduler.session_id.clone(),
                captured_at_ms: snapshot.scheduler.captured_at_ms,
                session: Some(snapshot.scheduler.session.clone()),
                items: snapshot.scheduler.items.clone(),
                diagnostics: snapshot.scheduler.diagnostics.clone(),
                error: None,
            });
        input
            .diagnostics_store
            .record_workflow_event(&scheduler_event, captured_at_ms);
        let _ = input.channel.send(scheduler_event);
        send_diagnostics_projection(input.channel, input.diagnostics_store, &workflow_run_id);
    } else {
        input
            .diagnostics_store
            .update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
                workflow_id: snapshot.scheduler.workflow_id.clone(),
                workflow_run_id: None,
                session_id: Some(snapshot.scheduler.session_id.clone()),
                session: Some(snapshot.scheduler.session.clone()),
                items: snapshot.scheduler.items.clone(),
                diagnostics: snapshot.scheduler.diagnostics.clone(),
                last_error: None,
                captured_at_ms: snapshot.scheduler.captured_at_ms,
            });
    }

    let runtime_workflow_run_id = diagnostics_event_workflow_run_id(
        snapshot.runtime.workflow_run_id.clone(),
        input.workflow_run_id_override,
    );
    if let Some(workflow_run_id) = runtime_workflow_run_id {
        let runtime_event = WorkflowEvent::runtime_snapshot(WorkflowRuntimeSnapshotEventInput {
            workflow_id: snapshot.runtime.workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            captured_at_ms: snapshot.runtime.captured_at_ms,
            capabilities: snapshot.runtime.capabilities.clone(),
            trace_runtime_metrics: snapshot.runtime.trace_runtime_metrics.clone(),
            active_model_target: snapshot.runtime.active_model_target.clone(),
            embedding_model_target: snapshot.runtime.embedding_model_target.clone(),
            active_runtime_snapshot: Some(snapshot.runtime.active_runtime_snapshot.clone()),
            embedding_runtime_snapshot: snapshot.runtime.embedding_runtime_snapshot.clone(),
            managed_runtimes,
            error: snapshot.runtime.error.clone(),
        });
        input
            .diagnostics_store
            .record_workflow_event(&runtime_event, captured_at_ms);
        let _ = input.channel.send(runtime_event);
        send_diagnostics_projection(input.channel, input.diagnostics_store, &workflow_run_id);
    } else {
        input
            .diagnostics_store
            .update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
                workflow_id: Some(snapshot.runtime.workflow_id.clone()),
                capabilities: snapshot.runtime.capabilities.clone(),
                last_error: snapshot.runtime.error.clone(),
                active_model_target: snapshot.runtime.active_model_target.clone(),
                embedding_model_target: snapshot.runtime.embedding_model_target.clone(),
                active_runtime_snapshot: Some(snapshot.runtime.active_runtime_snapshot.clone()),
                embedding_runtime_snapshot: snapshot.runtime.embedding_runtime_snapshot.clone(),
                managed_runtimes,
                captured_at_ms: snapshot.runtime.captured_at_ms,
            });
    }
}

async fn run_session_graph_snapshot(input: SessionGraphSnapshotInput<'_>) -> Result<(), String> {
    let SessionGraphSnapshotInput {
        app,
        session_id,
        workflow_run_id,
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
        workflow_run_id_override: Some(&workflow_run_id),
    })
    .await;

    diagnostics_store.set_execution_graph(&workflow_run_id, &session_graph);

    let event_workflow_id =
        workflow_id_for_runtime_events(workflow_service.inner(), &session_id).await;
    let event_adapter = Arc::new(
        TauriEventAdapter::new(
            channel,
            event_workflow_id,
            diagnostics_store.inner().clone(),
        )
        .with_execution_graph(session_graph.clone()),
    );
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
                workflow_run_id_override: Some(&workflow_run_id),
            })
            .await;
            return Err(error);
        }
    };
    let outcome = match runtime
        .execute_edit_session_graph(
            &session_id,
            &workflow_run_id,
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
                workflow_run_id_override: Some(&workflow_run_id),
            })
            .await;
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
        workflow_run_id_override: Some(&workflow_run_id),
    })
    .await;
    finalize_edit_session_execution(outcome.waiting_for_input, outcome.error)
}

pub async fn run_workflow_execution_session(
    input: RunWorkflowExecutionSessionInput<'_>,
) -> Result<WorkflowEditSessionRunResponse, String> {
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
    let workflow_run_id = state
        .workflow_service
        .workflow_graph_begin_edit_session_run(&session_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    let response = WorkflowEditSessionRunResponse {
        workflow_run_id: workflow_run_id.clone(),
    };
    run_session_graph_snapshot(SessionGraphSnapshotInput {
        app,
        session_id,
        workflow_run_id,
        session_graph,
        state,
        channel,
    })
    .await?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostics_event_workflow_run_id, finalize_edit_session_execution,
        persisted_workflow_id_for_runtime_capabilities,
    };

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

    #[test]
    fn diagnostics_event_workflow_run_id_prefers_scheduler_identity() {
        assert_eq!(
            diagnostics_event_workflow_run_id(Some("active-run".to_string()), Some("final-run"))
                .as_deref(),
            Some("active-run")
        );
    }

    #[test]
    fn diagnostics_event_workflow_run_id_uses_override_after_scheduler_finishes() {
        assert_eq!(
            diagnostics_event_workflow_run_id(None, Some("final-run")).as_deref(),
            Some("final-run")
        );
    }

    #[test]
    fn diagnostics_event_workflow_run_id_stays_empty_without_real_run() {
        assert_eq!(diagnostics_event_workflow_run_id(None, None), None);
    }
}
