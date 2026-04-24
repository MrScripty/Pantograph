use crate::workflow::WorkflowServiceError;

use super::super::connection_intent::{
    commit_connection, connection_candidates, insert_node_and_connect, insert_node_on_edge,
    preview_node_insert_on_edge, rejected_commit_response, rejected_edge_insert_preview_response,
    rejected_insert_on_edge_response, rejected_insert_response,
};
use super::super::memory_impact::graph_memory_impact_from_graph_change;
use super::super::registry::NodeRegistry;
use super::super::session_event::{dirty_tasks_from_seed_nodes, graph_modified_event};
use super::super::session_graph::sync_embedding_emit_metadata_flags;
use super::super::session_types::{
    WorkflowGraphConnectRequest, WorkflowGraphGetConnectionCandidatesRequest,
    WorkflowGraphInsertNodeAndConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest,
};
use super::super::types::{
    ConnectionCandidatesResponse, ConnectionCommitResponse, EdgeInsertionPreviewResponse,
    GraphEdge, InsertNodeConnectionResponse, InsertNodeOnEdgeResponse,
};

use super::{phase6_memory_impact_projection, GraphSessionStore};

impl GraphSessionStore {
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
        let before_graph = state.graph.clone();
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
        let target_node_id = request.target_anchor.node_id.clone();
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
        let memory_impact = phase6_memory_impact_projection(memory_impact);
        let workflow_execution_session_state = state.mutation_session_state_view(
            &request.session_id,
            Some(&workflow_event),
            memory_impact,
        );
        Ok(ConnectionCommitResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            graph: Some(state.graph.clone()),
            workflow_event: Some(workflow_event),
            workflow_execution_session_state: Some(workflow_execution_session_state),
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
        let before_graph = state.graph.clone();
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
        let dirty_tasks =
            dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&inserted_node.id));
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&inserted_node.id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let memory_impact = phase6_memory_impact_projection(memory_impact);
        let workflow_execution_session_state = state.mutation_session_state_view(
            &request.session_id,
            Some(&workflow_event),
            memory_impact,
        );

        Ok(InsertNodeConnectionResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            inserted_node_id: Some(inserted_node.id),
            graph: Some(state.graph.clone()),
            workflow_event: Some(workflow_event),
            workflow_execution_session_state: Some(workflow_execution_session_state),
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
        let before_graph = state.graph.clone();
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
        let dirty_tasks =
            dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&inserted_node.id));
        let memory_impact = graph_memory_impact_from_graph_change(
            &before_graph,
            &state.graph,
            &dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(&inserted_node.id)),
        );
        let workflow_event = graph_modified_event(
            &request.session_id,
            &request.session_id,
            dirty_tasks,
            memory_impact.clone(),
        );
        let memory_impact = phase6_memory_impact_projection(memory_impact);
        let workflow_execution_session_state = state.mutation_session_state_view(
            &request.session_id,
            Some(&workflow_event),
            memory_impact,
        );

        Ok(InsertNodeOnEdgeResponse {
            accepted: true,
            graph_revision: state.graph.compute_fingerprint(),
            inserted_node_id: Some(inserted_node.id),
            bridge: Some(bridge),
            graph: Some(state.graph.clone()),
            workflow_event: Some(workflow_event),
            workflow_execution_session_state: Some(workflow_execution_session_state),
            rejection: None,
        })
    }
}
