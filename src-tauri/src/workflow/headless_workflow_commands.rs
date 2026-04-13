//! Headless workflow API adapter for Tauri transport.
//!
//! This module now acts as a thin transport wrapper over the backend-owned
//! Pantograph embedded runtime.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_embedded_runtime::{
    EmbeddedRuntime, EmbeddedRuntimeConfig, RagBackend, RagDocument,
};
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowIoRequest,
    WorkflowIoResponse, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunRequest,
    WorkflowRunResponse, WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowServiceError, WorkflowSessionCloseRequest, WorkflowSessionCloseResponse,
    WorkflowSessionCreateRequest, WorkflowSessionCreateResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRunRequest,
    WorkflowSessionStatusRequest, WorkflowSessionStatusResponse, WorkflowTraceSnapshotRequest,
    WorkflowTraceSnapshotResponse,
};
use tauri::{AppHandle, Manager, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::SharedGateway;
use crate::project_root::resolve_project_root;

use super::commands::{SharedExtensions, SharedWorkflowDiagnosticsStore, SharedWorkflowService};
use super::diagnostics::{WorkflowDiagnosticsProjection, WorkflowDiagnosticsSnapshotRequest};

fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))
}

struct TauriRagBackend {
    rag_manager: SharedRagManager,
}

#[async_trait]
impl RagBackend for TauriRagBackend {
    async fn search_as_docs(&self, query: &str, limit: usize) -> Result<Vec<RagDocument>, String> {
        let guard = self.rag_manager.read().await;
        let docs = guard
            .search_as_docs(query, limit)
            .await
            .map_err(|err| err.to_string())?;
        Ok(docs
            .into_iter()
            .map(|doc| RagDocument {
                id: doc.id,
                title: doc.title,
                section: doc.section,
                summary: doc.summary,
                content: doc.content,
            })
            .collect())
    }
}

pub(super) fn build_runtime(
    app: &AppHandle,
    gateway: &SharedGateway,
    extensions: &SharedExtensions,
    workflow_service: &SharedWorkflowService,
    rag_manager: Option<&SharedRagManager>,
) -> Result<EmbeddedRuntime, String> {
    let config = EmbeddedRuntimeConfig::new(app_data_dir(app)?, resolve_project_root()?);
    let rag_backend = rag_manager.cloned().map(|manager| {
        Arc::new(TauriRagBackend {
            rag_manager: manager,
        }) as Arc<dyn RagBackend>
    });
    Ok(EmbeddedRuntime::with_default_python_runtime(
        config,
        gateway.inner_arc(),
        extensions.clone(),
        workflow_service.clone(),
        rag_backend,
    ))
}

pub async fn workflow_run(
    request: WorkflowRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )?;
    runtime
        .workflow_run(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_get_capabilities(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_io(
    request: WorkflowIoRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowIoResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_get_io(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_preflight(
    request: WorkflowPreflightRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowPreflightResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_preflight(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_create_session(
    request: WorkflowSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCreateResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .create_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_run_session(
    request: WorkflowSessionRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )?;
    runtime
        .run_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_close_session(
    request: WorkflowSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCloseResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .close_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_session_status(
    request: WorkflowSessionStatusRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionStatusResponse, String> {
    workflow_service
        .workflow_get_session_status(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_list_session_queue(
    request: WorkflowSessionQueueListRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionQueueListResponse, String> {
    workflow_service
        .workflow_list_session_queue(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_scheduler_snapshot(
    request: WorkflowSchedulerSnapshotRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSchedulerSnapshotResponse, String> {
    workflow_service
        .workflow_get_scheduler_snapshot(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_cancel_session_queue_item(
    request: WorkflowSessionQueueCancelRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionQueueCancelResponse, String> {
    workflow_service
        .workflow_cancel_session_queue_item(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_reprioritize_session_queue_item(
    request: WorkflowSessionQueueReprioritizeRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionQueueReprioritizeResponse, String> {
    workflow_service
        .workflow_reprioritize_session_queue_item(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_set_session_keep_alive(
    request: WorkflowSessionKeepAliveRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionKeepAliveResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_set_session_keep_alive(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_diagnostics_snapshot(
    request: WorkflowDiagnosticsSnapshotRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowDiagnosticsProjection, String> {
    let captured_at_ms = super::workflow_execution_commands::unix_timestamp_ms();
    let session_id = request
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let workflow_id = request
        .workflow_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let workflow_name = request
        .workflow_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(session_id) = session_id.as_deref() {
        diagnostics_store.set_execution_metadata(
            session_id,
            workflow_id.clone(),
            workflow_name.clone(),
        );

        match workflow_service
            .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                session_id: session_id.to_string(),
            })
            .await
        {
            Ok(snapshot) => {
                diagnostics_store.update_scheduler_snapshot(
                    snapshot.workflow_id,
                    Some(snapshot.session_id),
                    Some(snapshot.session),
                    snapshot.items,
                    None,
                    captured_at_ms,
                );
            }
            Err(error) => {
                diagnostics_store.update_scheduler_snapshot(
                    workflow_id.clone(),
                    Some(session_id.to_string()),
                    None,
                    Vec::new(),
                    Some(error.to_envelope_json()),
                    captured_at_ms,
                );
            }
        }
    } else {
        diagnostics_store.update_scheduler_snapshot(
            None,
            None,
            None,
            Vec::new(),
            None,
            captured_at_ms,
        );
    }

    if let Some(workflow_id) = workflow_id {
        let runtime = build_runtime(
            &app,
            gateway.inner(),
            extensions.inner(),
            workflow_service.inner(),
            None,
        )?;

        match runtime
            .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                workflow_id: workflow_id.clone(),
            })
            .await
        {
            Ok(capabilities) => {
                diagnostics_store.update_runtime_snapshot(
                    Some(workflow_id),
                    Some(capabilities),
                    None,
                    captured_at_ms,
                );
            }
            Err(error) => {
                diagnostics_store.update_runtime_snapshot(
                    Some(workflow_id),
                    None,
                    Some(error.to_envelope_json()),
                    captured_at_ms,
                );
            }
        }
    } else {
        diagnostics_store.update_runtime_snapshot(None, None, None, captured_at_ms);
    }

    Ok(diagnostics_store.snapshot())
}

pub async fn workflow_get_trace_snapshot(
    request: WorkflowTraceSnapshotRequest,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowTraceSnapshotResponse, String> {
    diagnostics_store
        .trace_snapshot(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_clear_diagnostics_history(
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowDiagnosticsProjection, String> {
    Ok(diagnostics_store.clear_history())
}
