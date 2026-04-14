use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent::rag::SharedRagManager;
use crate::llm::runtime_registry::reconcile_runtime_registry_snapshot_override;
use crate::llm::startup::build_resolved_embedding_request;
use crate::llm::{SharedAppConfig, SharedGateway, SharedRuntimeRegistry};
use node_engine::EventSink;
use pantograph_embedded_runtime::workflow_runtime::build_runtime_diagnostics_projection;
use pantograph_workflow_service::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse,
    EdgeInsertionPreviewResponse, GraphEdge, GraphNode, InsertNodeConnectionResponse,
    InsertNodeOnEdgeResponse, InsertNodePositionHint, Position, UndoRedoState,
    WorkflowCapabilitiesRequest, WorkflowGraph, WorkflowGraphAddEdgeRequest,
    WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest,
    WorkflowGraphEditSessionCreateRequest, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest, WorkflowSchedulerSnapshotRequest,
};
use tauri::{ipc::Channel, AppHandle, State};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::event_adapter::TauriEventAdapter;
use super::events::WorkflowEvent;

pub(crate) fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn send_diagnostics_projection(
    channel: &Channel<WorkflowEvent>,
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    execution_id: &str,
) {
    let _ = channel.send(WorkflowEvent::diagnostics_snapshot(
        execution_id.to_string(),
        diagnostics_store.snapshot(),
    ));
}

async fn emit_diagnostics_snapshots(
    app: &AppHandle,
    session_id: &str,
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    extensions: &SharedExtensions,
    workflow_service: &SharedWorkflowService,
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    channel: &Channel<WorkflowEvent>,
    runtime_snapshot_override: Option<inference::RuntimeLifecycleSnapshot>,
    runtime_model_target_override: Option<String>,
) {
    let scheduler_snapshot = match workflow_service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: session_id.to_string(),
        })
        .await
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            log::debug!(
                "Skipping diagnostics snapshots for session '{}' because scheduler snapshot is unavailable: {}",
                session_id,
                error
            );
            return;
        }
    };

    let workflow_id = scheduler_snapshot.workflow_id.clone();
    let runtime_workflow_id = workflow_id
        .clone()
        .unwrap_or_else(|| scheduler_snapshot.session.workflow_id.clone());
    let trace_execution_id = scheduler_snapshot
        .trace_execution_id
        .clone()
        .unwrap_or_else(|| session_id.to_string());
    let captured_at_ms = unix_timestamp_ms();

    let scheduler_event = WorkflowEvent::scheduler_snapshot(
        workflow_id,
        trace_execution_id.clone(),
        session_id.to_string(),
        captured_at_ms,
        Some(scheduler_snapshot.session.clone()),
        scheduler_snapshot.items.clone(),
        None,
    );
    diagnostics_store.record_workflow_event(&scheduler_event, captured_at_ms);
    let _ = channel.send(scheduler_event);
    send_diagnostics_projection(channel, diagnostics_store, session_id);

    let gateway_snapshot = gateway.runtime_lifecycle_snapshot().await;
    let gateway_mode_info = gateway.mode_info().await;
    let diagnostics_projection = build_runtime_diagnostics_projection(
        runtime_snapshot_override.as_ref(),
        &gateway_snapshot,
        &gateway_mode_info,
        runtime_model_target_override.as_deref(),
    );
    if let Some(snapshot) = runtime_snapshot_override.as_ref() {
        reconcile_runtime_registry_snapshot_override(
            runtime_registry.as_ref(),
            snapshot,
            diagnostics_projection.runtime_model_target.as_deref(),
        );
    }

    let runtime = match super::headless_workflow_commands::build_runtime(
        app,
        gateway,
        runtime_registry,
        extensions,
        workflow_service,
        None,
    )
    .await
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let embedding_runtime_snapshot = gateway.embedding_runtime_lifecycle_snapshot().await;
            let runtime_event = WorkflowEvent::runtime_snapshot(
                runtime_workflow_id.clone(),
                trace_execution_id.clone(),
                captured_at_ms,
                None,
                diagnostics_projection.trace_runtime_metrics.clone(),
                gateway_mode_info.active_model_target.clone(),
                gateway_mode_info.embedding_model_target.clone(),
                Some(diagnostics_projection.active_runtime_snapshot.clone()),
                embedding_runtime_snapshot,
                Some(error.clone()),
            );
            diagnostics_store.record_workflow_event(&runtime_event, captured_at_ms);
            let _ = channel.send(runtime_event);
            send_diagnostics_projection(channel, diagnostics_store, session_id);
            return;
        }
    };

    let (capabilities, runtime_error) = match runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: runtime_workflow_id.clone(),
        })
        .await
    {
        Ok(response) => (Some(response), None),
        Err(error) => {
            log::warn!(
                "Failed to collect runtime snapshot for workflow '{}' in session '{}': {}",
                runtime_workflow_id,
                session_id,
                error
            );
            (None, Some(error.to_envelope_json()))
        }
    };
    let embedding_runtime_snapshot = gateway.embedding_runtime_lifecycle_snapshot().await;

    let runtime_event = WorkflowEvent::runtime_snapshot(
        runtime_workflow_id,
        trace_execution_id,
        captured_at_ms,
        capabilities,
        diagnostics_projection.trace_runtime_metrics,
        gateway_mode_info.active_model_target.clone(),
        gateway_mode_info.embedding_model_target.clone(),
        Some(diagnostics_projection.active_runtime_snapshot),
        embedding_runtime_snapshot,
        runtime_error,
    );
    diagnostics_store.record_workflow_event(&runtime_event, captured_at_ms);
    let _ = channel.send(runtime_event);
    send_diagnostics_projection(channel, diagnostics_store, session_id);
}

async fn run_session_graph_snapshot(
    app: AppHandle,
    session_id: String,
    session_graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    let diagnostics_channel = channel.clone();

    emit_diagnostics_snapshots(
        &app,
        &session_id,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        diagnostics_store.inner(),
        &diagnostics_channel,
        None,
        None,
    )
    .await;

    diagnostics_store.set_execution_graph(&session_id, &session_graph);

    let event_adapter = Arc::new(TauriEventAdapter::new(
        channel,
        &session_id,
        diagnostics_store.inner().clone(),
    ));
    let guard = config.read().await;
    let device = guard.device.clone();
    drop(guard);
    let runtime = super::headless_workflow_commands::build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await?;
    let outcome = runtime
        .execute_edit_session_graph(
            &session_id,
            &session_graph,
            build_resolved_embedding_request(
                None,
                None,
                &device,
                Some("nomic-embed-text".to_string()),
            ),
            event_adapter.clone() as Arc<dyn EventSink>,
        )
        .await
        .map_err(|error| error.to_string())?;
    emit_diagnostics_snapshots(
        &app,
        &session_id,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        diagnostics_store.inner(),
        &diagnostics_channel,
        Some(outcome.runtime_snapshot),
        outcome.runtime_model_target,
    )
    .await;
    if let Some(error) = outcome.error {
        return Err(error);
    }
    Ok(())
}

pub async fn execute_workflow_v2(
    app: AppHandle,
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<String, String> {
    let session = workflow_service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest { graph })
        .await
        .map_err(|e| e.to_envelope_json())?;
    let execution_id = session.session_id.clone();
    let session_graph = workflow_service
        .workflow_graph_get_runtime_snapshot(&execution_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    run_session_graph_snapshot(
        app,
        execution_id.clone(),
        session_graph,
        gateway,
        runtime_registry,
        config,
        rag_manager,
        extensions,
        workflow_service,
        diagnostics_store,
        channel,
    )
    .await?;
    Ok(execution_id)
}

pub async fn get_undo_redo_state(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<UndoRedoState, String> {
    workflow_service
        .workflow_graph_get_undo_redo_state(WorkflowGraphUndoRedoStateRequest {
            session_id: execution_id,
        })
        .await
        .map(|state| UndoRedoState {
            can_undo: state.can_undo,
            can_redo: state.can_redo,
            undo_count: state.undo_count,
        })
        .map_err(|e| e.to_envelope_json())
}

pub async fn undo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_undo(WorkflowGraphEditSessionGraphRequest {
            session_id: execution_id,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn redo_workflow(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_redo(WorkflowGraphEditSessionGraphRequest {
            session_id: execution_id,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn update_node_data(
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: execution_id,
            node_id,
            data,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn update_node_position_in_execution(
    execution_id: String,
    node_id: String,
    position: Position,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: execution_id,
            node_id,
            position,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn add_node_to_execution(
    execution_id: String,
    node: GraphNode,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_add_node(WorkflowGraphAddNodeRequest {
            session_id: execution_id,
            node,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn remove_node_from_execution(
    execution_id: String,
    node_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_remove_node(WorkflowGraphRemoveNodeRequest {
            session_id: execution_id,
            node_id,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn add_edge_to_execution(
    execution_id: String,
    edge: GraphEdge,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_add_edge(WorkflowGraphAddEdgeRequest {
            session_id: execution_id,
            edge,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn get_connection_candidates(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    graph_revision: Option<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<ConnectionCandidatesResponse, String> {
    workflow_service
        .workflow_graph_get_connection_candidates(WorkflowGraphGetConnectionCandidatesRequest {
            session_id: execution_id,
            source_anchor,
            graph_revision,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn connect_anchors_in_execution(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    target_anchor: ConnectionAnchor,
    graph_revision: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<ConnectionCommitResponse, String> {
    workflow_service
        .workflow_graph_connect(WorkflowGraphConnectRequest {
            session_id: execution_id,
            source_anchor,
            target_anchor,
            graph_revision,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn insert_node_and_connect_in_execution(
    execution_id: String,
    source_anchor: ConnectionAnchor,
    node_type: String,
    graph_revision: String,
    position_hint: InsertNodePositionHint,
    preferred_input_port_id: Option<String>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<InsertNodeConnectionResponse, String> {
    workflow_service
        .workflow_graph_insert_node_and_connect(WorkflowGraphInsertNodeAndConnectRequest {
            session_id: execution_id,
            source_anchor,
            node_type,
            graph_revision,
            position_hint,
            preferred_input_port_id,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn preview_node_insert_on_edge_in_execution(
    execution_id: String,
    edge_id: String,
    node_type: String,
    graph_revision: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<EdgeInsertionPreviewResponse, String> {
    workflow_service
        .workflow_graph_preview_node_insert_on_edge(WorkflowGraphPreviewNodeInsertOnEdgeRequest {
            session_id: execution_id,
            edge_id,
            node_type,
            graph_revision,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn insert_node_on_edge_in_execution(
    execution_id: String,
    edge_id: String,
    node_type: String,
    graph_revision: String,
    position_hint: InsertNodePositionHint,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<InsertNodeOnEdgeResponse, String> {
    workflow_service
        .workflow_graph_insert_node_on_edge(WorkflowGraphInsertNodeOnEdgeRequest {
            session_id: execution_id,
            edge_id,
            node_type,
            graph_revision,
            position_hint,
        })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn remove_edge_from_execution(
    execution_id: String,
    edge_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_remove_edge(WorkflowGraphRemoveEdgeRequest {
            session_id: execution_id,
            edge_id,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn get_execution_graph(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowGraph, String> {
    workflow_service
        .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
            session_id: execution_id,
        })
        .await
        .map(|response| response.graph)
        .map_err(|e| e.to_envelope_json())
}

pub async fn create_workflow_session(
    graph: WorkflowGraph,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<pantograph_workflow_service::WorkflowGraphEditSessionCreateResponse, String> {
    workflow_service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest { graph })
        .await
        .map_err(|e| e.to_envelope_json())
}

pub async fn run_workflow_session(
    app: AppHandle,
    session_id: String,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    let session_graph = workflow_service
        .workflow_graph_get_runtime_snapshot(&session_id)
        .await
        .map_err(|e| e.to_envelope_json())?;
    run_session_graph_snapshot(
        app,
        session_id,
        session_graph,
        gateway,
        runtime_registry,
        config,
        rag_manager,
        extensions,
        workflow_service,
        diagnostics_store,
        channel,
    )
    .await
}

pub async fn remove_execution(
    execution_id: String,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<(), String> {
    workflow_service
        .workflow_graph_close_edit_session(
            pantograph_workflow_service::WorkflowGraphEditSessionCloseRequest {
                session_id: execution_id,
            },
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_envelope_json())
}
