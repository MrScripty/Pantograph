use crate::graph::{
    ConnectionCandidatesResponse, ConnectionCommitResponse, EdgeInsertionPreviewResponse,
    InsertNodeConnectionResponse, InsertNodeOnEdgeResponse, WorkflowFile, WorkflowGraph,
    WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest,
    WorkflowGraphCreateGroupRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCloseResponse, WorkflowGraphEditSessionCreateRequest,
    WorkflowGraphEditSessionCreateResponse, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphEditSessionGraphResponse, WorkflowGraphGetConnectionCandidatesRequest,
    WorkflowGraphInsertNodeAndConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest,
    WorkflowGraphListResponse, WorkflowGraphLoadRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest, WorkflowGraphSaveResponse,
    WorkflowGraphStore, WorkflowGraphUndoRedoStateRequest, WorkflowGraphUndoRedoStateResponse,
    WorkflowGraphUngroupRequest, WorkflowGraphUpdateGroupPortsRequest,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest,
};

use super::{WorkflowService, WorkflowServiceError};

impl WorkflowService {
    pub async fn workflow_graph_create_edit_session(
        &self,
        request: WorkflowGraphEditSessionCreateRequest,
    ) -> Result<WorkflowGraphEditSessionCreateResponse, WorkflowServiceError> {
        Ok(self.graph_session_store.create_session(request.graph).await)
    }

    pub async fn workflow_graph_close_edit_session(
        &self,
        request: WorkflowGraphEditSessionCloseRequest,
    ) -> Result<WorkflowGraphEditSessionCloseResponse, WorkflowServiceError> {
        self.graph_session_store
            .close_session(&request.session_id)
            .await
    }

    pub async fn workflow_graph_get_edit_session_graph(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store
            .get_session_graph(&request.session_id)
            .await
    }

    pub async fn workflow_graph_get_undo_redo_state(
        &self,
        request: WorkflowGraphUndoRedoStateRequest,
    ) -> Result<WorkflowGraphUndoRedoStateResponse, WorkflowServiceError> {
        self.graph_session_store
            .get_undo_redo_state(&request.session_id)
            .await
    }

    pub async fn workflow_graph_update_node_data(
        &self,
        request: WorkflowGraphUpdateNodeDataRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.update_node_data(request).await
    }

    pub async fn workflow_graph_update_node_position(
        &self,
        request: WorkflowGraphUpdateNodePositionRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.update_node_position(request).await
    }

    pub async fn workflow_graph_add_node(
        &self,
        request: WorkflowGraphAddNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.add_node(request).await
    }

    pub async fn workflow_graph_remove_node(
        &self,
        request: WorkflowGraphRemoveNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.remove_node(request).await
    }

    pub async fn workflow_graph_add_edge(
        &self,
        request: WorkflowGraphAddEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.add_edge(request).await
    }

    pub async fn workflow_graph_remove_edge(
        &self,
        request: WorkflowGraphRemoveEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.remove_edge(request).await
    }

    pub async fn workflow_graph_create_group(
        &self,
        request: WorkflowGraphCreateGroupRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.create_group(request).await
    }

    pub async fn workflow_graph_ungroup(
        &self,
        request: WorkflowGraphUngroupRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.ungroup(request).await
    }

    pub async fn workflow_graph_update_group_ports(
        &self,
        request: WorkflowGraphUpdateGroupPortsRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.update_group_ports(request).await
    }

    pub async fn workflow_graph_undo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.undo(request).await
    }

    pub async fn workflow_graph_redo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.graph_session_store.redo(request).await
    }

    pub async fn workflow_graph_get_connection_candidates(
        &self,
        request: WorkflowGraphGetConnectionCandidatesRequest,
    ) -> Result<ConnectionCandidatesResponse, WorkflowServiceError> {
        self.graph_session_store
            .get_connection_candidates(request)
            .await
    }

    pub async fn workflow_graph_connect(
        &self,
        request: WorkflowGraphConnectRequest,
    ) -> Result<ConnectionCommitResponse, WorkflowServiceError> {
        self.graph_session_store.connect(request).await
    }

    pub async fn workflow_graph_insert_node_and_connect(
        &self,
        request: WorkflowGraphInsertNodeAndConnectRequest,
    ) -> Result<InsertNodeConnectionResponse, WorkflowServiceError> {
        self.graph_session_store
            .insert_node_and_connect(request)
            .await
    }

    pub async fn workflow_graph_preview_node_insert_on_edge(
        &self,
        request: WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    ) -> Result<EdgeInsertionPreviewResponse, WorkflowServiceError> {
        self.graph_session_store
            .preview_node_insert_on_edge(request)
            .await
    }

    pub async fn workflow_graph_insert_node_on_edge(
        &self,
        request: WorkflowGraphInsertNodeOnEdgeRequest,
    ) -> Result<InsertNodeOnEdgeResponse, WorkflowServiceError> {
        self.graph_session_store.insert_node_on_edge(request).await
    }

    pub fn workflow_graph_save<S: WorkflowGraphStore>(
        &self,
        store: &S,
        request: WorkflowGraphSaveRequest,
    ) -> Result<WorkflowGraphSaveResponse, WorkflowServiceError> {
        let path = store.save_workflow(request.name, request.graph)?;
        Ok(WorkflowGraphSaveResponse { path })
    }

    pub fn workflow_graph_load<S: WorkflowGraphStore>(
        &self,
        store: &S,
        request: WorkflowGraphLoadRequest,
    ) -> Result<WorkflowFile, WorkflowServiceError> {
        store.load_workflow(request.path)
    }

    pub fn workflow_graph_list<S: WorkflowGraphStore>(
        &self,
        store: &S,
    ) -> Result<WorkflowGraphListResponse, WorkflowServiceError> {
        let workflows = store.list_workflows()?;
        Ok(WorkflowGraphListResponse { workflows })
    }

    pub async fn workflow_graph_get_runtime_snapshot(
        &self,
        session_id: &str,
    ) -> Result<WorkflowGraph, WorkflowServiceError> {
        let response = self
            .graph_session_store
            .get_session_graph(session_id)
            .await?;
        Ok(response.graph)
    }

    pub async fn workflow_graph_mark_edit_session_running(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.graph_session_store.mark_running(session_id).await
    }

    pub async fn workflow_graph_mark_edit_session_finished(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.graph_session_store.finish_run(session_id).await
    }
}
