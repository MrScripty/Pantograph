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

use rustler::{Atom, Env, NifResult, ResourceArc, Term};

// Force the linker to include workflow-nodes object files,
// which contain `inventory::submit!()` statics for built-in node types.
extern crate workflow_nodes;

mod binding_types;
mod callback_bridge;
mod elixir_data_graph_executor;
mod executor_nifs;
#[cfg(feature = "frontend-http")]
mod frontend_http_nifs;
mod orchestration_execution_nifs;
mod orchestration_store_nifs;
mod pumas_nifs;
mod registry_nifs;
mod resource_registration;
mod resources;
mod type_parsing_contract;
mod workflow_event_contract;
mod workflow_graph_contract;
#[cfg(feature = "frontend-http")]
mod workflow_host_contract;

pub use binding_types::{
    ElixirCacheStats, ElixirExecutionMode, ElixirNodeCategory, ElixirNodeDefinition,
    ElixirOrchestrationMetadata, ElixirOrchestrationNodeType, ElixirPortDataType,
};
use resource_registration::register_resources;
pub use resources::{
    ExtensionsResource, InferenceGatewayResource, NodeRegistryResource, OrchestrationStoreResource,
    PumasApiResource, WorkflowExecutorResource,
};
use type_parsing_contract::{
    parse_execution_mode_string, parse_node_category_string, parse_port_data_type_string,
};
use workflow_graph_contract::{
    workflow_add_edge_json, workflow_add_node_json, workflow_from_json_string, workflow_new_json,
    workflow_remove_edge_json, workflow_remove_node_json, workflow_update_node_data_json,
    workflow_validate_json,
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
        demand_complete,
        demand_error,
        node_stream,
        node_stream_done,
    }
}

// ============================================================================
// Headless embedding adapter for Rustler
// ============================================================================

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_run(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    frontend_http_nifs::workflow_run(base_url, request_json, pumas_resource)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_get_capabilities(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    frontend_http_nifs::workflow_get_capabilities(base_url, request_json, pumas_resource)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_preflight(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    frontend_http_nifs::workflow_preflight(base_url, request_json, pumas_resource)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_register_attribution_client(request_json: String) -> NifResult<String> {
    frontend_http_nifs::workflow_register_attribution_client(request_json)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_open_client_session(request_json: String) -> NifResult<String> {
    frontend_http_nifs::workflow_open_client_session(request_json)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_resume_client_session(request_json: String) -> NifResult<String> {
    frontend_http_nifs::workflow_resume_client_session(request_json)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_create_client_bucket(request_json: String) -> NifResult<String> {
    frontend_http_nifs::workflow_create_client_bucket(request_json)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_delete_client_bucket(request_json: String) -> NifResult<String> {
    frontend_http_nifs::workflow_delete_client_bucket(request_json)
}

#[cfg(feature = "frontend-http")]
#[rustler::nif(schedule = "DirtyCpu")]
fn frontend_http_workflow_run_attributed(
    base_url: String,
    request_json: String,
    pumas_resource: Option<ResourceArc<PumasApiResource>>,
) -> NifResult<String> {
    frontend_http_nifs::workflow_run_attributed(base_url, request_json, pumas_resource)
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
    parse_port_data_type_string(type_str)
}

/// Parse a node category string to enum.
#[rustler::nif]
fn parse_node_category(category_str: String) -> ElixirNodeCategory {
    parse_node_category_string(category_str)
}

/// Parse an execution mode string to enum.
#[rustler::nif]
fn parse_execution_mode(mode_str: String) -> ElixirExecutionMode {
    parse_execution_mode_string(mode_str)
}

// ============================================================================
// NIF Functions - Workflow CRUD (JSON marshaling)
// ============================================================================

/// Create a new empty workflow graph, returned as JSON.
#[rustler::nif]
fn workflow_new(id: String, name: String) -> NifResult<String> {
    workflow_new_json(id, name)
}

/// Parse a JSON string into a WorkflowGraph and re-serialize (validates structure).
#[rustler::nif]
fn workflow_from_json(json: String) -> NifResult<String> {
    workflow_from_json_string(json)
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
    workflow_add_node_json(graph_json, node_id, node_type, x, y, data_json)
}

/// Remove a node from a workflow graph.
#[rustler::nif]
fn workflow_remove_node(graph_json: String, node_id: String) -> NifResult<String> {
    workflow_remove_node_json(graph_json, node_id)
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
    workflow_add_edge_json(graph_json, source, source_handle, target, target_handle)
}

/// Remove an edge from a workflow graph.
#[rustler::nif]
fn workflow_remove_edge(graph_json: String, edge_id: String) -> NifResult<String> {
    workflow_remove_edge_json(graph_json, edge_id)
}

/// Update node data in a workflow graph.
#[rustler::nif]
fn workflow_update_node_data(
    graph_json: String,
    node_id: String,
    data_json: String,
) -> NifResult<String> {
    workflow_update_node_data_json(graph_json, node_id, data_json)
}

/// Validate a workflow graph. Returns error messages.
#[rustler::nif]
fn workflow_validate(graph_json: String) -> NifResult<Vec<String>> {
    workflow_validate_json(graph_json)
}

// ============================================================================
// NIF Functions - Executor (dirty CPU scheduler)
// ============================================================================

/// Create a new WorkflowExecutor with core-first task execution.
///
/// Standard node types (text-input, llm-inference, etc.) are handled natively
/// in Rust by `CoreTaskExecutor`. Custom node types fall through to Elixir
/// via the callback bridge.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    executor_nifs::new_executor(graph_json, caller_pid)
}

/// Create a new WorkflowExecutor with a custom callback timeout.
///
/// Same as `executor_new` but allows configuring the Elixir callback timeout.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new_with_timeout(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
    timeout_secs: u64,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    executor_nifs::new_executor_with_timeout(graph_json, caller_pid, timeout_secs)
}

// ============================================================================
// NIF Functions - Inference Gateway
// ============================================================================

/// Create a new InferenceGateway with a StdProcessSpawner.
///
/// The gateway manages the llama.cpp server lifecycle and is shared across
/// executors. Create once at app startup, then pass to `executor_new_with_inference`.
///
/// - `binaries_dir`: directory containing the `llama-server-wrapper` binary
/// - `data_dir`: directory for PID files and runtime data
#[rustler::nif(schedule = "DirtyCpu")]
fn inference_gateway_new(
    env: Env,
    binaries_dir: String,
    data_dir: String,
) -> NifResult<ResourceArc<InferenceGatewayResource>> {
    let _ = env;
    executor_nifs::new_inference_gateway(binaries_dir, data_dir)
}

// ============================================================================
// NIF Functions - Executor with Inference Gateway
// ============================================================================

/// Create a new WorkflowExecutor with inference gateway support.
///
/// Same as `executor_new` but wires an `InferenceGateway` into the
/// `CoreTaskExecutor`, enabling native handling of `llamacpp-inference`,
/// `llm-inference`, `vision-analysis`, and `unload-model` nodes.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new_with_inference(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
    gateway_resource: ResourceArc<InferenceGatewayResource>,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    executor_nifs::new_executor_with_inference(graph_json, caller_pid, gateway_resource)
}

/// Create a new WorkflowExecutor with inference gateway and custom timeout.
///
/// Combines `executor_new_with_inference` with a custom Elixir callback timeout.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new_with_inference_timeout(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
    gateway_resource: ResourceArc<InferenceGatewayResource>,
    timeout_secs: u64,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    executor_nifs::new_executor_with_inference_timeout(
        graph_json,
        caller_pid,
        gateway_resource,
        timeout_secs,
    )
}

/// Demand output from a node synchronously (blocks the DirtyCpu scheduler).
///
/// **Deprecated**: Use `executor_demand_async` instead for non-blocking execution.
/// This function blocks the calling scheduler thread until the demand completes,
/// which can cause throughput issues with many concurrent demands.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_demand(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> NifResult<String> {
    executor_nifs::demand(resource, node_id)
}

/// Demand output from a node asynchronously (non-blocking).
///
/// Returns immediately with `:ok`. The result is sent to `caller_pid` as:
/// - `{:demand_complete, node_id, outputs_json}` on success
/// - `{:demand_error, node_id, error_message}` on failure
///
/// This is the preferred way to demand nodes from Elixir, as it does not
/// block any scheduler thread. Multiple demands can run concurrently.
#[rustler::nif]
fn executor_demand_async(
    env: Env,
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    caller_pid: rustler::LocalPid,
) -> Atom {
    let _ = env;
    executor_nifs::demand_async(resource, node_id, caller_pid)
}

/// Update node data on the executor (marks the node modified).
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_update_node_data(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
    data_json: String,
) -> NifResult<Atom> {
    executor_nifs::update_node_data(resource, node_id, data_json)
}

/// Mark a node as modified (invalidates caches).
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_mark_modified(
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> NifResult<Atom> {
    executor_nifs::mark_modified(resource, node_id)
}

/// Get cache statistics from the executor.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_cache_stats(
    resource: ResourceArc<WorkflowExecutorResource>,
) -> NifResult<ElixirCacheStats> {
    executor_nifs::cache_stats(resource)
}

/// Get a snapshot of the current graph as JSON.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_get_graph_snapshot(
    resource: ResourceArc<WorkflowExecutorResource>,
) -> NifResult<String> {
    executor_nifs::get_graph_snapshot(resource)
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
    executor_nifs::set_input(resource, node_id, port, value_json)
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
    executor_nifs::get_output(resource, node_id, port)
}

// ============================================================================
// NIF Functions - Callback Bridge
// ============================================================================

/// Respond to a pending callback with success.
#[rustler::nif]
fn callback_respond(callback_id: String, outputs_json: String) -> NifResult<Atom> {
    callback_bridge::callback_respond(callback_id, outputs_json)
}

/// Respond to a pending callback with an error.
#[rustler::nif]
fn callback_error(callback_id: String, error_message: String) -> NifResult<Atom> {
    callback_bridge::callback_error(callback_id, error_message)
}

// ============================================================================
// NIF Functions - Orchestration
// ============================================================================

/// Create a new in-memory orchestration store.
#[rustler::nif]
fn orchestration_store_new() -> ResourceArc<OrchestrationStoreResource> {
    orchestration_store_nifs::new_store()
}

/// Create a persistent orchestration store.
#[rustler::nif]
fn orchestration_store_with_persistence(path: String) -> ResourceArc<OrchestrationStoreResource> {
    orchestration_store_nifs::with_persistence(path)
}

/// Insert an orchestration graph into the store (as JSON).
#[rustler::nif(schedule = "DirtyCpu")]
fn orchestration_store_insert(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_json: String,
) -> NifResult<Atom> {
    orchestration_store_nifs::insert(resource, graph_json)
}

/// Get an orchestration graph from the store by ID.
#[rustler::nif]
fn orchestration_store_get(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
) -> NifResult<Option<String>> {
    orchestration_store_nifs::get(resource, graph_id)
}

/// List all orchestration graph metadata.
#[rustler::nif]
fn orchestration_store_list(
    resource: ResourceArc<OrchestrationStoreResource>,
) -> Vec<ElixirOrchestrationMetadata> {
    orchestration_store_nifs::list(resource)
}

/// Remove an orchestration graph from the store.
#[rustler::nif]
fn orchestration_store_remove(
    resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
) -> NifResult<bool> {
    orchestration_store_nifs::remove(resource, graph_id)
}

// ============================================================================
// NIF Functions - Node Registry
// ============================================================================

/// Create a new empty node registry.
#[rustler::nif]
fn node_registry_new() -> ResourceArc<NodeRegistryResource> {
    registry_nifs::node_registry_new()
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
    registry_nifs::node_registry_register(resource, metadata_json)
}

/// List all registered node types and their metadata as JSON.
#[rustler::nif]
fn node_registry_list(resource: ResourceArc<NodeRegistryResource>) -> NifResult<String> {
    registry_nifs::node_registry_list(resource)
}

/// List backend-owned workflow-service node definitions as JSON.
#[rustler::nif]
fn node_registry_list_definitions() -> NifResult<String> {
    registry_nifs::node_registry_list_definitions()
}

/// Get one backend-owned workflow-service node definition as JSON.
#[rustler::nif]
fn node_registry_get_definition(node_type: String) -> NifResult<String> {
    registry_nifs::node_registry_get_definition(node_type)
}

/// List backend-owned workflow-service node definitions grouped by category.
#[rustler::nif]
fn node_registry_definitions_by_category() -> NifResult<String> {
    registry_nifs::node_registry_definitions_by_category()
}

/// Register all built-in node types from the workflow-nodes crate.
///
/// Uses the `inventory` crate to discover all TaskMetadata submitted via
/// `inventory::submit!()` in workflow-nodes and registers them as metadata-only.
#[rustler::nif]
fn node_registry_register_builtins(resource: ResourceArc<NodeRegistryResource>) -> NifResult<Atom> {
    registry_nifs::node_registry_register_builtins(resource)
}

/// List queryable backend port option providers as JSON.
#[rustler::nif]
fn node_registry_queryable_ports(resource: ResourceArc<NodeRegistryResource>) -> NifResult<String> {
    registry_nifs::node_registry_queryable_ports(resource)
}

// ============================================================================
// NIF Functions - Extensions & Port Options
// ============================================================================

/// Create empty executor extensions.
///
/// Extensions hold optional runtime dependencies (e.g. PumasApi) needed by
/// port options providers. Call `extensions_setup` to initialize them.
#[rustler::nif]
fn extensions_new() -> ResourceArc<ExtensionsResource> {
    registry_nifs::extensions_new()
}

/// Initialize extensions with PumasApi model library access.
///
/// Wraps `workflow_nodes::setup_extensions_with_path()` — the same function
/// the Pantograph Tauri app calls. Uses the 3-step discovery chain:
/// 1. Explicit `library_path` parameter (if provided)
/// 2. `PUMAS_LIBRARY_PATH` environment variable
/// 3. Global registry (~/.config/pumas/registry.db)
#[rustler::nif(schedule = "DirtyCpu")]
fn extensions_setup(
    resource: ResourceArc<ExtensionsResource>,
    library_path: Option<String>,
) -> NifResult<Atom> {
    registry_nifs::extensions_setup(resource, library_path)
}

/// Query available options for a node's port.
///
/// Dispatches to the registered `PortOptionsProvider` for the given node type
/// and port. Returns JSON-serialized `PortOptionsResult`.
///
/// This is the NIF equivalent of the Tauri `query_port_options` command.
#[rustler::nif(schedule = "DirtyCpu")]
fn node_registry_query_port_options(
    registry_resource: ResourceArc<NodeRegistryResource>,
    extensions_resource: ResourceArc<ExtensionsResource>,
    node_type: String,
    port_id: String,
    query_json: String,
) -> NifResult<String> {
    registry_nifs::node_registry_query_port_options(
        registry_resource,
        extensions_resource,
        node_type,
        port_id,
        query_json,
    )
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
    orchestration_execution_nifs::execute(store_resource, graph_id, initial_data_json, callback_pid)
}

/// Execute an orchestration graph with inference gateway support.
///
/// Same as `execute_orchestration` but wires an `InferenceGateway` into
/// the `CoreTaskExecutor`, enabling native inference node execution
/// (llamacpp-inference, llm-inference, vision-analysis, unload-model)
/// with streaming token events via `BeamEventSink`.
#[rustler::nif(schedule = "DirtyCpu")]
fn execute_orchestration_with_inference(
    env: Env,
    store_resource: ResourceArc<OrchestrationStoreResource>,
    graph_id: String,
    initial_data_json: String,
    callback_pid: rustler::LocalPid,
    gateway_resource: ResourceArc<InferenceGatewayResource>,
) -> NifResult<String> {
    let _ = env;
    orchestration_execution_nifs::execute_with_inference(
        store_resource,
        graph_id,
        initial_data_json,
        callback_pid,
        gateway_resource,
    )
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
    orchestration_execution_nifs::insert_data_graph(resource, graph_id, graph_json)
}

// ============================================================================
// NIF Functions - PumasApi (Model Library)
// ============================================================================

/// Discover a PumasApi instance via the global registry (~/.config/pumas/registry.db).
///
/// Tries to connect to a running instance first, then falls back to creating
/// a new primary from the registered library path.
/// Returns {:error, reason} if no libraries are registered.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_api_discover() -> NifResult<ResourceArc<PumasApiResource>> {
    pumas_nifs::api_discover()
}

/// Create a new PumasApi instance.
///
/// `launcher_root_path` is the root directory for the pumas library.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_api_new(launcher_root_path: String) -> NifResult<ResourceArc<PumasApiResource>> {
    pumas_nifs::api_new(launcher_root_path)
}

/// Inject a PumasApi into a WorkflowExecutor's extensions.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_set_pumas_api(
    executor_resource: ResourceArc<WorkflowExecutorResource>,
    pumas_resource: ResourceArc<PumasApiResource>,
) -> NifResult<Atom> {
    pumas_nifs::executor_set_pumas_api(executor_resource, pumas_resource)
}

/// Set a KV cache store on the workflow executor for cache save/load/truncate nodes.
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_set_kv_cache_store(
    executor_resource: ResourceArc<WorkflowExecutorResource>,
    cache_dir: String,
) -> NifResult<Atom> {
    pumas_nifs::executor_set_kv_cache_store(executor_resource, cache_dir)
}

// --- Local library NIFs ---

/// List all models in the local library. Returns JSON array of ModelRecord.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_list_models(resource: ResourceArc<PumasApiResource>) -> NifResult<String> {
    pumas_nifs::list_models(resource)
}

/// Search the local model library. Returns JSON SearchResult.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_search_models(
    resource: ResourceArc<PumasApiResource>,
    query: String,
    limit: usize,
    offset: usize,
) -> NifResult<String> {
    pumas_nifs::search_models(resource, query, limit, offset)
}

/// Get a single model by ID. Returns JSON ModelRecord or nil.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_get_model(
    resource: ResourceArc<PumasApiResource>,
    model_id: String,
) -> NifResult<Option<String>> {
    pumas_nifs::get_model(resource, model_id)
}

/// Rebuild the model index. Returns the number of models indexed.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_rebuild_index(resource: ResourceArc<PumasApiResource>) -> NifResult<usize> {
    pumas_nifs::rebuild_index(resource)
}

// --- HuggingFace NIFs ---

/// Search HuggingFace for models. Returns JSON array of HuggingFaceModel.
///
/// `kind` is optional and filters by model type (e.g., "llm", "diffusion", "embedding").
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_search_hf(
    resource: ResourceArc<PumasApiResource>,
    query: String,
    kind: Option<String>,
    limit: usize,
) -> NifResult<String> {
    pumas_nifs::search_hf(resource, query, kind, limit)
}

/// Get file tree for a HuggingFace repo. Returns JSON RepoFileTree.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_get_repo_files(
    resource: ResourceArc<PumasApiResource>,
    repo_id: String,
) -> NifResult<String> {
    pumas_nifs::get_repo_files(resource, repo_id)
}

// --- Download NIFs ---

/// Start a model download from HuggingFace. Returns the download ID.
///
/// `request_json` should be a JSON DownloadRequest:
/// `{"repo_id": "...", "family": "...", "official_name": "...", ...}`
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_start_download(
    resource: ResourceArc<PumasApiResource>,
    request_json: String,
) -> NifResult<String> {
    pumas_nifs::start_download(resource, request_json)
}

/// Get download progress for a download ID. Returns JSON ModelDownloadProgress or nil.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_get_download_progress(
    resource: ResourceArc<PumasApiResource>,
    download_id: String,
) -> NifResult<Option<String>> {
    pumas_nifs::get_download_progress(resource, download_id)
}

/// Cancel a download. Returns true if cancelled.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_cancel_download(
    resource: ResourceArc<PumasApiResource>,
    download_id: String,
) -> NifResult<bool> {
    pumas_nifs::cancel_download(resource, download_id)
}

// --- Import NIFs ---

/// Import a model into the library. Returns JSON ModelImportResult.
///
/// `spec_json` should be a JSON ModelImportSpec:
/// `{"path": "...", "family": "...", "official_name": "...", ...}`
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_import_model(
    resource: ResourceArc<PumasApiResource>,
    spec_json: String,
) -> NifResult<String> {
    pumas_nifs::import_model(resource, spec_json)
}

/// Import multiple models in batch. Returns JSON array of ModelImportResult.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_import_batch(
    resource: ResourceArc<PumasApiResource>,
    specs_json: String,
) -> NifResult<String> {
    pumas_nifs::import_batch(resource, specs_json)
}

// --- System NIFs ---

/// Get disk space info. Returns JSON DiskSpaceResponse.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_get_disk_space(resource: ResourceArc<PumasApiResource>) -> NifResult<String> {
    pumas_nifs::get_disk_space(resource)
}

/// Get system resources info. Returns JSON SystemResourcesResponse.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_get_system_resources(resource: ResourceArc<PumasApiResource>) -> NifResult<String> {
    pumas_nifs::get_system_resources(resource)
}

/// Check if Ollama is running.
#[rustler::nif(schedule = "DirtyCpu")]
fn pumas_is_ollama_running(resource: ResourceArc<PumasApiResource>) -> bool {
    pumas_nifs::is_ollama_running(resource)
}

// ============================================================================
// Resource registration and NIF init
// ============================================================================

fn load(env: Env, _info: Term) -> bool {
    register_resources(env);
    true
}

rustler::init!("Elixir.Pantograph.Native", load = load);

// Note: NIF-annotated functions cannot be called directly in Rust tests.
// Integration testing of NIF functions requires an Elixir/Erlang runtime.
// The crate-local tests verify the underlying non-NIF logic.
#[cfg(test)]
mod lib_tests;
