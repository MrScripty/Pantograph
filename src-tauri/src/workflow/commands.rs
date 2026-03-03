//! Tauri command entrypoints for workflow operations.
//!
//! This module intentionally stays thin: command wrappers live here while
//! implementation details are decomposed into focused sibling modules.

use std::sync::Arc;

use tauri::{command, ipc::Channel, AppHandle, State};
use tokio::sync::RwLock;

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedAppConfig, SharedGateway};

use super::events::WorkflowEvent;
use super::execution_manager::{SharedExecutionManager, UndoRedoState};
use super::types::{
    GraphEdge, GraphNode, NodeDefinition, PortDataType, WorkflowFile, WorkflowGraph,
    WorkflowMetadata,
};

/// Shared node-engine registry with port options providers.
pub type SharedNodeRegistry = Arc<node_engine::NodeRegistry>;

/// Shared executor extensions (holds PumasApi etc.).
pub type SharedExtensions = Arc<RwLock<node_engine::ExecutorExtensions>>;

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
pub fn save_workflow(name: String, graph: WorkflowGraph) -> Result<String, String> {
    super::workflow_persistence_commands::save_workflow(name, graph)
}

#[command]
pub fn load_workflow(path: String) -> Result<WorkflowFile, String> {
    super::workflow_persistence_commands::load_workflow(path)
}

#[command]
pub fn list_workflows() -> Result<Vec<WorkflowMetadata>, String> {
    super::workflow_persistence_commands::list_workflows()
}

#[command]
pub async fn workflow_run(
    request: pantograph_workflow_service::WorkflowRunRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<pantograph_workflow_service::WorkflowRunResponse, String> {
    super::headless_workflow_commands::workflow_run(request, gateway, extensions).await
}

#[command]
pub async fn workflow_get_capabilities(
    request: pantograph_workflow_service::WorkflowCapabilitiesRequest,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
) -> Result<pantograph_workflow_service::WorkflowCapabilitiesResponse, String> {
    super::headless_workflow_commands::workflow_get_capabilities(
        request, gateway, extensions,
    )
    .await
}

#[command]
pub async fn execute_workflow_v2(
    app: AppHandle,
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
    extensions: State<'_, SharedExtensions>,
    channel: Channel<WorkflowEvent>,
) -> Result<String, String> {
    super::workflow_execution_commands::execute_workflow_v2(
        app,
        graph,
        gateway,
        config,
        rag_manager,
        execution_manager,
        extensions,
        channel,
    )
    .await
}

#[command]
pub async fn get_undo_redo_state(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<UndoRedoState, String> {
    super::workflow_execution_commands::get_undo_redo_state(execution_id, execution_manager).await
}

#[command]
pub async fn undo_workflow(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::undo_workflow(execution_id, execution_manager).await
}

#[command]
pub async fn redo_workflow(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::redo_workflow(execution_id, execution_manager).await
}

#[command]
pub async fn update_node_data(
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    super::workflow_execution_commands::update_node_data(
        execution_id,
        node_id,
        data,
        execution_manager,
    )
    .await
}

#[command]
pub async fn add_node_to_execution(
    execution_id: String,
    node: GraphNode,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    super::workflow_execution_commands::add_node_to_execution(execution_id, node, execution_manager)
        .await
}

#[command]
pub async fn add_edge_to_execution(
    execution_id: String,
    edge: GraphEdge,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::add_edge_to_execution(execution_id, edge, execution_manager)
        .await
}

#[command]
pub async fn remove_edge_from_execution(
    execution_id: String,
    edge_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::remove_edge_from_execution(
        execution_id,
        edge_id,
        execution_manager,
    )
    .await
}

#[command]
pub async fn get_execution_graph(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    super::workflow_execution_commands::get_execution_graph(execution_id, execution_manager).await
}

#[command]
pub async fn create_workflow_session(
    graph: WorkflowGraph,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<String, String> {
    super::workflow_execution_commands::create_workflow_session(graph, execution_manager).await
}

#[command]
pub async fn run_workflow_session(
    app: AppHandle,
    session_id: String,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
    extensions: State<'_, SharedExtensions>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    super::workflow_execution_commands::run_workflow_session(
        app,
        session_id,
        gateway,
        config,
        rag_manager,
        execution_manager,
        extensions,
        channel,
    )
    .await
}

#[command]
pub async fn remove_execution(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    super::workflow_execution_commands::remove_execution(execution_id, execution_manager).await
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
