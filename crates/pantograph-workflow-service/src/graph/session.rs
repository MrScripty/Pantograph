use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::workflow::WorkflowServiceError;

use super::connection_intent::{
    commit_connection, connection_candidates, insert_node_and_connect, rejected_commit_response,
    rejected_insert_response,
};
use super::registry::NodeRegistry;
use super::types::{
    ConnectionCandidatesResponse, ConnectionCommitResponse, GraphEdge, GraphNode,
    InsertNodeConnectionResponse, WorkflowGraph,
};
use super::{ConnectionAnchor, InsertNodePositionHint};

const DEFAULT_MAX_UNDO_SNAPSHOTS: usize = 64;

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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionCreateResponse {
    pub session_id: String,
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionGraphResponse {
    pub session_id: String,
    pub graph_revision: String,
    pub graph: WorkflowGraph,
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

#[derive(Debug, Clone)]
struct GraphEditSession {
    graph: WorkflowGraph,
    undo_stack: Vec<WorkflowGraph>,
    redo_stack: Vec<WorkflowGraph>,
    last_accessed: Instant,
}

impl GraphEditSession {
    fn new(mut graph: WorkflowGraph) -> Self {
        graph = hydrate_embedding_emit_metadata_flags(graph);
        graph.refresh_derived_graph();
        let now = Instant::now();
        Self {
            graph,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_accessed: now,
        }
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }

    fn is_stale(&self, timeout: Duration) -> bool {
        self.last_accessed.elapsed() > timeout
    }

    fn push_undo_snapshot(&mut self) {
        if self.undo_stack.len() >= DEFAULT_MAX_UNDO_SNAPSHOTS {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.graph.clone());
        self.redo_stack.clear();
    }

    fn snapshot_response(&mut self, session_id: &str) -> WorkflowGraphEditSessionGraphResponse {
        self.touch();
        self.graph.refresh_derived_graph();
        WorkflowGraphEditSessionGraphResponse {
            session_id: session_id.to_string(),
            graph_revision: self.graph.compute_fingerprint(),
            graph: self.graph.clone(),
        }
    }

    fn undo(&mut self, session_id: &str) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let previous = self.undo_stack.pop().ok_or_else(|| {
            WorkflowServiceError::InvalidRequest("Nothing to undo".to_string())
        })?;
        self.redo_stack.push(self.graph.clone());
        self.graph = previous;
        Ok(self.snapshot_response(session_id))
    }

    fn redo(&mut self, session_id: &str) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let next = self.redo_stack.pop().ok_or_else(|| {
            WorkflowServiceError::InvalidRequest("Nothing to redo".to_string())
        })?;
        self.undo_stack.push(self.graph.clone());
        self.graph = next;
        Ok(self.snapshot_response(session_id))
    }

    fn undo_redo_state(&self) -> UndoRedoState {
        UndoRedoState {
            can_undo: !self.undo_stack.is_empty(),
            can_redo: !self.redo_stack.is_empty(),
            undo_count: self.undo_stack.len(),
        }
    }
}

type GraphSessionHandle = Arc<Mutex<GraphEditSession>>;

#[derive(Debug)]
pub struct GraphSessionStore {
    sessions: RwLock<HashMap<String, GraphSessionHandle>>,
    stale_timeout: Duration,
}

impl Default for GraphSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphSessionStore {
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(5 * 60))
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            stale_timeout: timeout,
        }
    }

    pub async fn create_session(
        &self,
        graph: WorkflowGraph,
    ) -> WorkflowGraphEditSessionCreateResponse {
        let session_id = Uuid::new_v4().to_string();
        let session = Arc::new(Mutex::new(GraphEditSession::new(graph)));
        let graph_revision = {
            let state = session.lock().await;
            state.graph.compute_fingerprint()
        };
        self.sessions.write().await.insert(session_id.clone(), session);
        WorkflowGraphEditSessionCreateResponse {
            session_id,
            graph_revision,
        }
    }

    pub async fn close_session(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraphEditSessionCloseResponse, WorkflowServiceError> {
        let removed = self.sessions.write().await.remove(session_id);
        if removed.is_none() {
            return Err(WorkflowServiceError::SessionNotFound(format!(
                "edit session '{}' not found",
                session_id
            )));
        }
        Ok(WorkflowGraphEditSessionCloseResponse { ok: true })
    }

    async fn get_session_handle(
        &self,
        session_id: &str,
    ) -> Result<GraphSessionHandle, WorkflowServiceError> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                WorkflowServiceError::SessionNotFound(format!(
                    "edit session '{}' not found",
                    session_id
                ))
            })
    }

    pub async fn get_session_graph(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        Ok(state.snapshot_response(session_id))
    }

    pub async fn get_undo_redo_state(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraphUndoRedoStateResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let undo = state.undo_redo_state();
        Ok(WorkflowGraphUndoRedoStateResponse {
            can_undo: undo.can_undo,
            can_redo: undo.can_redo,
            undo_count: undo.undo_count,
        })
    }

    pub async fn update_node_data(
        &self,
        request: WorkflowGraphUpdateNodeDataRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        if state.graph.find_node(&request.node_id).is_none() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "node '{}' was not found",
                request.node_id
            )));
        }
        state.push_undo_snapshot();
        let node = state.graph.find_node_mut(&request.node_id).ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "node '{}' was not found",
                request.node_id
            ))
        })?;
        node.data = request.data;
        sync_embedding_emit_metadata_flags(&mut state.graph);
        Ok(state.snapshot_response(&request.session_id))
    }

    pub async fn add_node(
        &self,
        request: WorkflowGraphAddNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.push_undo_snapshot();
        state.graph.nodes.push(request.node);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        Ok(state.snapshot_response(&request.session_id))
    }

    pub async fn add_edge(
        &self,
        request: WorkflowGraphAddEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.push_undo_snapshot();
        state.graph.edges.push(request.edge);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        Ok(state.snapshot_response(&request.session_id))
    }

    pub async fn remove_edge(
        &self,
        request: WorkflowGraphRemoveEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.push_undo_snapshot();
        state.graph.edges.retain(|edge| edge.id != request.edge_id);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        Ok(state.snapshot_response(&request.session_id))
    }

    pub async fn undo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.undo(&request.session_id)
    }

    pub async fn redo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.redo(&request.session_id)
    }

    pub async fn get_connection_candidates(
        &self,
        request: WorkflowGraphGetConnectionCandidatesRequest,
    ) -> Result<ConnectionCandidatesResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let registry = NodeRegistry::new();
        connection_candidates(
            &state.graph,
            &registry,
            request.source_anchor,
            request.graph_revision.as_deref(),
        )
        .map_err(|rejection| WorkflowServiceError::InvalidRequest(rejection.message))
    }

    pub async fn connect(
        &self,
        request: WorkflowGraphConnectRequest,
    ) -> Result<ConnectionCommitResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let registry = NodeRegistry::new();
        if let Err(rejection) = commit_connection(
            &state.graph,
            &registry,
            &request.graph_revision,
            &request.source_anchor,
            &request.target_anchor,
        ) {
            return Ok(rejected_commit_response(&state.graph, rejection));
        }

        state.push_undo_snapshot();
        state.graph.edges.push(GraphEdge {
            id: format!(
                "{}-{}-{}-{}",
                request.source_anchor.node_id,
                request.source_anchor.port_id,
                request.target_anchor.node_id,
                request.target_anchor.port_id
            ),
            source: request.source_anchor.node_id,
            source_handle: request.source_anchor.port_id,
            target: request.target_anchor.node_id,
            target_handle: request.target_anchor.port_id,
        });
        sync_embedding_emit_metadata_flags(&mut state.graph);
        state.graph.refresh_derived_graph();
        Ok(ConnectionCommitResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            graph: Some(state.graph.clone()),
            rejection: None,
        })
    }

    pub async fn insert_node_and_connect(
        &self,
        request: WorkflowGraphInsertNodeAndConnectRequest,
    ) -> Result<InsertNodeConnectionResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let registry = NodeRegistry::new();
        let (inserted_node, inserted_edge) = match insert_node_and_connect(
            &state.graph,
            &registry,
            &request.graph_revision,
            &request.source_anchor,
            &request.node_type,
            &request.position_hint,
            request.preferred_input_port_id.as_deref(),
        ) {
            Ok(result) => result,
            Err(rejection) => return Ok(rejected_insert_response(&state.graph, rejection)),
        };

        state.push_undo_snapshot();
        state.graph.nodes.push(inserted_node.clone());
        state.graph.edges.push(inserted_edge);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        state.graph.refresh_derived_graph();

        Ok(InsertNodeConnectionResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            inserted_node_id: Some(inserted_node.id),
            graph: Some(state.graph.clone()),
            rejection: None,
        })
    }

    pub async fn cleanup_stale(&self) -> usize {
        let handles: Vec<(String, GraphSessionHandle)> = {
            let sessions = self.sessions.read().await;
            sessions
                .iter()
                .map(|(id, handle)| (id.clone(), handle.clone()))
                .collect()
        };

        let mut stale_ids = Vec::new();
        for (id, handle) in handles {
            if handle.lock().await.is_stale(self.stale_timeout) {
                stale_ids.push(id);
            }
        }

        let count = stale_ids.len();
        let mut sessions = self.sessions.write().await;
        for id in stale_ids {
            sessions.remove(&id);
        }
        count
    }
}

fn hydrate_embedding_emit_metadata_flags(mut graph: WorkflowGraph) -> WorkflowGraph {
    let counts = graph.effective_consumer_count_map();
    for node in &mut graph.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        match node.data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                node.data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
    }
    graph
}

fn sync_embedding_emit_metadata_flags(graph: &mut WorkflowGraph) {
    let counts = graph.effective_consumer_count_map();
    for node in &mut graph.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        match node.data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                node.data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
    }
}

pub fn convert_graph_to_node_engine(graph: &WorkflowGraph) -> node_engine::WorkflowGraph {
    let mut ne_graph =
        node_engine::WorkflowGraph::new(Uuid::new_v4().to_string(), "Workflow".to_string());

    for node in &graph.nodes {
        let mut data = node.data.clone();
        if let serde_json::Value::Object(ref mut map) = data {
            map.insert("node_type".to_string(), serde_json::json!(node.node_type));
        }
        ne_graph.nodes.push(node_engine::GraphNode {
            id: node.id.clone(),
            node_type: node.node_type.clone(),
            data,
            position: (node.position.x, node.position.y),
        });
    }

    for edge in &graph.edges {
        ne_graph.edges.push(node_engine::GraphEdge {
            id: edge.id.clone(),
            source: edge.source.clone(),
            source_handle: edge.source_handle.clone(),
            target: edge.target.clone(),
            target_handle: edge.target_handle.clone(),
        });
    }

    ne_graph
}
