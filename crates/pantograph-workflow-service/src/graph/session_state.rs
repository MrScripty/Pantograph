use std::time::Duration;

use crate::workflow::{
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionSummary, WorkflowServiceError,
};

use super::canonicalization::canonicalize_workflow_graph;
use super::memory_impact::graph_memory_impact_from_graph_change;
use super::registry::NodeRegistry;
use super::session_contract::{
    build_workflow_execution_session_state_view, resolve_workflow_execution_session_memory_impact,
    WorkflowGraphEditSessionGraphResponse, WorkflowGraphSessionStateProjection,
    WorkflowGraphSessionStateView,
};
use super::session_event::{dirty_tasks_for_full_snapshot, graph_modified_event};
use super::session_graph::hydrate_embedding_emit_metadata_flags;
use super::session_runtime::GraphEditSessionRuntime;
use super::session_types::UndoRedoState;
use super::types::WorkflowGraph;

const DEFAULT_MAX_UNDO_SNAPSHOTS: usize = 64;

#[derive(Debug, Clone)]
pub(super) struct GraphEditSession {
    pub(super) graph: WorkflowGraph,
    workflow_id: Option<String>,
    undo_stack: Vec<WorkflowGraph>,
    redo_stack: Vec<WorkflowGraph>,
    last_memory_impact: Option<node_engine::GraphMemoryImpactSummary>,
    runtime: GraphEditSessionRuntime,
}

impl GraphEditSession {
    pub(super) fn new(mut graph: WorkflowGraph, workflow_id: Option<String>) -> Self {
        graph = hydrate_embedding_emit_metadata_flags(graph);
        let mut session = Self {
            graph,
            workflow_id: workflow_id.and_then(normalize_optional_session_text),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_memory_impact: None,
            runtime: GraphEditSessionRuntime::new(),
        };
        session.canonicalize_graph();
        session
    }

    pub(super) fn touch(&mut self) {
        self.runtime.touch();
    }

    pub(super) fn is_stale(&self, timeout: Duration) -> bool {
        self.runtime.is_stale(timeout)
    }

    pub(super) fn push_undo_snapshot(&mut self) {
        if self.undo_stack.len() >= DEFAULT_MAX_UNDO_SNAPSHOTS {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.graph.clone());
        self.redo_stack.clear();
    }

    pub(super) fn snapshot_response(
        &mut self,
        session_id: &str,
    ) -> WorkflowGraphEditSessionGraphResponse {
        self.touch();
        self.canonicalize_graph();
        build_graph_session_response_with_projection(
            session_id,
            &self.graph,
            None,
            phase6_memory_impact_projection(self.last_memory_impact.clone()),
        )
    }

    pub(super) fn snapshot_response_with_state(
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
            resolve_workflow_execution_session_memory_impact(
                workflow_event.as_ref(),
                Some(projection),
            )
        });
        build_graph_session_response_with_projection(
            session_id,
            &self.graph,
            workflow_event,
            projection,
        )
    }

    pub(super) fn undo(
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

    pub(super) fn redo(
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

    pub(super) fn undo_redo_state(&self) -> UndoRedoState {
        UndoRedoState {
            can_undo: !self.undo_stack.is_empty(),
            can_redo: !self.redo_stack.is_empty(),
            undo_count: self.undo_stack.len(),
        }
    }

    pub(super) fn session_summary(&self, session_id: &str) -> WorkflowExecutionSessionSummary {
        self.runtime
            .session_summary(session_id, self.workflow_id.as_deref())
    }

    pub(super) fn queue_items(&self) -> Vec<WorkflowExecutionSessionQueueItem> {
        self.runtime.queue_items()
    }

    pub(super) fn mark_running(&mut self, session_id: &str) {
        self.runtime.mark_running(session_id);
    }

    pub(super) fn finish_run(&mut self) {
        self.runtime.finish_run();
    }

    pub(super) fn mutation_session_state_view(
        &mut self,
        session_id: &str,
        workflow_event: Option<&node_engine::WorkflowEvent>,
        projection: Option<WorkflowGraphSessionStateProjection>,
    ) -> WorkflowGraphSessionStateView {
        let projection =
            resolved_phase6_memory_impact_projection(workflow_event, projection.as_ref());
        self.last_memory_impact = projection.as_ref().and_then(|projection| {
            resolve_workflow_execution_session_memory_impact(workflow_event, Some(projection))
        });
        build_workflow_execution_session_state_view(
            session_id,
            &self.graph.compute_fingerprint(),
            workflow_event,
            projection.as_ref(),
        )
    }

    pub(super) fn canonicalize_graph(&mut self) {
        let graph = std::mem::take(&mut self.graph);
        self.graph = canonicalize_workflow_graph(graph, &NodeRegistry::new());
        self.graph.refresh_derived_graph();
    }
}

pub(super) fn build_graph_session_response_with_projection(
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

pub(super) fn phase6_memory_impact_projection(
    memory_impact: Option<node_engine::GraphMemoryImpactSummary>,
) -> Option<WorkflowGraphSessionStateProjection> {
    memory_impact.map(|memory_impact| WorkflowGraphSessionStateProjection {
        memory_impact: Some(memory_impact),
        ..WorkflowGraphSessionStateProjection::default()
    })
}

pub(super) fn resolved_phase6_memory_impact_projection(
    workflow_event: Option<&node_engine::WorkflowEvent>,
    projection: Option<&WorkflowGraphSessionStateProjection>,
) -> Option<WorkflowGraphSessionStateProjection> {
    let resolved_memory_impact =
        resolve_workflow_execution_session_memory_impact(workflow_event, projection);
    match projection.cloned() {
        Some(mut projection) => {
            projection.memory_impact = resolved_memory_impact;
            Some(projection)
        }
        None => phase6_memory_impact_projection(resolved_memory_impact),
    }
}

fn normalize_optional_session_text(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}
