use super::types::{GraphEdge, GraphNode, PortMapping, Position, WorkflowGraph};
use super::{ConnectionAnchor, InsertNodePositionHint};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UndoRedoState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionCreateRequest {
    pub graph: WorkflowGraph,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionSessionKind {
    Edit,
    Workflow,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionCreateResponse {
    pub session_id: String,
    pub session_kind: WorkflowExecutionSessionKind,
    pub graph_revision: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionCloseRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionCloseResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionGraphRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphUndoRedoStateRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphUndoRedoStateResponse {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphUpdateNodeDataRequest {
    pub session_id: String,
    pub node_id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphUpdateNodePositionRequest {
    pub session_id: String,
    pub node_id: String,
    pub position: Position,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphRemoveNodeRequest {
    pub session_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphAddNodeRequest {
    pub session_id: String,
    pub node: GraphNode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphAddEdgeRequest {
    pub session_id: String,
    pub edge: GraphEdge,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphRemoveEdgeRequest {
    pub session_id: String,
    pub edge_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphCreateGroupRequest {
    pub session_id: String,
    pub name: String,
    pub selected_node_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphUngroupRequest {
    pub session_id: String,
    pub group_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphUpdateGroupPortsRequest {
    pub session_id: String,
    pub group_id: String,
    pub exposed_inputs: Vec<PortMapping>,
    pub exposed_outputs: Vec<PortMapping>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphGetConnectionCandidatesRequest {
    pub session_id: String,
    pub source_anchor: ConnectionAnchor,
    #[serde(default)]
    pub graph_revision: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphConnectRequest {
    pub session_id: String,
    pub source_anchor: ConnectionAnchor,
    pub target_anchor: ConnectionAnchor,
    pub graph_revision: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphInsertNodeAndConnectRequest {
    pub session_id: String,
    pub source_anchor: ConnectionAnchor,
    pub node_type: String,
    pub graph_revision: String,
    pub position_hint: InsertNodePositionHint,
    #[serde(default)]
    pub preferred_input_port_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphPreviewNodeInsertOnEdgeRequest {
    pub session_id: String,
    pub edge_id: String,
    pub node_type: String,
    pub graph_revision: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphInsertNodeOnEdgeRequest {
    pub session_id: String,
    pub edge_id: String,
    pub node_type: String,
    pub graph_revision: String,
    pub position_hint: InsertNodePositionHint,
}
