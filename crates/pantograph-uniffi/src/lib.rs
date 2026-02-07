//! UniFFI bindings for Pantograph workflow engine.
//!
//! This crate provides cross-language bindings for the Pantograph node-engine,
//! enabling native access from Python, C#, Swift, Kotlin, Go, and Ruby.
//!
//! # Architecture
//!
//! Types with `serde_json::Value` or `(f64, f64)` fields are wrapped in
//! FFI-safe records. Complex graphs are marshaled as JSON strings at the
//! boundary for maximum flexibility.
//!
//! # Usage
//!
//! ```bash
//! # Build the cdylib
//! cargo build -p pantograph-uniffi --release
//!
//! # Generate Python bindings
//! pantograph-uniffi-bindgen generate --library --language python \
//!     --out-dir ./bindings/python target/release/libpantograph_uniffi.so
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use node_engine::{
    Context, EventSink, OrchestrationGraph, OrchestrationStore, TaskExecutor, WorkflowExecutor,
    WorkflowGraph, WorkflowEvent,
};
use tokio::sync::RwLock;

// UniFFI scaffolding
uniffi::setup_scaffolding!();

// ============================================================================
// Error types
// ============================================================================

/// FFI-friendly error type mapping from NodeEngineError.
#[derive(Debug, Clone, uniffi::Error, thiserror::Error)]
pub enum FfiError {
    #[error("Graph execution error: {message}")]
    GraphFlow { message: String },

    #[error("Missing input: {message}")]
    MissingInput { message: String },

    #[error("Invalid input type: {message}")]
    InvalidInputType { message: String },

    #[error("Execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Context not found: {message}")]
    ContextNotFound { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Compression error: {message}")]
    Compression { message: String },

    #[error("Cancelled")]
    Cancelled,

    #[error("Gateway error: {message}")]
    Gateway { message: String },

    #[error("RAG error: {message}")]
    Rag { message: String },

    #[error("IO error: {message}")]
    Io { message: String },

    #[error("{message}")]
    Other { message: String },
}

impl From<node_engine::NodeEngineError> for FfiError {
    fn from(err: node_engine::NodeEngineError) -> Self {
        use node_engine::NodeEngineError;
        match err {
            NodeEngineError::GraphFlow(msg) => FfiError::GraphFlow { message: msg },
            NodeEngineError::MissingInput(msg) => FfiError::MissingInput { message: msg },
            NodeEngineError::InvalidInputType { port, expected } => FfiError::InvalidInputType {
                message: format!("{}: expected {}", port, expected),
            },
            NodeEngineError::ExecutionFailed(msg) => FfiError::ExecutionFailed { message: msg },
            NodeEngineError::ContextNotFound(msg) => FfiError::ContextNotFound { message: msg },
            NodeEngineError::Serialization(err) => FfiError::Serialization {
                message: err.to_string(),
            },
            NodeEngineError::Compression(msg) => FfiError::Compression { message: msg },
            NodeEngineError::Cancelled => FfiError::Cancelled,
            NodeEngineError::Gateway(msg) => FfiError::Gateway { message: msg },
            NodeEngineError::Rag(msg) => FfiError::Rag { message: msg },
            NodeEngineError::Io(err) => FfiError::Io {
                message: err.to_string(),
            },
        }
    }
}

pub type FfiResult<T> = Result<T, FfiError>;

// ============================================================================
// FFI Wrapper Records
// ============================================================================

/// FFI-safe representation of a graph node.
#[derive(uniffi::Record)]
pub struct FfiGraphNode {
    pub id: String,
    pub node_type: String,
    pub position_x: f64,
    pub position_y: f64,
    /// Node data as JSON string (from serde_json::Value)
    pub data_json: String,
}

/// FFI-safe representation of a graph edge.
#[derive(uniffi::Record)]
pub struct FfiGraphEdge {
    pub id: String,
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
}

/// FFI-safe representation of a workflow graph.
#[derive(uniffi::Record)]
pub struct FfiWorkflowGraph {
    pub id: String,
    pub name: String,
    pub nodes: Vec<FfiGraphNode>,
    pub edges: Vec<FfiGraphEdge>,
}

impl From<WorkflowGraph> for FfiWorkflowGraph {
    fn from(g: WorkflowGraph) -> Self {
        Self {
            id: g.id.clone(),
            name: g.name.clone(),
            nodes: g
                .nodes
                .iter()
                .map(|n| FfiGraphNode {
                    id: n.id.clone(),
                    node_type: n.node_type.clone(),
                    position_x: n.position.0,
                    position_y: n.position.1,
                    data_json: n.data.to_string(),
                })
                .collect(),
            edges: g
                .edges
                .iter()
                .map(|e| FfiGraphEdge {
                    id: e.id.clone(),
                    source: e.source.clone(),
                    source_handle: e.source_handle.clone(),
                    target: e.target.clone(),
                    target_handle: e.target_handle.clone(),
                })
                .collect(),
        }
    }
}

/// FFI-safe cache statistics.
#[derive(uniffi::Record)]
pub struct FfiCacheStats {
    pub cached_nodes: u64,
    pub total_versions: u64,
    pub global_version: u64,
}

/// FFI-safe orchestration metadata.
#[derive(uniffi::Record)]
pub struct FfiOrchestrationMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: u64,
}

/// FFI-safe workflow event.
#[derive(uniffi::Record)]
pub struct FfiWorkflowEvent {
    /// Event type identifier
    pub event_type: String,
    /// Full event data as JSON
    pub event_json: String,
}

// ============================================================================
// Simple TaskExecutor for UniFFI (synchronous JSON-based)
// ============================================================================

/// A no-op TaskExecutor for use when the host language handles execution
/// through the graph snapshot mechanism rather than callbacks.
struct NoopTaskExecutor;

#[async_trait::async_trait]
impl TaskExecutor for NoopTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        _inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &node_engine::ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        Err(node_engine::NodeEngineError::ExecutionFailed(format!(
            "No executor configured for task '{}'",
            task_id
        )))
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Get the version of the pantograph-uniffi bindings.
#[uniffi::export]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Validate a workflow graph JSON string, returning error messages.
#[uniffi::export]
pub fn validate_workflow_json(graph_json: String) -> Result<Vec<String>, FfiError> {
    let graph: WorkflowGraph = serde_json::from_str(&graph_json).map_err(|e| FfiError::Serialization {
        message: e.to_string(),
    })?;
    let errors = node_engine::validation::validate_workflow(&graph, None);
    Ok(errors.iter().map(|e| e.to_string()).collect())
}

/// Validate an orchestration graph JSON string, returning error messages.
#[uniffi::export]
pub fn validate_orchestration_json(graph_json: String) -> Result<Vec<String>, FfiError> {
    let graph: OrchestrationGraph = serde_json::from_str(&graph_json).map_err(|e| {
        FfiError::Serialization {
            message: e.to_string(),
        }
    })?;
    let errors = node_engine::validation::validate_orchestration(&graph);
    Ok(errors.iter().map(|e| e.to_string()).collect())
}

// ============================================================================
// FfiWorkflowEngine - Main workflow engine object
// ============================================================================

/// The main Pantograph workflow engine handle.
///
/// Wraps a `WorkflowExecutor` for graph CRUD, demand-driven execution,
/// and event collection.
///
/// # Example (Python)
///
/// ```python
/// engine = FfiWorkflowEngine("wf-1", "My Workflow")
/// engine.add_node("n1", "text-input", 0.0, 0.0, "{}")
/// engine.add_node("n2", "text-output", 200.0, 0.0, "{}")
/// engine.add_edge("n1", "text", "n2", "text")
/// graph = engine.get_graph()
/// ```
#[derive(uniffi::Object)]
pub struct FfiWorkflowEngine {
    executor: Arc<RwLock<WorkflowExecutor>>,
    task_executor: Arc<dyn TaskExecutor>,
    event_buffer: Arc<RwLock<Vec<FfiWorkflowEvent>>>,
}

/// Callback EventSink that buffers events for polling.
struct BufferedEventSink {
    buffer: Arc<RwLock<Vec<FfiWorkflowEvent>>>,
}

impl EventSink for BufferedEventSink {
    fn send(
        &self,
        event: WorkflowEvent,
    ) -> std::result::Result<(), node_engine::EventError> {
        let event_type = format!("{:?}", event).split('(').next().unwrap_or("Unknown").to_string();
        let event_json = serde_json::to_string(&event)
            .map_err(|e| node_engine::EventError { message: e.to_string() })?;

        // Use try_write to avoid blocking in sync context
        if let Ok(mut buf) = self.buffer.try_write() {
            buf.push(FfiWorkflowEvent {
                event_type,
                event_json,
            });
        }
        Ok(())
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiWorkflowEngine {
    /// Create a new workflow engine with an empty graph.
    #[uniffi::constructor]
    pub fn new(id: String, name: String) -> Arc<Self> {
        let graph = WorkflowGraph::new(&id, &name);
        let event_buffer = Arc::new(RwLock::new(Vec::new()));
        let event_sink: Arc<dyn EventSink> = Arc::new(BufferedEventSink {
            buffer: event_buffer.clone(),
        });
        let executor = WorkflowExecutor::new("uniffi-execution", graph, event_sink);

        Arc::new(Self {
            executor: Arc::new(RwLock::new(executor)),
            task_executor: Arc::new(NoopTaskExecutor),
            event_buffer,
        })
    }

    /// Create from a JSON-serialized workflow graph.
    #[uniffi::constructor]
    pub fn from_json(graph_json: String) -> Result<Arc<Self>, FfiError> {
        let graph: WorkflowGraph = serde_json::from_str(&graph_json).map_err(|e| {
            FfiError::Serialization {
                message: e.to_string(),
            }
        })?;
        let event_buffer = Arc::new(RwLock::new(Vec::new()));
        let event_sink: Arc<dyn EventSink> = Arc::new(BufferedEventSink {
            buffer: event_buffer.clone(),
        });
        let executor = WorkflowExecutor::new("uniffi-execution", graph, event_sink);

        Ok(Arc::new(Self {
            executor: Arc::new(RwLock::new(executor)),
            task_executor: Arc::new(NoopTaskExecutor),
            event_buffer,
        }))
    }

    // ============================
    // Graph CRUD
    // ============================

    /// Add a node to the graph.
    pub async fn add_node(
        &self,
        id: String,
        node_type: String,
        x: f64,
        y: f64,
        data_json: String,
    ) -> Result<(), FfiError> {
        let data: serde_json::Value =
            serde_json::from_str(&data_json).unwrap_or(serde_json::Value::Null);

        let node = node_engine::GraphNode {
            id,
            node_type,
            position: (x, y),
            data,
        };

        let exec = self.executor.read().await;
        exec.add_node(node).await;
        Ok(())
    }

    /// Add an edge to the graph.
    pub async fn add_edge(
        &self,
        source: String,
        source_handle: String,
        target: String,
        target_handle: String,
    ) -> Result<(), FfiError> {
        let edge_id = format!("e-{}-{}-{}-{}", source, source_handle, target, target_handle);
        let edge = node_engine::GraphEdge {
            id: edge_id,
            source,
            source_handle,
            target,
            target_handle,
        };

        let exec = self.executor.read().await;
        exec.add_edge(edge).await;
        Ok(())
    }

    /// Remove an edge by ID.
    pub async fn remove_edge(&self, edge_id: String) -> Result<(), FfiError> {
        let exec = self.executor.read().await;
        exec.remove_edge(&edge_id).await;
        Ok(())
    }

    /// Update a node's data.
    pub async fn update_node_data(&self, node_id: String, data_json: String) -> Result<(), FfiError> {
        let data: serde_json::Value =
            serde_json::from_str(&data_json).unwrap_or(serde_json::Value::Null);

        let exec = self.executor.read().await;
        exec.update_node_data(&node_id, data)
            .await
            .map_err(FfiError::from)
    }

    // ============================
    // Query
    // ============================

    /// Get the current graph state.
    pub async fn get_graph(&self) -> FfiWorkflowGraph {
        let exec = self.executor.read().await;
        let snapshot = exec.get_graph_snapshot().await;
        FfiWorkflowGraph::from(snapshot)
    }

    /// Export the graph as a JSON string.
    pub async fn export_graph_json(&self) -> Result<String, FfiError> {
        let exec = self.executor.read().await;
        let snapshot = exec.get_graph_snapshot().await;
        serde_json::to_string(&snapshot).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> FfiCacheStats {
        let exec = self.executor.read().await;
        let stats = exec.cache_stats().await;
        FfiCacheStats {
            cached_nodes: stats.cached_nodes as u64,
            total_versions: stats.total_versions as u64,
            global_version: stats.global_version,
        }
    }

    // ============================
    // Execution
    // ============================

    /// Mark a node as modified (invalidates caches).
    pub async fn mark_modified(&self, node_id: String) {
        let exec = self.executor.read().await;
        exec.mark_modified(&node_id).await;
    }

    // ============================
    // Events
    // ============================

    /// Drain all buffered events since last call.
    pub async fn drain_events(&self) -> Vec<FfiWorkflowEvent> {
        let mut buffer = self.event_buffer.write().await;
        std::mem::take(&mut *buffer)
    }
}

// ============================================================================
// FfiOrchestrationStore - Orchestration graph storage
// ============================================================================

/// Persistent orchestration graph store.
///
/// Manages orchestration graphs in memory with optional file persistence.
#[derive(uniffi::Object)]
pub struct FfiOrchestrationStore {
    store: Arc<RwLock<OrchestrationStore>>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiOrchestrationStore {
    /// Create a new in-memory store.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Arc::new(RwLock::new(OrchestrationStore::new())),
        })
    }

    /// Create a store with file persistence.
    #[uniffi::constructor]
    pub fn with_persistence(path: String) -> Arc<Self> {
        Arc::new(Self {
            store: Arc::new(RwLock::new(OrchestrationStore::with_persistence(path))),
        })
    }

    /// List all orchestration graph metadata.
    pub async fn list_graphs(&self) -> Vec<FfiOrchestrationMetadata> {
        let guard = self.store.read().await;
        guard
            .list_graphs()
            .into_iter()
            .map(|m| FfiOrchestrationMetadata {
                id: m.id,
                name: m.name,
                description: m.description,
                node_count: m.node_count as u64,
            })
            .collect()
    }

    /// Insert an orchestration graph (as JSON).
    pub async fn insert_graph(&self, graph_json: String) -> Result<(), FfiError> {
        let graph: OrchestrationGraph = serde_json::from_str(&graph_json).map_err(|e| {
            FfiError::Serialization {
                message: e.to_string(),
            }
        })?;
        let mut guard = self.store.write().await;
        guard.insert_graph(graph).map_err(FfiError::from)
    }

    /// Get an orchestration graph by ID (as JSON).
    pub async fn get_graph(&self, graph_id: String) -> Option<String> {
        let guard = self.store.read().await;
        guard
            .get_graph(&graph_id)
            .and_then(|g| serde_json::to_string(g).ok())
    }

    /// Remove an orchestration graph by ID.
    pub async fn remove_graph(&self, graph_id: String) -> Result<(), FfiError> {
        let mut guard = self.store.write().await;
        guard.remove_graph(&graph_id).map_err(FfiError::from)?;
        Ok(())
    }
}

// ============================================================================
// FfiPumasApi - Model Library API
// ============================================================================

/// Pumas model library API for model management, HuggingFace search,
/// downloads, and imports.
#[derive(uniffi::Object)]
pub struct FfiPumasApi {
    api: Arc<pumas_library::PumasApi>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiPumasApi {
    /// Create a new PumasApi instance.
    ///
    /// `launcher_root` is the root directory for the pumas library.
    #[uniffi::constructor]
    pub async fn new(launcher_root: String) -> Result<Arc<Self>, FfiError> {
        let api = pumas_library::PumasApi::builder(&launcher_root)
            .auto_create_dirs(true)
            .with_hf_client(true)
            .with_process_manager(false)
            .build()
            .await
            .map_err(|e| FfiError::Other {
                message: format!("PumasApi init error: {}", e),
            })?;

        Ok(Arc::new(Self {
            api: Arc::new(api),
        }))
    }

    // --- Local library ---

    /// List all models in the local library. Returns JSON array of ModelRecord.
    pub async fn list_models(&self) -> Result<String, FfiError> {
        let models = self.api.list_models().await.map_err(|e| FfiError::Other {
            message: e.to_string(),
        })?;
        serde_json::to_string(&models).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Search the local model library. Returns JSON SearchResult.
    pub async fn search_models(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<String, FfiError> {
        let result = self
            .api
            .search_models(&query, limit as usize, offset as usize)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&result).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Get a single model by ID. Returns JSON ModelRecord or None.
    pub async fn get_model(&self, model_id: String) -> Result<Option<String>, FfiError> {
        let model = self
            .api
            .get_model(&model_id)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        match model {
            Some(m) => {
                let json = serde_json::to_string(&m).map_err(|e| FfiError::Serialization {
                    message: e.to_string(),
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    // --- HuggingFace ---

    /// Search HuggingFace for models. Returns JSON array of HuggingFaceModel.
    pub async fn search_hf(
        &self,
        query: String,
        kind: Option<String>,
        limit: u32,
    ) -> Result<String, FfiError> {
        let models = self
            .api
            .search_hf_models(&query, kind.as_deref(), limit as usize)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&models).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Get file tree for a HuggingFace repo. Returns JSON RepoFileTree.
    pub async fn get_repo_files(&self, repo_id: String) -> Result<String, FfiError> {
        let tree = self
            .api
            .get_hf_repo_files(&repo_id)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&tree).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    // --- Download ---

    /// Start a model download. `request_json` is a JSON DownloadRequest.
    /// Returns the download ID.
    pub async fn start_download(&self, request_json: String) -> Result<String, FfiError> {
        let request: pumas_library::model_library::DownloadRequest =
            serde_json::from_str(&request_json).map_err(|e| FfiError::Serialization {
                message: e.to_string(),
            })?;
        self.api
            .start_hf_download(&request)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })
    }

    /// Get download progress. Returns JSON ModelDownloadProgress or None.
    pub async fn get_download_progress(
        &self,
        download_id: String,
    ) -> Result<Option<String>, FfiError> {
        let progress = self.api.get_hf_download_progress(&download_id).await;
        match progress {
            Some(p) => {
                let json = serde_json::to_string(&p).map_err(|e| FfiError::Serialization {
                    message: e.to_string(),
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    }

    /// Cancel a download. Returns true if cancelled.
    pub async fn cancel_download(&self, download_id: String) -> Result<bool, FfiError> {
        self.api
            .cancel_hf_download(&download_id)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })
    }

    // --- Import ---

    /// Import a model. `spec_json` is a JSON ModelImportSpec.
    /// Returns JSON ModelImportResult.
    pub async fn import_model(&self, spec_json: String) -> Result<String, FfiError> {
        let spec: pumas_library::model_library::ModelImportSpec =
            serde_json::from_str(&spec_json).map_err(|e| FfiError::Serialization {
                message: e.to_string(),
            })?;
        let result = self
            .api
            .import_model(&spec)
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&result).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    // --- System ---

    /// Get disk space info. Returns JSON DiskSpaceResponse.
    pub async fn get_disk_space(&self) -> Result<String, FfiError> {
        let info = self
            .api
            .get_disk_space()
            .await
            .map_err(|e| FfiError::Other {
                message: e.to_string(),
            })?;
        serde_json::to_string(&info).map_err(|e| FfiError::Serialization {
            message: e.to_string(),
        })
    }

    /// Check if Ollama is running.
    pub async fn is_ollama_running(&self) -> bool {
        self.api.is_ollama_running().await
    }
}

impl FfiPumasApi {
    fn api_arc(&self) -> Arc<pumas_library::PumasApi> {
        self.api.clone()
    }
}

/// Inject PumasApi into a workflow engine's extensions.
#[uniffi::export(async_runtime = "tokio")]
impl FfiWorkflowEngine {
    /// Set a PumasApi on this engine for model resolution in workflow nodes.
    pub async fn set_pumas_api(&self, api: Arc<FfiPumasApi>) {
        let mut exec = self.executor.write().await;
        exec.extensions_mut()
            .set(node_engine::extension_keys::PUMAS_API, api.api_arc());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_ffi_error_conversion() {
        let err = node_engine::NodeEngineError::ExecutionFailed("test".to_string());
        let ffi_err: FfiError = err.into();
        assert!(matches!(ffi_err, FfiError::ExecutionFailed { .. }));
    }

    #[test]
    fn test_ffi_error_cancelled() {
        let err = node_engine::NodeEngineError::Cancelled;
        let ffi_err: FfiError = err.into();
        assert!(matches!(ffi_err, FfiError::Cancelled));
    }

    #[test]
    fn test_ffi_graph_conversion() {
        let graph = WorkflowGraph::new("test", "Test Graph");
        let ffi = FfiWorkflowGraph::from(graph);
        assert_eq!(ffi.id, "test");
        assert_eq!(ffi.name, "Test Graph");
        assert!(ffi.nodes.is_empty());
        assert!(ffi.edges.is_empty());
    }

    #[test]
    fn test_validate_empty_workflow() {
        let graph = WorkflowGraph::new("test", "Test");
        let json = serde_json::to_string(&graph).unwrap();
        let errors = validate_workflow_json(json).unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_workflow_engine_new() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        let graph = engine.get_graph().await;
        assert_eq!(graph.id, "wf-1");
        assert_eq!(graph.name, "Test");
    }

    #[tokio::test]
    async fn test_workflow_engine_add_node() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        engine
            .add_node(
                "n1".to_string(),
                "text-input".to_string(),
                0.0,
                0.0,
                "{}".to_string(),
            )
            .await
            .unwrap();

        let graph = engine.get_graph().await;
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].id, "n1");
    }

    #[tokio::test]
    async fn test_workflow_engine_export_json() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        let json = engine.export_graph_json().await.unwrap();
        assert!(json.contains("wf-1"));
    }

    #[tokio::test]
    async fn test_orchestration_store() {
        let store = FfiOrchestrationStore::new();
        let list = store.list_graphs().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_drain_events_empty() {
        let engine = FfiWorkflowEngine::new("wf-1".to_string(), "Test".to_string());
        let events = engine.drain_events().await;
        assert!(events.is_empty());
    }
}
