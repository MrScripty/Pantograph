use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::workflow::{
    scheduler_snapshot_trace_execution_id, WorkflowSchedulerSnapshotResponse, WorkflowServiceError,
    WorkflowSessionQueueItem, WorkflowSessionSummary,
};

use super::canonicalization::canonicalize_workflow_graph;
use super::group_mutation::{
    create_node_group_graph, ungroup_node_graph, update_group_ports_graph,
};
use super::memory_impact::graph_memory_impact_from_graph_change;
use super::registry::NodeRegistry;
use super::session_contract::{
    build_workflow_session_state_view, resolve_workflow_session_memory_impact,
    WorkflowGraphEditSessionGraphResponse, WorkflowGraphSessionStateProjection,
    WorkflowGraphSessionStateView,
};
use super::session_event::{
    dirty_tasks_for_full_snapshot, dirty_tasks_from_seed_nodes, graph_modified_event,
};
use super::session_graph::{
    hydrate_embedding_emit_metadata_flags, merge_node_data, sync_embedding_emit_metadata_flags,
};
use super::session_runtime::GraphEditSessionRuntime;
use super::session_types::{
    UndoRedoState, WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest,
    WorkflowGraphCreateGroupRequest, WorkflowGraphEditSessionCloseResponse,
    WorkflowGraphEditSessionCreateResponse, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphUndoRedoStateResponse, WorkflowGraphUngroupRequest,
    WorkflowGraphUpdateGroupPortsRequest, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest, WorkflowSessionKind,
};
use super::types::WorkflowGraph;
#[cfg(test)]
use super::{
    session_types::{WorkflowGraphConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest},
    types::GraphEdge,
};
const DEFAULT_MAX_UNDO_SNAPSHOTS: usize = 64;

#[path = "session_connection_api.rs"]
mod session_connection_api;

#[derive(Debug, Clone)]
struct GraphEditSession {
    graph: WorkflowGraph,
    undo_stack: Vec<WorkflowGraph>,
    redo_stack: Vec<WorkflowGraph>,
    last_memory_impact: Option<node_engine::GraphMemoryImpactSummary>,
    runtime: GraphEditSessionRuntime,
}

impl GraphEditSession {
    fn new(mut graph: WorkflowGraph) -> Self {
        graph = hydrate_embedding_emit_metadata_flags(graph);
        let mut session = Self {
            graph,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_memory_impact: None,
            runtime: GraphEditSessionRuntime::new(),
        };
        session.canonicalize_graph();
        session
    }

    fn touch(&mut self) {
        self.runtime.touch();
    }

    fn is_stale(&self, timeout: Duration) -> bool {
        self.runtime.is_stale(timeout)
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
        build_graph_session_response_with_projection(
            session_id,
            &self.graph,
            None,
            phase6_memory_impact_projection(self.last_memory_impact.clone()),
        )
    }

    fn snapshot_response_with_state(
        &mut self,
        session_id: &str,
        workflow_event: Option<node_engine::WorkflowEvent>,
        projection: Option<WorkflowGraphSessionStateProjection>,
    ) -> WorkflowGraphEditSessionGraphResponse {
        self.touch();
        self.canonicalize_graph();
        let projection =
            resolved_phase6_memory_impact_projection(workflow_event.as_ref(), projection.as_ref());
        self.last_memory_impact = projection.as_ref().and_then(|projection| {
            resolve_workflow_session_memory_impact(workflow_event.as_ref(), Some(projection))
        });
        build_graph_session_response_with_projection(
            session_id,
            &self.graph,
            workflow_event,
            projection,
        )
    }

    fn undo(
        &mut self,
        session_id: &str,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let before_graph = self.graph.clone();
        let previous = self
            .undo_stack
            .pop()
            .ok_or_else(|| WorkflowServiceError::InvalidRequest("Nothing to undo".to_string()))?;
        self.redo_stack.push(self.graph.clone());
        self.graph = previous;
        let dirty_tasks = dirty_tasks_for_full_snapshot(&self.graph);
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &self.graph,
            &dirty_tasks_for_full_snapshot(&self.graph),
        );
        let workflow_event =
            graph_modified_event(session_id, session_id, dirty_tasks, memory_impact.clone());
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(self.snapshot_response_with_state(session_id, Some(workflow_event), projection))
    }

    fn redo(
        &mut self,
        session_id: &str,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let before_graph = self.graph.clone();
        let next = self
            .redo_stack
            .pop()
            .ok_or_else(|| WorkflowServiceError::InvalidRequest("Nothing to redo".to_string()))?;
        self.undo_stack.push(self.graph.clone());
        self.graph = next;
        let dirty_tasks = dirty_tasks_for_full_snapshot(&self.graph);
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &self.graph,
            &dirty_tasks_for_full_snapshot(&self.graph),
        );
        let workflow_event =
            graph_modified_event(session_id, session_id, dirty_tasks, memory_impact.clone());
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(self.snapshot_response_with_state(session_id, Some(workflow_event), projection))
    }

    fn undo_redo_state(&self) -> UndoRedoState {
        UndoRedoState {
            can_undo: !self.undo_stack.is_empty(),
            can_redo: !self.redo_stack.is_empty(),
            undo_count: self.undo_stack.len(),
        }
    }

    fn session_summary(&self, session_id: &str) -> WorkflowSessionSummary {
        self.runtime.session_summary(session_id)
    }

    fn queue_items(&self) -> Vec<WorkflowSessionQueueItem> {
        self.runtime.queue_items()
    }

    fn mark_running(&mut self, session_id: &str) {
        self.runtime.mark_running(session_id);
    }

    fn finish_run(&mut self) {
        self.runtime.finish_run();
    }

    fn mutation_session_state_view(
        &mut self,
        session_id: &str,
        workflow_event: Option<&node_engine::WorkflowEvent>,
        projection: Option<WorkflowGraphSessionStateProjection>,
    ) -> WorkflowGraphSessionStateView {
        let projection =
            resolved_phase6_memory_impact_projection(workflow_event, projection.as_ref());
        self.last_memory_impact = projection.as_ref().and_then(|projection| {
            resolve_workflow_session_memory_impact(workflow_event, Some(projection))
        });
        build_workflow_session_state_view(
            session_id,
            &self.graph.compute_fingerprint(),
            workflow_event,
            projection.as_ref(),
        )
    }
}

type GraphSessionHandle = Arc<Mutex<GraphEditSession>>;

fn build_graph_session_response_with_projection(
    session_id: &str,
    graph: &WorkflowGraph,
    workflow_event: Option<node_engine::WorkflowEvent>,
    projection: Option<WorkflowGraphSessionStateProjection>,
) -> WorkflowGraphEditSessionGraphResponse {
    super::session_contract::build_graph_session_response_with_state(
        session_id,
        graph,
        workflow_event,
        projection.as_ref(),
    )
}

fn phase6_memory_impact_projection(
    memory_impact: Option<node_engine::GraphMemoryImpactSummary>,
) -> Option<WorkflowGraphSessionStateProjection> {
    memory_impact.map(|memory_impact| WorkflowGraphSessionStateProjection {
        memory_impact: Some(memory_impact),
        ..WorkflowGraphSessionStateProjection::default()
    })
}

fn resolved_phase6_memory_impact_projection(
    workflow_event: Option<&node_engine::WorkflowEvent>,
    projection: Option<&WorkflowGraphSessionStateProjection>,
) -> Option<WorkflowGraphSessionStateProjection> {
    let resolved_memory_impact = resolve_workflow_session_memory_impact(workflow_event, projection);
    match projection.cloned() {
        Some(mut projection) => {
            projection.memory_impact = resolved_memory_impact;
            Some(projection)
        }
        None => phase6_memory_impact_projection(resolved_memory_impact),
    }
}

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
        let items = state.queue_items();
        Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: None,
            session_id: session_id.to_string(),
            trace_execution_id: scheduler_snapshot_trace_execution_id(&items),
            session: state.session_summary(session_id),
            items,
            diagnostics: None,
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
        let before_graph = state.graph.clone();
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
        let dirty_tasks =
            dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&request.node_id));
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&request.node_id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
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
        let workflow_event =
            graph_modified_event(&request.session_id, &request.session_id, Vec::new(), None);
        Ok(state.snapshot_response_with_state(&request.session_id, Some(workflow_event), None))
    }

    pub async fn add_node(
        &self,
        request: WorkflowGraphAddNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        state.push_undo_snapshot();
        let node_id = request.node.id.clone();
        state.graph.nodes.push(request.node);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let dirty_tasks = dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&node_id));
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&node_id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
    }

    pub async fn remove_node(
        &self,
        request: WorkflowGraphRemoveNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        if state.graph.find_node(&request.node_id).is_none() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "node '{}' was not found",
                request.node_id
            )));
        }
        state.push_undo_snapshot();
        let dirty_tasks =
            dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&request.node_id));
        state.graph.nodes.retain(|node| node.id != request.node_id);
        state
            .graph
            .edges
            .retain(|edge| edge.source != request.node_id && edge.target != request.node_id);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&before_graph, std::slice::from_ref(&request.node_id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
    }

    pub async fn add_edge(
        &self,
        request: WorkflowGraphAddEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        state.push_undo_snapshot();
        let target_node_id = request.edge.target.clone();
        state.graph.edges.push(request.edge);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let dirty_tasks =
            dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&target_node_id));
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&target_node_id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
    }

    pub async fn remove_edge(
        &self,
        request: WorkflowGraphRemoveEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        state.push_undo_snapshot();
        let target_node_id = state
            .graph
            .edges
            .iter()
            .find(|edge| edge.id == request.edge_id)
            .map(|edge| edge.target.clone());
        state.graph.edges.retain(|edge| edge.id != request.edge_id);
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let dirty_tasks = target_node_id
            .as_ref()
            .map(|node_id| dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(node_id)))
            .unwrap_or_default();
        let memory_impact = target_node_id.as_ref().and_then(|node_id| {
            graph_memory_impact_from_graph_change(
                &before_graph,
                &state.graph,
                &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(node_id)),
            )
        });
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
    }

    pub async fn create_group(
        &self,
        request: WorkflowGraphCreateGroupRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        let next_graph =
            create_node_group_graph(&state.graph, request.name, &request.selected_node_ids)?;
        state.push_undo_snapshot();
        state.graph = next_graph;
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let dirty_tasks = dirty_tasks_for_full_snapshot(&state.graph);
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_for_full_snapshot(&state.graph),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
    }

    pub async fn ungroup(
        &self,
        request: WorkflowGraphUngroupRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        let next_graph = ungroup_node_graph(&state.graph, &request.group_id)?;
        state.push_undo_snapshot();
        state.graph = next_graph;
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let dirty_tasks = dirty_tasks_for_full_snapshot(&state.graph);
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_for_full_snapshot(&state.graph),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
    }

    pub async fn update_group_ports(
        &self,
        request: WorkflowGraphUpdateGroupPortsRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        let next_graph = update_group_ports_graph(
            &state.graph,
            &request.group_id,
            request.exposed_inputs,
            request.exposed_outputs,
        )?;
        state.push_undo_snapshot();
        state.graph = next_graph;
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let dirty_tasks =
            dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&request.group_id));
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&request.group_id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let projection = phase6_memory_impact_projection(memory_impact);
        Ok(state.snapshot_response_with_state(
            &request.session_id,
            Some(workflow_event),
            projection,
        ))
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

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
