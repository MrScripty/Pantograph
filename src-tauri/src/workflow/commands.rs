//! Tauri command entrypoints for workflow operations.
//!
//! This module intentionally stays thin: command wrappers live here while
//! implementation details are decomposed into focused sibling modules.

use std::sync::Arc;

use tauri::{command, ipc::Channel, AppHandle, State};
use tokio::sync::RwLock;

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedAppConfig, SharedGateway, SharedRuntimeRegistry};

use super::events::WorkflowEvent;
use pantograph_workflow_service::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse,
    EdgeInsertionPreviewResponse, FileSystemWorkflowGraphStore, GraphEdge, GraphNode,
    InsertNodeConnectionResponse, InsertNodeOnEdgeResponse, InsertNodePositionHint, NodeDefinition,
    PortDataType, Position, UndoRedoState, WorkflowFile, WorkflowGraph, WorkflowGraphMetadata,
};

/// Shared node-engine registry with port options providers.
pub type SharedNodeRegistry = Arc<node_engine::NodeRegistry>;

/// Shared executor extensions (holds PumasApi etc.).
pub type SharedExtensions = Arc<RwLock<node_engine::ExecutorExtensions>>;
/// Shared headless workflow service state (session-aware).
pub type SharedWorkflowService = Arc<pantograph_workflow_service::WorkflowService>;
/// Shared backend-owned stale workflow-session cleanup worker.
pub type SharedWorkflowSessionStaleCleanupWorker =
    Arc<pantograph_workflow_service::WorkflowSessionStaleCleanupWorker>;
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
pub async fn workflow_run(
    request: pantograph_workflow_service::WorkflowRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRunResponse, String> {
    super::headless_workflow_commands::workflow_run(
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
pub async fn workflow_create_session(
    request: pantograph_workflow_service::WorkflowSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionCreateResponse, String> {
    super::headless_workflow_commands::workflow_create_session(
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
pub async fn workflow_run_session(
    request: pantograph_workflow_service::WorkflowSessionRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowRunResponse, String> {
    super::headless_workflow_commands::workflow_run_session(
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
pub async fn workflow_close_session(
    request: pantograph_workflow_service::WorkflowSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionCloseResponse, String> {
    super::headless_workflow_commands::workflow_close_session(
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
pub async fn workflow_get_session_status(
    request: pantograph_workflow_service::WorkflowSessionStatusRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionStatusResponse, String> {
    super::headless_workflow_commands::workflow_get_session_status(request, workflow_service).await
}

#[command]
pub async fn workflow_list_session_queue(
    request: pantograph_workflow_service::WorkflowSessionQueueListRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionQueueListResponse, String> {
    super::headless_workflow_commands::workflow_list_session_queue(request, workflow_service).await
}

#[command]
pub async fn workflow_cleanup_stale_sessions(
    request: pantograph_workflow_service::WorkflowSessionStaleCleanupRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionStaleCleanupResponse, String> {
    super::headless_workflow_commands::workflow_cleanup_stale_sessions(request, workflow_service)
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
pub async fn workflow_cancel_session_queue_item(
    request: pantograph_workflow_service::WorkflowSessionQueueCancelRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionQueueCancelResponse, String> {
    super::headless_workflow_commands::workflow_cancel_session_queue_item(request, workflow_service)
        .await
}

#[command]
pub async fn workflow_reprioritize_session_queue_item(
    request: pantograph_workflow_service::WorkflowSessionQueueReprioritizeRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionQueueReprioritizeResponse, String> {
    super::headless_workflow_commands::workflow_reprioritize_session_queue_item(
        request,
        workflow_service,
    )
    .await
}

#[command]
pub async fn workflow_set_session_keep_alive(
    request: pantograph_workflow_service::WorkflowSessionKeepAliveRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowSessionKeepAliveResponse, String> {
    super::headless_workflow_commands::workflow_set_session_keep_alive(
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
pub async fn workflow_get_diagnostics_snapshot(
    request: super::diagnostics::WorkflowDiagnosticsSnapshotRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<super::diagnostics::WorkflowDiagnosticsProjection, String> {
    super::headless_workflow_commands::workflow_get_diagnostics_snapshot(
        request,
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
        diagnostics_store,
    )
    .await
}

#[command]
pub async fn workflow_get_trace_snapshot(
    request: pantograph_workflow_service::WorkflowTraceSnapshotRequest,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<pantograph_workflow_service::WorkflowTraceSnapshotResponse, String> {
    super::headless_workflow_commands::workflow_get_trace_snapshot(request, diagnostics_store).await
}

#[command]
pub async fn workflow_clear_diagnostics_history(
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<super::diagnostics::WorkflowDiagnosticsProjection, String> {
    super::headless_workflow_commands::workflow_clear_diagnostics_history(diagnostics_store).await
}

#[command]
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
    super::workflow_execution_commands::execute_workflow_v2(
        app,
        graph,
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

#[command]
pub async fn get_undo_redo_state(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<UndoRedoState, String> {
    super::workflow_execution_commands::get_undo_redo_state(execution_id, workflow_service).await
}

#[command]
pub async fn undo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::undo_workflow(execution_id, workflow_service).await
}

#[command]
pub async fn redo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::redo_workflow(execution_id, workflow_service).await
}

#[command]
pub async fn update_node_data(
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::update_node_data(
        execution_id,
        node_id,
        data,
        workflow_service,
    )
    .await
}

#[command]
pub async fn update_node_position_in_execution(
    execution_id: String,
    node_id: String,
    position: Position,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::update_node_position_in_execution(
        execution_id,
        node_id,
        position,
        workflow_service,
    )
    .await
}

#[command]
pub async fn add_node_to_execution(
    execution_id: String,
    node: GraphNode,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::add_node_to_execution(execution_id, node, workflow_service)
        .await
}

#[command]
pub async fn remove_node_from_execution(
    execution_id: String,
    node_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::remove_node_from_execution(
        execution_id,
        node_id,
        workflow_service,
    )
    .await
}

#[command]
pub async fn add_edge_to_execution(
    execution_id: String,
    edge: GraphEdge,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::add_edge_to_execution(execution_id, edge, workflow_service)
        .await
}

#[command]
pub async fn get_connection_candidates(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    graph_revision: Option<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<ConnectionCandidatesResponse, String> {
    super::workflow_execution_commands::get_connection_candidates(
        execution_id,
        source_anchor,
        graph_revision,
        workflow_service,
    )
    .await
}

#[command]
pub async fn connect_anchors_in_execution(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    target_anchor: ConnectionAnchor,
    graph_revision: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<ConnectionCommitResponse, String> {
    super::workflow_execution_commands::connect_anchors_in_execution(
        execution_id,
        source_anchor,
        target_anchor,
        graph_revision,
        workflow_service,
    )
    .await
}

#[command]
pub async fn insert_node_and_connect_in_execution(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    node_type: String,
    graph_revision: String,
    position_hint: InsertNodePositionHint,
    preferred_input_port_id: Option<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<InsertNodeConnectionResponse, String> {
    super::workflow_execution_commands::insert_node_and_connect_in_execution(
        execution_id,
        source_anchor,
        node_type,
        graph_revision,
        position_hint,
        preferred_input_port_id,
        workflow_service,
    )
    .await
}

#[command]
pub async fn preview_node_insert_on_edge_in_execution(
    execution_id: String,
    edge_id: String,
    node_type: String,
    graph_revision: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<EdgeInsertionPreviewResponse, String> {
    super::workflow_execution_commands::preview_node_insert_on_edge_in_execution(
        execution_id,
        edge_id,
        node_type,
        graph_revision,
        workflow_service,
    )
    .await
}

#[command]
pub async fn insert_node_on_edge_in_execution(
    execution_id: String,
    edge_id: String,
    node_type: String,
    graph_revision: String,
    position_hint: InsertNodePositionHint,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<InsertNodeOnEdgeResponse, String> {
    super::workflow_execution_commands::insert_node_on_edge_in_execution(
        execution_id,
        edge_id,
        node_type,
        graph_revision,
        position_hint,
        workflow_service,
    )
    .await
}

#[command]
pub async fn remove_edge_from_execution(
    execution_id: String,
    edge_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::remove_edge_from_execution(
        execution_id,
        edge_id,
        workflow_service,
    )
    .await
}

#[command]
pub async fn get_execution_graph(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::get_execution_graph(execution_id, workflow_service).await
}

#[command]
pub async fn create_workflow_session(
    graph: WorkflowGraph,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowGraphEditSessionCreateResponse, String> {
    super::workflow_execution_commands::create_workflow_session(graph, workflow_service).await
}

#[command]
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
    super::workflow_execution_commands::run_workflow_session(
        app,
        session_id,
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

#[command]
pub async fn remove_execution(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<(), String> {
    super::workflow_execution_commands::remove_execution(execution_id, workflow_service).await
}

#[command]
pub async fn query_port_options(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    node_type: String,
    port_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<node_engine::PortOptionsResult, String> {
    super::workflow_port_query_commands::query_port_options(
        registry, extensions, node_type, port_id, search, limit, offset,
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
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
    dependency_override_patches: Option<Vec<node_engine::DependencyOverridePatchV1>>,
) -> Result<node_engine::ModelDependencyRequirements, String> {
    super::model_dependency_commands::resolve_model_dependency_requirements(
        resolver,
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
        dependency_override_patches,
    )
    .await
}

#[command]
pub async fn check_model_dependencies(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
    dependency_override_patches: Option<Vec<node_engine::DependencyOverridePatchV1>>,
) -> Result<node_engine::ModelDependencyStatus, String> {
    super::model_dependency_commands::check_model_dependencies(
        resolver,
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
        dependency_override_patches,
    )
    .await
}

#[command]
pub async fn install_model_dependencies(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
    dependency_override_patches: Option<Vec<node_engine::DependencyOverridePatchV1>>,
) -> Result<node_engine::ModelDependencyInstallResult, String> {
    super::model_dependency_commands::install_model_dependencies(
        resolver,
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
        dependency_override_patches,
    )
    .await
}

#[command]
pub async fn get_model_dependency_status(
    resolver: State<'_, super::model_dependencies::SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
    dependency_override_patches: Option<Vec<node_engine::DependencyOverridePatchV1>>,
) -> Result<node_engine::ModelDependencyStatus, String> {
    super::model_dependency_commands::get_model_dependency_status(
        resolver,
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
        dependency_override_patches,
    )
    .await
}

#[command]
pub async fn audit_dependency_pin_compliance(
    extensions: State<'_, SharedExtensions>,
) -> Result<pumas_library::model_library::DependencyPinAuditReport, String> {
    super::model_dependency_commands::audit_dependency_pin_compliance(extensions).await
}
