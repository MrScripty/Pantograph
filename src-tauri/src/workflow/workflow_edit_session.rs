use tauri::State;

use pantograph_workflow_service::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse,
    EdgeInsertionPreviewResponse, GraphEdge, GraphNode, InsertNodeConnectionResponse,
    InsertNodeOnEdgeResponse, InsertNodePositionHint, PortMapping, Position, UndoRedoState,
    WorkflowGraph, WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest,
    WorkflowGraphConnectRequest, WorkflowGraphCreateGroupRequest,
    WorkflowGraphEditSessionCloseRequest, WorkflowGraphEditSessionCreateRequest,
    WorkflowGraphEditSessionGraphRequest, WorkflowGraphEditSessionGraphResponse,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUngroupRequest,
    WorkflowGraphUpdateGroupPortsRequest, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest,
};

use super::commands::SharedWorkflowService;

pub async fn get_undo_redo_state(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<UndoRedoState, String> {
    workflow_service
        .workflow_graph_get_undo_redo_state(WorkflowGraphUndoRedoStateRequest {
            session_id: execution_id,
        })
        .await
        .map(|state| UndoRedoState {
            can_undo: state.can_undo,
            can_redo: state.can_redo,
            undo_count: state.undo_count,
        })
        .map_err(|e| e.to_envelope_json())
}

pub async fn undo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_undo(WorkflowGraphEditSessionGraphRequest {
            session_id: execution_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn redo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_redo(WorkflowGraphEditSessionGraphRequest {
            session_id: execution_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn update_node_data(
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: execution_id,
            node_id,
            data,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn update_node_position_in_execution(
    execution_id: String,
    node_id: String,
    position: Position,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: execution_id,
            node_id,
            position,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn add_node_to_execution(
    execution_id: String,
    node: GraphNode,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_add_node(WorkflowGraphAddNodeRequest {
            session_id: execution_id,
            node,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn remove_node_from_execution(
    execution_id: String,
    node_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_remove_node(WorkflowGraphRemoveNodeRequest {
            session_id: execution_id,
            node_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn add_edge_to_execution(
    execution_id: String,
    edge: GraphEdge,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_add_edge(WorkflowGraphAddEdgeRequest {
            session_id: execution_id,
            edge,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn get_connection_candidates(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    graph_revision: Option<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<ConnectionCandidatesResponse, String> {
    workflow_service
        .workflow_graph_get_connection_candidates(WorkflowGraphGetConnectionCandidatesRequest {
            session_id: execution_id,
            source_anchor,
            graph_revision,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn connect_anchors_in_execution(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    target_anchor: ConnectionAnchor,
    graph_revision: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<ConnectionCommitResponse, String> {
    workflow_service
        .workflow_graph_connect(WorkflowGraphConnectRequest {
            session_id: execution_id,
            source_anchor,
            target_anchor,
            graph_revision,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn insert_node_and_connect_in_execution(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    node_type: String,
    graph_revision: String,
    position_hint: InsertNodePositionHint,
    preferred_input_port_id: Option<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<InsertNodeConnectionResponse, String> {
    workflow_service
        .workflow_graph_insert_node_and_connect(WorkflowGraphInsertNodeAndConnectRequest {
            session_id: execution_id,
            source_anchor,
            node_type,
            graph_revision,
            position_hint,
            preferred_input_port_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn preview_node_insert_on_edge_in_execution(
    execution_id: String,
    edge_id: String,
    node_type: String,
    graph_revision: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<EdgeInsertionPreviewResponse, String> {
    workflow_service
        .workflow_graph_preview_node_insert_on_edge(WorkflowGraphPreviewNodeInsertOnEdgeRequest {
            session_id: execution_id,
            edge_id,
            node_type,
            graph_revision,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn insert_node_on_edge_in_execution(
    execution_id: String,
    edge_id: String,
    node_type: String,
    graph_revision: String,
    position_hint: InsertNodePositionHint,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<InsertNodeOnEdgeResponse, String> {
    workflow_service
        .workflow_graph_insert_node_on_edge(WorkflowGraphInsertNodeOnEdgeRequest {
            session_id: execution_id,
            edge_id,
            node_type,
            graph_revision,
            position_hint,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn remove_edge_from_execution(
    execution_id: String,
    edge_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_remove_edge(WorkflowGraphRemoveEdgeRequest {
            session_id: execution_id,
            edge_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn create_group_in_execution(
    execution_id: String,
    name: String,
    selected_node_ids: Vec<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_create_group(WorkflowGraphCreateGroupRequest {
            session_id: execution_id,
            name,
            selected_node_ids,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn ungroup_in_execution(
    execution_id: String,
    group_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_ungroup(WorkflowGraphUngroupRequest {
            session_id: execution_id,
            group_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn update_group_ports_in_execution(
    execution_id: String,
    group_id: String,
    exposed_inputs: Vec<PortMapping>,
    exposed_outputs: Vec<PortMapping>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraphEditSessionGraphResponse, String> {
    workflow_service
        .workflow_graph_update_group_ports(WorkflowGraphUpdateGroupPortsRequest {
            session_id: execution_id,
            group_id,
            exposed_inputs,
            exposed_outputs,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn get_execution_graph(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
            session_id: execution_id,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn create_workflow_execution_session(
    graph: WorkflowGraph,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowGraphEditSessionCreateResponse, String> {
    workflow_service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest { graph })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn remove_execution(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<(), String> {
    workflow_service
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: execution_id,
        })
        .await
        .map(|_| ())
        .map_err(|e| e.to_envelope_json())
}
