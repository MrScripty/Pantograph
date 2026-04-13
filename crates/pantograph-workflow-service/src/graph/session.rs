use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::workflow::{
    WorkflowSchedulerSnapshotResponse, WorkflowServiceError, WorkflowSessionQueueItem,
    WorkflowSessionQueueItemStatus, WorkflowSessionState, WorkflowSessionSummary,
};

use super::canonicalization::canonicalize_workflow_graph;
use super::connection_intent::{
    commit_connection, connection_candidates, insert_node_and_connect, insert_node_on_edge,
    preview_node_insert_on_edge, rejected_commit_response, rejected_edge_insert_preview_response,
    rejected_insert_on_edge_response, rejected_insert_response,
};
use super::registry::NodeRegistry;
use super::types::{
    ConnectionCandidatesResponse, ConnectionCommitResponse, EdgeInsertionPreviewResponse,
    GraphEdge, GraphNode, InsertNodeConnectionResponse, InsertNodeOnEdgeResponse, Position,
    WorkflowGraph,
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
pub enum WorkflowSessionKind {
    Edit,
    Workflow,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphEditSessionCreateResponse {
    pub session_id: String,
    pub session_kind: WorkflowSessionKind,
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

#[derive(Debug, Clone)]
struct GraphEditSession {
    graph: WorkflowGraph,
    undo_stack: Vec<WorkflowGraph>,
    redo_stack: Vec<WorkflowGraph>,
    active_execution_id: Option<String>,
    run_count: u64,
    last_accessed: Instant,
}

impl GraphEditSession {
    fn new(mut graph: WorkflowGraph) -> Self {
        graph = hydrate_embedding_emit_metadata_flags(graph);
        let now = Instant::now();
        let mut session = Self {
            graph,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            active_execution_id: None,
            run_count: 0,
            last_accessed: now,
        };
        session.canonicalize_graph();
        session
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

    fn canonicalize_graph(&mut self) {
        let graph = std::mem::take(&mut self.graph);
        self.graph = canonicalize_workflow_graph(graph, &NodeRegistry::new());
        self.graph.refresh_derived_graph();
    }

    fn snapshot_response(&mut self, session_id: &str) -> WorkflowGraphEditSessionGraphResponse {
        self.touch();
        self.canonicalize_graph();
        WorkflowGraphEditSessionGraphResponse {
            session_id: session_id.to_string(),
            graph_revision: self.graph.compute_fingerprint(),
            graph: self.graph.clone(),
        }
    }

    fn undo(
        &mut self,
        session_id: &str,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let previous = self
            .undo_stack
            .pop()
            .ok_or_else(|| WorkflowServiceError::InvalidRequest("Nothing to undo".to_string()))?;
        self.redo_stack.push(self.graph.clone());
        self.graph = previous;
        Ok(self.snapshot_response(session_id))
    }

    fn redo(
        &mut self,
        session_id: &str,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let next = self
            .redo_stack
            .pop()
            .ok_or_else(|| WorkflowServiceError::InvalidRequest("Nothing to redo".to_string()))?;
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

    fn session_summary(&self, session_id: &str) -> WorkflowSessionSummary {
        WorkflowSessionSummary {
            session_id: session_id.to_string(),
            workflow_id: session_id.to_string(),
            session_kind: WorkflowSessionKind::Edit,
            usage_profile: None,
            keep_alive: false,
            state: if self.active_execution_id.is_some() {
                WorkflowSessionState::Running
            } else {
                WorkflowSessionState::IdleLoaded
            },
            queued_runs: usize::from(self.active_execution_id.is_some()),
            run_count: self.run_count,
        }
    }

    fn queue_items(&self) -> Vec<WorkflowSessionQueueItem> {
        self.active_execution_id
            .as_ref()
            .map(|execution_id| WorkflowSessionQueueItem {
                queue_id: execution_id.clone(),
                run_id: Some(execution_id.clone()),
                enqueued_at_ms: None,
                dequeued_at_ms: None,
                priority: 0,
                status: WorkflowSessionQueueItemStatus::Running,
            })
            .into_iter()
            .collect()
    }

    fn mark_running(&mut self, session_id: &str) {
        self.touch();
        self.active_execution_id = Some(session_id.to_string());
    }

    fn finish_run(&mut self) {
        self.touch();
        if self.active_execution_id.take().is_some() {
            self.run_count = self.run_count.saturating_add(1);
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
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        WorkflowGraphEditSessionCreateResponse {
            session_id,
            session_kind: WorkflowSessionKind::Edit,
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

    pub async fn get_scheduler_snapshot(
        &self,
        session_id: &str,
    ) -> Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: None,
            session_id: session_id.to_string(),
            session: state.session_summary(session_id),
            items: state.queue_items(),
        })
    }

    pub async fn mark_running(&self, session_id: &str) -> Result<(), WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.mark_running(session_id);
        Ok(())
    }

    pub async fn finish_run(&self, session_id: &str) -> Result<(), WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.finish_run();
        Ok(())
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
        merge_node_data(&mut node.data, request.data);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        Ok(state.snapshot_response(&request.session_id))
    }

    pub async fn update_node_position(
        &self,
        request: WorkflowGraphUpdateNodePositionRequest,
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
        node.position = request.position;
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

    pub async fn remove_node(
        &self,
        request: WorkflowGraphRemoveNodeRequest,
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
        state.graph.nodes.retain(|node| node.id != request.node_id);
        state
            .graph
            .edges
            .retain(|edge| edge.source != request.node_id && edge.target != request.node_id);
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
        state.canonicalize_graph();
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
        state.canonicalize_graph();

        Ok(InsertNodeConnectionResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            inserted_node_id: Some(inserted_node.id),
            graph: Some(state.graph.clone()),
            rejection: None,
        })
    }

    pub async fn preview_node_insert_on_edge(
        &self,
        request: WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    ) -> Result<EdgeInsertionPreviewResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let registry = NodeRegistry::new();

        match preview_node_insert_on_edge(
            &state.graph,
            &registry,
            &request.graph_revision,
            &request.edge_id,
            &request.node_type,
        ) {
            Ok(bridge) => Ok(EdgeInsertionPreviewResponse {
                accepted: true,
                graph_revision: state.graph.compute_fingerprint(),
                bridge: Some(bridge),
                rejection: None,
            }),
            Err(rejection) => Ok(rejected_edge_insert_preview_response(
                &state.graph,
                rejection,
            )),
        }
    }

    pub async fn insert_node_on_edge(
        &self,
        request: WorkflowGraphInsertNodeOnEdgeRequest,
    ) -> Result<InsertNodeOnEdgeResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let registry = NodeRegistry::new();

        let (inserted_node, incoming_edge, outgoing_edge, bridge) = match insert_node_on_edge(
            &state.graph,
            &registry,
            &request.graph_revision,
            &request.edge_id,
            &request.node_type,
            &request.position_hint,
        ) {
            Ok(result) => result,
            Err(rejection) => return Ok(rejected_insert_on_edge_response(&state.graph, rejection)),
        };

        state.push_undo_snapshot();
        state.graph.edges.retain(|edge| edge.id != request.edge_id);
        state.graph.nodes.push(inserted_node.clone());
        state.graph.edges.push(incoming_edge);
        state.graph.edges.push(outgoing_edge);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        state.canonicalize_graph();

        Ok(InsertNodeOnEdgeResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            inserted_node_id: Some(inserted_node.id),
            bridge: Some(bridge),
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

fn merge_node_data(existing: &mut serde_json::Value, patch: serde_json::Value) {
    match (existing, patch) {
        (serde_json::Value::Object(existing_map), serde_json::Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                existing_map.insert(key, value);
            }
        }
        (existing_value, replacement) => {
            *existing_value = replacement;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "text-input".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({
                        "label": "Text Input",
                        "text": "hello",
                        "definition": {
                            "node_type": "text-input"
                        }
                    }),
                },
                GraphNode {
                    id: "text-output".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 120.0, y: 0.0 },
                    data: serde_json::json!({
                        "label": "Text Output"
                    }),
                },
            ],
            edges: vec![GraphEdge {
                id: "text-input-text-text-output-text".to_string(),
                source: "text-input".to_string(),
                source_handle: "text".to_string(),
                target: "text-output".to_string(),
                target_handle: "text".to_string(),
            }],
            derived_graph: None,
        }
    }

    #[tokio::test]
    async fn create_session_returns_backend_owned_edit_kind() {
        let store = GraphSessionStore::new();

        let session = store.create_session(sample_graph()).await;

        assert_eq!(session.session_kind, WorkflowSessionKind::Edit);
        assert!(!session.session_id.is_empty());
        assert!(!session.graph_revision.is_empty());
    }

    #[tokio::test]
    async fn update_node_data_merges_patch_into_existing_data() {
        let store = GraphSessionStore::new();
        let session = store.create_session(sample_graph()).await;

        let response = store
            .update_node_data(WorkflowGraphUpdateNodeDataRequest {
                session_id: session.session_id.clone(),
                node_id: "text-input".to_string(),
                data: serde_json::json!({
                    "text": "updated",
                    "placeholder": "Prompt"
                }),
            })
            .await
            .expect("update node data");

        let node = response
            .graph
            .find_node("text-input")
            .expect("text-input node");
        assert_eq!(node.data["text"], "updated");
        assert_eq!(node.data["placeholder"], "Prompt");
        assert_eq!(node.data["label"], "Text Input");
        assert!(node.data.get("definition").is_some());
    }

    #[tokio::test]
    async fn update_node_position_updates_session_graph() {
        let store = GraphSessionStore::new();
        let session = store.create_session(sample_graph()).await;

        let response = store
            .update_node_position(WorkflowGraphUpdateNodePositionRequest {
                session_id: session.session_id.clone(),
                node_id: "text-output".to_string(),
                position: Position { x: 320.0, y: 48.0 },
            })
            .await
            .expect("update node position");

        let node = response
            .graph
            .find_node("text-output")
            .expect("text-output node");
        assert_eq!(node.position, Position { x: 320.0, y: 48.0 });
    }

    #[tokio::test]
    async fn remove_node_prunes_attached_edges() {
        let store = GraphSessionStore::new();
        let session = store.create_session(sample_graph()).await;

        let response = store
            .remove_node(WorkflowGraphRemoveNodeRequest {
                session_id: session.session_id.clone(),
                node_id: "text-output".to_string(),
            })
            .await
            .expect("remove node");

        assert!(response.graph.find_node("text-output").is_none());
        assert!(response.graph.edges.is_empty());
    }

    #[tokio::test]
    async fn insert_node_on_edge_replaces_original_edge_in_session_graph() {
        let store = GraphSessionStore::new();
        let session = store.create_session(sample_graph()).await;

        let response = store
            .insert_node_on_edge(WorkflowGraphInsertNodeOnEdgeRequest {
                session_id: session.session_id,
                edge_id: "text-input-text-text-output-text".to_string(),
                node_type: "llm-inference".to_string(),
                graph_revision: session.graph_revision,
                position_hint: InsertNodePositionHint {
                    position: Position { x: 80.0, y: 24.0 },
                },
            })
            .await
            .expect("insert node on edge");

        assert!(response.accepted);
        let graph = response.graph.expect("updated graph");
        assert_eq!(graph.edges.len(), 2);
        assert!(
            graph
                .edges
                .iter()
                .all(|edge| edge.id != "text-input-text-text-output-text")
        );
        let inserted_node_id = response.inserted_node_id.expect("inserted node id");
        assert!(graph.find_node(&inserted_node_id).is_some());
    }
}
