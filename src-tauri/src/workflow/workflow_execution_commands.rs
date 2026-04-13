use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agent::rag::SharedRagManager;
use crate::llm::commands::resolve_embedding_model_path;
use crate::llm::startup::{
    build_resolved_embedding_request, capture_inference_restore_config,
    restore_inference_runtime_best_effort,
};
use crate::llm::{SharedAppConfig, SharedGateway};
use node_engine::EventSink;
use pantograph_runtime_identity::canonical_runtime_backend_key;
use pantograph_workflow_service::{
    convert_graph_to_node_engine, ConnectionAnchor, ConnectionCandidatesResponse,
    ConnectionCommitResponse, EdgeInsertionPreviewResponse, GraphEdge, GraphNode,
    InsertNodeConnectionResponse, InsertNodeOnEdgeResponse, InsertNodePositionHint, Position,
    UndoRedoState, WorkflowCapabilitiesRequest, WorkflowGraph, WorkflowGraphAddEdgeRequest,
    WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest,
    WorkflowGraphEditSessionCreateRequest, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest, WorkflowSchedulerSnapshotRequest,
    WorkflowTraceRuntimeMetrics,
};
use tauri::{ipc::Channel, AppHandle, State};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::event_adapter::TauriEventAdapter;
use super::events::WorkflowEvent;
use super::task_executor::TauriTaskExecutor;

#[derive(Clone, Default)]
struct RuntimeExtensionsSnapshot {
    pumas_api: Option<Arc<pumas_library::PumasApi>>,
    kv_cache_store: Option<Arc<inference::kv_cache::KvCacheStore>>,
    dependency_resolver: Option<Arc<dyn node_engine::ModelDependencyResolver>>,
}

fn snapshot_runtime_extensions(
    shared: &node_engine::ExecutorExtensions,
) -> RuntimeExtensionsSnapshot {
    RuntimeExtensionsSnapshot {
        pumas_api: shared
            .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned(),
        kv_cache_store: shared
            .get::<Arc<inference::kv_cache::KvCacheStore>>(
                node_engine::extension_keys::KV_CACHE_STORE,
            )
            .cloned(),
        dependency_resolver: shared
            .get::<Arc<dyn node_engine::ModelDependencyResolver>>(
                node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
            )
            .cloned(),
    }
}

fn apply_runtime_extensions(
    executor: &mut node_engine::WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
    event_sink: Arc<dyn EventSink>,
    execution_id: &str,
) {
    if let Some(api) = &snapshot.pumas_api {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::PUMAS_API, api.clone());
    }
    if let Some(store) = &snapshot.kv_cache_store {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::KV_CACHE_STORE, store.clone());
    }
    if let Some(resolver) = &snapshot.dependency_resolver {
        executor.extensions_mut().set(
            node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
            resolver.clone(),
        );
    }
    executor.extensions_mut().set(
        super::task_executor::runtime_extension_keys::EVENT_SINK,
        event_sink,
    );
    executor.extensions_mut().set(
        super::task_executor::runtime_extension_keys::EXECUTION_ID,
        execution_id.to_string(),
    );
}

async fn sync_embedding_emit_metadata_flags_for_executor(
    executor: &mut node_engine::WorkflowExecutor,
) -> Result<(), String> {
    let snapshot = executor.get_graph_snapshot().await;
    let mut counts = std::collections::HashMap::<String, u32>::new();
    for edge in &snapshot.edges {
        let key = format!("{}:{}", edge.source, edge.source_handle);
        counts
            .entry(key)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    for node in &snapshot.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        let mut data = node.data.clone();
        match data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
        executor
            .update_node_data(&node.id, data)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn graph_has_embedding_node(graph: &WorkflowGraph) -> bool {
    graph.nodes.iter().any(|node| node.node_type == "embedding")
}

fn graph_has_llamacpp_inference_node(graph: &WorkflowGraph) -> bool {
    graph
        .nodes
        .iter()
        .any(|node| node.node_type == "llamacpp-inference")
}

fn node_engine_graph_has_embedding_node(graph: &node_engine::WorkflowGraph) -> bool {
    graph.nodes.iter().any(|node| node.node_type == "embedding")
}

fn node_engine_graph_has_llamacpp_inference_node(graph: &node_engine::WorkflowGraph) -> bool {
    graph
        .nodes
        .iter()
        .any(|node| node.node_type == "llamacpp-inference")
}

fn node_data_string(data: &serde_json::Value, keys: &[&str]) -> Option<String> {
    let obj = data.as_object()?;
    keys.iter().find_map(|key| {
        obj.get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn resolve_embedding_model_id_from_graph(graph: &WorkflowGraph) -> Result<Option<String>, String> {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();

    let embedding_nodes = graph
        .nodes
        .iter()
        .filter(|node| node.node_type == "embedding")
        .collect::<Vec<_>>();
    if embedding_nodes.is_empty() {
        return Ok(None);
    }

    let mut selected_model_ids = std::collections::BTreeSet::new();
    for embedding_node in embedding_nodes {
        let mut model_ids_for_node = std::collections::BTreeSet::new();
        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.target == embedding_node.id && edge.target_handle == "model")
        {
            let source_node = node_by_id.get(edge.source.as_str()).ok_or_else(|| {
                format!(
                    "Embedding node '{}' references unknown source node '{}'",
                    embedding_node.id, edge.source
                )
            })?;
            if source_node.node_type != "puma-lib" {
                return Err(format!(
                    "Embedding node '{}' must receive `model` from a Puma-Lib node",
                    embedding_node.id
                ));
            }
            let model_id = node_data_string(&source_node.data, &["model_id", "modelId"])
                .ok_or_else(|| {
                    format!(
                        "Puma-Lib node '{}' is missing `model_id`. Re-select a model in Puma-Lib.",
                        source_node.id
                    )
                })?;
            model_ids_for_node.insert(model_id);
        }

        if model_ids_for_node.is_empty() {
            return Err(format!(
                "Embedding node '{}' must connect Puma-Lib `model_path` output to `model` input",
                embedding_node.id
            ));
        }
        if model_ids_for_node.len() > 1 {
            return Err(format!(
                "Embedding node '{}' has multiple Puma-Lib model IDs connected to `model`; use exactly one",
                embedding_node.id
            ));
        }
        selected_model_ids.extend(model_ids_for_node);
    }

    if selected_model_ids.len() > 1 {
        return Err(
            "All embedding nodes in one workflow run must use the same Puma-Lib model".to_string(),
        );
    }

    Ok(selected_model_ids.into_iter().next())
}

fn resolve_embedding_model_id_from_node_engine_graph(
    graph: &node_engine::WorkflowGraph,
) -> Result<Option<String>, String> {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<std::collections::HashMap<_, _>>();

    let embedding_nodes = graph
        .nodes
        .iter()
        .filter(|node| node.node_type == "embedding")
        .collect::<Vec<_>>();
    if embedding_nodes.is_empty() {
        return Ok(None);
    }

    let mut selected_model_ids = std::collections::BTreeSet::new();
    for embedding_node in embedding_nodes {
        let mut model_ids_for_node = std::collections::BTreeSet::new();
        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.target == embedding_node.id && edge.target_handle == "model")
        {
            let source_node = node_by_id.get(edge.source.as_str()).ok_or_else(|| {
                format!(
                    "Embedding node '{}' references unknown source node '{}'",
                    embedding_node.id, edge.source
                )
            })?;
            if source_node.node_type != "puma-lib" {
                return Err(format!(
                    "Embedding node '{}' must receive `model` from a Puma-Lib node",
                    embedding_node.id
                ));
            }
            let model_id = node_data_string(&source_node.data, &["model_id", "modelId"])
                .ok_or_else(|| {
                    format!(
                        "Puma-Lib node '{}' is missing `model_id`. Re-select a model in Puma-Lib.",
                        source_node.id
                    )
                })?;
            model_ids_for_node.insert(model_id);
        }

        if model_ids_for_node.is_empty() {
            return Err(format!(
                "Embedding node '{}' must connect Puma-Lib `model_path` output to `model` input",
                embedding_node.id
            ));
        }
        if model_ids_for_node.len() > 1 {
            return Err(format!(
                "Embedding node '{}' has multiple Puma-Lib model IDs connected to `model`; use exactly one",
                embedding_node.id
            ));
        }
        selected_model_ids.extend(model_ids_for_node);
    }

    if selected_model_ids.len() > 1 {
        return Err(
            "All embedding nodes in one workflow run must use the same Puma-Lib model".to_string(),
        );
    }

    Ok(selected_model_ids.into_iter().next())
}

async fn prepare_embedding_runtime(
    gateway: &SharedGateway,
    config: &SharedAppConfig,
    pumas_api: Option<Arc<pumas_library::PumasApi>>,
    embedding_model_id_from_graph: Option<String>,
    needs_embedding_node: bool,
    needs_llamacpp_inference_node: bool,
) -> Result<Option<inference::BackendConfig>, String> {
    if !needs_embedding_node {
        return Ok(None);
    }

    if needs_llamacpp_inference_node {
        return Err(
            "Workflow includes both `embedding` and `llamacpp-inference` nodes. They currently require different llama.cpp runtime modes; run them in separate workflow executions."
                .to_string(),
        );
    }

    let backend_name = gateway.current_backend_name().await;
    if !is_llamacpp_backend_name(&backend_name) {
        return Err(format!(
            "Embedding nodes currently require the `llama.cpp` backend, but active backend is '{}'",
            backend_name
        ));
    }

    if gateway.is_ready().await && gateway.is_embedding_mode().await {
        return Ok(None);
    }

    let restore_config = capture_inference_restore_config(gateway).await;

    let model_id = embedding_model_id_from_graph.ok_or_else(|| {
        "Embedding workflows must connect Puma-Lib `model_path` to embedding `model`".to_string()
    })?;
    let api = pumas_api.ok_or_else(|| {
        "Puma-Lib runtime is not initialized; cannot resolve model path from model_id".to_string()
    })?;

    let model = api
        .get_model(&model_id)
        .await
        .map_err(|e| {
            format!(
                "Failed to resolve model '{}' from Puma-Lib: {}",
                model_id, e
            )
        })?
        .ok_or_else(|| {
            format!(
                "Puma-Lib model '{}' was not found. Re-select the model in Puma-Lib node.",
                model_id
            )
        })?;

    if !model.model_type.eq_ignore_ascii_case("embedding") {
        return Err(format!(
            "Puma-Lib model '{}' is type '{}' but embedding node requires an embedding model",
            model_id, model.model_type
        ));
    }

    let resolved_embedding_model_path = resolve_embedding_model_path(&model.path)?;

    let guard = config.read().await;
    let device = guard.device.clone();
    drop(guard);

    let embedding_config = gateway
        .build_embedding_start_config(build_resolved_embedding_request(
            Some(resolved_embedding_model_path),
            None,
            &device,
            Some("nomic-embed-text".to_string()),
        ))
        .await
        .map_err(|e| e.to_string())?;

    gateway
        .start(&embedding_config)
        .await
        .map_err(|e| format!("Failed to start llama.cpp in embedding mode: {}", e))?;

    Ok(restore_config)
}

fn is_llamacpp_backend_name(backend_name: &str) -> bool {
    canonical_runtime_backend_key(backend_name) == "llama_cpp"
}

pub(crate) fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub(crate) fn trace_runtime_metrics(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_target: Option<&str>,
) -> WorkflowTraceRuntimeMetrics {
    WorkflowTraceRuntimeMetrics {
        runtime_id: snapshot.runtime_id.clone(),
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.lifecycle_decision_reason.clone().or_else(|| {
            match (
                snapshot.last_error.as_ref(),
                snapshot.runtime_reused,
                snapshot.active,
            ) {
                (Some(_), _, _) => Some("runtime_start_failed".to_string()),
                (None, Some(true), true) => Some("runtime_reused".to_string()),
                (None, _, true) => Some("runtime_ready".to_string()),
                (None, _, false) => None,
            }
        }),
    }
}

fn diagnostics_runtime_trace_metrics(
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    runtime_model_target_override: Option<&str>,
    gateway_runtime_model_target: Option<&str>,
) -> WorkflowTraceRuntimeMetrics {
    trace_runtime_metrics(
        runtime_snapshot_override.unwrap_or(gateway_snapshot),
        runtime_model_target_override.or(gateway_runtime_model_target),
    )
}

fn diagnostics_active_runtime_snapshot(
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
) -> inference::RuntimeLifecycleSnapshot {
    runtime_snapshot_override
        .cloned()
        .unwrap_or_else(|| gateway_snapshot.clone())
}

pub(crate) fn resolve_runtime_model_target(
    mode_info: &crate::config::ServerModeInfo,
    snapshot: &inference::RuntimeLifecycleSnapshot,
) -> Option<String> {
    if snapshot.runtime_id.as_deref() == Some("llama.cpp.embedding") {
        return mode_info.embedding_model_target.clone();
    }
    mode_info.active_model_target.clone()
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
    let gateway_runtime_model_target =
        resolve_runtime_model_target(&gateway_mode_info, &gateway_snapshot);

    let runtime = match super::headless_workflow_commands::build_runtime(
        app,
        gateway,
        extensions,
        workflow_service,
        None,
    )
    .await
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let active_runtime_snapshot = diagnostics_active_runtime_snapshot(
                runtime_snapshot_override.as_ref(),
                &gateway_snapshot,
            );
            let embedding_runtime_snapshot = gateway.embedding_runtime_lifecycle_snapshot().await;
            let runtime_model_target_override =
                runtime_snapshot_override.as_ref().and_then(|snapshot| {
                    runtime_model_target_override
                        .as_deref()
                        .map(ToOwned::to_owned)
                        .or_else(|| resolve_runtime_model_target(&gateway_mode_info, snapshot))
                });
            let runtime_trace_metrics = diagnostics_runtime_trace_metrics(
                runtime_snapshot_override.as_ref(),
                &gateway_snapshot,
                runtime_model_target_override.as_deref(),
                gateway_runtime_model_target.as_deref(),
            );
            let runtime_event = WorkflowEvent::runtime_snapshot(
                runtime_workflow_id.clone(),
                trace_execution_id.clone(),
                captured_at_ms,
                None,
                runtime_trace_metrics,
                gateway_mode_info.active_model_target.clone(),
                gateway_mode_info.embedding_model_target.clone(),
                Some(active_runtime_snapshot),
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
    let active_runtime_snapshot =
        diagnostics_active_runtime_snapshot(runtime_snapshot_override.as_ref(), &gateway_snapshot);
    let embedding_runtime_snapshot = gateway.embedding_runtime_lifecycle_snapshot().await;
    let runtime_model_target_override = runtime_snapshot_override.as_ref().and_then(|snapshot| {
        runtime_model_target_override
            .as_deref()
            .map(ToOwned::to_owned)
            .or_else(|| resolve_runtime_model_target(&gateway_mode_info, snapshot))
    });
    let runtime_trace_metrics = diagnostics_runtime_trace_metrics(
        runtime_snapshot_override.as_ref(),
        &gateway_snapshot,
        runtime_model_target_override.as_deref(),
        gateway_runtime_model_target.as_deref(),
    );

    let runtime_event = WorkflowEvent::runtime_snapshot(
        runtime_workflow_id,
        trace_execution_id,
        captured_at_ms,
        capabilities,
        runtime_trace_metrics,
        gateway_mode_info.active_model_target.clone(),
        gateway_mode_info.embedding_model_target.clone(),
        Some(active_runtime_snapshot),
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
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let diagnostics_channel = channel.clone();

    emit_diagnostics_snapshots(
        &app,
        &session_id,
        gateway.inner(),
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
    let runtime_ext = {
        let shared = extensions.read().await;
        snapshot_runtime_extensions(&shared)
    };
    let restore_config = prepare_embedding_runtime(
        gateway.inner(),
        config.inner(),
        runtime_ext.pumas_api.clone(),
        resolve_embedding_model_id_from_graph(&session_graph)?,
        graph_has_embedding_node(&session_graph),
        graph_has_llamacpp_inference_node(&session_graph),
    )
    .await?;

    let core = Arc::new(
        node_engine::CoreTaskExecutor::new()
            .with_project_root(project_root)
            .with_gateway(gateway.inner_arc())
            .with_event_sink(event_adapter.clone() as Arc<dyn EventSink>)
            .with_execution_id(session_id.clone()),
    );
    let host = Arc::new(TauriTaskExecutor::new(rag_manager.inner().clone()));
    let task_executor = node_engine::CompositeTaskExecutor::new(
        Some(host as Arc<dyn node_engine::TaskExecutor>),
        core,
    );

    let terminal_nodes: Vec<String> = session_graph
        .nodes
        .iter()
        .filter(|node| !session_graph.edges.iter().any(|e| e.source == node.id))
        .map(|node| node.id.clone())
        .collect();

    let mut executor = node_engine::WorkflowExecutor::new(
        &session_id,
        convert_graph_to_node_engine(&session_graph),
        event_adapter.clone(),
    );
    apply_runtime_extensions(
        &mut executor,
        &runtime_ext,
        event_adapter.clone() as Arc<dyn EventSink>,
        &session_id,
    );
    executor.set_event_sink(event_adapter.clone());
    sync_embedding_emit_metadata_flags_for_executor(&mut executor).await?;

    workflow_service
        .workflow_graph_mark_edit_session_running(&session_id)
        .await
        .map_err(|error| error.to_envelope_json())?;

    let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowStarted {
        workflow_id: session_id.clone(),
        execution_id: session_id.clone(),
    });

    let mut workflow_result: Result<(), String> = Ok(());
    for node_id in &terminal_nodes {
        match executor.demand(node_id, &task_executor).await {
            Ok(_outputs) => {
                log::debug!("Demanded outputs from node: {}", node_id);
            }
            Err(e) => {
                log::error!("Error demanding from node {}: {}", node_id, e);
                workflow_result = Err(e.to_string());
                break;
            }
        }
    }

    workflow_service
        .workflow_graph_mark_edit_session_finished(&session_id)
        .await
        .map_err(|error| error.to_envelope_json())?;

    if workflow_result.is_ok() {
        let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: session_id.clone(),
            execution_id: session_id.clone(),
        });
    } else if let Err(error) = &workflow_result {
        let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id: session_id.clone(),
            execution_id: session_id.clone(),
            error: error.clone(),
        });
    }

    let execution_runtime_snapshot = gateway.runtime_lifecycle_snapshot().await;
    let execution_mode_info = gateway.mode_info().await;
    let execution_runtime_model_target =
        resolve_runtime_model_target(&execution_mode_info, &execution_runtime_snapshot);
    restore_inference_runtime_best_effort(
        gateway.inner(),
        restore_config,
        "Workflow executed in embedding mode but failed to restore previous inference mode",
    )
    .await;
    emit_diagnostics_snapshots(
        &app,
        &session_id,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        diagnostics_store.inner(),
        &diagnostics_channel,
        Some(execution_runtime_snapshot),
        execution_runtime_model_target,
    )
    .await;
    workflow_result
}

#[cfg(test)]
mod tests {
    use super::{
        diagnostics_runtime_trace_metrics, is_llamacpp_backend_name, trace_runtime_metrics,
    };

    #[test]
    fn llama_cpp_backend_gate_accepts_stable_aliases() {
        assert!(is_llamacpp_backend_name("llama.cpp"));
        assert!(is_llamacpp_backend_name("llama_cpp"));
        assert!(is_llamacpp_backend_name("llamacpp"));
        assert!(!is_llamacpp_backend_name("ollama"));
    }

    #[test]
    fn trace_runtime_metrics_prefers_backend_lifecycle_reason() {
        let metrics = trace_runtime_metrics(
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("pytorch-1".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("reused_loaded_pytorch_model".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/demo"),
        );

        assert_eq!(
            metrics.lifecycle_decision_reason.as_deref(),
            Some("reused_loaded_pytorch_model")
        );
        assert_eq!(metrics.model_target.as_deref(), Some("/models/demo"));
    }

    #[test]
    fn diagnostics_runtime_trace_metrics_prefers_execution_snapshot_override() {
        let execution_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
            warmup_started_at_ms: Some(100),
            warmup_completed_at_ms: Some(110),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("reused_embedding_runtime".to_string()),
            active: true,
            last_error: None,
        };
        let restored_gateway_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-restore-9".to_string()),
            warmup_started_at_ms: Some(200),
            warmup_completed_at_ms: Some(240),
            warmup_duration_ms: Some(40),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_llamacpp_inference".to_string()),
            active: true,
            last_error: None,
        };

        let metrics = diagnostics_runtime_trace_metrics(
            Some(&execution_snapshot),
            &restored_gateway_snapshot,
            Some("/models/embed.gguf"),
            Some("/models/restore.gguf"),
        );

        assert_eq!(metrics.runtime_id.as_deref(), Some("llama.cpp.embedding"));
        assert_eq!(
            metrics.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-2")
        );
        assert_eq!(metrics.runtime_reused, Some(true));
        assert_eq!(metrics.model_target.as_deref(), Some("/models/embed.gguf"));
        assert_eq!(
            metrics.lifecycle_decision_reason.as_deref(),
            Some("reused_embedding_runtime")
        );
    }
}

pub async fn execute_workflow_v2(
    app: AppHandle,
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
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
