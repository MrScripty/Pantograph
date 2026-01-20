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

use std::collections::HashMap;
use std::sync::Arc;

use graph_flow::Context;
use tokio::sync::RwLock;

// Note: Result and NodeEngineError available via crate::error if needed
use crate::events::{EventSink, WorkflowEvent};
use crate::types::{NodeId, WorkflowGraph};

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
/// This combines the DemandEngine with graph-flow's execution model.
pub struct WorkflowExecutor {
    /// The demand engine for caching
    demand_engine: Arc<RwLock<DemandEngine>>,
    /// The graph-flow context
    context: Context,
    /// Event sink for streaming events
    event_sink: Arc<dyn EventSink>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(execution_id: impl Into<String>, event_sink: Arc<dyn EventSink>) -> Self {
        let execution_id = execution_id.into();
        Self {
            demand_engine: Arc::new(RwLock::new(DemandEngine::new(&execution_id))),
            context: Context::new(),
            event_sink,
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GraphEdge, GraphNode};

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
}
