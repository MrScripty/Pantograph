//! Composite task executor that chains a host-specific executor with the core.
//!
//! The host executor is tried first. If it signals that the node type is not
//! host-specific (by returning an error containing "requires host-specific executor"),
//! the request falls through to the `CoreTaskExecutor`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::core_executor::CoreTaskExecutor;
use crate::engine::TaskExecutor;
use crate::error::{NodeEngineError, Result};
use crate::extensions::ExecutorExtensions;

/// A composite executor that tries a host-specific executor first,
/// then falls back to the core executor for standard node types.
pub struct CompositeTaskExecutor {
    /// Host-specific executor (tried first). None if no host overrides.
    host: Option<Arc<dyn TaskExecutor>>,
    /// Core executor (fallback for standard nodes).
    core: Arc<CoreTaskExecutor>,
}

impl CompositeTaskExecutor {
    /// Create a new composite executor.
    ///
    /// - `host`: optional host-specific executor for platform-dependent nodes.
    /// - `core`: the core executor that handles all standard nodes.
    pub fn new(host: Option<Arc<dyn TaskExecutor>>, core: Arc<CoreTaskExecutor>) -> Self {
        Self { host, core }
    }
}

#[async_trait]
impl TaskExecutor for CompositeTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &graph_flow::Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // Try host-specific executor first
        if let Some(ref host) = self.host {
            match host
                .execute_task(task_id, inputs.clone(), context, extensions)
                .await
            {
                Ok(result) => return Ok(result),
                Err(NodeEngineError::ExecutionFailed(msg))
                    if msg.contains("requires host-specific executor") =>
                {
                    // Host doesn't handle this type — fall through to core
                }
                Err(e) => return Err(e),
            }
        }

        // Fall back to core executor
        self.core
            .execute_task(task_id, inputs, context, extensions)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_executor::CoreTaskExecutor;

    #[tokio::test]
    async fn test_composite_falls_through_to_core() {
        let core = Arc::new(CoreTaskExecutor::new());
        let composite = CompositeTaskExecutor::new(None, core);

        let mut inputs = HashMap::new();
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({"node_type": "text-input", "text": "hello"}),
        );

        let context = graph_flow::Context::new();
        let extensions = ExecutorExtensions::new();
        let result = composite
            .execute_task("text-input-1", inputs, &context, &extensions)
            .await
            .unwrap();

        assert_eq!(result["text"], "hello");
    }

    #[tokio::test]
    async fn test_composite_host_handles_node() {
        /// A mock host executor that only handles "custom-node".
        struct MockHostExecutor;

        #[async_trait]
        impl TaskExecutor for MockHostExecutor {
            async fn execute_task(
                &self,
                _task_id: &str,
                inputs: HashMap<String, serde_json::Value>,
                _context: &graph_flow::Context,
                _extensions: &ExecutorExtensions,
            ) -> Result<HashMap<String, serde_json::Value>> {
                let node_type = crate::core_executor::resolve_node_type(_task_id, &inputs);
                if node_type == "custom-node" {
                    let mut out = HashMap::new();
                    out.insert("handled_by".to_string(), serde_json::json!("host"));
                    Ok(out)
                } else {
                    Err(NodeEngineError::ExecutionFailed(format!(
                        "Node type '{}' requires host-specific executor",
                        node_type
                    )))
                }
            }
        }

        let core = Arc::new(CoreTaskExecutor::new());
        let host = Arc::new(MockHostExecutor) as Arc<dyn TaskExecutor>;
        let composite = CompositeTaskExecutor::new(Some(host), core);
        let context = graph_flow::Context::new();
        let extensions = ExecutorExtensions::new();

        // Host handles custom-node
        let mut inputs = HashMap::new();
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({"node_type": "custom-node"}),
        );
        let result = composite
            .execute_task("custom-node-1", inputs, &context, &extensions)
            .await
            .unwrap();
        assert_eq!(result["handled_by"], "host");

        // Core handles text-input (host falls through)
        let mut inputs = HashMap::new();
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({"node_type": "text-input", "text": "core handled"}),
        );
        let result = composite
            .execute_task("text-input-1", inputs, &context, &extensions)
            .await
            .unwrap();
        assert_eq!(result["text"], "core handled");
    }
}
