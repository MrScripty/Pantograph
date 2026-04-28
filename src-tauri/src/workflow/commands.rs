//! Tauri command entrypoints for workflow operations.
//!
//! This module intentionally stays thin: command wrappers live here while
//! implementation details are decomposed into focused sibling modules.

use std::sync::Arc;

use tauri::{command, AppHandle, State};
use tokio::sync::RwLock;

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use pantograph_workflow_service::{
    FileSystemWorkflowGraphStore, NodeDefinition, PortDataType, WorkflowFile, WorkflowGraph,
    WorkflowGraphMetadata,
};

/// Shared node-engine registry with port options providers.
pub type SharedNodeRegistry = Arc<node_engine::NodeRegistry>;

/// Shared executor extensions (holds PumasApi etc.).
pub type SharedExtensions = Arc<RwLock<node_engine::ExecutorExtensions>>;
/// Shared headless workflow service state (session-aware).
pub type SharedWorkflowService = Arc<pantograph_workflow_service::WorkflowService>;
/// Shared backend-owned stale workflow execution session cleanup worker.
pub type SharedWorkflowExecutionSessionStaleCleanupWorker =
    Arc<pantograph_workflow_service::WorkflowExecutionSessionStaleCleanupWorker>;
/// Shared backend-owned diagnostics projection store.
pub type SharedWorkflowDiagnosticsStore = Arc<super::diagnostics::WorkflowDiagnosticsStore>;
/// Shared filesystem-backed workflow graph store.
pub type SharedWorkflowGraphStore = Arc<FileSystemWorkflowGraphStore>;

#[command]
pub fn validate_workflow_connection(source_type: PortDataType, target_type: PortDataType) -> bool {
    super::workflow_definition_commands::validate_workflow_connection(source_type, target_type)
}

#[command]
pub fn get_node_definitions() -> Vec<NodeDefinition> {
    super::workflow_definition_commands::get_node_definitions()
}

#[command]
pub fn get_node_definitions_by_category() -> std::collections::HashMap<String, Vec<NodeDefinition>>
{
    super::workflow_definition_commands::get_node_definitions_by_category()
}

#[command]
pub fn get_node_definition(node_type: String) -> Option<NodeDefinition> {
    super::workflow_definition_commands::get_node_definition(node_type)
}

#[command]
pub fn save_workflow(
    name: String,
    graph: WorkflowGraph,
    workflow_service: State<'_, SharedWorkflowService>,
    workflow_graph_store: State<'_, SharedWorkflowGraphStore>,
) -> Result<String, String> {
    workflow_service
        .workflow_graph_save(
            workflow_graph_store.inner().as_ref(),
            pantograph_workflow_service::WorkflowGraphSaveRequest { name, graph },
        )
        .map(|response| response.path)
        .map_err(|e| e.to_envelope_json())
}

#[command]
pub fn load_workflow(
    path: String,
    workflow_service: State<'_, SharedWorkflowService>,
    workflow_graph_store: State<'_, SharedWorkflowGraphStore>,
) -> Result<WorkflowFile, String> {
    workflow_service
        .workflow_graph_load(
            workflow_graph_store.inner().as_ref(),
            pantograph_workflow_service::WorkflowGraphLoadRequest { path },
        )
        .map_err(|e| e.to_envelope_json())
}

#[command]
pub fn list_workflows(
    workflow_service: State<'_, SharedWorkflowService>,
    workflow_graph_store: State<'_, SharedWorkflowGraphStore>,
) -> Result<Vec<WorkflowGraphMetadata>, String> {
    workflow_service
        .workflow_graph_list(workflow_graph_store.inner().as_ref())
        .map(|response| response.workflows)
        .map_err(|e| e.to_envelope_json())
}

#[command]
pub async fn workflow_get_capabilities(
    request: pantograph_workflow_service::WorkflowCapabilitiesRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowCapabilitiesResponse, String> {
    super::headless_workflow_commands::workflow_get_capabilities(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_get_io(
    request: pantograph_workflow_service::WorkflowIoRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowIoResponse, String> {
    super::headless_workflow_commands::workflow_get_io(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_preflight(
    request: pantograph_workflow_service::WorkflowPreflightRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowPreflightResponse, String> {
    super::headless_workflow_commands::workflow_preflight(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_create_execution_session(
    request: pantograph_workflow_service::WorkflowExecutionSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionCreateResponse, String> {
    super::headless_workflow_commands::workflow_create_execution_session(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_run_execution_session(
    request: pantograph_workflow_service::WorkflowExecutionSessionRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRunResponse, String> {
    super::headless_workflow_commands::workflow_run_execution_session(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        rag_manager,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_close_execution_session(
    request: pantograph_workflow_service::WorkflowExecutionSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionCloseResponse, String> {
    super::headless_workflow_commands::workflow_close_execution_session(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_get_execution_session_status(
    request: pantograph_workflow_service::WorkflowExecutionSessionStatusRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionStatusResponse, String> {
    super::headless_workflow_commands::workflow_get_execution_session_status(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_list_execution_session_queue(
    request: pantograph_workflow_service::WorkflowExecutionSessionQueueListRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionQueueListResponse, String> {
    super::headless_workflow_commands::workflow_list_execution_session_queue(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_cleanup_stale_execution_sessions(
    request: pantograph_workflow_service::WorkflowExecutionSessionStaleCleanupRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionStaleCleanupResponse, String> {
    super::headless_workflow_commands::workflow_cleanup_stale_execution_sessions(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_get_scheduler_snapshot(
    request: pantograph_workflow_service::WorkflowSchedulerSnapshotRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSchedulerSnapshotResponse, String> {
    super::headless_workflow_commands::workflow_get_scheduler_snapshot(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_scheduler_timeline_query(
    request: pantograph_workflow_service::WorkflowSchedulerTimelineQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSchedulerTimelineQueryResponse, String> {
    super::headless_workflow_commands::workflow_scheduler_timeline_query(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_run_list_query(
    request: pantograph_workflow_service::WorkflowRunListQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRunListQueryResponse, String> {
    super::headless_workflow_commands::workflow_run_list_query(request, workflow_service).await
}

#[command]
pub async fn workflow_run_detail_query(
    request: pantograph_workflow_service::WorkflowRunDetailQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRunDetailQueryResponse, String> {
    super::headless_workflow_commands::workflow_run_detail_query(request, workflow_service).await
}

#[command]
pub async fn workflow_scheduler_estimate_query(
    request: pantograph_workflow_service::WorkflowSchedulerEstimateQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSchedulerEstimateQueryResponse, String> {
    super::headless_workflow_commands::workflow_scheduler_estimate_query(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_run_graph_query(
    request: pantograph_workflow_service::WorkflowRunGraphQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRunGraphQueryResponse, String> {
    super::headless_workflow_commands::workflow_run_graph_query(request, workflow_service).await
}

#[command]
pub async fn workflow_io_artifact_query(
    request: pantograph_workflow_service::WorkflowIoArtifactQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowIoArtifactQueryResponse, String> {
    super::headless_workflow_commands::workflow_io_artifact_query(request, workflow_service).await
}

#[command]
pub async fn workflow_node_status_query(
    request: pantograph_workflow_service::WorkflowNodeStatusQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowNodeStatusQueryResponse, String> {
    super::headless_workflow_commands::workflow_node_status_query(request, workflow_service).await
}

#[command]
pub async fn workflow_projection_rebuild(
    request: pantograph_workflow_service::WorkflowProjectionRebuildRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowProjectionRebuildResponse, String> {
    super::headless_workflow_commands::workflow_projection_rebuild(request, workflow_service).await
}

#[command]
pub async fn workflow_library_usage_query(
    request: pantograph_workflow_service::WorkflowLibraryUsageQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowLibraryUsageQueryResponse, String> {
    super::headless_workflow_commands::workflow_library_usage_query(request, workflow_service).await
}

#[command]
pub async fn workflow_retention_policy_query(
    request: pantograph_workflow_service::WorkflowRetentionPolicyQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRetentionPolicyQueryResponse, String> {
    super::headless_workflow_commands::workflow_retention_policy_query(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_retention_policy_update(
    request: pantograph_workflow_service::WorkflowRetentionPolicyUpdateRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRetentionPolicyUpdateResponse, String> {
    super::headless_workflow_commands::workflow_retention_policy_update(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_retention_cleanup_apply(
    request: pantograph_workflow_service::WorkflowRetentionCleanupRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRetentionCleanupResponse, String> {
    super::headless_workflow_commands::workflow_retention_cleanup_apply(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_local_network_status_query(
    request: pantograph_workflow_service::WorkflowLocalNetworkStatusQueryRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowLocalNetworkStatusQueryResponse, String> {
    super::headless_workflow_commands::workflow_local_network_status_query(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_cancel_execution_session_queue_item(
    request: pantograph_workflow_service::WorkflowExecutionSessionQueueCancelRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionQueueCancelResponse, String> {
    super::headless_workflow_commands::workflow_cancel_execution_session_queue_item(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_admin_cancel_queue_item(
    request: pantograph_workflow_service::WorkflowAdminQueueCancelRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowAdminQueueCancelResponse, String> {
    super::headless_workflow_commands::workflow_admin_cancel_queue_item(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_reprioritize_execution_session_queue_item(
    request: pantograph_workflow_service::WorkflowExecutionSessionQueueReprioritizeRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionQueueReprioritizeResponse, String>
{
    super::headless_workflow_commands::workflow_reprioritize_execution_session_queue_item(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_admin_reprioritize_queue_item(
    request: pantograph_workflow_service::WorkflowAdminQueueReprioritizeRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowAdminQueueReprioritizeResponse, String> {
    super::headless_workflow_commands::workflow_admin_reprioritize_queue_item(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_push_execution_session_queue_item_to_front(
    request: pantograph_workflow_service::WorkflowExecutionSessionQueuePushFrontRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionQueuePushFrontResponse, String> {
    super::headless_workflow_commands::workflow_push_execution_session_queue_item_to_front(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_admin_push_queue_item_to_front(
    request: pantograph_workflow_service::WorkflowAdminQueuePushFrontRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowAdminQueuePushFrontResponse, String> {
    super::headless_workflow_commands::workflow_admin_push_queue_item_to_front(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_set_execution_session_keep_alive(
    request: pantograph_workflow_service::WorkflowExecutionSessionKeepAliveRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowExecutionSessionKeepAliveResponse, String> {
    super::headless_workflow_commands::workflow_set_execution_session_keep_alive(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
    )
    .await
}

#[command]
pub async fn query_port_options(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    node_type: String,
    port_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<node_engine::PortOptionsResult, String> {
    super::workflow_port_query_commands::query_port_options(
        registry,
        extensions,
        workflow_service,
        node_type,
        port_id,
        search,
        limit,
        offset,
    )
    .await
}

#[command]
pub fn get_queryable_ports(registry: State<'_, SharedNodeRegistry>) -> Vec<(String, String)> {
    super::workflow_port_query_commands::get_queryable_ports(registry)
}

#[command]
pub async fn list_models_needing_review(
    extensions: State<'_, SharedExtensions>,
    filter: Option<pumas_library::model_library::ModelReviewFilter>,
) -> Result<Vec<pumas_library::model_library::ModelReviewItem>, String> {
    super::workflow_model_review_commands::list_models_needing_review(extensions, filter).await
}

#[command]
pub async fn submit_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    patch: serde_json::Value,
    reviewer: String,
    reason: Option<String>,
) -> Result<pumas_library::model_library::SubmitModelReviewResult, String> {
    super::workflow_model_review_commands::submit_model_review(
        extensions, model_id, patch, reviewer, reason,
    )
    .await
}

#[command]
pub async fn reset_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    reviewer: String,
    reason: Option<String>,
) -> Result<bool, String> {
    super::workflow_model_review_commands::reset_model_review(
        extensions, model_id, reviewer, reason,
    )
    .await
}

#[command]
pub async fn get_effective_model_metadata(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
) -> Result<Option<pumas_library::models::ModelMetadata>, String> {
    super::workflow_model_review_commands::get_effective_model_metadata(extensions, model_id).await
}

#[command]
pub async fn hydrate_puma_lib_node(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    model_path: Option<String>,
    model_id: Option<String>,
    selected_binding_ids: Option<Vec<String>>,
    resolve_requirements: Option<bool>,
) -> Result<super::puma_lib_commands::PumaLibNodeHydrationResponse, String> {
    super::puma_lib_commands::hydrate_puma_lib_node(
        registry,
        extensions,
        resolver,
        model_path,
        model_id,
        selected_binding_ids,
        resolve_requirements,
    )
    .await
}

#[command]
pub async fn delete_pumas_model_with_audit(
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    model_id: String,
) -> Result<super::puma_lib_commands::PumaModelDeleteAuditResponse, String> {
    super::puma_lib_commands::delete_pumas_model_with_audit(extensions, workflow_service, model_id)
        .await
}

#[command]
pub async fn search_hf_models_with_audit(
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    query: String,
    kind: Option<String>,
    limit: Option<usize>,
    hydrate_limit: Option<usize>,
) -> Result<super::puma_lib_commands::PumaHfModelSearchAuditResponse, String> {
    super::puma_lib_commands::search_hf_models_with_audit(
        extensions,
        workflow_service,
        query,
        kind,
        limit,
        hydrate_limit,
    )
    .await
}

#[command]
pub async fn start_hf_download_with_audit(
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    request: pumas_library::model_library::DownloadRequest,
) -> Result<super::puma_lib_commands::PumaHfDownloadStartAuditResponse, String> {
    super::puma_lib_commands::start_hf_download_with_audit(extensions, workflow_service, request)
        .await
}

#[command]
pub async fn run_dependency_environment_action(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    request: super::dependency_environment_commands::DependencyEnvironmentActionRequest,
) -> Result<super::dependency_environment_commands::DependencyEnvironmentActionResponse, String> {
    super::dependency_environment_commands::run_dependency_environment_action(resolver, request)
        .await
}

#[command]
pub async fn resolve_model_dependency_requirements(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyRequirements, String> {
    super::model_dependency_commands::resolve_model_dependency_requirements(resolver, request).await
}

#[command]
pub async fn check_model_dependencies(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyStatus, String> {
    super::model_dependency_commands::check_model_dependencies(resolver, request).await
}

#[command]
pub async fn install_model_dependencies(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyInstallResult, String> {
    super::model_dependency_commands::install_model_dependencies(resolver, request).await
}

#[command]
pub async fn get_model_dependency_status(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    request: node_engine::ModelDependencyRequest,
) -> Result<node_engine::ModelDependencyStatus, String> {
    super::model_dependency_commands::get_model_dependency_status(resolver, request).await
}

#[command]
pub async fn audit_dependency_pin_compliance(
    extensions: State<'_, SharedExtensions>,
) -> Result<pumas_library::model_library::DependencyPinAuditReport, String> {
    super::model_dependency_commands::audit_dependency_pin_compliance(extensions).await
}
