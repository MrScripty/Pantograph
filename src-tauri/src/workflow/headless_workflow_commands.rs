//! Headless workflow API adapter for Tauri transport.
//!
//! This module now acts as a thin transport wrapper over the backend-owned
//! Pantograph embedded runtime.

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
    WorkflowSessionStaleCleanupRequest, WorkflowSessionStaleCleanupResponse,
    WorkflowSessionStatusRequest, WorkflowSessionStatusResponse,
};
use tauri::{AppHandle, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::headless_diagnostics::workflow_scheduler_snapshot_response;
pub(crate) use super::headless_runtime::build_runtime;

fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

pub async fn workflow_run(
    request: WorkflowRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await?;
    runtime
        .workflow_run(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_get_capabilities(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_io(
    request: WorkflowIoRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowIoResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_get_io(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_preflight(
    request: WorkflowPreflightRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowPreflightResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_preflight(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_create_session(
    request: WorkflowSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCreateResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .create_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_run_session(
    request: WorkflowSessionRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await?;
    runtime
        .run_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_close_session(
    request: WorkflowSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCloseResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
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

pub async fn workflow_cleanup_stale_sessions(
    request: WorkflowSessionStaleCleanupRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionStaleCleanupResponse, String> {
    workflow_service
        .workflow_cleanup_stale_sessions(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_scheduler_snapshot(
    request: WorkflowSchedulerSnapshotRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSchedulerSnapshotResponse, String> {
    workflow_scheduler_snapshot_response(workflow_service.inner(), request).await
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
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionKeepAliveResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_set_session_keep_alive(request)
        .await
        .map_err(workflow_error_json)
}

#[cfg(test)]
#[path = "headless_workflow_commands_tests.rs"]
mod tests;
