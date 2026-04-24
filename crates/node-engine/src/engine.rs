//! Demand-driven lazy evaluation engine
//!
//! This module implements pull-based lazy evaluation with version-tracked
//! caching. Instead of eagerly propagating dirty flags forward, we
//! traverse dependencies backward from requested outputs.
//!
//! # Key Concepts
//!
//! - **Version tracking**: Each node's inputs have a combined version hash
//! - **Cache invalidation**: If input version differs from cached version, recompute
//! - **Pull-based**: Only compute what's needed for requested outputs
//! - **O(path length)**: For cache hits, only traverse the dependency path
//!
//! # Example
//!
//! ```ignore
//! use node_engine::engine::DemandEngine;
//! use node_engine::types::WorkflowGraph;
//!
//! let mut engine = DemandEngine::new("exec_1");
//! let graph = WorkflowGraph::new("workflow_1", "My Workflow");
//!
//! // Demand output from a specific node - only computes what's needed
//! let output = engine.demand("output_node", &graph, &executor).await?;
//!
//! // If we demand again without changes, it returns cached value
//! let cached = engine.demand("output_node", &graph, &executor).await?;
//!
//! // Mark a node as modified to invalidate downstream caches
//! engine.mark_modified("input_node");
//! ```

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::Context;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::events::{EventSink, WorkflowEvent};
use crate::extensions::ExecutorExtensions;
use crate::types::{NodeId, WorkflowGraph};

pub(super) type NodeOutputMap = HashMap<String, serde_json::Value>;
pub(super) type MultiNodeOutputMap = HashMap<NodeId, NodeOutputMap>;
pub(super) type DemandFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

#[derive(Clone, Copy)]
pub(super) struct DemandRuntimeContext<'a> {
    graph: &'a WorkflowGraph,
    executor: &'a dyn TaskExecutor,
    context: &'a Context,
    event_sink: &'a dyn EventSink,
    extensions: &'a ExecutorExtensions,
    node_memories: Option<&'a HashMap<NodeId, NodeMemorySnapshot>>,
}

impl<'a> DemandRuntimeContext<'a> {
    fn new(
        graph: &'a WorkflowGraph,
        executor: &'a dyn TaskExecutor,
        context: &'a Context,
        event_sink: &'a dyn EventSink,
        extensions: &'a ExecutorExtensions,
        node_memories: Option<&'a HashMap<NodeId, NodeMemorySnapshot>>,
    ) -> Self {
        Self {
            graph,
            executor,
            context,
            event_sink,
            extensions,
            node_memories,
        }
    }
}

mod dependency_inputs;
mod execution_core;
mod execution_events;
mod graph_events;
mod graph_state;
mod inflight_tracking;
mod multi_demand;
mod node_preparation;
mod output_cache;
mod session_state;
mod single_demand;
mod workflow_execution_session;

pub use session_state::{
    GraphMemoryImpactSummary, NodeMemoryCompatibility, NodeMemoryCompatibilitySnapshot,
    NodeMemoryIdentity, NodeMemoryIndirectStateReference, NodeMemoryRestoreStrategy,
    NodeMemorySnapshot, NodeMemoryStatus, WorkflowExecutionSessionCheckpointSummary,
    WorkflowExecutionSessionResidencyState,
};

/// Trait for executing a single node/task
///
/// This abstracts the actual execution logic, allowing different
/// execution strategies (graph-flow, direct function call, etc.)
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task with the given inputs and return its outputs
    ///
    /// # Arguments
    /// * `task_id` - The ID of the task to execute
    /// * `inputs` - Map of input port names to their values
    /// * `context` - The graph-flow context for shared state
    /// * `extensions` - Typed extension map for non-serializable dependencies
    ///
    /// # Returns
    /// A map of output port names to their values
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>>;
}

/// Cached output for a node with its version
#[derive(Debug, Clone, PartialEq)]
pub struct CachedOutput {
    /// Version of inputs when this output was computed
    pub version: u64,
    /// The cached output value
    pub value: serde_json::Value,
}

/// Demand-driven lazy evaluation engine
///
/// Implements pull-based evaluation where outputs are only computed
/// when demanded, and only recomputed when inputs have changed.
#[derive(Clone)]
pub struct DemandEngine {
    /// Version counter for each node (incremented when inputs change)
    versions: HashMap<NodeId, u64>,
    /// Cached outputs with their computed-at version
    cache: HashMap<NodeId, CachedOutput>,
    /// Last resolved task inputs captured at execution time for node-memory
    /// projection and inspection.
    last_inputs: HashMap<NodeId, serde_json::Value>,
    /// Global version counter (for marking external changes)
    global_version: u64,
    /// Execution ID for events
    execution_id: String,
}

impl DemandEngine {
    /// Create a new demand engine
    pub fn new(execution_id: impl Into<String>) -> Self {
        Self {
            versions: HashMap::new(),
            cache: HashMap::new(),
            last_inputs: HashMap::new(),
            global_version: 0,
            execution_id: execution_id.into(),
        }
    }

    /// Mark a node as modified (externally changed, e.g., user edited data)
    ///
    /// This increments the node's version, invalidating its cached output
    /// and any downstream caches.
    pub fn mark_modified(&mut self, node_id: &NodeId) {
        self.global_version += 1;
        self.versions.insert(node_id.clone(), self.global_version);
        // Clear this node's cache (downstream caches will auto-invalidate
        // due to version mismatch on next demand)
        self.cache.remove(node_id);
        self.last_inputs.remove(node_id);
    }

    /// Get the cached output for a node, if valid
    pub fn get_cached(
        &self,
        node_id: &NodeId,
        graph: &WorkflowGraph,
    ) -> Option<&serde_json::Value> {
        let cached = self.cache.get(node_id)?;
        let current_version = self.compute_input_version(node_id, graph);
        if cached.version == current_version {
            Some(&cached.value)
        } else {
            None
        }
    }

    /// Store a computed output in the cache
    pub fn cache_output(
        &mut self,
        node_id: &NodeId,
        value: serde_json::Value,
        graph: &WorkflowGraph,
    ) {
        let version = self.compute_input_version(node_id, graph);
        self.cache
            .insert(node_id.clone(), CachedOutput { version, value });
    }

    /// Compute the version hash for a node's inputs
    ///
    /// This is the sum of all upstream node versions, used to detect
    /// when inputs have changed.
    pub fn compute_input_version(&self, node_id: &NodeId, graph: &WorkflowGraph) -> u64 {
        graph
            .get_dependencies(node_id)
            .iter()
            .map(|dep| self.versions.get(dep).unwrap_or(&0))
            .fold(0u64, |acc, v| acc.wrapping_add(*v))
    }

    /// Clear the entire cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.last_inputs.clear();
    }

    /// Get the execution ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Get statistics about the cache
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            cached_nodes: self.cache.len(),
            total_versions: self.versions.len(),
            global_version: self.global_version,
        }
    }

    fn reconcile_isolated_run(&mut self, base: &Self, isolated: &Self) {
        reconcile_changed_node_entries(&mut self.versions, &base.versions, &isolated.versions);
        reconcile_changed_node_entries(&mut self.cache, &base.cache, &isolated.cache);
        reconcile_changed_node_entries(
            &mut self.last_inputs,
            &base.last_inputs,
            &isolated.last_inputs,
        );
        self.global_version = self.global_version.max(isolated.global_version);
    }

    fn record_input_snapshot(
        &mut self,
        node_id: &NodeId,
        inputs: HashMap<String, serde_json::Value>,
    ) {
        self.last_inputs.insert(
            node_id.clone(),
            serde_json::Value::Object(inputs.into_iter().collect()),
        );
    }

    /// Demand output from a node - the core lazy evaluation method
    ///
    /// This is the heart of demand-driven evaluation. It:
    /// 1. Checks if the cached output is still valid
    /// 2. If not, recursively demands all dependencies first
    /// 3. Executes this node with resolved inputs
    /// 4. Caches the result for future demands
    ///
    /// # Arguments
    /// * `node_id` - The node to demand output from
    /// * `graph` - The workflow graph
    /// * `executor` - The task executor for running nodes
    /// * `context` - The graph-flow context
    /// * `event_sink` - For sending progress events
    ///
    /// # Returns
    /// The outputs from the node as a map of port names to values
    pub async fn demand(
        &mut self,
        node_id: &NodeId,
        graph: &WorkflowGraph,
        executor: &dyn TaskExecutor,
        context: &Context,
        event_sink: &dyn EventSink,
        extensions: &ExecutorExtensions,
    ) -> Result<NodeOutputMap> {
        self.demand_with_context(
            DemandRuntimeContext::new(graph, executor, context, event_sink, extensions, None),
            node_id,
        )
        .await
    }

    pub(crate) async fn demand_with_context(
        &mut self,
        runtime: DemandRuntimeContext<'_>,
        node_id: &NodeId,
    ) -> Result<NodeOutputMap> {
        // Track which nodes we're currently computing to detect cycles
        let mut computing = HashSet::new();
        self.demand_internal(node_id, runtime, &mut computing).await
    }

    /// Internal demand method with cycle detection
    ///
    /// Key insight: We must demand all dependencies FIRST to ensure their
    /// versions are up-to-date before we can check our own cache validity.
    fn demand_internal<'a>(
        &'a mut self,
        node_id: &'a NodeId,
        runtime: DemandRuntimeContext<'a>,
        computing: &'a mut HashSet<NodeId>,
    ) -> DemandFuture<'a, NodeOutputMap> {
        Box::pin(async move {
            execution_core::DemandExecutionCore::new(self, runtime, computing)
                .run_node(node_id)
                .await
        })
    }

    /// Demand outputs from multiple nodes in parallel where possible
    ///
    /// This analyzes the dependency graph and executes independent nodes
    /// concurrently while respecting dependencies.
    pub async fn demand_multiple(
        &mut self,
        node_ids: &[NodeId],
        graph: &WorkflowGraph,
        executor: &dyn TaskExecutor,
        context: &Context,
        event_sink: &dyn EventSink,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        let runtime =
            DemandRuntimeContext::new(graph, executor, context, event_sink, extensions, None);
        multi_demand::demand_multiple_with_default_budget(self, node_ids, runtime).await
    }

    /// Invalidate cache for a node and all its downstream dependents
    ///
    /// This is useful when you want to force re-execution of a subgraph,
    /// for example after external changes that the version system can't detect.
    pub fn invalidate_downstream(&mut self, node_id: &NodeId, graph: &WorkflowGraph) {
        let mut to_invalidate = vec![node_id.clone()];
        let mut invalidated = HashSet::new();

        while let Some(current) = to_invalidate.pop() {
            if invalidated.insert(current.clone()) {
                // Remove from cache
                self.cache.remove(&current);
                self.last_inputs.remove(&current);

                // Add all dependents (downstream nodes)
                for dependent in graph.get_dependents(&current) {
                    to_invalidate.push(dependent);
                }
            }
        }

        log::debug!(
            "Invalidated {} nodes downstream from '{}'",
            invalidated.len(),
            node_id
        );
    }
}

fn reconcile_changed_node_entries<T>(
    target: &mut HashMap<NodeId, T>,
    base: &HashMap<NodeId, T>,
    isolated: &HashMap<NodeId, T>,
) where
    T: Clone + PartialEq,
{
    let changed_node_ids: HashSet<NodeId> = base.keys().chain(isolated.keys()).cloned().collect();

    for node_id in changed_node_ids {
        if base.get(&node_id) == isolated.get(&node_id) {
            continue;
        }

        match isolated.get(&node_id) {
            Some(value) => {
                target.insert(node_id.clone(), value.clone());
            }
            None => {
                target.remove(&node_id);
            }
        }
    }
}

/// Statistics about the demand engine's cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of nodes with cached outputs
    pub cached_nodes: usize,
    /// Number of nodes with version tracking
    pub total_versions: usize,
    /// Global version counter
    pub global_version: u64,
}

/// Wrapper for executing workflows with graph-flow
///
/// This combines the DemandEngine with graph-flow's execution model,
/// providing a high-level API for demand-driven workflow execution.
pub struct WorkflowExecutor {
    /// The demand engine for caching and version tracking
    demand_engine: Arc<RwLock<DemandEngine>>,
    /// The graph-flow context for shared state
    context: Context,
    /// Event sink for streaming events
    event_sink: Arc<dyn EventSink>,
    /// The workflow graph
    graph: Arc<RwLock<WorkflowGraph>>,
    /// Phase 6 session-state scaffold for workflow execution session residency and
    /// checkpoint integration.
    session_state: Arc<session_state::WorkflowExecutorSessionState>,
    /// Execution ID
    execution_id: String,
    /// Typed extensions for non-serializable dependencies (API clients, etc.)
    extensions: ExecutorExtensions,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(
        execution_id: impl Into<String>,
        graph: WorkflowGraph,
        event_sink: Arc<dyn EventSink>,
    ) -> Self {
        let execution_id = execution_id.into();
        Self {
            demand_engine: Arc::new(RwLock::new(DemandEngine::new(&execution_id))),
            context: Context::new(),
            event_sink,
            graph: Arc::new(RwLock::new(graph)),
            session_state: Arc::new(session_state::WorkflowExecutorSessionState::new()),
            execution_id,
            extensions: ExecutorExtensions::new(),
        }
    }

    /// Get the execution ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Get a reference to the demand engine
    pub fn demand_engine(&self) -> &Arc<RwLock<DemandEngine>> {
        &self.demand_engine
    }

    /// Get a reference to the graph-flow context
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Get a mutable reference to the graph-flow context
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Get a reference to the executor extensions
    pub fn extensions(&self) -> &ExecutorExtensions {
        &self.extensions
    }

    /// Get a mutable reference to the executor extensions
    pub fn extensions_mut(&mut self) -> &mut ExecutorExtensions {
        &mut self.extensions
    }

    /// Get a reference to the workflow graph
    pub fn graph(&self) -> &Arc<RwLock<WorkflowGraph>> {
        &self.graph
    }

    /// Return the current workflow execution session residency state used by the Phase 6
    /// checkpoint scaffold.
    pub async fn workflow_execution_session_residency(
        &self,
    ) -> WorkflowExecutionSessionResidencyState {
        workflow_execution_session::workflow_execution_session_residency(self).await
    }

    /// Bind this executor to one logical workflow execution session so later Phase 6
    /// node-memory reads and writes do not infer session identity from
    /// transport-local execution ids.
    pub async fn bind_workflow_execution_session(
        &self,
        workflow_execution_session_id: impl Into<String>,
    ) {
        workflow_execution_session::bind_workflow_execution_session(
            self,
            workflow_execution_session_id,
        )
        .await;
    }

    /// Return the currently bound logical workflow execution session id, if any.
    pub async fn bound_workflow_execution_session_id(&self) -> Option<String> {
        workflow_execution_session::bound_workflow_execution_session_id(self).await
    }

    /// Clear the current logical workflow execution session binding from this executor.
    pub async fn clear_bound_workflow_execution_session(&self) {
        workflow_execution_session::clear_bound_workflow_execution_session(self).await;
    }

    /// Update the workflow execution session residency state used by the Phase 6
    /// checkpoint scaffold.
    pub async fn set_workflow_execution_session_residency(
        &self,
        state: WorkflowExecutionSessionResidencyState,
    ) {
        workflow_execution_session::set_workflow_execution_session_residency(self, state).await;
    }

    /// Return the current bounded checkpoint summary for a workflow execution session.
    pub async fn workflow_execution_session_checkpoint_summary(
        &self,
        workflow_execution_session_id: &str,
    ) -> WorkflowExecutionSessionCheckpointSummary {
        workflow_execution_session::workflow_execution_session_checkpoint_summary(
            self,
            workflow_execution_session_id,
        )
        .await
    }

    /// Mark the bound workflow execution session as having a backend-owned checkpoint
    /// artifact available for restore.
    pub async fn mark_workflow_execution_session_checkpoint_available(
        &self,
        workflow_execution_session_id: &str,
    ) {
        workflow_execution_session::mark_workflow_execution_session_checkpoint_available(
            self,
            workflow_execution_session_id,
        )
        .await;
    }

    /// Clear any backend-owned checkpoint marker for one workflow execution session.
    pub async fn clear_workflow_execution_session_checkpoint(
        &self,
        workflow_execution_session_id: &str,
    ) {
        workflow_execution_session::clear_workflow_execution_session_checkpoint(
            self,
            workflow_execution_session_id,
        )
        .await;
    }

    /// Return the backend-owned logical node-memory snapshots for one workflow
    /// session.
    pub async fn workflow_execution_session_node_memory_snapshots(
        &self,
        workflow_execution_session_id: &str,
    ) -> Vec<NodeMemorySnapshot> {
        workflow_execution_session::workflow_execution_session_node_memory_snapshots(
            self,
            workflow_execution_session_id,
        )
        .await
    }

    /// Record or replace the logical node-memory snapshot for one node in one
    /// workflow execution session.
    pub async fn record_workflow_execution_session_node_memory(
        &self,
        snapshot: NodeMemorySnapshot,
    ) {
        workflow_execution_session::record_workflow_execution_session_node_memory(self, snapshot)
            .await;
    }

    /// Clear all recorded logical node memory for one workflow execution session.
    pub async fn clear_workflow_execution_session_node_memory(
        &self,
        workflow_execution_session_id: &str,
    ) {
        workflow_execution_session::clear_workflow_execution_session_node_memory(
            self,
            workflow_execution_session_id,
        )
        .await;
    }

    /// Apply backend-owned graph memory impact reconciliation rules to one
    /// workflow execution session's recorded node memory.
    pub async fn reconcile_workflow_execution_session_node_memory(
        &self,
        workflow_execution_session_id: &str,
        memory_impact: &GraphMemoryImpactSummary,
    ) {
        workflow_execution_session::reconcile_workflow_execution_session_node_memory(
            self,
            workflow_execution_session_id,
            memory_impact,
        )
        .await;
    }

    /// Set a value in the context
    pub async fn set_context_value<T: serde::Serialize + Send + Sync>(&self, key: &str, value: T) {
        self.context.set(key, value).await;
    }

    /// Get a value from the context
    pub async fn get_context_value<T: serde::de::DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
    ) -> Option<T> {
        self.context.get(key).await
    }

    /// Send an event to the event sink
    pub fn send_event(
        &self,
        event: WorkflowEvent,
    ) -> std::result::Result<(), crate::events::EventError> {
        self.event_sink.send(event)
    }

    /// Set the event sink (used when transitioning from editing to running)
    pub fn set_event_sink(&mut self, event_sink: Arc<dyn EventSink>) {
        self.event_sink = event_sink;
    }

    fn emit_graph_modified(
        &self,
        workflow_id: String,
        dirty_tasks: Vec<NodeId>,
        memory_impact: Option<GraphMemoryImpactSummary>,
    ) {
        if let Some(event) = graph_events::graph_modified_event(
            workflow_id,
            &self.execution_id,
            dirty_tasks,
            memory_impact,
        ) {
            let _ = self.send_event(event);
        }
    }

    fn emit_incremental_execution_started(&self, workflow_id: String, task_ids: Vec<NodeId>) {
        if let Some(event) = graph_events::incremental_execution_started_event(
            workflow_id,
            &self.execution_id,
            task_ids,
        ) {
            let _ = self.send_event(event);
        }
    }

    /// Demand output from a specific node
    ///
    /// This is the main entry point for demand-driven execution.
    /// It will recursively compute all dependencies and cache results.
    pub async fn demand(
        &self,
        node_id: &NodeId,
        executor: &dyn TaskExecutor,
    ) -> Result<HashMap<String, serde_json::Value>> {
        single_demand::demand_with_executor(self, node_id, executor).await
    }

    /// Demand outputs from multiple nodes
    pub async fn demand_multiple(
        &self,
        node_ids: &[NodeId],
        executor: &dyn TaskExecutor,
    ) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        multi_demand::demand_multiple_with_executor(self, node_ids, executor).await
    }

    /// Mark a node as modified (e.g., user changed its data)
    ///
    /// This will invalidate the node's cache and mark downstream nodes
    /// for re-execution on next demand.
    pub async fn mark_modified(&self, node_id: &NodeId) {
        let (workflow_id, dirty_tasks) = {
            let graph = self.graph.read().await;
            (
                graph.id.clone(),
                graph_events::collect_dirty_tasks(&graph, node_id),
            )
        };
        let mut engine = self.demand_engine.write().await;
        engine.mark_modified(node_id);
        drop(engine);
        self.emit_graph_modified(workflow_id, dirty_tasks, None);
    }

    /// Update a node's data and mark it as modified
    pub async fn update_node_data(&self, node_id: &NodeId, data: serde_json::Value) -> Result<()> {
        graph_state::update_node_data(self, node_id, data).await
    }

    /// Add a new node to the graph
    pub async fn add_node(&self, node: crate::types::GraphNode) {
        graph_state::add_node(self, node).await;
    }

    /// Add a new edge to the graph
    ///
    /// This marks the target node as modified since its inputs changed.
    pub async fn add_edge(&self, edge: crate::types::GraphEdge) {
        graph_state::add_edge(self, edge).await;
    }

    /// Remove an edge from the graph
    ///
    /// This marks the target node as modified since its inputs changed.
    pub async fn remove_edge(&self, edge_id: &str) {
        graph_state::remove_edge(self, edge_id).await;
    }

    /// Get the current graph state (for undo snapshots)
    pub async fn get_graph_snapshot(&self) -> WorkflowGraph {
        graph_state::get_graph_snapshot(self).await
    }

    /// Restore graph from a snapshot (for undo/redo)
    ///
    /// This clears all caches since the graph structure may have changed.
    pub async fn restore_graph_snapshot(&self, graph: WorkflowGraph) {
        graph_state::restore_graph_snapshot(self, graph).await;
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let engine = self.demand_engine.read().await;
        engine.cache_stats()
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
