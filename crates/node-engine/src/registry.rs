//! Node type registry for dynamic node resolution
//!
//! This module provides a registry that maps node type strings to executors
//! and metadata. It replaces the hardcoded match-statement dispatch pattern
//! with a dynamic, extensible registry.
//!
//! # Usage
//!
//! ```ignore
//! use node_engine::{NodeRegistry, NodeExecutor, TaskMetadata};
//!
//! let mut registry = NodeRegistry::new();
//! registry.register(MyTask::descriptor(), Arc::new(MyTaskFactory));
//!
//! // Use with DemandEngine via RegistryTaskExecutor
//! let task_executor = RegistryTaskExecutor::new(Arc::new(registry));
//! ```

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use graph_flow::Context;

use crate::descriptor::TaskMetadata;
use crate::error::{NodeEngineError, Result};
use crate::extensions::ExecutorExtensions;
use crate::types::NodeCategory;

/// Per-node-type executor
///
/// Unlike `TaskExecutor` which handles ALL node types via dispatch,
/// a `NodeExecutor` handles exactly one node type.
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute this node type with the given inputs
    async fn execute(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>>;
}

/// Factory for creating or returning a shared NodeExecutor
pub trait NodeExecutorFactory: Send + Sync {
    fn create_executor(&self) -> Arc<dyn NodeExecutor>;
}

/// A registration entry combining metadata with an optional executor factory
struct RegistryEntry {
    metadata: TaskMetadata,
    factory: Option<Arc<dyn NodeExecutorFactory>>,
}

/// Registry of node types with their metadata and executors
///
/// This is the central registry that maps node_type strings to:
/// 1. Metadata (ports, category, label) from TaskDescriptor
/// 2. Executor factories that create per-node executors
///
/// # Composability
///
/// Registries can be composed by merging:
/// ```ignore
/// let mut registry = NodeRegistry::new();
/// // Register built-in nodes...
/// registry.merge(external_registry); // Add plugin nodes
/// ```
pub struct NodeRegistry {
    entries: HashMap<String, RegistryEntry>,
}

impl NodeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register a node type with metadata and an executor factory
    pub fn register(
        &mut self,
        metadata: TaskMetadata,
        factory: Arc<dyn NodeExecutorFactory>,
    ) {
        self.entries.insert(
            metadata.node_type.clone(),
            RegistryEntry {
                metadata,
                factory: Some(factory),
            },
        );
    }

    /// Register a node type using an async callback function (FFI-friendly)
    ///
    /// The callback receives (task_id, inputs) and returns outputs.
    pub fn register_callback<F, Fut>(&mut self, metadata: TaskMetadata, callback: F)
    where
        F: Fn(String, HashMap<String, serde_json::Value>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<HashMap<String, serde_json::Value>>> + Send + 'static,
    {
        let executor = Arc::new(CallbackNodeExecutor {
            callback: Box::new(move |task_id, inputs| {
                Box::pin(callback(task_id, inputs))
            }),
        });
        let factory = Arc::new(SharedExecutorFactory {
            executor: executor as Arc<dyn NodeExecutor>,
        });
        self.register(metadata, factory);
    }

    /// Register a node type with metadata only (no executor)
    ///
    /// Used for metadata-only registrations (e.g., UI palette listing).
    pub fn register_metadata(&mut self, metadata: TaskMetadata) {
        self.entries.insert(
            metadata.node_type.clone(),
            RegistryEntry {
                metadata,
                factory: None,
            },
        );
    }

    /// Get metadata for a node type
    pub fn get_metadata(&self, node_type: &str) -> Option<&TaskMetadata> {
        self.entries.get(node_type).map(|e| &e.metadata)
    }

    /// Get all registered metadata
    pub fn all_metadata(&self) -> Vec<&TaskMetadata> {
        self.entries.values().map(|e| &e.metadata).collect()
    }

    /// Get metadata grouped by category
    pub fn metadata_by_category(&self) -> HashMap<NodeCategory, Vec<&TaskMetadata>> {
        let mut grouped: HashMap<NodeCategory, Vec<&TaskMetadata>> = HashMap::new();
        for entry in self.entries.values() {
            grouped
                .entry(entry.metadata.category.clone())
                .or_default()
                .push(&entry.metadata);
        }
        grouped
    }

    /// Get the executor for a node type
    pub fn get_executor(&self, node_type: &str) -> Option<Arc<dyn NodeExecutor>> {
        self.entries
            .get(node_type)
            .and_then(|e| e.factory.as_ref())
            .map(|f| f.create_executor())
    }

    /// Check if a node type is registered
    pub fn has_node_type(&self, node_type: &str) -> bool {
        self.entries.contains_key(node_type)
    }

    /// List all registered node type strings
    pub fn node_types(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    /// Merge another registry into this one
    ///
    /// Entries from `other` override entries in `self` if they share the same node_type.
    pub fn merge(&mut self, other: NodeRegistry) {
        self.entries.extend(other.entries);
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Async callback-based NodeExecutor for FFI bridging
///
/// Wraps an async closure as a NodeExecutor. Critical for Rustler NIFs
/// where each node type is backed by an Elixir callback.
pub struct CallbackNodeExecutor {
    callback: Box<
        dyn Fn(
                String,
                HashMap<String, serde_json::Value>,
            ) -> Pin<Box<dyn std::future::Future<Output = Result<HashMap<String, serde_json::Value>>> + Send>>
            + Send
            + Sync,
    >,
}

#[async_trait]
impl NodeExecutor for CallbackNodeExecutor {
    async fn execute(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        (self.callback)(task_id.to_string(), inputs).await
    }
}

/// Synchronous callback-based NodeExecutor
///
/// Wraps a synchronous closure for simpler FFI scenarios.
pub struct SyncCallbackNodeExecutor {
    callback: Box<
        dyn Fn(&str, HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>>
            + Send
            + Sync,
    >,
}

impl SyncCallbackNodeExecutor {
    pub fn new(
        callback: impl Fn(&str, HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
}

#[async_trait]
impl NodeExecutor for SyncCallbackNodeExecutor {
    async fn execute(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        (self.callback)(task_id, inputs)
    }
}

/// Factory that returns a shared executor instance
struct SharedExecutorFactory {
    executor: Arc<dyn NodeExecutor>,
}

impl NodeExecutorFactory for SharedExecutorFactory {
    fn create_executor(&self) -> Arc<dyn NodeExecutor> {
        self.executor.clone()
    }
}

/// TaskExecutor implementation that delegates to a NodeRegistry
///
/// This bridges the existing `TaskExecutor` interface (used by DemandEngine)
/// with the new per-node-type `NodeExecutor` pattern.
///
/// Node type is extracted from `inputs._data.node_type`, matching the
/// existing dispatch pattern in PantographTaskExecutor.
pub struct RegistryTaskExecutor {
    registry: Arc<NodeRegistry>,
}

impl RegistryTaskExecutor {
    /// Create a new registry-based task executor
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        Self { registry }
    }

    /// Get a reference to the underlying registry
    pub fn registry(&self) -> &NodeRegistry {
        &self.registry
    }
}

#[async_trait]
impl crate::engine::TaskExecutor for RegistryTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Extract node_type from _data.node_type (same pattern as PantographTaskExecutor)
        let node_type = inputs
            .get("_data")
            .and_then(|d| d.get("node_type"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Fallback: infer from task_id pattern (e.g., "text-input-1" -> "text-input")
                let parts: Vec<&str> = task_id.rsplitn(2, '-').collect();
                if parts.len() == 2 {
                    parts[1].to_string()
                } else {
                    task_id.to_string()
                }
            });

        let executor = self.registry.get_executor(&node_type).ok_or_else(|| {
            NodeEngineError::ExecutionFailed(format!(
                "No executor registered for node type '{}' (task_id: '{}')",
                node_type, task_id
            ))
        })?;

        executor.execute(task_id, inputs, context, extensions).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{PortMetadata, TaskMetadata};
    use crate::engine::TaskExecutor;
    use crate::types::{ExecutionMode, NodeCategory, PortDataType};

    fn test_metadata(node_type: &str) -> TaskMetadata {
        TaskMetadata {
            node_type: node_type.to_string(),
            category: NodeCategory::Processing,
            label: format!("Test {}", node_type),
            description: "Test node".to_string(),
            inputs: vec![PortMetadata::optional("input", "Input", PortDataType::String)],
            outputs: vec![PortMetadata::optional("output", "Output", PortDataType::String)],
            execution_mode: ExecutionMode::Reactive,
        }
    }

    #[test]
    fn test_register_and_lookup_metadata() {
        let mut registry = NodeRegistry::new();
        registry.register_metadata(test_metadata("test-node"));

        assert!(registry.has_node_type("test-node"));
        assert!(!registry.has_node_type("unknown"));

        let meta = registry.get_metadata("test-node").unwrap();
        assert_eq!(meta.label, "Test test-node");
    }

    #[test]
    fn test_all_metadata() {
        let mut registry = NodeRegistry::new();
        registry.register_metadata(test_metadata("node-a"));
        registry.register_metadata(test_metadata("node-b"));

        assert_eq!(registry.all_metadata().len(), 2);
        assert_eq!(registry.node_types().len(), 2);
    }

    #[test]
    fn test_merge_registries() {
        let mut registry1 = NodeRegistry::new();
        registry1.register_metadata(test_metadata("node-a"));

        let mut registry2 = NodeRegistry::new();
        registry2.register_metadata(test_metadata("node-b"));
        registry2.register_metadata(test_metadata("node-c"));

        registry1.merge(registry2);
        assert_eq!(registry1.all_metadata().len(), 3);
    }

    #[test]
    fn test_merge_override() {
        let mut registry1 = NodeRegistry::new();
        let mut meta1 = test_metadata("node-a");
        meta1.label = "Original".to_string();
        registry1.register_metadata(meta1);

        let mut registry2 = NodeRegistry::new();
        let mut meta2 = test_metadata("node-a");
        meta2.label = "Override".to_string();
        registry2.register_metadata(meta2);

        registry1.merge(registry2);
        assert_eq!(
            registry1.get_metadata("node-a").unwrap().label,
            "Override"
        );
    }

    #[tokio::test]
    async fn test_sync_callback_executor() {
        let executor = SyncCallbackNodeExecutor::new(|task_id, _inputs| {
            let mut outputs = HashMap::new();
            outputs.insert("result".to_string(), serde_json::json!(task_id));
            Ok(outputs)
        });

        let context = Context::new();
        let extensions = ExecutorExtensions::new();
        let result = executor
            .execute("test-1", HashMap::new(), &context, &extensions)
            .await
            .unwrap();

        assert_eq!(result.get("result").unwrap(), "test-1");
    }

    #[tokio::test]
    async fn test_register_with_callback() {
        let mut registry = NodeRegistry::new();
        registry.register_callback(test_metadata("echo"), |_task_id, inputs| async move {
            Ok(inputs)
        });

        assert!(registry.has_node_type("echo"));
        let executor = registry.get_executor("echo").unwrap();
        let context = Context::new();

        let mut inputs = HashMap::new();
        inputs.insert("value".to_string(), serde_json::json!("hello"));

        let extensions = ExecutorExtensions::new();
        let result = executor.execute("echo-1", inputs, &context, &extensions).await.unwrap();
        assert_eq!(result.get("value").unwrap(), "hello");
    }

    #[tokio::test]
    async fn test_registry_task_executor() {
        let mut registry = NodeRegistry::new();
        registry.register_callback(test_metadata("echo"), |task_id, _inputs| async move {
            let mut outputs = HashMap::new();
            outputs.insert("out".to_string(), serde_json::json!({"task": task_id}));
            Ok(outputs)
        });

        let task_executor = RegistryTaskExecutor::new(Arc::new(registry));
        let context = Context::new();

        // With explicit node_type in _data
        let mut inputs = HashMap::new();
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({"node_type": "echo"}),
        );

        let extensions = ExecutorExtensions::new();
        let result = task_executor
            .execute_task("echo-1", inputs, &context, &extensions)
            .await
            .unwrap();

        assert_eq!(result.get("out").unwrap().get("task").unwrap(), "echo-1");
    }

    #[tokio::test]
    async fn test_registry_task_executor_unknown_type() {
        let registry = NodeRegistry::new();
        let task_executor = RegistryTaskExecutor::new(Arc::new(registry));
        let context = Context::new();

        let mut inputs = HashMap::new();
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({"node_type": "unknown"}),
        );

        let extensions = ExecutorExtensions::new();
        let result: Result<HashMap<String, serde_json::Value>> =
            task_executor.execute_task("unknown-1", inputs, &context, &extensions).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_by_category() {
        let mut registry = NodeRegistry::new();

        let mut meta_input = test_metadata("text-input");
        meta_input.category = NodeCategory::Input;
        registry.register_metadata(meta_input);

        let mut meta_proc = test_metadata("llm-inference");
        meta_proc.category = NodeCategory::Processing;
        registry.register_metadata(meta_proc);

        let grouped = registry.metadata_by_category();
        assert_eq!(grouped.get(&NodeCategory::Input).unwrap().len(), 1);
        assert_eq!(grouped.get(&NodeCategory::Processing).unwrap().len(), 1);
    }

    #[test]
    fn test_no_executor_for_metadata_only() {
        let mut registry = NodeRegistry::new();
        registry.register_metadata(test_metadata("metadata-only"));

        assert!(registry.has_node_type("metadata-only"));
        assert!(registry.get_executor("metadata-only").is_none());
    }
}
