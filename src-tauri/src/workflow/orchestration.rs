//! Tauri commands for orchestration graph management and execution.

use node_engine::{
    DataGraphExecutor, EventSink, OrchestrationEdge, OrchestrationExecutor,
    OrchestrationGraph, OrchestrationNode, OrchestrationNodeType,
    OrchestrationResult, Result as EngineResult, WorkflowGraph,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{command, ipc::Channel, AppHandle, State};
use tokio::sync::RwLock;

use super::events::WorkflowEvent;
use super::SharedExecutionManager;
use crate::agent::rag::SharedRagManager;
use crate::llm::gateway::SharedGateway;

// Re-export types from node_engine for use by other modules
pub use node_engine::{OrchestrationGraphMetadata, OrchestrationStore};

/// Shared orchestration store type.
pub type SharedOrchestrationStore = Arc<RwLock<OrchestrationStore>>;

/// Data graph executor that uses CompositeTaskExecutor (Tauri host + core).
pub struct PantographDataGraphExecutor {
    store: SharedOrchestrationStore,
    gateway: SharedGateway,
    rag_manager: SharedRagManager,
    project_root: PathBuf,
}

impl PantographDataGraphExecutor {
    pub fn new(
        store: SharedOrchestrationStore,
        gateway: SharedGateway,
        rag_manager: SharedRagManager,
        project_root: PathBuf,
    ) -> Self {
        Self {
            store,
            gateway,
            rag_manager,
            project_root,
        }
    }
}

#[async_trait::async_trait]
impl DataGraphExecutor for PantographDataGraphExecutor {
    async fn execute_data_graph(
        &self,
        graph_id: &str,
        inputs: HashMap<String, Value>,
        event_sink: &dyn EventSink,
    ) -> EngineResult<HashMap<String, Value>> {
        use node_engine::{Context, DemandEngine};

        // Get the data graph from the store
        let store = self.store.read().await;
        let graph = store.get_data_graph(graph_id).ok_or_else(|| {
            node_engine::NodeEngineError::failed(format!("Data graph '{}' not found", graph_id))
        })?;

        // Clone what we need before releasing the lock
        let graph = graph.clone();
        drop(store);

        // Create composite task executor: Tauri-specific (rag-search) + core (everything else)
        let core = Arc::new(
            node_engine::CoreTaskExecutor::new()
                .with_project_root(self.project_root.clone())
                .with_gateway(self.gateway.inner_arc()),
        );
        let host = Arc::new(super::task_executor::TauriTaskExecutor::new(
            self.rag_manager.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );

        // Create graph-flow context and demand engine
        let context = Context::new();
        let execution_id = format!("data-graph-{}-{}", graph_id, uuid::Uuid::new_v4());
        let mut demand_engine = DemandEngine::new(&execution_id);

        // Find input nodes (nodes with type "text-input" or that have input port names matching our inputs)
        // and inject the input values into their data
        let mut modified_graph = graph.clone();
        for (port_name, value) in &inputs {
            // Find nodes that might accept this input
            // Strategy 1: Look for input nodes with matching data field
            for node in &mut modified_graph.nodes {
                if node.node_type == "text-input" {
                    // If the input port matches, set the text value
                    if port_name == "text" || port_name == "input" {
                        if let Some(obj) = node.data.as_object_mut() {
                            obj.insert("text".to_string(), value.clone());
                        } else {
                            node.data = serde_json::json!({ "text": value });
                        }
                    }
                }
                // Strategy 2: Inject as node data for any node whose output port matches
                // This allows orchestration to feed data to specific nodes
                if let Some(obj) = node.data.as_object_mut() {
                    obj.insert(format!("_input_{}", port_name), value.clone());
                }
            }
        }

        // Find terminal nodes (nodes with no outgoing edges) - these are our output nodes
        let terminal_nodes: Vec<String> = modified_graph
            .nodes
            .iter()
            .filter(|node| {
                !modified_graph.edges.iter().any(|e| e.source == node.id)
            })
            .map(|n| n.id.clone())
            .collect();

        // Execute the workflow by demanding outputs from terminal nodes
        let mut outputs = HashMap::new();
        for terminal_id in &terminal_nodes {
            // Demand execution from this terminal node
            let extensions = node_engine::ExecutorExtensions::new();
            let result = demand_engine
                .demand(
                    terminal_id,
                    &modified_graph,
                    &task_executor,
                    &context,
                    event_sink,
                    &extensions,
                )
                .await;

            match result {
                Ok(node_outputs) => {
                    // Collect all outputs from this terminal node
                    for (output_port, output_value) in node_outputs {
                        // Use format "nodeId.portName" for disambiguation
                        outputs.insert(
                            format!("{}.{}", terminal_id, output_port),
                            output_value.clone(),
                        );
                        // Also store just the port name for simple access
                        outputs.insert(output_port, output_value);
                    }
                }
                Err(e) => {
                    // Log the error but continue with other terminal nodes
                    log::error!(
                        "Error executing terminal node '{}' in data graph '{}': {}",
                        terminal_id,
                        graph_id,
                        e
                    );
                    outputs.insert(
                        format!("{}.error", terminal_id),
                        Value::String(e.to_string()),
                    );
                }
            }
        }

        // Also include metadata about successful execution
        outputs.insert("_graph_id".to_string(), Value::String(graph_id.to_string()));
        outputs.insert(
            "_terminal_nodes".to_string(),
            Value::Array(terminal_nodes.into_iter().map(Value::String).collect()),
        );

        Ok(outputs)
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
        "e1",
        "start",
        "next",
        "end",
        "input",
    ));

    let mut store = orchestration_store.write().await;
    store.insert_graph(graph.clone()).map_err(|e| e.to_string())?;

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
    store.insert_graph(updated.clone()).map_err(|e| e.to_string())?;
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
    store.insert_graph(updated.clone()).map_err(|e| e.to_string())?;
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
    store.insert_graph(updated.clone()).map_err(|e| e.to_string())?;
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
    store.insert_graph(updated.clone()).map_err(|e| e.to_string())?;
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
    store.insert_graph(updated.clone()).map_err(|e| e.to_string())?;
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
    _app: AppHandle,
    orchestration_id: String,
    initial_data: HashMap<String, Value>,
    orchestration_store: State<'_, SharedOrchestrationStore>,
    gateway: State<'_, SharedGateway>,
    rag_manager: State<'_, SharedRagManager>,
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

    // Get project root from environment or use current directory
    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create the data graph executor with composite task execution
    let data_executor = PantographDataGraphExecutor::new(
        orchestration_store.inner().clone(),
        gateway.inner().clone(),
        rag_manager.inner().clone(),
        project_root,
    );

    // Create the orchestration executor
    let executor = OrchestrationExecutor::new(data_executor);

    // Create an event adapter for the channel
    let event_sink = super::event_adapter::TauriEventAdapter::new(channel, &orchestration_id);

    // Execute the orchestration
    let result = executor
        .execute(&graph, initial_data, &event_sink)
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
