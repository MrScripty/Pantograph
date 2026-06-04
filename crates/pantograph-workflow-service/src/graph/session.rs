use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use pantograph_dependency_environment_service::{
    DependencyEnvironmentService, NotImplementedDependencyEnvironmentProvider,
    SharedDependencyEnvironmentProvider, SharedDependencyEnvironmentService,
};
use pantograph_dependency_planning::{DependencyBindingId, DependencyOverridePatchV1};
use pantograph_inference_interface_contracts::{
    DependencyEnvironmentActionIntent, DependencyEnvironmentActionIntentResult,
    DependencyEnvironmentActionIntentStatus, InferenceDiagnosticCode, InferenceDiagnosticSeverity,
    InferenceInterfaceDiagnostic, InferencePortId, ValidatedDependencyEnvironmentActionIntent,
    WorkflowGraphRevision, WorkflowGraphSessionId,
};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::workflow::{
    scheduler_snapshot_workflow_run_id, WorkflowSchedulerSnapshotResponse, WorkflowServiceError,
};

use super::dependency_environment_subject::resolve_dependency_environment_action_subject;
use super::group_mutation::{
    create_node_group_graph, ungroup_node_graph, update_group_ports_graph,
};
use super::inference_interface_facts::{
    InferenceInterfaceFactsProvider, UnavailableInferenceInterfaceFactsProvider,
};
use super::inference_validation_lifecycle::{
    WorkflowGraphValidationLifecycleEventSink, WorkflowGraphValidationLifecycleOwner,
};
use super::inference_validation_state::{
    CurrentInferenceValidationStateStore, DependencyEnvironmentActionIntentStateRequest,
    DependencyEnvironmentActionIntentStateResolution,
};
use super::inference_validation_task_owner::WorkflowGraphValidationTaskOwner;
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
    WorkflowGraphRemoveEdgesRequest, WorkflowGraphUndoRedoStateResponse,
    WorkflowGraphUngroupRequest, WorkflowGraphUpdateGroupPortsRequest,
};
use super::types::WorkflowGraph;
#[cfg(test)]
use super::{
    session_types::{WorkflowGraphConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest},
    types::GraphEdge,
};
#[path = "session_connection_api.rs"]
mod session_connection_api;
#[path = "session_inference_validation_api.rs"]
mod session_inference_validation_api;
#[path = "session_node_api.rs"]
mod session_node_api;

pub(crate) type GraphSessionHandle = Arc<Mutex<GraphEditSession>>;

fn dirty_tasks_from_seed_nodes_unique(graph: &WorkflowGraph, node_ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    node_ids
        .iter()
        .flat_map(|node_id| dirty_tasks_from_seed_nodes(graph, std::slice::from_ref(node_id)))
        .filter(|task_id| seen.insert((*task_id).clone()))
        .collect()
}

pub struct GraphSessionStore {
    sessions: RwLock<HashMap<String, GraphSessionHandle>>,
    validation_state: Arc<CurrentInferenceValidationStateStore>,
    validation_lifecycle: Arc<WorkflowGraphValidationLifecycleOwner>,
    validation_tasks: WorkflowGraphValidationTaskOwner,
    inference_interface_facts_provider: Arc<dyn InferenceInterfaceFactsProvider>,
    dependency_environment_service: SharedDependencyEnvironmentService,
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
        Self::with_timeout_and_inference_interface_facts_provider(
            timeout,
            Arc::new(UnavailableInferenceInterfaceFactsProvider),
        )
    }

    pub fn with_inference_interface_facts_provider(
        provider: Arc<dyn InferenceInterfaceFactsProvider>,
    ) -> Self {
        Self::with_timeout_and_inference_interface_facts_provider(
            Duration::from_secs(5 * 60),
            provider,
        )
    }

    pub fn with_dependency_environment_provider(
        provider: SharedDependencyEnvironmentProvider,
    ) -> Self {
        Self::with_timeout_and_providers(
            Duration::from_secs(5 * 60),
            Arc::new(UnavailableInferenceInterfaceFactsProvider),
            provider,
        )
    }

    pub fn with_timeout_and_inference_interface_facts_provider(
        timeout: Duration,
        provider: Arc<dyn InferenceInterfaceFactsProvider>,
    ) -> Self {
        let dependency_provider: SharedDependencyEnvironmentProvider =
            Arc::new(NotImplementedDependencyEnvironmentProvider);
        Self::with_timeout_and_providers(timeout, provider, dependency_provider)
    }

    pub fn with_timeout_and_providers(
        timeout: Duration,
        inference_provider: Arc<dyn InferenceInterfaceFactsProvider>,
        dependency_provider: SharedDependencyEnvironmentProvider,
    ) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            validation_state: Arc::new(CurrentInferenceValidationStateStore::new()),
            validation_lifecycle: Arc::new(WorkflowGraphValidationLifecycleOwner::new()),
            validation_tasks: WorkflowGraphValidationTaskOwner::new(),
            inference_interface_facts_provider: inference_provider,
            dependency_environment_service: DependencyEnvironmentService::new(dependency_provider),
            stale_timeout: timeout,
        }
    }

    pub async fn set_validation_lifecycle_event_sink(
        &self,
        event_sink: Option<Arc<dyn WorkflowGraphValidationLifecycleEventSink>>,
    ) {
        self.validation_lifecycle.set_event_sink(event_sink).await;
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
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let removed = self.sessions.write().await.remove(session_id);
        if removed.is_none() {
            return Err(WorkflowServiceError::SessionNotFound(format!(
                "edit session '{}' not found",
                session_id
            )));
        }
        self.validation_tasks
            .close_graph_session(&graph_session_id)
            .await;
        self.validation_lifecycle
            .close_graph_session(&graph_session_id)
            .await;
        self.validation_state
            .clear_graph_session(&graph_session_id)
            .await;
        Ok(WorkflowGraphEditSessionCloseResponse { ok: true })
    }

    async fn cancel_active_validation_after_graph_mutation(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        self.validation_lifecycle
            .cancel_active_validation_for_graph_change(&graph_session_id)
            .await;
        Ok(())
    }

    pub async fn shutdown_validation_tasks(&self) {
        self.validation_tasks
            .shutdown_with_lifecycle(self.validation_lifecycle.as_ref())
            .await;
    }

    #[cfg(test)]
    pub(crate) async fn validation_task_owner_is_shut_down_for_tests(&self) -> bool {
        self.validation_tasks.is_shut_down().await
    }

    #[cfg(test)]
    pub(crate) async fn active_validation_task_count_for_tests(&self) -> usize {
        self.validation_tasks.active_task_count().await
    }

    #[cfg(test)]
    pub(crate) async fn validation_state_record_count_for_session(
        &self,
        session_id: &str,
    ) -> Result<usize, WorkflowServiceError> {
        let graph_session_id = WorkflowGraphSessionId::parse(session_id)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        Ok(self
            .validation_state
            .record_count_for_graph_session(&graph_session_id)
            .await)
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

    pub async fn resolve_dependency_environment_action_intent(
        &self,
        intent: DependencyEnvironmentActionIntent,
    ) -> Result<DependencyEnvironmentActionIntentResult, WorkflowServiceError> {
        let intent = ValidatedDependencyEnvironmentActionIntent::try_from(intent)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let session_id = intent.as_intent().graph_session_id.as_str();
        let handle = self.get_session_handle(session_id).await?;
        let mut state = handle.lock().await;
        state.touch();
        state.canonicalize_graph();
        let current_revision = state.graph.compute_fingerprint();
        let current_graph_revision = WorkflowGraphRevision::parse(&current_revision)
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        let subject = resolve_dependency_environment_action_subject(
            &state.graph,
            &intent.as_intent().target_node_id,
        );
        let sidecar_choices = dependency_environment_sidecar_choices(
            &state.graph,
            intent.as_intent().target_node_id.as_str(),
        );
        drop(state);

        let resolution = self
            .validation_state
            .resolve_dependency_environment_action_request(
                DependencyEnvironmentActionIntentStateRequest {
                    intent,
                    current_graph_revision,
                    subject,
                    selected_binding_ids: sidecar_choices.selected_binding_ids,
                    dependency_override_patches: sidecar_choices.dependency_override_patches,
                    sidecar_diagnostic: sidecar_choices.diagnostic,
                },
            )
            .await;

        let (intent, environment_request) = match resolution {
            DependencyEnvironmentActionIntentStateResolution::Blocked(result) => return Ok(result),
            DependencyEnvironmentActionIntentStateResolution::RequestReady {
                intent,
                environment_request,
            } => (intent, environment_request),
        };

        match self
            .dependency_environment_service
            .handle(&environment_request)
        {
            Ok(_result) => Ok(request_ready_dependency_environment_action_result(&intent)),
            Err(error) => Ok(blocked_dependency_environment_action_result(
                &intent,
                InferenceDiagnosticCode::DependencySidecarDescriptorInvalid,
                "Dependency environment service rejected provider output.",
                Some(error.to_string()),
            )),
        }
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
        let response = {
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
            state.snapshot_response_with_state(
                &request.session_id,
                Some(workflow_event),
                projection,
            )
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn remove_edge(
        &self,
        request: WorkflowGraphRemoveEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
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
                .map(|node_id| {
                    dirty_tasks_from_seed_nodes(&state.graph, std::slice::from_ref(node_id))
                })
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
            state.snapshot_response_with_state(
                &request.session_id,
                Some(workflow_event),
                projection,
            )
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn remove_edges(
        &self,
        request: WorkflowGraphRemoveEdgesRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
            let handle = self.get_session_handle(&request.session_id).await?;
            let mut state = handle.lock().await;
            state.touch();
            let before_graph = state.graph.clone();
            let edge_ids = request.edge_ids.into_iter().collect::<HashSet<_>>();
            let target_node_ids = state
                .graph
                .edges
                .iter()
                .filter(|edge| edge_ids.contains(&edge.id))
                .map(|edge| edge.target.clone())
                .collect::<Vec<_>>();
            state.push_undo_snapshot();
            state
                .graph
                .edges
                .retain(|edge| !edge_ids.contains(&edge.id));
            sync_embedding_emit_metadata_flags(&mut state.graph);
            let dirty_tasks = dirty_tasks_from_seed_nodes_unique(&state.graph, &target_node_ids);
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
            state.snapshot_response_with_state(
                &request.session_id,
                Some(workflow_event),
                projection,
            )
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn create_group(
        &self,
        request: WorkflowGraphCreateGroupRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
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
            state.snapshot_response_with_state(
                &request.session_id,
                Some(workflow_event),
                projection,
            )
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn ungroup(
        &self,
        request: WorkflowGraphUngroupRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
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
            state.snapshot_response_with_state(
                &request.session_id,
                Some(workflow_event),
                projection,
            )
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn update_group_ports(
        &self,
        request: WorkflowGraphUpdateGroupPortsRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
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
            state.snapshot_response_with_state(
                &request.session_id,
                Some(workflow_event),
                projection,
            )
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn undo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
            let handle = self.get_session_handle(&request.session_id).await?;
            let mut state = handle.lock().await;
            state.undo(&request.session_id)?
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
    }

    pub async fn redo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let response = {
            let handle = self.get_session_handle(&request.session_id).await?;
            let mut state = handle.lock().await;
            state.redo(&request.session_id)?
        };
        self.cancel_active_validation_after_graph_mutation(&request.session_id)
            .await?;
        Ok(response)
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

#[derive(Debug, Default)]
struct DependencyEnvironmentSidecarChoices {
    selected_binding_ids: Vec<DependencyBindingId>,
    dependency_override_patches: Vec<DependencyOverridePatchV1>,
    diagnostic: Option<InferenceInterfaceDiagnostic>,
}

fn request_ready_dependency_environment_action_result(
    intent: &DependencyEnvironmentActionIntent,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::RequestReady,
        diagnostics: Vec::new(),
    }
}

fn blocked_dependency_environment_action_result(
    intent: &DependencyEnvironmentActionIntent,
    code: InferenceDiagnosticCode,
    message: &str,
    hint: Option<String>,
) -> DependencyEnvironmentActionIntentResult {
    DependencyEnvironmentActionIntentResult {
        contract_version: intent.contract_version,
        graph_session_id: intent.graph_session_id.clone(),
        graph_revision: intent.graph_revision.clone(),
        validation_session_id: intent.validation_session_id.clone(),
        target_node_id: intent.target_node_id.clone(),
        action: intent.action,
        status: DependencyEnvironmentActionIntentStatus::Blocked,
        diagnostics: vec![InferenceInterfaceDiagnostic {
            severity: InferenceDiagnosticSeverity::Error,
            code,
            message: message.to_string(),
            hint,
            port_id: None,
        }],
    }
}

fn dependency_environment_sidecar_choices(
    graph: &WorkflowGraph,
    target_node_id: &str,
) -> DependencyEnvironmentSidecarChoices {
    let Some(node) = graph.nodes.iter().find(|node| node.id == target_node_id) else {
        return DependencyEnvironmentSidecarChoices::default();
    };

    let selected_binding_ids = match parse_optional_sidecar_field::<Vec<DependencyBindingId>>(
        &node.data,
        "selected_binding_ids",
    ) {
        Ok(value) => value.unwrap_or_default(),
        Err(diagnostic) => {
            return DependencyEnvironmentSidecarChoices {
                diagnostic: Some(diagnostic),
                ..Default::default()
            };
        }
    };
    let dependency_override_patches = match parse_optional_sidecar_field::<
        Vec<DependencyOverridePatchV1>,
    >(&node.data, "manual_overrides")
    {
        Ok(value) => value.unwrap_or_default(),
        Err(diagnostic) => {
            return DependencyEnvironmentSidecarChoices {
                selected_binding_ids,
                diagnostic: Some(diagnostic),
                ..Default::default()
            };
        }
    };

    DependencyEnvironmentSidecarChoices {
        selected_binding_ids,
        dependency_override_patches,
        diagnostic: None,
    }
}

fn parse_optional_sidecar_field<T>(
    data: &serde_json::Value,
    field: &'static str,
) -> Result<Option<T>, InferenceInterfaceDiagnostic>
where
    T: serde::de::DeserializeOwned,
{
    let Some(value) = data.get(field) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|error| {
            InferenceInterfaceDiagnostic {
                severity: InferenceDiagnosticSeverity::Error,
                code: InferenceDiagnosticCode::InvalidOption,
                message: format!("Dependency environment field `{field}` is invalid: {error}"),
                hint: Some(
                    "Use the dependency-environment node's typed sidecar fields for dependency choices."
                        .to_string(),
                ),
                port_id: InferencePortId::parse(field).ok(),
            }
        })
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
