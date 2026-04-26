use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::workflow::{
    scheduler_snapshot_workflow_run_id, WorkflowSchedulerSnapshotResponse, WorkflowServiceError,
};

use super::group_mutation::{
    create_node_group_graph, ungroup_node_graph, update_group_ports_graph,
};
use super::memory_impact::graph_memory_impact_from_graph_change;
use super::session_contract::WorkflowGraphEditSessionGraphResponse;
use super::session_event::{
    dirty_tasks_for_full_snapshot, dirty_tasks_from_seed_nodes, graph_modified_event,
};
use super::session_graph::sync_embedding_emit_metadata_flags;
use super::session_state::{phase6_memory_impact_projection, GraphEditSession};
use super::session_types::{
    WorkflowExecutionSessionKind, WorkflowGraphAddEdgeRequest, WorkflowGraphCreateGroupRequest,
    WorkflowGraphEditSessionCloseResponse, WorkflowGraphEditSessionCreateResponse,
    WorkflowGraphEditSessionGraphRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphUndoRedoStateResponse, WorkflowGraphUngroupRequest,
    WorkflowGraphUpdateGroupPortsRequest,
};
use super::types::WorkflowGraph;
#[cfg(test)]
use super::{
    session_types::{WorkflowGraphConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest},
    types::GraphEdge,
};
#[path = "session_connection_api.rs"]
mod session_connection_api;
#[path = "session_node_api.rs"]
mod session_node_api;

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
        workflow_id: Option<String>,
    ) -> WorkflowGraphEditSessionCreateResponse {
        let session_id = Uuid::new_v4().to_string();
        let session = Arc::new(Mutex::new(GraphEditSession::new(graph, workflow_id)));
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
            session_kind: WorkflowExecutionSessionKind::Edit,
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
            workflow_run_id: scheduler_snapshot_workflow_run_id(&items),
            session: state.session_summary(session_id),
            items,
            diagnostics: None,
        })
    }

    pub async fn mark_running(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.mark_running(workflow_run_id);
        Ok(())
    }

    pub async fn finish_run(&self, session_id: &str) -> Result<(), WorkflowServiceError> {
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.finish_run();
        Ok(())
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
