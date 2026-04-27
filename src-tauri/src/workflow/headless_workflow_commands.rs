//! Headless workflow API adapter for Tauri transport.
//!
//! This module now acts as a thin transport wrapper over the backend-owned
//! Pantograph embedded runtime.

use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse,
    WorkflowExecutionSessionCloseRequest, WorkflowExecutionSessionCloseResponse,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionCreateResponse,
    WorkflowExecutionSessionKeepAliveRequest, WorkflowExecutionSessionKeepAliveResponse,
    WorkflowExecutionSessionQueueCancelRequest, WorkflowExecutionSessionQueueCancelResponse,
    WorkflowExecutionSessionQueueListRequest, WorkflowExecutionSessionQueueListResponse,
    WorkflowExecutionSessionQueueReprioritizeRequest,
    WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowExecutionSessionRunRequest,
    WorkflowExecutionSessionStaleCleanupRequest, WorkflowExecutionSessionStaleCleanupResponse,
    WorkflowExecutionSessionStatusRequest, WorkflowExecutionSessionStatusResponse,
    WorkflowIoArtifactQueryRequest, WorkflowIoArtifactQueryResponse, WorkflowIoRequest,
    WorkflowIoResponse, WorkflowLibraryUsageQueryRequest, WorkflowLibraryUsageQueryResponse,
    WorkflowLocalNetworkStatusQueryRequest, WorkflowLocalNetworkStatusQueryResponse,
    WorkflowNodeStatusQueryRequest, WorkflowNodeStatusQueryResponse, WorkflowPreflightRequest,
    WorkflowPreflightResponse, WorkflowProjectionRebuildRequest, WorkflowProjectionRebuildResponse,
    WorkflowRetentionPolicyQueryRequest, WorkflowRetentionPolicyQueryResponse,
    WorkflowRetentionPolicyUpdateRequest, WorkflowRetentionPolicyUpdateResponse,
    WorkflowRunDetailQueryRequest, WorkflowRunDetailQueryResponse, WorkflowRunGraphQueryRequest,
    WorkflowRunGraphQueryResponse, WorkflowRunListQueryRequest, WorkflowRunListQueryResponse,
    WorkflowRunResponse, WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowSchedulerTimelineQueryRequest, WorkflowSchedulerTimelineQueryResponse,
    WorkflowServiceError,
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

pub async fn workflow_create_execution_session(
    request: WorkflowExecutionSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionCreateResponse, String> {
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
        .create_workflow_execution_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_run_execution_session(
    request: WorkflowExecutionSessionRunRequest,
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
        .run_workflow_execution_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_close_execution_session(
    request: WorkflowExecutionSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionCloseResponse, String> {
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
        .close_workflow_execution_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_execution_session_status(
    request: WorkflowExecutionSessionStatusRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionStatusResponse, String> {
    workflow_service
        .workflow_get_execution_session_status(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_list_execution_session_queue(
    request: WorkflowExecutionSessionQueueListRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionQueueListResponse, String> {
    workflow_service
        .workflow_list_execution_session_queue(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_cleanup_stale_execution_sessions(
    request: WorkflowExecutionSessionStaleCleanupRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionStaleCleanupResponse, String> {
    workflow_service
        .workflow_cleanup_stale_execution_sessions(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_scheduler_snapshot(
    request: WorkflowSchedulerSnapshotRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSchedulerSnapshotResponse, String> {
    workflow_scheduler_snapshot_response(workflow_service.inner(), request).await
}

pub async fn workflow_scheduler_timeline_query(
    request: WorkflowSchedulerTimelineQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSchedulerTimelineQueryResponse, String> {
    workflow_service
        .workflow_scheduler_timeline_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_run_list_query(
    request: WorkflowRunListQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunListQueryResponse, String> {
    workflow_service
        .workflow_run_list_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_run_detail_query(
    request: WorkflowRunDetailQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunDetailQueryResponse, String> {
    workflow_service
        .workflow_run_detail_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_run_graph_query(
    request: WorkflowRunGraphQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunGraphQueryResponse, String> {
    workflow_service
        .workflow_run_graph_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_io_artifact_query(
    request: WorkflowIoArtifactQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowIoArtifactQueryResponse, String> {
    workflow_service
        .workflow_io_artifact_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_node_status_query(
    request: WorkflowNodeStatusQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowNodeStatusQueryResponse, String> {
    workflow_service
        .workflow_node_status_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_projection_rebuild(
    request: WorkflowProjectionRebuildRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowProjectionRebuildResponse, String> {
    workflow_service
        .workflow_projection_rebuild(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_library_usage_query(
    request: WorkflowLibraryUsageQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowLibraryUsageQueryResponse, String> {
    workflow_service
        .workflow_library_usage_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_retention_policy_query(
    request: WorkflowRetentionPolicyQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRetentionPolicyQueryResponse, String> {
    workflow_service
        .workflow_retention_policy_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_retention_policy_update(
    request: WorkflowRetentionPolicyUpdateRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRetentionPolicyUpdateResponse, String> {
    workflow_service
        .workflow_retention_policy_update(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_local_network_status_query(
    request: WorkflowLocalNetworkStatusQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowLocalNetworkStatusQueryResponse, String> {
    workflow_service
        .workflow_local_network_status_query(request)
        .map_err(workflow_error_json)
}

pub async fn workflow_cancel_execution_session_queue_item(
    request: WorkflowExecutionSessionQueueCancelRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionQueueCancelResponse, String> {
    workflow_service
        .workflow_cancel_execution_session_queue_item(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_reprioritize_execution_session_queue_item(
    request: WorkflowExecutionSessionQueueReprioritizeRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionQueueReprioritizeResponse, String> {
    workflow_service
        .workflow_reprioritize_execution_session_queue_item(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_set_execution_session_keep_alive(
    request: WorkflowExecutionSessionKeepAliveRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowExecutionSessionKeepAliveResponse, String> {
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
        .workflow_set_execution_session_keep_alive(request)
        .await
        .map_err(workflow_error_json)
}

#[cfg(test)]
#[path = "headless_workflow_commands_tests.rs"]
mod tests;
