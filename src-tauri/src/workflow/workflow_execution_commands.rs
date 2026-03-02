use std::path::PathBuf;
use std::sync::Arc;

use tauri::{ipc::Channel, AppHandle, State};
use uuid::Uuid;

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedAppConfig, SharedGateway};
use node_engine::EventSink;

use super::commands::SharedExtensions;
use super::event_adapter::TauriEventAdapter;
use super::events::WorkflowEvent;
use super::execution_manager::{SharedExecutionManager, UndoRedoState};
use super::task_executor::TauriTaskExecutor;
use super::types::{GraphEdge, GraphNode, WorkflowGraph};

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

async fn prepare_embedding_runtime(
    gateway: &SharedGateway,
    config: &SharedAppConfig,
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
    if backend_name != "llama.cpp" {
        return Err(format!(
            "Embedding nodes currently require the `llama.cpp` backend, but active backend is '{}'",
            backend_name
        ));
    }

    if gateway.is_ready().await && gateway.is_embedding_mode().await {
        return Ok(None);
    }

    let restore_config = if gateway.is_ready().await && !gateway.is_embedding_mode().await {
        gateway.last_inference_config().await
    } else {
        None
    };

    let guard = config.read().await;
    let embedding_model_path = guard
        .models
        .embedding_model_path
        .clone()
        .ok_or_else(|| "Embedding model path not configured".to_string())?;
    let device = guard.device.clone();
    drop(guard);

    let embedding_config = inference::BackendConfig {
        model_path: Some(std::path::PathBuf::from(embedding_model_path)),
        device: Some(device.device),
        gpu_layers: Some(device.gpu_layers),
        embedding_mode: true,
        ..Default::default()
    };

    gateway
        .start(&embedding_config)
        .await
        .map_err(|e| format!("Failed to start llama.cpp in embedding mode: {}", e))?;

    Ok(restore_config)
}

async fn restore_runtime_if_needed(
    gateway: &SharedGateway,
    restore_config: Option<inference::BackendConfig>,
) {
    if let Some(config) = restore_config {
        if let Err(err) = gateway.start(&config).await {
            log::warn!(
                "Workflow executed in embedding mode but failed to restore previous inference mode: {}",
                err
            );
        }
    }
}

pub async fn execute_workflow_v2(
    _app: AppHandle,
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
    extensions: State<'_, SharedExtensions>,
    channel: Channel<WorkflowEvent>,
) -> Result<String, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let execution_id = Uuid::new_v4().to_string();

    let event_adapter = Arc::new(TauriEventAdapter::new(channel, &execution_id));

    let graph = hydrate_embedding_emit_metadata_flags(graph);
    let ne_graph = convert_graph_to_node_engine(&graph);
    let restore_config = prepare_embedding_runtime(
        gateway.inner(),
        config.inner(),
        graph_has_embedding_node(&graph),
        graph_has_llamacpp_inference_node(&graph),
    )
    .await?;

    execution_manager
        .create_execution(&execution_id, ne_graph, event_adapter.clone())
        .await;

    let core = Arc::new(
        node_engine::CoreTaskExecutor::new()
            .with_project_root(project_root)
            .with_gateway(gateway.inner_arc())
            .with_event_sink(event_adapter.clone() as Arc<dyn EventSink>)
            .with_execution_id(execution_id.clone()),
    );
    let host = Arc::new(TauriTaskExecutor::new(rag_manager.inner().clone()));
    let task_executor = node_engine::CompositeTaskExecutor::new(
        Some(host as Arc<dyn node_engine::TaskExecutor>),
        core,
    );

    let terminal_nodes: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| !graph.edges.iter().any(|e| e.source == node.id))
        .map(|node| node.id.clone())
        .collect();

    let runtime_ext = {
        let shared = extensions.read().await;
        snapshot_runtime_extensions(&shared)
    };

    let workflow_result = {
        let mut executions = execution_manager.executions().await;
        let state = executions
            .get_mut(&execution_id)
            .ok_or_else(|| "Execution not found".to_string())?;
        state.touch();
        apply_runtime_extensions(&mut state.executor, &runtime_ext);

        let _ = state.push_undo_snapshot().await;

        let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: execution_id.clone(),
            execution_id: execution_id.clone(),
        });

        let mut workflow_result: Result<(), String> = Ok(());
        for node_id in &terminal_nodes {
            match state.executor.demand(node_id, &task_executor).await {
                Ok(_outputs) => {
                    log::debug!("Demanded outputs from node: {}", node_id);
                }
                Err(e) => {
                    log::error!("Error demanding from node {}: {}", node_id, e);
                    let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowFailed {
                        workflow_id: execution_id.clone(),
                        execution_id: execution_id.clone(),
                        error: e.to_string(),
                    });
                    workflow_result = Err(e.to_string());
                    break;
                }
            }
        }

        if workflow_result.is_ok() {
            let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowCompleted {
                workflow_id: execution_id.clone(),
                execution_id: execution_id.clone(),
            });
        }

        workflow_result
    };

    restore_runtime_if_needed(gateway.inner(), restore_config).await;
    workflow_result?;

    Ok(execution_id)
}

pub async fn get_undo_redo_state(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<UndoRedoState, String> {
    execution_manager
        .get_undo_redo_state(&execution_id)
        .await
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))
}

pub async fn undo_workflow(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    match state.undo().await {
        Some(Ok(graph)) => Ok(convert_graph_from_node_engine(&graph)),
        Some(Err(e)) => Err(format!("Undo failed: {}", e)),
        None => Err("Nothing to undo".to_string()),
    }
}

pub async fn redo_workflow(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    match state.redo().await {
        Some(Ok(graph)) => Ok(convert_graph_from_node_engine(&graph)),
        Some(Err(e)) => Err(format!("Redo failed: {}", e)),
        None => Err("Nothing to redo".to_string()),
    }
}

pub async fn update_node_data(
    execution_id: String,
    node_id: String,
    data: serde_json::Value,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    let _ = state.push_undo_snapshot().await;

    state
        .executor
        .update_node_data(&node_id, data)
        .await
        .map_err(|e| e.to_string())
}

pub async fn add_node_to_execution(
    execution_id: String,
    node: GraphNode,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    let ne_node = convert_node_to_node_engine(&node);

    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    let _ = state.push_undo_snapshot().await;

    state.executor.add_node(ne_node).await;
    sync_embedding_emit_metadata_flags_for_executor(&mut state.executor).await?;
    Ok(())
}

pub async fn add_edge_to_execution(
    execution_id: String,
    edge: GraphEdge,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    let ne_edge = convert_edge_to_node_engine(&edge);

    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    let _ = state.push_undo_snapshot().await;

    state.executor.add_edge(ne_edge).await;
    sync_embedding_emit_metadata_flags_for_executor(&mut state.executor).await?;

    let graph = state.executor.get_graph_snapshot().await;
    Ok(convert_graph_from_node_engine(&graph))
}

pub async fn remove_edge_from_execution(
    execution_id: String,
    edge_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    let _ = state.push_undo_snapshot().await;

    state.executor.remove_edge(&edge_id).await;
    sync_embedding_emit_metadata_flags_for_executor(&mut state.executor).await?;

    let graph = state.executor.get_graph_snapshot().await;
    Ok(convert_graph_from_node_engine(&graph))
}

pub async fn get_execution_graph(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<WorkflowGraph, String> {
    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&execution_id)
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))?;
    state.touch();

    let graph = state.executor.get_graph_snapshot().await;
    Ok(convert_graph_from_node_engine(&graph))
}

pub async fn create_workflow_session(
    graph: WorkflowGraph,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<String, String> {
    let session_id = Uuid::new_v4().to_string();

    let graph = hydrate_embedding_emit_metadata_flags(graph);
    let ne_graph = convert_graph_to_node_engine(&graph);

    let event_sink = Arc::new(node_engine::NullEventSink);
    execution_manager
        .create_execution(&session_id, ne_graph, event_sink)
        .await;

    {
        let mut executions = execution_manager.executions().await;
        if let Some(state) = executions.get_mut(&session_id) {
            let _ = state.push_undo_snapshot().await;
        }
    }

    Ok(session_id)
}

pub async fn run_workflow_session(
    _app: AppHandle,
    session_id: String,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
    extensions: State<'_, SharedExtensions>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let event_adapter = Arc::new(TauriEventAdapter::new(channel, &session_id));

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

    let runtime_ext = {
        let shared = extensions.read().await;
        snapshot_runtime_extensions(&shared)
    };

    let session_graph = {
        let mut executions = execution_manager.executions().await;
        let state = executions
            .get_mut(&session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;
        state.touch();
        state.executor.get_graph_snapshot().await
    };

    let restore_config = prepare_embedding_runtime(
        gateway.inner(),
        config.inner(),
        node_engine_graph_has_embedding_node(&session_graph),
        node_engine_graph_has_llamacpp_inference_node(&session_graph),
    )
    .await?;

    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    state.touch();
    apply_runtime_extensions(&mut state.executor, &runtime_ext);

    state.executor.set_event_sink(event_adapter.clone());

    let graph = state.executor.get_graph_snapshot().await;
    let terminal_nodes: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| !graph.edges.iter().any(|e| e.source == node.id))
        .map(|node| node.id.clone())
        .collect();

    let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowStarted {
        workflow_id: session_id.clone(),
        execution_id: session_id.clone(),
    });

    let mut workflow_result: Result<(), String> = Ok(());
    for node_id in &terminal_nodes {
        match state.executor.demand(node_id, &task_executor).await {
            Ok(_outputs) => {
                log::debug!("Demanded outputs from node: {}", node_id);
            }
            Err(e) => {
                log::error!("Error demanding from node {}: {}", node_id, e);
                let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowFailed {
                    workflow_id: session_id.clone(),
                    execution_id: session_id.clone(),
                    error: e.to_string(),
                });
                workflow_result = Err(e.to_string());
                break;
            }
        }
    }

    if workflow_result.is_ok() {
        let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: session_id.clone(),
            execution_id: session_id.clone(),
        });
    }

    drop(executions);
    restore_runtime_if_needed(gateway.inner(), restore_config).await;
    workflow_result?;

    Ok(())
}

pub async fn remove_execution(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    execution_manager.remove_execution(&execution_id).await;
    Ok(())
}

fn convert_graph_to_node_engine(graph: &WorkflowGraph) -> node_engine::WorkflowGraph {
    let mut ne_graph =
        node_engine::WorkflowGraph::new(Uuid::new_v4().to_string(), "Workflow".to_string());

    for node in &graph.nodes {
        ne_graph.nodes.push(convert_node_to_node_engine(node));
    }

    for edge in &graph.edges {
        ne_graph.edges.push(convert_edge_to_node_engine(edge));
    }

    ne_graph
}

fn convert_node_to_node_engine(node: &GraphNode) -> node_engine::GraphNode {
    let mut data = node.data.clone();
    if let serde_json::Value::Object(ref mut map) = data {
        map.insert("node_type".to_string(), serde_json::json!(node.node_type));
    }

    node_engine::GraphNode {
        id: node.id.clone(),
        node_type: node.node_type.clone(),
        data,
        position: (node.position.x, node.position.y),
    }
}

fn convert_edge_to_node_engine(edge: &GraphEdge) -> node_engine::GraphEdge {
    node_engine::GraphEdge {
        id: edge.id.clone(),
        source: edge.source.clone(),
        source_handle: edge.source_handle.clone(),
        target: edge.target.clone(),
        target_handle: edge.target_handle.clone(),
    }
}

fn convert_graph_from_node_engine(graph: &node_engine::WorkflowGraph) -> WorkflowGraph {
    WorkflowGraph {
        nodes: graph
            .nodes
            .iter()
            .map(|n| GraphNode {
                id: n.id.clone(),
                node_type: n.node_type.clone(),
                position: super::types::Position {
                    x: n.position.0,
                    y: n.position.1,
                },
                data: n.data.clone(),
            })
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|e| GraphEdge {
                id: e.id.clone(),
                source: e.source.clone(),
                source_handle: e.source_handle.clone(),
                target: e.target.clone(),
                target_handle: e.target_handle.clone(),
            })
            .collect(),
        derived_graph: None,
    }
}
