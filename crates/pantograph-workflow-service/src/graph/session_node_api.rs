use crate::workflow::WorkflowServiceError;

use std::collections::HashSet;

use super::super::memory_impact::graph_memory_impact_from_graph_change;
use super::super::session_contract::WorkflowGraphEditSessionGraphResponse;
use super::super::session_event::{dirty_tasks_from_seed_nodes, graph_modified_event};
use super::super::session_graph::{merge_node_data, sync_embedding_emit_metadata_flags};
use super::super::session_types::{
    WorkflowGraphAddNodeRequest, WorkflowGraphDeleteSelectionRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest,
};
use super::{
    GraphSessionStore, dirty_tasks_from_seed_nodes_unique, phase6_memory_impact_projection,
};

impl GraphSessionStore {
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

    pub async fn delete_selection(
        &self,
        request: WorkflowGraphDeleteSelectionRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let handle = self.get_session_handle(&request.session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        let before_graph = state.graph.clone();
        let selected_node_ids = request.node_ids;
        let selected_edge_ids = request.edge_ids;
        if selected_node_ids.is_empty() && selected_edge_ids.is_empty() {
            return Ok(state.snapshot_response(&request.session_id));
        }
        let node_ids = selected_node_ids.iter().cloned().collect::<HashSet<_>>();
        let edge_ids = selected_edge_ids.iter().cloned().collect::<HashSet<_>>();

        for node_id in &node_ids {
            if state.graph.find_node(node_id).is_none() {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "node '{}' was not found",
                    node_id
                )));
            }
        }

        let edge_target_node_ids = state
            .graph
            .edges
            .iter()
            .filter(|edge| {
                edge_ids.contains(&edge.id)
                    || node_ids.contains(&edge.source)
                    || node_ids.contains(&edge.target)
            })
            .filter(|edge| !node_ids.contains(&edge.target))
            .map(|edge| edge.target.clone())
            .collect::<Vec<_>>();
        let mut dirty_tasks = dirty_tasks_from_seed_nodes_unique(&before_graph, &selected_node_ids);
        super::append_unique_strings(
            &mut dirty_tasks,
            dirty_tasks_from_seed_nodes_unique(&before_graph, &edge_target_node_ids),
        );

        state.push_undo_snapshot();
        state
            .graph
            .nodes
            .retain(|node| !node_ids.contains(&node.id));
        state.graph.edges.retain(|edge| {
            !edge_ids.contains(&edge.id)
                && !node_ids.contains(&edge.source)
                && !node_ids.contains(&edge.target)
        });
        sync_embedding_emit_metadata_flags(&mut state.graph);
        let memory_impact = if dirty_tasks.is_empty() {
            None
        } else {
            graph_memory_impact_from_graph_change(&before_graph, &state.graph, &dirty_tasks)
        };
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
}
