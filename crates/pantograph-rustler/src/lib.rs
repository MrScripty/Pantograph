//! Rustler NIFs for Pantograph workflow engine.
//!
//! This crate provides Elixir/Erlang bindings for the Pantograph node-engine
//! via Rustler NIFs (Native Implemented Functions).
//!
//! # Architecture
//!
//! Complex types (WorkflowGraph, GraphNode, etc.) are marshaled as JSON strings
//! across the NIF boundary, since their `serde_json::Value` fields are incompatible
//! with NIF struct derivation.
//!
//! Stateful objects (WorkflowExecutor, OrchestrationStore) are wrapped in
//! `ResourceArc` for safe sharing between NIF calls.
//!
//! Node execution is bridged back to the BEAM via a callback mechanism:
//! 1. `ElixirCallbackTaskExecutor` implements `TaskExecutor`
//! 2. On `execute_task`: stores a `oneshot::Sender` in `PENDING_CALLBACKS`
//! 3. Sends `{:node_execute, callback_id, task_id, inputs_json}` to an Elixir PID
//! 4. Elixir handles the node, then calls `callback_respond/2` NIF
//! 5. The oneshot channel unblocks, execution continues
//!
//! # Usage in Elixir
//!
//! ```elixir
//! defmodule Pantograph.Native do
//!   use Rustler, otp_app: :pantograph, crate: "pantograph_rustler"
//!
//!   def version(), do: :erlang.nif_error(:nif_not_loaded)
//!   def workflow_new(_id, _name), do: :erlang.nif_error(:nif_not_loaded)
//!   # ... etc
//! end
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rustler::{Atom, Encoder, Env, NifResult, NifStruct, NifUnitEnum, OwnedEnv, ResourceArc, Term};
use tokio::sync::oneshot;

use node_engine::{
    EventSink, OrchestrationGraph, OrchestrationStore, TaskExecutor, WorkflowExecutor,
    WorkflowGraph,
};

// ============================================================================
// Atoms
// ============================================================================

mod atoms {
    rustler::atoms! {
        ok,
        error,
        node_execute,
        workflow_event,
    }
}

// ============================================================================
// NIF Enums
// ============================================================================

/// Port data type enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirPortDataType {
    String,
    Number,
    Boolean,
    Json,
    Image,
    Audio,
    Video,
    Embedding,
    Document,
    Binary,
    Any,
}

/// Node category enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirNodeCategory {
    Input,
    Output,
    Processing,
    Control,
    Storage,
    Integration,
}

/// Execution mode enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirExecutionMode {
    Reactive,
    Manual,
    Stream,
}

/// Orchestration node type enum for Elixir.
#[derive(NifUnitEnum)]
pub enum ElixirOrchestrationNodeType {
    Start,
    End,
    DataGraph,
    Condition,
    Loop,
    Merge,
}

// ============================================================================
// NIF Structs
// ============================================================================

/// Node definition struct for Elixir (metadata about a node type).
#[derive(NifStruct)]
#[module = "Pantograph.NodeDefinition"]
pub struct ElixirNodeDefinition {
    pub node_type: String,
    pub category: ElixirNodeCategory,
    pub label: String,
    pub description: String,
    pub input_count: u32,
    pub output_count: u32,
    pub execution_mode: ElixirExecutionMode,
}

/// Cache statistics struct for Elixir.
#[derive(NifStruct)]
#[module = "Pantograph.CacheStats"]
pub struct ElixirCacheStats {
    pub cached_nodes: u32,
    pub total_versions: u32,
    pub global_version: u64,
}

/// Orchestration graph metadata for Elixir.
#[derive(NifStruct)]
#[module = "Pantograph.OrchestrationMetadata"]
pub struct ElixirOrchestrationMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: u32,
}

// ============================================================================
// Resource types (stateful objects shared across NIF calls)
// ============================================================================

/// Wrapper for WorkflowExecutor shared via ResourceArc.
pub struct WorkflowExecutorResource {
    pub executor: Arc<tokio::sync::RwLock<WorkflowExecutor>>,
    pub task_executor: Arc<dyn TaskExecutor>,
    pub runtime: Arc<tokio::runtime::Runtime>,
}

/// Wrapper for OrchestrationStore shared via ResourceArc.
pub struct OrchestrationStoreResource {
    pub store: Arc<tokio::sync::RwLock<OrchestrationStore>>,
}

/// Wrapper for NodeRegistry shared via ResourceArc.
pub struct NodeRegistryResource {
    pub registry: Arc<tokio::sync::RwLock<node_engine::NodeRegistry>>,
}

/// Pending callback channels for bridging node execution to BEAM.
static PENDING_CALLBACKS: std::sync::LazyLock<
    Mutex<HashMap<String, oneshot::Sender<Result<String, String>>>>,
> = std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Counter for generating unique callback IDs.
static CALLBACK_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

// ============================================================================
// Elixir callback-based TaskExecutor
// ============================================================================

/// TaskExecutor that bridges node execution to Elixir via callback NIFs.
pub struct ElixirCallbackTaskExecutor {
    pid: rustler::LocalPid,
    owned_env: Arc<Mutex<OwnedEnv>>,
    timeout_secs: u64,
}

impl ElixirCallbackTaskExecutor {
    pub fn new(pid: rustler::LocalPid) -> Self {
        Self {
            pid,
            owned_env: Arc::new(Mutex::new(OwnedEnv::new())),
            timeout_secs: 300,
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

#[async_trait::async_trait]
impl TaskExecutor for ElixirCallbackTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let callback_id = format!(
            "cb-{}",
            CALLBACK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );

        let (tx, rx) = oneshot::channel();

        // Store the sender
        {
            let mut callbacks = PENDING_CALLBACKS.lock().map_err(|e| {
                node_engine::NodeEngineError::ExecutionFailed(format!("Lock poisoned: {}", e))
            })?;
            callbacks.insert(callback_id.clone(), tx);
        }

        // Serialize inputs to JSON
        let inputs_json = serde_json::to_string(&inputs)?;

        // Send message to Elixir PID — must drop MutexGuard before await
        let pid = self.pid;
        let cb_id = callback_id.clone();
        let t_id = task_id.to_string();
        {
            let mut env = self.owned_env.lock().map_err(|e| {
                node_engine::NodeEngineError::ExecutionFailed(format!("Env lock poisoned: {}", e))
            })?;

            let _ = env.send_and_clear(&pid, |env| {
                let msg = (
                    atoms::node_execute().encode(env),
                    cb_id.encode(env),
                    t_id.encode(env),
                    inputs_json.encode(env),
                );
                msg.encode(env)
            });
        } // MutexGuard dropped here, before the await

        // Wait for response with timeout
        let result = tokio::time::timeout(std::time::Duration::from_secs(self.timeout_secs), rx)
            .await
            .map_err(|_| {
                // Clean up on timeout
                let mut callbacks = PENDING_CALLBACKS.lock().unwrap_or_else(|e| e.into_inner());
                callbacks.remove(&callback_id);
                node_engine::NodeEngineError::ExecutionFailed(format!(
                    "Callback timeout for task '{}'",
                    task_id
                ))
            })?
            .map_err(|_| {
                node_engine::NodeEngineError::ExecutionFailed(format!(
                    "Callback channel dropped for task '{}'",
                    task_id
                ))
            })?;

        match result {
            Ok(json_str) => {
                let outputs: HashMap<String, serde_json::Value> =
                    serde_json::from_str(&json_str)?;
                Ok(outputs)
            }
            Err(err_msg) => Err(node_engine::NodeEngineError::ExecutionFailed(err_msg)),
        }
    }
}

/// EventSink that sends events to an Elixir PID.
pub struct BeamEventSink {
    pid: rustler::LocalPid,
    owned_env: Arc<Mutex<OwnedEnv>>,
}

impl BeamEventSink {
    pub fn new(pid: rustler::LocalPid) -> Self {
        Self {
            pid,
            owned_env: Arc::new(Mutex::new(OwnedEnv::new())),
        }
    }
}

impl EventSink for BeamEventSink {
    fn send(
        &self,
        event: node_engine::WorkflowEvent,
    ) -> std::result::Result<(), node_engine::EventError> {
        let json = serde_json::to_string(&event).map_err(|e| node_engine::EventError {
            message: format!("Serialization error: {}", e),
        })?;

        let pid = self.pid;
        let mut env = self.owned_env.lock().map_err(|e| node_engine::EventError {
            message: format!("Lock poisoned: {}", e),
        })?;

        let _ = env.send_and_clear(&pid, |env| {
            (atoms::workflow_event().encode(env), json.encode(env)).encode(env)
        });

        Ok(())
    }
}

// ============================================================================
// NIF Functions - Version
// ============================================================================

/// Get the version of the pantograph-rustler bindings.
#[rustler::nif]
fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// NIF Functions - Type Parsing
// ============================================================================

/// Parse a port data type string to enum.
#[rustler::nif]
fn parse_port_data_type(type_str: String) -> ElixirPortDataType {
    match type_str.to_lowercase().as_str() {
        "string" => ElixirPortDataType::String,
        "number" => ElixirPortDataType::Number,
        "boolean" => ElixirPortDataType::Boolean,
        "json" => ElixirPortDataType::Json,
        "image" => ElixirPortDataType::Image,
        "audio" => ElixirPortDataType::Audio,
        "video" => ElixirPortDataType::Video,
        "embedding" => ElixirPortDataType::Embedding,
        "document" => ElixirPortDataType::Document,
        "binary" => ElixirPortDataType::Binary,
        _ => ElixirPortDataType::Any,
    }
}

/// Parse a node category string to enum.
#[rustler::nif]
fn parse_node_category(category_str: String) -> ElixirNodeCategory {
    match category_str.to_lowercase().as_str() {
        "input" => ElixirNodeCategory::Input,
        "output" => ElixirNodeCategory::Output,
        "processing" => ElixirNodeCategory::Processing,
        "control" => ElixirNodeCategory::Control,
        "storage" => ElixirNodeCategory::Storage,
        _ => ElixirNodeCategory::Integration,
    }
}

/// Parse an execution mode string to enum.
#[rustler::nif]
fn parse_execution_mode(mode_str: String) -> ElixirExecutionMode {
    match mode_str.to_lowercase().as_str() {
        "manual" => ElixirExecutionMode::Manual,
        "stream" => ElixirExecutionMode::Stream,
        _ => ElixirExecutionMode::Reactive,
    }
}

// ============================================================================
// NIF Functions - Workflow CRUD (JSON marshaling)
// ============================================================================

/// Create a new empty workflow graph, returned as JSON.
#[rustler::nif]
fn workflow_new(id: String, name: String) -> NifResult<String> {
    let graph = WorkflowGraph::new(&id, &name);
    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Parse a JSON string into a WorkflowGraph and re-serialize (validates structure).
#[rustler::nif]
fn workflow_from_json(json: String) -> NifResult<String> {
    let graph: WorkflowGraph = serde_json::from_str(&json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;
    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Add a node to a workflow graph (JSON in, JSON out).
#[rustler::nif]
fn workflow_add_node(
    graph_json: String,
    node_id: String,
    node_type: String,
    x: f64,
    y: f64,
    data_json: String,
) -> NifResult<String> {
    let mut graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let data: serde_json::Value = serde_json::from_str(&data_json).unwrap_or_default();

    let node = node_engine::GraphNode {
        id: node_id,
        node_type,
        position: (x, y),
        data,
    };
    graph.nodes.push(node);

    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Remove a node from a workflow graph.
#[rustler::nif]
fn workflow_remove_node(graph_json: String, node_id: String) -> NifResult<String> {
    let mut graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    graph.nodes.retain(|n| n.id != node_id);
    graph
        .edges
        .retain(|e| e.source != node_id && e.target != node_id);

    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Add an edge to a workflow graph.
#[rustler::nif]
fn workflow_add_edge(
    graph_json: String,
    source: String,
    source_handle: String,
    target: String,
    target_handle: String,
) -> NifResult<String> {
    let mut graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let edge_id = format!("e-{}-{}-{}-{}", source, source_handle, target, target_handle);
    let edge = node_engine::GraphEdge {
        id: edge_id,
        source,
        source_handle,
        target,
        target_handle,
    };
    graph.edges.push(edge);

    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Remove an edge from a workflow graph.
#[rustler::nif]
fn workflow_remove_edge(graph_json: String, edge_id: String) -> NifResult<String> {
    let mut graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    graph.edges.retain(|e| e.id != edge_id);

    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Update node data in a workflow graph.
#[rustler::nif]
fn workflow_update_node_data(
    graph_json: String,
    node_id: String,
    data_json: String,
) -> NifResult<String> {
    let mut graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let data: serde_json::Value = serde_json::from_str(&data_json).unwrap_or_default();

    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        node.data = data;
    }

    serde_json::to_string(&graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

/// Validate a workflow graph. Returns error messages.
#[rustler::nif]
fn workflow_validate(graph_json: String) -> NifResult<Vec<String>> {
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let errors = node_engine::validation::validate_workflow(&graph, None);
    Ok(errors.iter().map(|e| e.to_string()).collect())
}

// ============================================================================
// NIF Functions - Executor (dirty CPU scheduler)
// ============================================================================

/// Create a new WorkflowExecutor with callback-based task execution.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let task_executor: Arc<dyn TaskExecutor> =
        Arc::new(ElixirCallbackTaskExecutor::new(caller_pid));
    let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(caller_pid));

    let executor = WorkflowExecutor::new("nif-execution", graph, event_sink);

    Ok(ResourceArc::new(WorkflowExecutorResource {
        executor: Arc::new(tokio::sync::RwLock::new(executor)),
        task_executor,
        runtime: Arc::new(runtime),
    }))
}

/// Create a new WorkflowExecutor with a custom callback timeout.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new_with_timeout(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
    timeout_secs: u64,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let task_executor: Arc<dyn TaskExecutor> = Arc::new(
        ElixirCallbackTaskExecutor::new(caller_pid).with_timeout(timeout_secs),
    );
    let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(caller_pid));

    let executor = WorkflowExecutor::new("nif-execution", graph, event_sink);

    Ok(ResourceArc::new(WorkflowExecutorResource {
        executor: Arc::new(tokio::sync::RwLock::new(executor)),
        task_executor,
        runtime: Arc::new(runtime),
    }))
}

/// Demand output from a node (triggers lazy evaluation).
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_demand(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> NifResult<String> {
    let rt = &resource.runtime;
    let executor = &resource.executor;
    let task_exec = &resource.task_executor;

    rt.block_on(async {
        let exec = executor.read().await;
        let result = exec.demand(&node_id, task_exec.as_ref()).await.map_err(|e| {
            rustler::Error::Term(Box::new(format!("Demand error: {}", e)))
        })?;
        serde_json::to_string(&result)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
    })
}

/// Update node data on the executor (marks the node modified).
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_update_node_data(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    data_json: String,
) -> NifResult<Atom> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    let data: serde_json::Value = serde_json::from_str(&data_json).unwrap_or_default();

    rt.block_on(async {
        let exec = executor.read().await;
        exec.update_node_data(&node_id, data).await.map_err(|e| {
            rustler::Error::Term(Box::new(format!("Update error: {}", e)))
        })?;
        Ok(atoms::ok())
    })
}

/// Mark a node as modified (invalidates caches).
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_mark_modified(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> NifResult<Atom> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    rt.block_on(async {
        let exec = executor.read().await;
        exec.mark_modified(&node_id).await;
        Ok(atoms::ok())
    })
}

/// Get cache statistics from the executor.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_cache_stats(
    resource: ResourceArc<WorkflowExecutorResource>,
) -> NifResult<ElixirCacheStats> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    rt.block_on(async {
        let exec = executor.read().await;
        let stats = exec.cache_stats().await;
        Ok(ElixirCacheStats {
            cached_nodes: stats.cached_nodes as u32,
            total_versions: stats.total_versions as u32,
            global_version: stats.global_version,
        })
    })
}

/// Get a snapshot of the current graph as JSON.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_get_graph_snapshot(
    resource: ResourceArc<WorkflowExecutorResource>,
) -> NifResult<String> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    rt.block_on(async {
        let exec = executor.read().await;
        let graph = exec.get_graph_snapshot().await;
        serde_json::to_string(&graph)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
    })
}

// ============================================================================
// NIF Functions - Executor I/O
// ============================================================================

/// Set an input value for a node in the executor context.
///
/// Sets the value at key "{node_id}.input.{port}" using ContextKeys convention.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_set_input(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    port: String,
    value_json: String,
) -> NifResult<Atom> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    let value: serde_json::Value = serde_json::from_str(&value_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let key = node_engine::ContextKeys::input(&node_id, &port);

    rt.block_on(async {
        let exec = executor.read().await;
        exec.set_context_value(&key, value).await;
        Ok(atoms::ok())
    })
}

/// Get an output value from a node in the executor context.
///
/// Gets the value at key "{node_id}.output.{port}" using ContextKeys convention.
/// Returns the JSON string or nil if not set.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_get_output(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    port: String,
) -> NifResult<Option<String>> {
    let rt = &resource.runtime;
    let executor = &resource.executor;

    let key = node_engine::ContextKeys::output(&node_id, &port);

    rt.block_on(async {
        let exec = executor.read().await;
        let value: Option<serde_json::Value> = exec.get_context_value(&key).await;
        match value {
            Some(v) => {
                let json = serde_json::to_string(&v).map_err(|e| {
                    rustler::Error::Term(Box::new(format!("Serialization error: {}", e)))
                })?;
                Ok(Some(json))
            }
            None => Ok(None),
        }
    })
}

// ============================================================================
// NIF Functions - Callback Bridge
// ============================================================================

/// Respond to a pending callback with success.
#[rustler::nif]
fn callback_respond(callback_id: String, outputs_json: String) -> NifResult<Atom> {
    let mut callbacks = PENDING_CALLBACKS
        .lock()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Lock poisoned: {}", e))))?;

    if let Some(sender) = callbacks.remove(&callback_id) {
        let _ = sender.send(Ok(outputs_json));
        Ok(atoms::ok())
    } else {
        Err(rustler::Error::Term(Box::new(format!(
            "Unknown callback: {}",
            callback_id
        ))))
    }
}

/// Respond to a pending callback with an error.
#[rustler::nif]
fn callback_error(callback_id: String, error_message: String) -> NifResult<Atom> {
    let mut callbacks = PENDING_CALLBACKS
        .lock()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Lock poisoned: {}", e))))?;

    if let Some(sender) = callbacks.remove(&callback_id) {
        let _ = sender.send(Err(error_message));
        Ok(atoms::ok())
    } else {
        Err(rustler::Error::Term(Box::new(format!(
            "Unknown callback: {}",
            callback_id
        ))))
    }
}

// ============================================================================
// NIF Functions - Orchestration
// ============================================================================

/// Create a new in-memory orchestration store.
#[rustler::nif]
fn orchestration_store_new() -> ResourceArc<OrchestrationStoreResource> {
    ResourceArc::new(OrchestrationStoreResource {
        store: Arc::new(tokio::sync::RwLock::new(OrchestrationStore::new())),
    })
}

/// Create a persistent orchestration store.
#[rustler::nif]
fn orchestration_store_with_persistence(
    path: String,
) -> ResourceArc<OrchestrationStoreResource> {
    ResourceArc::new(OrchestrationStoreResource {
        store: Arc::new(tokio::sync::RwLock::new(
            OrchestrationStore::with_persistence(path),
        )),
    })
}

/// Insert an orchestration graph into the store (as JSON).
#[rustler::nif(schedule = "DirtyCpu")]
fn orchestration_store_insert(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_json: String,
) -> NifResult<Atom> {
    let graph: OrchestrationGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let mut guard = resource.store.blocking_write();
    guard
        .insert_graph(graph)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Insert error: {}", e))))?;

    Ok(atoms::ok())
}

/// Get an orchestration graph from the store by ID.
#[rustler::nif]
fn orchestration_store_get(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
) -> NifResult<Option<String>> {
    let guard = resource.store.blocking_read();
    match guard.get_graph(&graph_id) {
        Some(graph) => {
            let json = serde_json::to_string(graph).map_err(|e| {
                rustler::Error::Term(Box::new(format!("Serialization error: {}", e)))
            })?;
            Ok(Some(json))
        }
        None => Ok(None),
    }
}

/// List all orchestration graph metadata.
#[rustler::nif]
fn orchestration_store_list(
    resource: ResourceArc<OrchestrationStoreResource>,
) -> Vec<ElixirOrchestrationMetadata> {
    let guard = resource.store.blocking_read();
    guard
        .list_graphs()
        .into_iter()
        .map(|m| ElixirOrchestrationMetadata {
            id: m.id,
            name: m.name,
            description: m.description,
            node_count: m.node_count as u32,
        })
        .collect()
}

/// Remove an orchestration graph from the store.
#[rustler::nif]
fn orchestration_store_remove(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
) -> NifResult<bool> {
    let mut guard = resource.store.blocking_write();
    guard
        .remove_graph(&graph_id)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Remove error: {}", e))))?;
    Ok(true)
}

// ============================================================================
// NIF Functions - Node Registry
// ============================================================================

/// Create a new empty node registry.
#[rustler::nif]
fn node_registry_new() -> ResourceArc<NodeRegistryResource> {
    ResourceArc::new(NodeRegistryResource {
        registry: Arc::new(tokio::sync::RwLock::new(node_engine::NodeRegistry::new())),
    })
}

/// Register a node type with metadata in the registry.
///
/// metadata_json should be a JSON object matching TaskMetadata:
/// `{"nodeType": "...", "category": "...", "label": "...", ...}`
#[rustler::nif]
fn node_registry_register(
    resource: ResourceArc<NodeRegistryResource>,
    metadata_json: String,
) -> NifResult<Atom> {
    let metadata: node_engine::TaskMetadata = serde_json::from_str(&metadata_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let mut registry = resource.registry.blocking_write();
    registry.register_metadata(metadata);

    Ok(atoms::ok())
}

/// List all registered node types and their metadata as JSON.
#[rustler::nif]
fn node_registry_list(
    resource: ResourceArc<NodeRegistryResource>,
) -> NifResult<String> {
    let registry = resource.registry.blocking_read();
    let metadata: Vec<&node_engine::TaskMetadata> = registry.all_metadata();

    serde_json::to_string(&metadata)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e))))
}

// ============================================================================
// ElixirDataGraphExecutor - bridges orchestration to BEAM
// ============================================================================

/// DataGraphExecutor that executes data graphs using the Elixir callback bridge.
pub struct ElixirDataGraphExecutor {
    store: Arc<tokio::sync::RwLock<OrchestrationStore>>,
    task_executor: Arc<dyn TaskExecutor>,
    event_sink_pid: rustler::LocalPid,
}

impl ElixirDataGraphExecutor {
    pub fn new(
        store: Arc<tokio::sync::RwLock<OrchestrationStore>>,
        task_executor: Arc<dyn TaskExecutor>,
        event_sink_pid: rustler::LocalPid,
    ) -> Self {
        Self {
            store,
            task_executor,
            event_sink_pid,
        }
    }
}

#[async_trait::async_trait]
impl node_engine::DataGraphExecutor for ElixirDataGraphExecutor {
    async fn execute_data_graph(
        &self,
        graph_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _event_sink: &dyn EventSink,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        // Look up the data graph from the store
        let graph = {
            let store = self.store.read().await;
            store.get_data_graph(graph_id).cloned().ok_or_else(|| {
                node_engine::NodeEngineError::ExecutionFailed(format!(
                    "Data graph '{}' not found in store",
                    graph_id
                ))
            })?
        };

        // Create event sink for this execution
        let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(self.event_sink_pid));

        // Create a WorkflowExecutor for this data graph
        let exec_id = format!("data-graph-{}", graph_id);
        let executor = WorkflowExecutor::new(&exec_id, graph.clone(), event_sink);

        // Set inputs into context using ContextKeys convention
        for (port, value) in &inputs {
            // Find input nodes and set their values
            for node in &graph.nodes {
                let key = node_engine::ContextKeys::input(&node.id, port);
                executor.set_context_value(&key, value.clone()).await;
            }
        }

        // Find terminal nodes (nodes with no outgoing edges) and demand them
        let terminal_nodes: Vec<String> = graph
            .nodes
            .iter()
            .filter(|n| !graph.edges.iter().any(|e| e.source == n.id))
            .map(|n| n.id.clone())
            .collect();

        // If no terminal nodes found, demand all nodes
        let demand_nodes = if terminal_nodes.is_empty() {
            graph.nodes.iter().map(|n| n.id.clone()).collect()
        } else {
            terminal_nodes
        };

        let results = executor
            .demand_multiple(&demand_nodes, self.task_executor.as_ref())
            .await?;

        // Flatten all outputs into a single map
        let mut outputs = HashMap::new();
        for (node_id, node_outputs) in results {
            for (port, value) in node_outputs {
                outputs.insert(format!("{}.{}", node_id, port), value);
            }
        }

        Ok(outputs)
    }

    fn get_data_graph(&self, graph_id: &str) -> Option<WorkflowGraph> {
        let store = self.store.blocking_read();
        store.get_data_graph(graph_id).cloned()
    }
}

// ============================================================================
// NIF Functions - Orchestration Execution
// ============================================================================

/// Execute an orchestration graph.
///
/// Retrieves the orchestration graph from the store, creates an
/// ElixirDataGraphExecutor to handle data graph nodes, and runs
/// the orchestration to completion. Events stream to callback_pid.
///
/// Returns JSON string of OrchestrationResult.
#[rustler::nif(schedule = "DirtyCpu")]
fn execute_orchestration(
    env: Env,
    store_resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
    initial_data_json: String,
    callback_pid: rustler::LocalPid,
) -> NifResult<String> {
    let _ = env;

    let initial_data: HashMap<String, serde_json::Value> =
        serde_json::from_str(&initial_data_json)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    // Look up the orchestration graph
    let graph = {
        let store = store_resource.store.blocking_read();
        store.get_graph(&graph_id).cloned().ok_or_else(|| {
            rustler::Error::Term(Box::new(format!(
                "Orchestration graph '{}' not found",
                graph_id
            )))
        })?
    };

    // Create runtime for this execution
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    let task_executor: Arc<dyn TaskExecutor> =
        Arc::new(ElixirCallbackTaskExecutor::new(callback_pid));
    let event_sink = BeamEventSink::new(callback_pid);

    // Create the data graph executor
    let data_executor = ElixirDataGraphExecutor::new(
        store_resource.store.clone(),
        task_executor,
        callback_pid,
    );

    // Create and run the orchestration executor
    let orch_executor = node_engine::OrchestrationExecutor::new(data_executor)
        .with_execution_id(format!("nif-orch-{}", graph_id));

    let result = runtime.block_on(async {
        orch_executor
            .execute(&graph, initial_data, &event_sink)
            .await
    });

    match result {
        Ok(orch_result) => serde_json::to_string(&orch_result)
            .map_err(|e| rustler::Error::Term(Box::new(format!("Serialization error: {}", e)))),
        Err(e) => Err(rustler::Error::Term(Box::new(format!(
            "Orchestration error: {}",
            e
        )))),
    }
}

/// Insert a data graph (workflow) into the orchestration store.
///
/// Data graphs are the low-level workflow graphs that orchestration
/// DataGraph nodes reference and execute.
#[rustler::nif(schedule = "DirtyCpu")]
fn orchestration_store_insert_data_graph(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
    graph_json: String,
) -> NifResult<Atom> {
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let mut guard = resource.store.blocking_write();
    guard.insert_data_graph(graph_id, graph);

    Ok(atoms::ok())
}

// ============================================================================
// Resource registration and NIF init
// ============================================================================

fn load(env: Env, _info: Term) -> bool {
    rustler::resource!(WorkflowExecutorResource, env);
    rustler::resource!(OrchestrationStoreResource, env);
    rustler::resource!(NodeRegistryResource, env);
    true
}

rustler::init!(
    "Elixir.Pantograph.Native",
    load = load
);

// Note: NIF-annotated functions cannot be called directly in Rust tests.
// Integration testing of NIF functions requires an Elixir/Erlang runtime.
// The tests below verify the underlying non-NIF logic.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_graph_json_roundtrip() {
        let graph = WorkflowGraph::new("wf-1", "Test");
        let json = serde_json::to_string(&graph).unwrap();
        let parsed: WorkflowGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "wf-1");
        assert_eq!(parsed.name, "Test");
    }

    #[test]
    fn test_workflow_graph_add_node() {
        let mut graph = WorkflowGraph::new("wf-1", "Test");
        graph.nodes.push(node_engine::GraphNode {
            id: "n1".to_string(),
            node_type: "text-input".to_string(),
            position: (0.0, 0.0),
            data: serde_json::Value::Null,
        });
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].id, "n1");
    }

    #[test]
    fn test_validation_empty_graph() {
        let graph = WorkflowGraph::new("wf-1", "Test");
        let errors = node_engine::validation::validate_workflow(&graph, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_callback_channel_lifecycle() {
        let (tx, rx) = oneshot::channel::<Result<String, String>>();
        let callback_id = "test-cb-1".to_string();

        {
            let mut callbacks = PENDING_CALLBACKS.lock().unwrap();
            callbacks.insert(callback_id.clone(), tx);
        }

        // Simulate callback response
        {
            let mut callbacks = PENDING_CALLBACKS.lock().unwrap();
            let sender = callbacks.remove(&callback_id).unwrap();
            sender.send(Ok(r#"{"result": "ok"}"#.to_string())).unwrap();
        }

        let result = rx.blocking_recv().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_orchestration_store_roundtrip() {
        let store = OrchestrationStore::new();
        assert!(store.list_graphs().is_empty());
    }

    #[test]
    fn test_context_keys_input_output() {
        let input_key = node_engine::ContextKeys::input("node-1", "prompt");
        assert_eq!(input_key, "node-1.input.prompt");

        let output_key = node_engine::ContextKeys::output("node-1", "response");
        assert_eq!(output_key, "node-1.output.response");
    }

    #[test]
    fn test_node_registry_metadata() {
        let mut registry = node_engine::NodeRegistry::new();
        assert!(registry.all_metadata().is_empty());

        let metadata = node_engine::TaskMetadata {
            node_type: "test-node".to_string(),
            category: node_engine::NodeCategory::Processing,
            label: "Test Node".to_string(),
            description: "A test node".to_string(),
            inputs: vec![],
            outputs: vec![],
            execution_mode: node_engine::ExecutionMode::Reactive,
        };

        registry.register_metadata(metadata);
        assert_eq!(registry.all_metadata().len(), 1);
        assert!(registry.has_node_type("test-node"));

        // Verify JSON serialization
        let all = registry.all_metadata();
        let json = serde_json::to_string(&all).unwrap();
        assert!(json.contains("test-node"));
    }

    #[test]
    fn test_task_metadata_json_roundtrip() {
        let json = r#"{
            "nodeType": "my-node",
            "category": "processing",
            "label": "My Node",
            "description": "Does things",
            "inputs": [],
            "outputs": [],
            "executionMode": "reactive"
        }"#;
        let metadata: node_engine::TaskMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.node_type, "my-node");
        assert_eq!(metadata.label, "My Node");
    }
}
