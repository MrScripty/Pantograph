//! Tauri commands for orchestration graph management and execution.

use node_engine::{
    DataGraphExecutor, EventSink, OrchestrationEdge, OrchestrationExecutor, OrchestrationGraph,
    OrchestrationNode, OrchestrationNodeType, OrchestrationResult, Result as EngineResult,
    WorkflowGraph,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, ipc::Channel, AppHandle, State};
use tokio::sync::RwLock;

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::events::WorkflowEvent;
use super::SharedExecutionManager;
use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use pantograph_embedded_runtime::EmbeddedRuntime;

// Re-export types from node_engine for use by other modules
pub use node_engine::{OrchestrationGraphMetadata, OrchestrationStore};

/// Shared orchestration store type.
pub type SharedOrchestrationStore = Arc<RwLock<OrchestrationStore>>;

/// Data graph executor that delegates execution to the backend-owned embedded runtime.
pub struct PantographDataGraphExecutor {
    store: SharedOrchestrationStore,
    runtime: Arc<EmbeddedRuntime>,
    event_sink: Arc<dyn EventSink>,
}

impl PantographDataGraphExecutor {
    pub fn new(
        store: SharedOrchestrationStore,
        runtime: Arc<EmbeddedRuntime>,
        event_sink: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            store,
            runtime,
            event_sink,
        }
    }
}

#[async_trait::async_trait]
impl DataGraphExecutor for PantographDataGraphExecutor {
    async fn execute_data_graph(
        &self,
        graph_id: &str,
        inputs: HashMap<String, Value>,
        _event_sink: &dyn EventSink,
    ) -> EngineResult<HashMap<String, Value>> {
        let store = self.store.read().await;
        let graph = store.get_data_graph(graph_id).ok_or_else(|| {
            node_engine::NodeEngineError::failed(format!("Data graph '{}' not found", graph_id))
        })?;
        let graph = graph.clone();
        drop(store);

        self.runtime
            .execute_data_graph(graph_id, &graph, &inputs, self.event_sink.clone())
            .await
            .map_err(|error| node_engine::NodeEngineError::failed(error.to_string()))
    }

    fn get_data_graph(&self, graph_id: &str) -> Option<WorkflowGraph> {
        // This is sync, so we can't use async. Use try_read instead.
        if let Ok(store) = self.store.try_read() {
            store.get_data_graph(graph_id).cloned()
        } else {
            None
        }
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Create a new orchestration graph.
#[command]
pub async fn create_orchestration(
    name: String,
    description: Option<String>,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let id = format!("orch-{}", uuid::Uuid::new_v4());

    let mut graph = OrchestrationGraph::new(&id, &name);
    graph.description = description.unwrap_or_default();

    // Add default Start and End nodes
    graph.nodes.push(OrchestrationNode::new(
        "start",
        OrchestrationNodeType::Start,
        (100.0, 200.0),
    ));
    graph.nodes.push(OrchestrationNode::new(
        "end",
        OrchestrationNodeType::End,
        (400.0, 200.0),
    ));

    // Add default edge from Start to End
    graph.edges.push(OrchestrationEdge::new(
        "e1", "start", "next", "end", "input",
    ));

    let mut store = orchestration_store.write().await;
    store
        .insert_graph(graph.clone())
        .map_err(|e| e.to_string())?;

    Ok(graph)
}

/// Get an orchestration graph by ID.
#[command]
pub async fn get_orchestration(
    id: String,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let store = orchestration_store.read().await;
    store
        .get_graph(&id)
        .cloned()
        .ok_or_else(|| format!("Orchestration '{}' not found", id))
}

/// List all orchestration graphs.
#[command]
pub async fn list_orchestrations(
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<Vec<OrchestrationGraphMetadata>, String> {
    let store = orchestration_store.read().await;
    Ok(store.list_graphs())
}

/// Save an orchestration graph.
#[command]
pub async fn save_orchestration(
    graph: OrchestrationGraph,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<(), String> {
    let mut store = orchestration_store.write().await;
    store.insert_graph(graph).map_err(|e| e.to_string())
}

/// Delete an orchestration graph.
#[command]
pub async fn delete_orchestration(
    id: String,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<(), String> {
    let mut store = orchestration_store.write().await;
    store
        .remove_graph(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Orchestration '{}' not found", id))?;
    Ok(())
}

/// Add a node to an orchestration graph.
#[command]
pub async fn add_orchestration_node(
    orchestration_id: String,
    node: OrchestrationNode,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    graph.nodes.push(node);
    let updated = graph.clone();

    // Persist the change
    store
        .insert_graph(updated.clone())
        .map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Remove a node from an orchestration graph.
#[command]
pub async fn remove_orchestration_node(
    orchestration_id: String,
    node_id: String,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    // Remove the node
    graph.nodes.retain(|n| n.id != node_id);
    // Remove edges connected to the node
    graph
        .edges
        .retain(|e| e.source != node_id && e.target != node_id);

    let updated = graph.clone();

    // Persist the change
    store
        .insert_graph(updated.clone())
        .map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Add an edge to an orchestration graph.
#[command]
pub async fn add_orchestration_edge(
    orchestration_id: String,
    edge: OrchestrationEdge,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    graph.edges.push(edge);
    let updated = graph.clone();

    // Persist the change
    store
        .insert_graph(updated.clone())
        .map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Remove an edge from an orchestration graph.
#[command]
pub async fn remove_orchestration_edge(
    orchestration_id: String,
    edge_id: String,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    graph.edges.retain(|e| e.id != edge_id);
    let updated = graph.clone();

    // Persist the change
    store
        .insert_graph(updated.clone())
        .map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Update a node's configuration.
#[command]
pub async fn update_orchestration_node(
    orchestration_id: String,
    node_id: String,
    config: Value,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<OrchestrationGraph, String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        node.config = config;
    } else {
        return Err(format!("Node '{}' not found", node_id));
    }

    let updated = graph.clone();

    // Persist the change
    store
        .insert_graph(updated.clone())
        .map_err(|e| e.to_string())?;
    Ok(updated)
}

/// Update a node's position.
#[command]
pub async fn update_orchestration_node_position(
    orchestration_id: String,
    node_id: String,
    x: f64,
    y: f64,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<(), String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        node.position = (x, y);
    } else {
        return Err(format!("Node '{}' not found", node_id));
    }

    let updated = graph.clone();

    // Persist the change
    store.insert_graph(updated).map_err(|e| e.to_string())?;
    Ok(())
}

/// Associate a data graph with a DataGraph node.
#[command]
pub async fn set_orchestration_data_graph(
    orchestration_id: String,
    node_id: String,
    data_graph_id: String,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<(), String> {
    let mut store = orchestration_store.write().await;
    let graph = store
        .get_graph_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    // Verify the node exists and is a DataGraph node
    let node = graph
        .nodes
        .iter()
        .find(|n| n.id == node_id)
        .ok_or_else(|| format!("Node '{}' not found", node_id))?;

    if node.node_type != OrchestrationNodeType::DataGraph {
        return Err(format!("Node '{}' is not a DataGraph node", node_id));
    }

    graph.data_graphs.insert(node_id, data_graph_id);
    let updated = graph.clone();

    // Persist the change
    store.insert_graph(updated).map_err(|e| e.to_string())?;
    Ok(())
}

/// Register a workflow graph as a data graph for use in orchestrations.
#[command]
pub async fn register_data_graph(
    id: String,
    graph: WorkflowGraph,
    orchestration_store: State<'_, SharedOrchestrationStore>,
) -> Result<(), String> {
    let mut store = orchestration_store.write().await;
    store.insert_data_graph(id, graph);
    Ok(())
}

/// Execute an orchestration graph.
#[command]
pub async fn execute_orchestration(
    app: AppHandle,
    orchestration_id: String,
    initial_data: HashMap<String, Value>,
    orchestration_store: State<'_, SharedOrchestrationStore>,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
    _execution_manager: State<'_, SharedExecutionManager>,
    channel: Channel<WorkflowEvent>,
) -> Result<OrchestrationResult, String> {
    // Get the orchestration graph
    let store = orchestration_store.read().await;
    let graph = store
        .get_graph(&orchestration_id)
        .cloned()
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;
    drop(store);

    let runtime = Arc::new(
        super::headless_workflow_commands::build_runtime(
            &app,
            gateway.inner(),
            runtime_registry.inner(),
            extensions.inner(),
            workflow_service.inner(),
            Some(rag_manager.inner()),
        )
        .await?,
    );

    let event_sink = Arc::new(super::event_adapter::TauriEventAdapter::new(
        channel,
        &orchestration_id,
        Arc::new(super::diagnostics::WorkflowDiagnosticsStore::default()),
    ));

    let data_executor = PantographDataGraphExecutor::new(
        orchestration_store.inner().clone(),
        runtime,
        event_sink.clone(),
    );

    let executor = OrchestrationExecutor::new(data_executor);

    let result = executor
        .execute(&graph, initial_data, event_sink.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get the status of orchestration execution.
#[command]
pub fn get_orchestration_node_types() -> Vec<OrchestrationNodeTypeInfo> {
    vec![
        OrchestrationNodeTypeInfo {
            node_type: "start".to_string(),
            label: "Start".to_string(),
            description: "Entry point of the orchestration".to_string(),
            input_handles: vec![],
            output_handles: vec!["next".to_string()],
            category: "control".to_string(),
        },
        OrchestrationNodeTypeInfo {
            node_type: "end".to_string(),
            label: "End".to_string(),
            description: "Exit point of the orchestration".to_string(),
            input_handles: vec!["input".to_string()],
            output_handles: vec![],
            category: "control".to_string(),
        },
        OrchestrationNodeTypeInfo {
            node_type: "condition".to_string(),
            label: "Condition".to_string(),
            description: "Branch based on a boolean condition".to_string(),
            input_handles: vec!["input".to_string()],
            output_handles: vec!["true".to_string(), "false".to_string()],
            category: "control".to_string(),
        },
        OrchestrationNodeTypeInfo {
            node_type: "loop".to_string(),
            label: "Loop".to_string(),
            description: "Iterate with max iterations and exit conditions".to_string(),
            input_handles: vec!["input".to_string(), "loop_back".to_string()],
            output_handles: vec!["iteration".to_string(), "complete".to_string()],
            category: "control".to_string(),
        },
        OrchestrationNodeTypeInfo {
            node_type: "data_graph".to_string(),
            label: "Data Graph".to_string(),
            description: "Execute a data graph workflow".to_string(),
            input_handles: vec!["input".to_string()],
            output_handles: vec!["next".to_string(), "error".to_string()],
            category: "execution".to_string(),
        },
        OrchestrationNodeTypeInfo {
            node_type: "merge".to_string(),
            label: "Merge".to_string(),
            description: "Combine multiple execution paths".to_string(),
            input_handles: vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
            output_handles: vec!["next".to_string()],
            category: "control".to_string(),
        },
    ]
}

/// Information about an orchestration node type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationNodeTypeInfo {
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub input_handles: Vec<String>,
    pub output_handles: Vec<String>,
    pub category: String,
}
