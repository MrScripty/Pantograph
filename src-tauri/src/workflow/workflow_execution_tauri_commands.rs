use tauri::{command, ipc::Channel, AppHandle, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedAppConfig, SharedGateway, SharedRuntimeRegistry};

use super::commands::{SharedExtensions, SharedWorkflowDiagnosticsStore, SharedWorkflowService};
use super::events::WorkflowEvent;
use pantograph_workflow_service::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse,
    EdgeInsertionPreviewResponse, GraphEdge, GraphNode, InsertNodeConnectionResponse,
    InsertNodeOnEdgeResponse, InsertNodePositionHint, Position, UndoRedoState, WorkflowGraph,
    WorkflowGraphEditSessionGraphResponse,
};

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
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    super::workflow_execution_commands::undo_workflow(execution_id, workflow_service).await
}

#[command]
pub async fn redo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    super::workflow_execution_commands::redo_workflow(execution_id, workflow_service).await
}

#[command]
pub async fn update_node_data(
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
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
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
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
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    super::workflow_execution_commands::add_node_to_execution(execution_id, node, workflow_service)
        .await
}

#[command]
pub async fn remove_node_from_execution(
    execution_id: String,
    node_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
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
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
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
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
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
