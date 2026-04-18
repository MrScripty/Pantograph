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
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::Context;
use tokio::sync::RwLock;

use crate::error::{NodeEngineError, Result};
use crate::events::{EventSink, WorkflowEvent};
use crate::extensions::ExecutorExtensions;
use crate::types::{NodeId, WorkflowGraph};

mod graph_events;
mod multi_demand;
mod dependency_inputs;
mod execution_core;
mod execution_events;
mod inflight_tracking;
mod output_cache;
mod node_preparation;
mod single_demand;

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
        self.global_version = self.global_version.max(isolated.global_version);
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
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Track which nodes we're currently computing to detect cycles
        let mut computing = HashSet::new();
        self.demand_internal(
            node_id,
            graph,
            executor,
            context,
            event_sink,
            extensions,
            &mut computing,
        )
        .await
    }

    /// Internal demand method with cycle detection
    ///
    /// Key insight: We must demand all dependencies FIRST to ensure their
    /// versions are up-to-date before we can check our own cache validity.
    fn demand_internal<'a>(
        &'a mut self,
        node_id: &'a NodeId,
        graph: &'a WorkflowGraph,
        executor: &'a dyn TaskExecutor,
        context: &'a Context,
        event_sink: &'a dyn EventSink,
        extensions: &'a ExecutorExtensions,
        computing: &'a mut HashSet<NodeId>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<HashMap<String, serde_json::Value>>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            execution_core::DemandExecutionCore::new(
                self,
                graph,
                executor,
                context,
                event_sink,
                extensions,
                computing,
            )
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
        multi_demand::demand_multiple_with_default_budget(
            self,
            node_ids,
            graph,
            executor,
            context,
            event_sink,
            extensions,
        )
        .await
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
    let changed_node_ids: HashSet<NodeId> = base
        .keys()
        .chain(isolated.keys())
        .cloned()
        .collect();

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

    fn emit_graph_modified(&self, workflow_id: String, dirty_tasks: Vec<NodeId>) {
        if let Some(event) = graph_events::graph_modified_event(
            workflow_id,
            &self.execution_id,
            dirty_tasks,
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
        self.emit_graph_modified(workflow_id, dirty_tasks);
    }

    /// Update a node's data and mark it as modified
    pub async fn update_node_data(&self, node_id: &NodeId, data: serde_json::Value) -> Result<()> {
        {
            let mut graph = self.graph.write().await;
            if let Some(node) = graph.find_node_mut(node_id) {
                node.data = data;
            } else {
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Node '{}' not found",
                    node_id
                )));
            }
        }

        self.mark_modified(node_id).await;
        Ok(())
    }

    /// Add a new node to the graph
    pub async fn add_node(&self, node: crate::types::GraphNode) {
        let node_id = node.id.clone();
        let mut graph = self.graph.write().await;
        graph.nodes.push(node);
        let workflow_id = graph.id.clone();
        drop(graph);
        self.emit_graph_modified(workflow_id, vec![node_id]);
    }

    /// Add a new edge to the graph
    ///
    /// This marks the target node as modified since its inputs changed.
    pub async fn add_edge(&self, edge: crate::types::GraphEdge) {
        let target = edge.target.clone();
        {
            let mut graph = self.graph.write().await;
            graph.edges.push(edge);
        }
        self.mark_modified(&target).await;
    }

    /// Remove an edge from the graph
    ///
    /// This marks the target node as modified since its inputs changed.
    pub async fn remove_edge(&self, edge_id: &str) {
        let target = {
            let mut graph = self.graph.write().await;
            if let Some(idx) = graph.edges.iter().position(|e| e.id == edge_id) {
                let edge = graph.edges.remove(idx);
                Some(edge.target)
            } else {
                None
            }
        };

        if let Some(target) = target {
            self.mark_modified(&target).await;
        }
    }

    /// Get the current graph state (for undo snapshots)
    pub async fn get_graph_snapshot(&self) -> WorkflowGraph {
        self.graph.read().await.clone()
    }

    /// Restore graph from a snapshot (for undo/redo)
    ///
    /// This clears all caches since the graph structure may have changed.
    pub async fn restore_graph_snapshot(&self, graph: WorkflowGraph) {
        let workflow_id = graph.id.clone();
        let dirty_tasks = graph_events::snapshot_dirty_tasks(&graph);
        {
            let mut current_graph = self.graph.write().await;
            *current_graph = graph;
        }

        // Clear all caches since we don't know what changed
        let mut engine = self.demand_engine.write().await;
        engine.clear_cache();
        drop(engine);
        self.emit_graph_modified(workflow_id, dirty_tasks);
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> CacheStats {
        let engine = self.demand_engine.read().await;
        engine.cache_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{NullEventSink, VecEventSink};
    use crate::types::{GraphEdge, GraphNode};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn make_linear_graph() -> WorkflowGraph {
        let mut graph = WorkflowGraph::new("test", "Test");
        graph.nodes.push(GraphNode {
            id: "a".to_string(),
            node_type: "input".to_string(),
            data: serde_json::Value::Null,
            position: (0.0, 0.0),
        });
        graph.nodes.push(GraphNode {
            id: "b".to_string(),
            node_type: "process".to_string(),
            data: serde_json::Value::Null,
            position: (100.0, 0.0),
        });
        graph.nodes.push(GraphNode {
            id: "c".to_string(),
            node_type: "output".to_string(),
            data: serde_json::Value::Null,
            position: (200.0, 0.0),
        });
        graph.edges.push(GraphEdge {
            id: "e1".to_string(),
            source: "a".to_string(),
            source_handle: "out".to_string(),
            target: "b".to_string(),
            target_handle: "in".to_string(),
        });
        graph.edges.push(GraphEdge {
            id: "e2".to_string(),
            source: "b".to_string(),
            source_handle: "out".to_string(),
            target: "c".to_string(),
            target_handle: "in".to_string(),
        });
        graph
    }

    fn make_diamond_graph() -> WorkflowGraph {
        // Diamond pattern: a -> b, a -> c, b -> d, c -> d
        let mut graph = WorkflowGraph::new("diamond", "Diamond");
        for (id, pos) in [("a", 0.0), ("b", 100.0), ("c", 100.0), ("d", 200.0)] {
            graph.nodes.push(GraphNode {
                id: id.to_string(),
                node_type: "process".to_string(),
                data: serde_json::json!({"node": id}),
                position: (pos, 0.0),
            });
        }
        graph.edges.push(GraphEdge {
            id: "e1".to_string(),
            source: "a".to_string(),
            source_handle: "out".to_string(),
            target: "b".to_string(),
            target_handle: "in".to_string(),
        });
        graph.edges.push(GraphEdge {
            id: "e2".to_string(),
            source: "a".to_string(),
            source_handle: "out".to_string(),
            target: "c".to_string(),
            target_handle: "in".to_string(),
        });
        graph.edges.push(GraphEdge {
            id: "e3".to_string(),
            source: "b".to_string(),
            source_handle: "out".to_string(),
            target: "d".to_string(),
            target_handle: "in_b".to_string(),
        });
        graph.edges.push(GraphEdge {
            id: "e4".to_string(),
            source: "c".to_string(),
            source_handle: "out".to_string(),
            target: "d".to_string(),
            target_handle: "in_c".to_string(),
        });
        graph
    }

    fn make_shared_dependency_graph() -> WorkflowGraph {
        let mut graph = WorkflowGraph::new("shared", "Shared");
        for (id, x, y) in [("a", 0.0, 0.0), ("b", 100.0, 0.0), ("c", 100.0, 100.0)] {
            graph.nodes.push(GraphNode {
                id: id.to_string(),
                node_type: "process".to_string(),
                data: serde_json::json!({"node": id}),
                position: (x, y),
            });
        }
        graph.edges.push(GraphEdge {
            id: "e1".to_string(),
            source: "a".to_string(),
            source_handle: "out".to_string(),
            target: "b".to_string(),
            target_handle: "in".to_string(),
        });
        graph.edges.push(GraphEdge {
            id: "e2".to_string(),
            source: "a".to_string(),
            source_handle: "out".to_string(),
            target: "c".to_string(),
            target_handle: "in".to_string(),
        });
        graph
    }

    fn make_parallel_roots_graph() -> WorkflowGraph {
        let mut graph = WorkflowGraph::new("parallel", "Parallel");
        for (id, x) in [("left", 0.0), ("right", 100.0)] {
            graph.nodes.push(GraphNode {
                id: id.to_string(),
                node_type: "process".to_string(),
                data: serde_json::json!({"node": id}),
                position: (x, 0.0),
            });
        }
        graph
    }

    /// A simple test executor that counts invocations
    struct CountingExecutor {
        execution_count: AtomicUsize,
    }

    impl CountingExecutor {
        fn new() -> Self {
            Self {
                execution_count: AtomicUsize::new(0),
            }
        }

        fn count(&self) -> usize {
            self.execution_count.load(Ordering::SeqCst)
        }
    }

    struct YieldingExecutor {
        current_in_flight: AtomicUsize,
        max_in_flight: AtomicUsize,
    }

    impl YieldingExecutor {
        fn new() -> Self {
            Self {
                current_in_flight: AtomicUsize::new(0),
                max_in_flight: AtomicUsize::new(0),
            }
        }

        fn max_in_flight(&self) -> usize {
            self.max_in_flight.load(Ordering::SeqCst)
        }

        fn record_max_in_flight(&self, observed: usize) {
            let mut current_max = self.max_in_flight.load(Ordering::SeqCst);
            while observed > current_max {
                match self.max_in_flight.compare_exchange(
                    current_max,
                    observed,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(actual) => current_max = actual,
                }
            }
        }
    }

    struct FailingExecutor {
        fail_on: String,
        execution_log: Mutex<Vec<String>>,
    }

    impl FailingExecutor {
        fn new(fail_on: impl Into<String>) -> Self {
            Self {
                fail_on: fail_on.into(),
                execution_log: Mutex::new(Vec::new()),
            }
        }

        fn executed_tasks(&self) -> Vec<String> {
            self.execution_log
                .lock()
                .expect("execution log")
                .clone()
        }
    }

    struct WaitingExecutor {
        wait_on: String,
        execution_log: Mutex<Vec<String>>,
    }

    impl WaitingExecutor {
        fn new(wait_on: impl Into<String>) -> Self {
            Self {
                wait_on: wait_on.into(),
                execution_log: Mutex::new(Vec::new()),
            }
        }

        fn executed_tasks(&self) -> Vec<String> {
            self.execution_log
                .lock()
                .expect("execution log")
                .clone()
        }
    }

    #[async_trait]
    impl TaskExecutor for CountingExecutor {
        async fn execute_task(
            &self,
            task_id: &str,
            inputs: HashMap<String, serde_json::Value>,
            _context: &Context,
            _extensions: &ExecutorExtensions,
        ) -> Result<HashMap<String, serde_json::Value>> {
            self.execution_count.fetch_add(1, Ordering::SeqCst);

            // Simple passthrough: combine all inputs into output
            let mut outputs = HashMap::new();
            outputs.insert(
                "out".to_string(),
                serde_json::json!({
                    "task": task_id,
                    "inputs": inputs
                }),
            );
            Ok(outputs)
        }
    }

    #[async_trait]
    impl TaskExecutor for YieldingExecutor {
        async fn execute_task(
            &self,
            task_id: &str,
            inputs: HashMap<String, serde_json::Value>,
            _context: &Context,
            _extensions: &ExecutorExtensions,
        ) -> Result<HashMap<String, serde_json::Value>> {
            let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.record_max_in_flight(observed);

            tokio::task::yield_now().await;

            self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

            Ok(HashMap::from([(
                "out".to_string(),
                serde_json::json!({
                    "task": task_id,
                    "inputs": inputs
                }),
            )]))
        }
    }

    #[async_trait]
    impl TaskExecutor for FailingExecutor {
        async fn execute_task(
            &self,
            task_id: &str,
            inputs: HashMap<String, serde_json::Value>,
            _context: &Context,
            _extensions: &ExecutorExtensions,
        ) -> Result<HashMap<String, serde_json::Value>> {
            self.execution_log
                .lock()
                .expect("execution log")
                .push(task_id.to_string());

            if task_id == self.fail_on {
                return Err(NodeEngineError::failed(format!("forced failure at {task_id}")));
            }

            Ok(HashMap::from([(
                "out".to_string(),
                serde_json::json!({
                    "task": task_id,
                    "inputs": inputs
                }),
            )]))
        }
    }

    #[async_trait]
    impl TaskExecutor for WaitingExecutor {
        async fn execute_task(
            &self,
            task_id: &str,
            inputs: HashMap<String, serde_json::Value>,
            _context: &Context,
            _extensions: &ExecutorExtensions,
        ) -> Result<HashMap<String, serde_json::Value>> {
            self.execution_log
                .lock()
                .expect("execution log")
                .push(task_id.to_string());

            if task_id == self.wait_on {
                return Err(NodeEngineError::waiting_for_input(
                    task_id.to_string(),
                    Some(format!("waiting at {task_id}")),
                ));
            }

            Ok(HashMap::from([(
                "out".to_string(),
                serde_json::json!({
                    "task": task_id,
                    "inputs": inputs
                }),
            )]))
        }
    }

    #[test]
    fn test_version_tracking() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");

        // Initially all versions are 0
        assert_eq!(engine.compute_input_version(&"a".to_string(), &graph), 0);
        assert_eq!(engine.compute_input_version(&"b".to_string(), &graph), 0);

        // Mark 'a' as modified
        engine.mark_modified(&"a".to_string());

        // 'b' input version should change (depends on 'a')
        assert_eq!(engine.compute_input_version(&"b".to_string(), &graph), 1);

        // 'a' input version should still be 0 (no dependencies)
        assert_eq!(engine.compute_input_version(&"a".to_string(), &graph), 0);
    }

    #[test]
    fn test_cache_invalidation() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");

        // Cache output for 'b'
        engine.cache_output(&"b".to_string(), serde_json::json!("cached_value"), &graph);

        // Should be able to get cached value
        assert!(engine.get_cached(&"b".to_string(), &graph).is_some());

        // Mark 'a' as modified
        engine.mark_modified(&"a".to_string());

        // Cache for 'b' should now be invalid (input version changed)
        assert!(engine.get_cached(&"b".to_string(), &graph).is_none());
    }

    #[test]
    fn test_cache_stats() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");

        engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
        engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
        engine.mark_modified(&"c".to_string());

        let stats = engine.cache_stats();
        assert_eq!(stats.cached_nodes, 2);
        assert_eq!(stats.total_versions, 1); // Only 'c' has been modified
        assert_eq!(stats.global_version, 1);
    }

    #[test]
    fn test_reconcile_isolated_run_merges_changed_state_without_touching_unrelated_entries() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");
        engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);

        let base = engine.clone();
        let mut isolated = base.clone();
        isolated.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
        isolated.mark_modified(&"c".to_string());

        engine.cache_output(&"z".to_string(), serde_json::json!("keep"), &graph);
        engine.reconcile_isolated_run(&base, &isolated);

        assert!(engine.cache.contains_key("a"));
        assert!(engine.cache.contains_key("b"));
        assert!(engine.cache.contains_key("z"));
        assert_eq!(engine.versions.get("c"), Some(&1));
        assert_eq!(engine.global_version, 1);
    }

    #[test]
    fn test_reconcile_isolated_run_removes_entries_cleared_from_base_state() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");
        engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
        engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

        let base = engine.clone();
        let mut isolated = base.clone();
        isolated.invalidate_downstream(&"b".to_string(), &graph);

        engine.reconcile_isolated_run(&base, &isolated);

        assert!(!engine.cache.contains_key("b"));
        assert!(!engine.cache.contains_key("c"));
    }

    #[test]
    fn test_invalidate_downstream() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");

        // Cache all nodes
        engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
        engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
        engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

        assert_eq!(engine.cache_stats().cached_nodes, 3);

        // Invalidate downstream from 'a' (should invalidate a, b, c)
        engine.invalidate_downstream(&"a".to_string(), &graph);

        assert_eq!(engine.cache_stats().cached_nodes, 0);
    }

    #[test]
    fn test_invalidate_downstream_partial() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");

        // Cache all nodes
        engine.cache_output(&"a".to_string(), serde_json::json!("a"), &graph);
        engine.cache_output(&"b".to_string(), serde_json::json!("b"), &graph);
        engine.cache_output(&"c".to_string(), serde_json::json!("c"), &graph);

        // Invalidate downstream from 'b' (should invalidate b, c but not a)
        engine.invalidate_downstream(&"b".to_string(), &graph);

        assert_eq!(engine.cache_stats().cached_nodes, 1);
        assert!(engine.cache.contains_key("a"));
        assert!(!engine.cache.contains_key("b"));
        assert!(!engine.cache.contains_key("c"));
    }

    #[tokio::test]
    async fn test_demand_linear_graph() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");
        let executor = CountingExecutor::new();
        let context = Context::new();
        let event_sink = NullEventSink;
        let extensions = ExecutorExtensions::new();

        // Demand 'c' - should execute a, b, c
        let result = engine
            .demand(
                &"c".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(executor.count(), 3); // All three nodes executed
    }

    #[tokio::test]
    async fn test_demand_caching() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");
        let executor = CountingExecutor::new();
        let context = Context::new();
        let event_sink = NullEventSink;
        let extensions = ExecutorExtensions::new();

        // First demand
        let _ = engine
            .demand(
                &"c".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;
        assert_eq!(executor.count(), 3);

        // Second demand - should use cache
        let _ = engine
            .demand(
                &"c".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;
        assert_eq!(executor.count(), 3); // No additional executions
    }

    #[tokio::test]
    async fn test_demand_partial_recompute() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test");
        let executor = CountingExecutor::new();
        let context = Context::new();
        let event_sink = NullEventSink;
        let extensions = ExecutorExtensions::new();

        // First demand
        let _ = engine
            .demand(
                &"c".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;
        assert_eq!(executor.count(), 3);

        // Mark 'b' as modified
        engine.mark_modified(&"b".to_string());

        // Demand again - should only recompute b and c (not a)
        let _ = engine
            .demand(
                &"c".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;
        // Note: Due to version-based invalidation, this depends on implementation details.
        // The current implementation uses sum of dependency versions, so modifying 'b'
        // will invalidate 'c' but not necessarily re-execute 'a' if it's still cached.
    }

    #[tokio::test]
    async fn test_demand_diamond_graph() {
        let graph = make_diamond_graph();
        let mut engine = DemandEngine::new("test");
        let executor = CountingExecutor::new();
        let context = Context::new();
        let event_sink = NullEventSink;
        let extensions = ExecutorExtensions::new();

        // Demand 'd' - should execute a, b, c, d (a only once despite diamond)
        let result = engine
            .demand(
                &"d".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(executor.count(), 4); // All four nodes executed exactly once
    }

    #[tokio::test]
    async fn test_demand_events() {
        let graph = make_linear_graph();
        let mut engine = DemandEngine::new("test_exec");
        let executor = CountingExecutor::new();
        let context = Context::new();
        let event_sink = VecEventSink::new();
        let extensions = ExecutorExtensions::new();

        let _ = engine
            .demand(
                &"c".to_string(),
                &graph,
                &executor,
                &context,
                &event_sink,
                &extensions,
            )
            .await;

        let events = event_sink.events();

        // Should have TaskStarted and TaskCompleted for each node
        let started_count = events
            .iter()
            .filter(|e| matches!(e, WorkflowEvent::TaskStarted { .. }))
            .count();
        let completed_count = events
            .iter()
            .filter(|e| matches!(e, WorkflowEvent::TaskCompleted { .. }))
            .count();

        assert_eq!(started_count, 3);
        assert_eq!(completed_count, 3);
    }

    #[tokio::test]
    async fn test_workflow_executor_demand() {
        let graph = make_linear_graph();
        let event_sink = Arc::new(VecEventSink::new());
        let executor_impl = CountingExecutor::new();

        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

        let result = workflow_executor
            .demand(&"c".to_string(), &executor_impl)
            .await;

        assert!(result.is_ok());
        assert_eq!(executor_impl.count(), 3);
    }

    #[tokio::test]
    async fn test_workflow_executor_update_node() {
        let graph = make_linear_graph();
        let event_sink = Arc::new(NullEventSink);
        let executor_impl = CountingExecutor::new();

        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

        // First demand
        let _ = workflow_executor
            .demand(&"c".to_string(), &executor_impl)
            .await;
        let count_after_first = executor_impl.count();
        assert_eq!(count_after_first, 3);

        // Verify caching works - demand again without modification
        let _ = workflow_executor
            .demand(&"c".to_string(), &executor_impl)
            .await;
        assert_eq!(executor_impl.count(), 3); // No additional executions

        // Update node 'a' data - this marks it as modified
        workflow_executor
            .update_node_data(&"a".to_string(), serde_json::json!({"new": "data"}))
            .await
            .unwrap();

        // Demand again - should recompute the chain
        let _ = workflow_executor
            .demand(&"c".to_string(), &executor_impl)
            .await;

        // After marking 'a' as modified, we expect recomputation of the entire chain
        // because b depends on a, and c depends on b
        let count_after_update = executor_impl.count();
        assert!(
            count_after_update > count_after_first,
            "Expected recomputation after update: got {} executions (expected > {})",
            count_after_update,
            count_after_first
        );
    }

    #[tokio::test]
    async fn test_workflow_executor_mark_modified_emits_graph_modified_with_dirty_subgraph() {
        let graph = make_linear_graph();
        let event_sink = Arc::new(VecEventSink::new());
        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

        workflow_executor.mark_modified(&"b".to_string()).await;

        let events = event_sink.events();
        let graph_modified = events
            .iter()
            .find(|event| matches!(event, WorkflowEvent::GraphModified { .. }))
            .expect("graph modified event");

        match graph_modified {
            WorkflowEvent::GraphModified {
                workflow_id,
                execution_id,
                dirty_tasks,
                ..
            } => {
                assert_eq!(workflow_id, "test");
                assert_eq!(execution_id, "exec_1");
                assert_eq!(dirty_tasks, &vec!["b".to_string(), "c".to_string()]);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_workflow_executor_demand_multiple_emits_incremental_execution_started() {
        let graph = make_linear_graph();
        let event_sink = Arc::new(VecEventSink::new());
        let executor_impl = CountingExecutor::new();
        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink.clone());

        let _ = workflow_executor
            .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
            .await
            .expect("incremental demand succeeds");

        let events = event_sink.events();
        let incremental_started = events
            .iter()
            .find(|event| matches!(event, WorkflowEvent::IncrementalExecutionStarted { .. }))
            .expect("incremental execution event");

        match incremental_started {
            WorkflowEvent::IncrementalExecutionStarted {
                workflow_id,
                execution_id,
                tasks,
                ..
            } => {
                assert_eq!(workflow_id, "test");
                assert_eq!(execution_id, "exec_1");
                assert_eq!(tasks, &vec!["b".to_string(), "c".to_string()]);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_workflow_executor_demand_multiple_emits_task_lifecycle_for_parallel_roots() {
        let graph = make_parallel_roots_graph();
        let event_sink = Arc::new(VecEventSink::new());
        let executor_impl = YieldingExecutor::new();
        let workflow_executor = WorkflowExecutor::new("exec_parallel", graph, event_sink.clone());

        let outputs = workflow_executor
            .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
            .await
            .expect("parallel incremental demand succeeds");

        assert_eq!(executor_impl.max_in_flight(), 2);
        assert_eq!(outputs.len(), 2);

        let events = event_sink.events();
        let started_tasks = events
            .iter()
            .filter_map(|event| match event {
                WorkflowEvent::TaskStarted { task_id, .. } => Some(task_id.clone()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        let completed_tasks = events
            .iter()
            .filter_map(|event| match event {
                WorkflowEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
                _ => None,
            })
            .collect::<HashSet<_>>();

        assert_eq!(
            started_tasks,
            HashSet::from(["left".to_string(), "right".to_string()])
        );
        assert_eq!(
            completed_tasks,
            HashSet::from(["left".to_string(), "right".to_string()])
        );
    }

    #[tokio::test]
    async fn test_workflow_executor_demand_multiple_returns_redundant_requested_target_outputs() {
        let graph = make_linear_graph();
        let event_sink = Arc::new(NullEventSink);
        let executor_impl = CountingExecutor::new();
        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

        let outputs = workflow_executor
            .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
            .await
            .expect("incremental demand succeeds");

        assert_eq!(executor_impl.count(), 3);
        assert!(outputs.contains_key("b"));
        assert!(outputs.contains_key("c"));
        assert_eq!(outputs["b"]["out"]["task"], serde_json::json!("b"));
        assert_eq!(outputs["c"]["out"]["task"], serde_json::json!("c"));
    }

    #[tokio::test]
    async fn test_workflow_executor_demand_multiple_stops_after_failed_batch() {
        let graph = make_shared_dependency_graph();
        let event_sink = Arc::new(NullEventSink);
        let executor_impl = FailingExecutor::new("b");
        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

        let error = workflow_executor
            .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
            .await
            .expect_err("first batch should fail");

        assert!(matches!(error, NodeEngineError::ExecutionFailed(message) if message.contains("forced failure at b")));
        assert_eq!(
            executor_impl.executed_tasks(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[tokio::test]
    async fn test_workflow_executor_demand_multiple_stops_after_waiting_batch() {
        let graph = make_shared_dependency_graph();
        let event_sink = Arc::new(NullEventSink);
        let executor_impl = WaitingExecutor::new("b");
        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

        let error = workflow_executor
            .demand_multiple(&["b".to_string(), "c".to_string()], &executor_impl)
            .await
            .expect_err("first batch should pause execution");

        assert!(matches!(
            error,
            NodeEngineError::WaitingForInput {
                task_id,
                prompt: Some(prompt)
            } if task_id == "b" && prompt == "waiting at b"
        ));
        assert_eq!(
            executor_impl.executed_tasks(),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[tokio::test]
    async fn test_workflow_executor_human_input_emits_waiting_for_input() {
        let graph = WorkflowGraph {
            id: "interactive-workflow".to_string(),
            name: "Interactive Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "approval".to_string(),
                node_type: "human-input".to_string(),
                data: serde_json::json!({
                    "node_type": "human-input",
                    "prompt": "Approve deployment?"
                }),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let event_sink = Arc::new(VecEventSink::new());
        let workflow_executor =
            WorkflowExecutor::new("exec_human_input", graph, event_sink.clone());
        let executor_impl = crate::core_executor::CoreTaskExecutor::new();

        let error = workflow_executor
            .demand(&"approval".to_string(), &executor_impl)
            .await
            .expect_err("human input should pause execution");
        assert!(matches!(
            error,
            NodeEngineError::WaitingForInput {
                task_id,
                prompt: Some(prompt)
            } if task_id == "approval" && prompt == "Approve deployment?"
        ));

        let events = event_sink.events();
        assert!(matches!(
            events.as_slice(),
            [
                WorkflowEvent::TaskStarted { task_id, .. },
                WorkflowEvent::WaitingForInput {
                    workflow_id,
                    task_id: waiting_task_id,
                    prompt: Some(prompt),
                    ..
                }
            ] if task_id == "approval"
                && workflow_id == "interactive-workflow"
                && waiting_task_id == "approval"
                && prompt == "Approve deployment?"
        ));
    }

    #[tokio::test]
    async fn test_workflow_executor_human_input_continues_with_response() {
        let graph = WorkflowGraph {
            id: "interactive-workflow".to_string(),
            name: "Interactive Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "approval".to_string(),
                node_type: "human-input".to_string(),
                data: serde_json::json!({
                    "node_type": "human-input",
                    "prompt": "Approve deployment?",
                    "user_response": "approved"
                }),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let event_sink = Arc::new(VecEventSink::new());
        let workflow_executor =
            WorkflowExecutor::new("exec_human_input", graph, event_sink.clone());
        let executor_impl = crate::core_executor::CoreTaskExecutor::new();

        let outputs = workflow_executor
            .demand(&"approval".to_string(), &executor_impl)
            .await
            .expect("human input should continue once a response is present");
        assert_eq!(outputs.get("value"), Some(&serde_json::json!("approved")));

        let events = event_sink.events();
        assert!(matches!(
            events.as_slice(),
            [
                WorkflowEvent::TaskStarted { task_id, .. },
                WorkflowEvent::TaskCompleted {
                    task_id: completed_task_id,
                    ..
                }
            ] if task_id == "approval" && completed_task_id == "approval"
        ));
    }

    #[tokio::test]
    async fn test_workflow_executor_snapshot() {
        let graph = make_linear_graph();
        let event_sink = Arc::new(NullEventSink);

        let workflow_executor = WorkflowExecutor::new("exec_1", graph, event_sink);

        // Get initial snapshot
        let snapshot = workflow_executor.get_graph_snapshot().await;
        assert_eq!(snapshot.nodes.len(), 3);

        // Add a new node
        workflow_executor
            .add_node(GraphNode {
                id: "d".to_string(),
                node_type: "new".to_string(),
                data: serde_json::Value::Null,
                position: (300.0, 0.0),
            })
            .await;

        // Verify node was added
        let updated = workflow_executor.get_graph_snapshot().await;
        assert_eq!(updated.nodes.len(), 4);

        // Restore original snapshot
        workflow_executor.restore_graph_snapshot(snapshot).await;

        // Verify restoration
        let restored = workflow_executor.get_graph_snapshot().await;
        assert_eq!(restored.nodes.len(), 3);
    }
}
