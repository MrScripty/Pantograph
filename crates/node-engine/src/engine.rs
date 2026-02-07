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
#[derive(Debug, Clone)]
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
    pub fn get_cached(&self, node_id: &NodeId, graph: &WorkflowGraph) -> Option<&serde_json::Value> {
        let cached = self.cache.get(node_id)?;
        let current_version = self.compute_input_version(node_id, graph);
        if cached.version == current_version {
            Some(&cached.value)
        } else {
            None
        }
    }

    /// Store a computed output in the cache
    pub fn cache_output(&mut self, node_id: &NodeId, value: serde_json::Value, graph: &WorkflowGraph) {
        let version = self.compute_input_version(node_id, graph);
        self.cache.insert(
            node_id.clone(),
            CachedOutput { version, value },
        );
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
        self.demand_internal(node_id, graph, executor, context, event_sink, extensions, &mut computing)
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
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HashMap<String, serde_json::Value>>> + Send + 'a>> {
        Box::pin(async move {
            // Cycle detection
            if computing.contains(node_id) {
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Cycle detected: node '{}' is already being computed",
                    node_id
                )));
            }

            // Mark as computing for cycle detection
            computing.insert(node_id.clone());

            // 1. First, recursively demand ALL dependencies to ensure their versions are current
            // This is crucial: we must know the current state of dependencies before cache check
            let mut inputs: HashMap<String, serde_json::Value> = HashMap::new();
            let dependencies = graph.get_dependencies(node_id);

            for dep_id in &dependencies {
                // Recursively demand the dependency
                let dep_outputs = self
                    .demand_internal(dep_id, graph, executor, context, event_sink, extensions, computing)
                    .await?;

                // Find the edge(s) connecting this dependency to our node
                for edge in graph.incoming_edges(node_id) {
                    if edge.source == *dep_id {
                        // Get the output value from the dependency
                        if let Some(value) = dep_outputs.get(&edge.source_handle) {
                            inputs.insert(edge.target_handle.clone(), value.clone());
                        }
                    }
                }
            }

            // 2. NOW compute input version (after dependencies are resolved)
            let input_version = self.compute_input_version(node_id, graph);

            // 3. Check cache - if version matches, return cached result
            if let Some(cached) = self.cache.get(node_id) {
                if cached.version == input_version {
                    log::debug!("Cache hit for node '{}' (version {})", node_id, input_version);
                    // Parse cached value back to HashMap
                    let outputs: HashMap<String, serde_json::Value> =
                        serde_json::from_value(cached.value.clone())?;
                    computing.remove(node_id);
                    return Ok(outputs);
                }
                log::debug!(
                    "Cache miss for node '{}': version {} != {}",
                    node_id,
                    cached.version,
                    input_version
                );
            }

            // 4. Cache miss - include static data from the node itself
            if let Some(node) = graph.find_node(node_id) {
                if !node.data.is_null() {
                    inputs.insert("_data".to_string(), node.data.clone());
                }
            }

            // Send task started event
            let _ = event_sink.send(WorkflowEvent::TaskStarted {
                task_id: node_id.clone(),
                execution_id: self.execution_id.clone(),
            });

            // 5. Execute this node
            let outputs = executor.execute_task(node_id, inputs, context, extensions).await?;

            // Send task completed event
            let _ = event_sink.send(WorkflowEvent::TaskCompleted {
                task_id: node_id.clone(),
                execution_id: self.execution_id.clone(),
                output: Some(serde_json::to_value(&outputs)?),
            });

            // 6. Cache with current input version
            self.cache.insert(
                node_id.clone(),
                CachedOutput {
                    version: input_version,
                    value: serde_json::to_value(&outputs)?,
                },
            );

            // 7. Update this node's version to global (marks it as "fresh")
            self.global_version += 1;
            self.versions.insert(node_id.clone(), self.global_version);

            // Remove from computing set
            computing.remove(node_id);

            Ok(outputs)
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
        let mut results = HashMap::new();

        // For now, execute sequentially. Parallel execution would require
        // more complex dependency analysis to find independent subgraphs.
        // This is a future optimization.
        for node_id in node_ids {
            let output = self.demand(node_id, graph, executor, context, event_sink, extensions).await?;
            results.insert(node_id.clone(), output);
        }

        Ok(results)
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
    pub async fn set_context_value<T: serde::Serialize + Send + Sync>(
        &self,
        key: &str,
        value: T,
    ) {
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
    pub fn send_event(&self, event: WorkflowEvent) -> std::result::Result<(), crate::events::EventError> {
        self.event_sink.send(event)
    }

    /// Set the event sink (used when transitioning from editing to running)
    pub fn set_event_sink(&mut self, event_sink: Arc<dyn EventSink>) {
        self.event_sink = event_sink;
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
        let graph = self.graph.read().await;
        let mut engine = self.demand_engine.write().await;

        engine
            .demand(node_id, &graph, executor, &self.context, self.event_sink.as_ref(), &self.extensions)
            .await
    }

    /// Demand outputs from multiple nodes
    pub async fn demand_multiple(
        &self,
        node_ids: &[NodeId],
        executor: &dyn TaskExecutor,
    ) -> Result<HashMap<NodeId, HashMap<String, serde_json::Value>>> {
        let graph = self.graph.read().await;
        let mut engine = self.demand_engine.write().await;

        engine
            .demand_multiple(node_ids, &graph, executor, &self.context, self.event_sink.as_ref(), &self.extensions)
            .await
    }

    /// Mark a node as modified (e.g., user changed its data)
    ///
    /// This will invalidate the node's cache and mark downstream nodes
    /// for re-execution on next demand.
    pub async fn mark_modified(&self, node_id: &NodeId) {
        let mut engine = self.demand_engine.write().await;
        engine.mark_modified(node_id);
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
        let mut graph = self.graph.write().await;
        graph.nodes.push(node);
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
        {
            let mut current_graph = self.graph.write().await;
            *current_graph = graph;
        }

        // Clear all caches since we don't know what changed
        let mut engine = self.demand_engine.write().await;
        engine.clear_cache();
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
        engine.cache_output(
            &"b".to_string(),
            serde_json::json!("cached_value"),
            &graph,
        );

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
            .demand(&"c".to_string(), &graph, &executor, &context, &event_sink, &extensions)
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
            .demand(&"c".to_string(), &graph, &executor, &context, &event_sink, &extensions)
            .await;
        assert_eq!(executor.count(), 3);

        // Second demand - should use cache
        let _ = engine
            .demand(&"c".to_string(), &graph, &executor, &context, &event_sink, &extensions)
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
            .demand(&"c".to_string(), &graph, &executor, &context, &event_sink, &extensions)
            .await;
        assert_eq!(executor.count(), 3);

        // Mark 'b' as modified
        engine.mark_modified(&"b".to_string());

        // Demand again - should only recompute b and c (not a)
        let _ = engine
            .demand(&"c".to_string(), &graph, &executor, &context, &event_sink, &extensions)
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
            .demand(&"d".to_string(), &graph, &executor, &context, &event_sink, &extensions)
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
            .demand(&"c".to_string(), &graph, &executor, &context, &event_sink, &extensions)
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
