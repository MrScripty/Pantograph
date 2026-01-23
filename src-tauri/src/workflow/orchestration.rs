//! Tauri commands for orchestration graph management and execution.

use node_engine::{
    DataGraphExecutor, EventSink, NullEventSink, OrchestrationEdge, OrchestrationExecutor,
    OrchestrationGraph, OrchestrationNode, OrchestrationNodeType, OrchestrationResult,
    Result as EngineResult, WorkflowGraph,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{command, Channel, State};
use tokio::sync::RwLock;

use super::events::WorkflowEvent;
use super::SharedExecutionManager;
use crate::inference::SharedGateway;
use crate::rag::SharedRagManager;

/// Storage for orchestration graphs.
#[derive(Debug, Default)]
pub struct OrchestrationStore {
    /// Stored orchestration graphs, keyed by ID.
    graphs: HashMap<String, OrchestrationGraph>,
    /// Mapping from data graph node IDs to their workflow graphs.
    data_graphs: HashMap<String, WorkflowGraph>,
}

impl OrchestrationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_graph(&self, id: &str) -> Option<&OrchestrationGraph> {
        self.graphs.get(id)
    }

    pub fn insert_graph(&mut self, graph: OrchestrationGraph) {
        self.graphs.insert(graph.id.clone(), graph);
    }

    pub fn remove_graph(&mut self, id: &str) -> Option<OrchestrationGraph> {
        self.graphs.remove(id)
    }

    pub fn list_graphs(&self) -> Vec<OrchestrationGraphMetadata> {
        self.graphs
            .values()
            .map(|g| OrchestrationGraphMetadata {
                id: g.id.clone(),
                name: g.name.clone(),
                description: g.description.clone(),
                node_count: g.nodes.len(),
            })
            .collect()
    }

    pub fn get_data_graph(&self, id: &str) -> Option<&WorkflowGraph> {
        self.data_graphs.get(id)
    }

    pub fn insert_data_graph(&mut self, id: String, graph: WorkflowGraph) {
        self.data_graphs.insert(id, graph);
    }
}

/// Shared orchestration store type.
pub type SharedOrchestrationStore = Arc<RwLock<OrchestrationStore>>;

/// Metadata for an orchestration graph (for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationGraphMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: usize,
}

/// Data graph executor that uses the PantographTaskExecutor.
pub struct PantographDataGraphExecutor {
    store: SharedOrchestrationStore,
    gateway: SharedGateway,
    rag_manager: SharedRagManager,
    execution_manager: SharedExecutionManager,
    project_root: PathBuf,
}

impl PantographDataGraphExecutor {
    pub fn new(
        store: SharedOrchestrationStore,
        gateway: SharedGateway,
        rag_manager: SharedRagManager,
        execution_manager: SharedExecutionManager,
        project_root: PathBuf,
    ) -> Self {
        Self {
            store,
            gateway,
            rag_manager,
            execution_manager,
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
        // Get the data graph from the store
        let store = self.store.read().await;
        let graph = store.get_data_graph(graph_id).ok_or_else(|| {
            node_engine::NodeEngineError::failed(format!("Data graph '{}' not found", graph_id))
        })?;

        // Clone what we need before releasing the lock
        let graph = graph.clone();
        drop(store);

        // Create a task executor for this data graph
        let task_executor = super::task_executor::PantographTaskExecutor::new(
            self.gateway.clone(),
            self.rag_manager.clone(),
            self.project_root.clone(),
        );

        // Create a workflow executor
        let mut workflow_executor = node_engine::WorkflowExecutor::new(graph.clone(), task_executor);

        // Set initial inputs in the context
        // This maps input port values to the appropriate context keys
        for (port_name, value) in inputs {
            // Find input nodes and set their values
            for node in &graph.nodes {
                let input_key = format!("{}.output.{}", node.id, port_name);
                workflow_executor.set_context_value(&input_key, value.clone());
            }
        }

        // Find terminal nodes (nodes with no outgoing edges) to demand from
        let terminal_nodes: Vec<String> = graph
            .nodes
            .iter()
            .filter(|node| {
                !graph.edges.iter().any(|e| e.source == node.id)
            })
            .map(|n| n.id.clone())
            .collect();

        // Execute the workflow by demanding outputs from terminal nodes
        let mut outputs = HashMap::new();
        for terminal_id in terminal_nodes {
            // Find the node to get its output ports
            if let Some(node) = graph.nodes.iter().find(|n| n.id == terminal_id) {
                // Demand execution
                let result = workflow_executor
                    .demand(&terminal_id, event_sink)
                    .await;

                match result {
                    Ok(_) => {
                        // Collect outputs from the terminal node
                        // For now, collect all context values that match this node's outputs
                        let prefix = format!("{}.output.", terminal_id);
                        // We'd need to iterate the context, but it's private
                        // For now, store a success marker
                        outputs.insert(
                            format!("{}_completed", node.node_type),
                            Value::Bool(true),
                        );
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }

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
    store.insert_graph(graph.clone());

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
    store.insert_graph(graph);
    Ok(())
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
        .graphs
        .get_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    graph.nodes.push(node);
    Ok(graph.clone())
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
        .graphs
        .get_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    // Remove the node
    graph.nodes.retain(|n| n.id != node_id);
    // Remove edges connected to the node
    graph
        .edges
        .retain(|e| e.source != node_id && e.target != node_id);

    Ok(graph.clone())
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
        .graphs
        .get_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    graph.edges.push(edge);
    Ok(graph.clone())
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
        .graphs
        .get_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    graph.edges.retain(|e| e.id != edge_id);
    Ok(graph.clone())
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
        .graphs
        .get_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        node.config = config;
    } else {
        return Err(format!("Node '{}' not found", node_id));
    }

    Ok(graph.clone())
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
        .graphs
        .get_mut(&orchestration_id)
        .ok_or_else(|| format!("Orchestration '{}' not found", orchestration_id))?;

    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        node.position = (x, y);
        Ok(())
    } else {
        Err(format!("Node '{}' not found", node_id))
    }
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
        .graphs
        .get_mut(&orchestration_id)
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
    orchestration_id: String,
    initial_data: HashMap<String, Value>,
    orchestration_store: State<'_, SharedOrchestrationStore>,
    gateway: State<'_, SharedGateway>,
    rag_manager: State<'_, SharedRagManager>,
    execution_manager: State<'_, SharedExecutionManager>,
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

    // Create the data graph executor
    let data_executor = PantographDataGraphExecutor::new(
        orchestration_store.inner().clone(),
        gateway.inner().clone(),
        rag_manager.inner().clone(),
        execution_manager.inner().clone(),
        project_root,
    );

    // Create the orchestration executor
    let executor = OrchestrationExecutor::new(data_executor);

    // Create an event adapter for the channel
    let event_sink = super::event_adapter::TauriEventAdapter::new(channel);

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
