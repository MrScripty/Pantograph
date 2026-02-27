//! Tauri commands for workflow operations
//!
//! These commands expose the workflow engine to the frontend.
//!
//! ## Command Categories
//!
//! - **Execution**: Execute workflows using the node-engine
//! - **Definitions**: Get node definitions for the palette
//! - **Persistence**: Save/load workflows
//! - **Undo/Redo**: Undo/redo graph modifications
//! - **Graph Modification**: Update nodes/edges during execution

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{command, ipc::Channel, AppHandle, State};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::agent::rag::SharedRagManager;
use crate::llm::gateway::SharedGateway;
use node_engine::{resolve_path_within_root, EventSink};

use super::event_adapter::TauriEventAdapter;
use super::events::WorkflowEvent;
use super::execution_manager::{SharedExecutionManager, UndoRedoState};
use super::model_dependencies::SharedModelDependencyResolver;
use super::registry::NodeRegistry;
use super::task_executor::TauriTaskExecutor;
use super::types::{
    GraphEdge, GraphNode, NodeDefinition, PortDataType, WorkflowFile, WorkflowGraph,
    WorkflowMetadata,
};
use super::validation::validate_connection as validate_connection_internal;

/// Shared node-engine registry with port options providers.
pub type SharedNodeRegistry = Arc<node_engine::NodeRegistry>;

/// Shared executor extensions (holds PumasApi etc.).
pub type SharedExtensions = Arc<RwLock<node_engine::ExecutorExtensions>>;

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

/// Get the workflows directory path
fn get_workflows_dir() -> Result<PathBuf, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let workflows_dir = project_root.join(".pantograph").join("workflows");
    fs::create_dir_all(&workflows_dir)
        .map_err(|e| format!("Failed to create workflows directory: {}", e))?;
    Ok(workflows_dir)
}

/// Validate a connection between two port types
///
/// Returns true if the source type can connect to the target type.
/// Used by the frontend to provide real-time connection validation.
#[command]
pub fn validate_workflow_connection(source_type: PortDataType, target_type: PortDataType) -> bool {
    validate_connection_internal(&source_type, &target_type)
}

/// Get all available node definitions
///
/// Returns the complete catalog of node types that can be used
/// in workflow graphs. Used to populate the node palette.
#[command]
pub fn get_node_definitions() -> Vec<NodeDefinition> {
    NodeRegistry::new().all_definitions()
}

/// Get node definitions grouped by category
///
/// Returns node definitions organized by their category (input, processing, etc.)
/// for easier display in the node palette.
#[command]
pub fn get_node_definitions_by_category() -> std::collections::HashMap<String, Vec<NodeDefinition>>
{
    NodeRegistry::new().definitions_by_category()
}

/// Get a single node definition by type
///
/// Returns the definition for a specific node type, or None if not found.
#[command]
pub fn get_node_definition(node_type: String) -> Option<NodeDefinition> {
    NodeRegistry::new().get_definition(&node_type).cloned()
}

// --- Workflow Persistence ---

/// Save a workflow to disk
///
/// Saves the workflow graph with metadata to a JSON file in the workflows directory.
/// Returns the path to the saved file.
#[command]
pub fn save_workflow(name: String, graph: WorkflowGraph) -> Result<String, String> {
    let workflows_dir = get_workflows_dir()?;

    // Sanitize the name for filesystem
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let file_path = workflows_dir.join(format!("{}.json", safe_name));

    // Check if file exists and update modified time, otherwise create new
    let workflow_file = if file_path.exists() {
        // Load existing to preserve created time
        let existing = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read existing workflow: {}", e))?;
        let mut existing: WorkflowFile = serde_json::from_str(&existing)
            .map_err(|e| format!("Failed to parse existing workflow: {}", e))?;

        existing.metadata.name = name;
        existing.metadata.modified = chrono::Utc::now().to_rfc3339();
        existing.graph = graph;
        existing
    } else {
        WorkflowFile::new(name, graph)
    };

    let json = serde_json::to_string_pretty(&workflow_file)
        .map_err(|e| format!("Failed to serialize workflow: {}", e))?;

    fs::write(&file_path, json).map_err(|e| format!("Failed to write workflow file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Load a workflow from disk
///
/// Loads a workflow file from the given path (relative to project root).
#[command]
pub fn load_workflow(path: String) -> Result<WorkflowFile, String> {
    // Resolve path relative to project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let full_path = resolve_path_within_root(&path, &project_root)
        .map_err(|e| format!("Invalid workflow path '{}': {}", path, e))?;

    let content = fs::read_to_string(&full_path)
        .map_err(|e| format!("Failed to read workflow file: {}", e))?;

    let workflow: WorkflowFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow file: {}", e))?;

    Ok(workflow)
}

/// List all saved workflows
///
/// Returns metadata for all workflows in the workflows directory.
#[command]
pub fn list_workflows() -> Result<Vec<WorkflowMetadata>, String> {
    let workflows_dir = get_workflows_dir()?;

    let mut workflows = Vec::new();

    let entries = fs::read_dir(&workflows_dir)
        .map_err(|e| format!("Failed to read workflows directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "json") {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if let Ok(mut workflow) = serde_json::from_str::<WorkflowFile>(&content) {
                        // Extract filename stem as ID for loading
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            workflow.metadata.id = Some(stem.to_string());
                        }
                        workflows.push(workflow.metadata);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read workflow file {:?}: {}", path, e);
                }
            }
        }
    }

    // Sort by modified date descending (most recent first)
    workflows.sort_by(|a, b| b.modified.cmp(&a.modified));

    Ok(workflows)
}

// ==============================================================================
// Node-Engine Based Commands (Phase 5 Integration)
// ==============================================================================

/// Execute a workflow using the node-engine with demand-driven evaluation
///
/// Creates a new execution context and demands outputs from terminal nodes.
/// Returns the execution ID which can be used for subsequent operations.
#[command]
pub async fn execute_workflow_v2(
    _app: AppHandle,
    graph: WorkflowGraph,
    gateway: State<'_, SharedGateway>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
    extensions: State<'_, SharedExtensions>,
    channel: Channel<WorkflowEvent>,
) -> Result<String, String> {
    // Get project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Generate execution ID
    let execution_id = Uuid::new_v4().to_string();

    // Create event adapter
    let event_adapter = Arc::new(TauriEventAdapter::new(channel, &execution_id));

    // Convert Tauri graph to node-engine graph
    let ne_graph = convert_graph_to_node_engine(&graph);

    // Create the execution
    execution_manager
        .create_execution(&execution_id, ne_graph, event_adapter.clone())
        .await;

    // Create composite task executor: Tauri-specific (rag-search) + core (everything else)
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

    // Find terminal nodes (nodes with no outgoing edges)
    let terminal_nodes: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| !graph.edges.iter().any(|e| e.source == node.id))
        .map(|node| node.id.clone())
        .collect();

    // Execute by demanding outputs from terminal nodes
    let runtime_ext = {
        let shared = extensions.read().await;
        snapshot_runtime_extensions(&shared)
    };

    {
        let mut executions = execution_manager.executions().await;
        let state = executions
            .get_mut(&execution_id)
            .ok_or_else(|| "Execution not found".to_string())?;
        state.touch();
        apply_runtime_extensions(&mut state.executor, &runtime_ext);

        // Push initial snapshot for undo
        let _ = state.push_undo_snapshot().await;

        // Send workflow started event
        let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: execution_id.clone(),
            execution_id: execution_id.clone(),
        });

        // Demand from each terminal node
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
                    return Err(e.to_string());
                }
            }
        }

        // Send workflow completed event
        let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: execution_id.clone(),
            execution_id: execution_id.clone(),
        });
    }

    Ok(execution_id)
}

/// Get the undo/redo state for an execution
#[command]
pub async fn get_undo_redo_state(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<UndoRedoState, String> {
    execution_manager
        .get_undo_redo_state(&execution_id)
        .await
        .ok_or_else(|| format!("Execution '{}' not found", execution_id))
}

/// Undo the last graph modification
#[command]
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

/// Redo the last undone graph modification
#[command]
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

/// Update node data during execution
///
/// This marks the node as modified and will trigger re-execution
/// of downstream nodes on the next demand.
#[command]
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

    // Push snapshot before modification
    let _ = state.push_undo_snapshot().await;

    // Update node data
    state
        .executor
        .update_node_data(&node_id, data)
        .await
        .map_err(|e| e.to_string())
}

/// Add a node to the graph during execution
#[command]
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

    // Push snapshot before modification
    let _ = state.push_undo_snapshot().await;

    // Add node
    state.executor.add_node(ne_node).await;
    Ok(())
}

/// Add an edge to the graph during execution
///
/// Returns the updated graph so the frontend can sync its state.
#[command]
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

    // Push snapshot before modification
    let _ = state.push_undo_snapshot().await;

    // Add edge (this marks target as modified)
    state.executor.add_edge(ne_edge).await;

    // Return updated graph
    let graph = state.executor.get_graph_snapshot().await;
    Ok(convert_graph_from_node_engine(&graph))
}

/// Remove an edge from the graph during execution
///
/// Returns the updated graph so the frontend can sync its state.
#[command]
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

    // Push snapshot before modification
    let _ = state.push_undo_snapshot().await;

    // Remove edge (this marks target as modified)
    state.executor.remove_edge(&edge_id).await;

    // Return updated graph
    let graph = state.executor.get_graph_snapshot().await;
    Ok(convert_graph_from_node_engine(&graph))
}

/// Get the current graph state from an execution
#[command]
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

/// Remove an execution from the manager
/// Create a workflow editing session without executing
///
/// This creates an ExecutionState that can be used for editing the graph
/// (adding/removing nodes and edges) with undo/redo support. Nodes will not
/// be executed until `run_workflow_session` is called.
#[command]
pub async fn create_workflow_session(
    graph: WorkflowGraph,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<String, String> {
    // Generate session ID
    let session_id = Uuid::new_v4().to_string();

    // Convert Tauri graph to node-engine graph
    let ne_graph = convert_graph_to_node_engine(&graph);

    // Create execution with NullEventSink (no events during editing)
    let event_sink = Arc::new(node_engine::NullEventSink);
    execution_manager
        .create_execution(&session_id, ne_graph, event_sink)
        .await;

    // Push initial undo snapshot
    {
        let mut executions = execution_manager.executions().await;
        if let Some(state) = executions.get_mut(&session_id) {
            let _ = state.push_undo_snapshot().await;
        }
    }

    Ok(session_id)
}

/// Run an existing workflow session by demanding outputs from terminal nodes
///
/// This takes an existing session (created by `create_workflow_session`) and
/// executes it by demanding outputs from all terminal nodes (nodes with no outgoing edges).
#[command]
pub async fn run_workflow_session(
    _app: AppHandle,
    session_id: String,
    gateway: State<'_, SharedGateway>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
    extensions: State<'_, SharedExtensions>,
    channel: Channel<WorkflowEvent>,
) -> Result<(), String> {
    // Get project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Create event adapter and update the executor's event sink
    let event_adapter = Arc::new(TauriEventAdapter::new(channel, &session_id));

    // Create composite task executor: Tauri-specific (rag-search) + core (everything else)
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

    // Get the graph to find terminal nodes, then execute
    let runtime_ext = {
        let shared = extensions.read().await;
        snapshot_runtime_extensions(&shared)
    };

    let mut executions = execution_manager.executions().await;
    let state = executions
        .get_mut(&session_id)
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    state.touch();
    apply_runtime_extensions(&mut state.executor, &runtime_ext);

    // Update the executor's event sink to use the channel
    state.executor.set_event_sink(event_adapter.clone());

    // Get graph snapshot to find terminal nodes
    let graph = state.executor.get_graph_snapshot().await;
    let terminal_nodes: Vec<String> = graph
        .nodes
        .iter()
        .filter(|node| !graph.edges.iter().any(|e| e.source == node.id))
        .map(|node| node.id.clone())
        .collect();

    // Send workflow started event
    let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowStarted {
        workflow_id: session_id.clone(),
        execution_id: session_id.clone(),
    });

    // Demand from each terminal node
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
                return Err(e.to_string());
            }
        }
    }

    // Send workflow completed event
    let _ = event_adapter.send(node_engine::WorkflowEvent::WorkflowCompleted {
        workflow_id: session_id.clone(),
        execution_id: session_id.clone(),
    });

    Ok(())
}

#[command]
pub async fn remove_execution(
    execution_id: String,
    execution_manager: State<'_, SharedExecutionManager>,
) -> Result<(), String> {
    execution_manager.remove_execution(&execution_id).await;
    Ok(())
}

// ==============================================================================
// Graph Conversion Utilities
// ==============================================================================

/// Convert Tauri WorkflowGraph to node-engine WorkflowGraph
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

/// Convert a Tauri GraphNode to node-engine GraphNode
fn convert_node_to_node_engine(node: &GraphNode) -> node_engine::GraphNode {
    // Include node_type in the data so TaskExecutor can dispatch correctly
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

/// Convert a Tauri GraphEdge to node-engine GraphEdge
fn convert_edge_to_node_engine(edge: &GraphEdge) -> node_engine::GraphEdge {
    node_engine::GraphEdge {
        id: edge.id.clone(),
        source: edge.source.clone(),
        source_handle: edge.source_handle.clone(),
        target: edge.target.clone(),
        target_handle: edge.target_handle.clone(),
    }
}

/// Convert node-engine WorkflowGraph to Tauri WorkflowGraph
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
    }
}

// ==============================================================================
// Port Options Query
// ==============================================================================

/// Query available options for a node's port.
///
/// This is a generic command — any node type that has registered a
/// `PortOptionsProvider` via `inventory` can be queried here. The actual
/// provider logic lives in `workflow-nodes`, not in this Tauri layer.
#[command]
pub async fn query_port_options(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    node_type: String,
    port_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<node_engine::PortOptionsResult, String> {
    let ext = extensions.read().await;
    let query = node_engine::PortOptionsQuery {
        search,
        limit,
        offset,
    };
    registry
        .query_port_options(&node_type, &port_id, &query, &ext)
        .await
        .map_err(|e| e.to_string())
}

/// List all (node_type, port_id) pairs that have options providers registered.
///
/// The frontend can use this to discover which ports support dynamic selection.
#[command]
pub fn get_queryable_ports(registry: State<'_, SharedNodeRegistry>) -> Vec<(String, String)> {
    registry
        .queryable_ports()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p.to_string()))
        .collect()
}

async fn require_pumas_api(
    extensions: &State<'_, SharedExtensions>,
) -> Result<Arc<pumas_library::PumasApi>, String> {
    let ext = extensions.read().await;
    ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
        .cloned()
        .ok_or_else(|| "Pumas API not available in executor extensions".to_string())
}

/// List models that currently require metadata review.
#[command]
pub async fn list_models_needing_review(
    extensions: State<'_, SharedExtensions>,
    filter: Option<pumas_library::model_library::ModelReviewFilter>,
) -> Result<Vec<pumas_library::model_library::ModelReviewItem>, String> {
    let api = require_pumas_api(&extensions).await?;
    api.list_models_needing_review(filter)
        .await
        .map_err(|e| e.to_string())
}

/// Submit a metadata review patch for one model.
#[command]
pub async fn submit_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    patch: serde_json::Value,
    reviewer: String,
    reason: Option<String>,
) -> Result<pumas_library::model_library::SubmitModelReviewResult, String> {
    let api = require_pumas_api(&extensions).await?;
    api.submit_model_review(&model_id, patch, &reviewer, reason.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Reset review edits and restore baseline metadata for a model.
#[command]
pub async fn reset_model_review(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
    reviewer: String,
    reason: Option<String>,
) -> Result<bool, String> {
    let api = require_pumas_api(&extensions).await?;
    api.reset_model_review(&model_id, &reviewer, reason.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// Read effective metadata (`baseline + active overlay`) for a model.
#[command]
pub async fn get_effective_model_metadata(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
) -> Result<Option<pumas_library::models::ModelMetadata>, String> {
    let api = require_pumas_api(&extensions).await?;
    api.get_effective_model_metadata(&model_id)
        .await
        .map_err(|e| e.to_string())
}

fn build_model_dependency_request(
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyRequest, String> {
    fn clean_optional(value: Option<String>) -> Option<String> {
        value.and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    }

    let node_type = node_type.trim().to_string();
    if node_type.is_empty() {
        return Err("node_type is required".to_string());
    }

    let model_path = model_path.trim().to_string();
    if model_path.is_empty() {
        return Err("model_path is required".to_string());
    }

    let mut selected_out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for binding_id in selected_binding_ids.unwrap_or_default() {
        let trimmed = binding_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            selected_out.push(owned);
        }
    }

    Ok(node_engine::ModelDependencyRequest {
        node_type,
        model_path,
        model_id: clean_optional(model_id),
        model_type: clean_optional(model_type),
        task_type_primary: clean_optional(task_type_primary),
        backend_key: clean_optional(backend_key),
        platform_context,
        selected_binding_ids: selected_out,
    })
}

/// Resolve model dependency plan for a model-backed workflow node.
#[command]
pub async fn resolve_model_dependency_plan(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyPlan, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    resolver.resolve_plan_request(request).await
}

/// Check model dependencies for a model-backed workflow node.
#[command]
pub async fn check_model_dependencies(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyStatus, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    resolver.check_request(request).await
}

/// Install dependencies for a model-backed workflow node.
#[command]
pub async fn install_model_dependencies(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyInstallResult, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    resolver.install_request(request).await
}

/// Read the latest cached dependency status, or run a fresh check if absent.
#[command]
pub async fn get_model_dependency_status(
    resolver: State<'_, SharedModelDependencyResolver>,
    node_type: String,
    model_path: String,
    model_id: Option<String>,
    model_type: Option<String>,
    task_type_primary: Option<String>,
    backend_key: Option<String>,
    platform_context: Option<serde_json::Value>,
    selected_binding_ids: Option<Vec<String>>,
) -> Result<node_engine::ModelDependencyStatus, String> {
    let request = build_model_dependency_request(
        node_type,
        model_path,
        model_id,
        model_type,
        task_type_primary,
        backend_key,
        platform_context,
        selected_binding_ids,
    )?;
    if let Some(cached) = resolver.cached_status(&request).await {
        Ok(cached)
    } else {
        resolver.check_request(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_connection() {
        assert!(validate_workflow_connection(
            PortDataType::String,
            PortDataType::String
        ));
        assert!(validate_workflow_connection(
            PortDataType::String,
            PortDataType::Prompt
        ));
        assert!(validate_workflow_connection(
            PortDataType::Any,
            PortDataType::Image
        ));
        assert!(!validate_workflow_connection(
            PortDataType::Image,
            PortDataType::String
        ));
    }

    #[test]
    fn test_get_node_definitions() {
        let defs = get_node_definitions();
        assert!(!defs.is_empty());

        // Check for some expected nodes
        assert!(defs.iter().any(|d| d.node_type == "text-input"));
        assert!(defs.iter().any(|d| d.node_type == "llm-inference"));
        assert!(defs.iter().any(|d| d.node_type == "text-output"));
    }

    #[test]
    fn test_get_node_definitions_by_category() {
        let grouped = get_node_definitions_by_category();

        assert!(grouped.contains_key("input"));
        assert!(grouped.contains_key("processing"));
        assert!(grouped.contains_key("output"));
    }

    #[test]
    fn test_get_node_definition() {
        let def = get_node_definition("text-input".to_string());
        assert!(def.is_some());
        assert_eq!(def.unwrap().node_type, "text-input");

        let missing = get_node_definition("nonexistent".to_string());
        assert!(missing.is_none());
    }

    #[test]
    fn test_build_model_dependency_request_rejects_empty_required_fields() {
        let err = build_model_dependency_request(
            "  ".to_string(),
            "/tmp/model".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("node_type"));

        let err = build_model_dependency_request(
            "pytorch-inference".to_string(),
            " ".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(err.contains("model_path"));
    }

    #[test]
    fn test_build_model_dependency_request_normalizes_optional_and_bindings() {
        let request = build_model_dependency_request(
            " pytorch-inference ".to_string(),
            " /tmp/model ".to_string(),
            Some(" ".to_string()),
            Some(" diffusion ".to_string()),
            Some(" text-to-image ".to_string()),
            Some(" pytorch ".to_string()),
            None,
            Some(vec![
                " binding-a ".to_string(),
                "".to_string(),
                "binding-a".to_string(),
                "binding-b".to_string(),
            ]),
        )
        .unwrap();

        assert_eq!(request.node_type, "pytorch-inference");
        assert_eq!(request.model_path, "/tmp/model");
        assert_eq!(request.model_id, None);
        assert_eq!(request.model_type.as_deref(), Some("diffusion"));
        assert_eq!(request.task_type_primary.as_deref(), Some("text-to-image"));
        assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
        assert_eq!(
            request.selected_binding_ids,
            vec!["binding-a".to_string(), "binding-b".to_string()]
        );
    }
}
