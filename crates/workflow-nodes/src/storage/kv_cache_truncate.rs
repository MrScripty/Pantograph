//! KV Cache Truncate Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `kv-cache-truncate` node type. Actual execution is delegated to
//! `CoreTaskExecutor`, so `run()` always returns an error directing callers to
//! that backend-owned path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_CACHE_ID: &str = "cache_id";
const PORT_MARKER_NAME: &str = "marker_name";
const PORT_TOKEN_POSITION: &str = "token_position";
const PORT_METADATA: &str = "metadata";

/// Stub descriptor for the KV-cache truncate node.
#[derive(Clone)]
pub struct KvCacheTruncateTask {
    task_id: String,
}

impl KvCacheTruncateTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for KvCacheTruncateTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "kv-cache-truncate".to_string(),
            category: NodeCategory::Tool,
            label: "KV Cache Truncate".to_string(),
            description: "Truncate KV cache to a marker or position".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::optional(PORT_MARKER_NAME, "Marker Name", PortDataType::String),
                PortMetadata::optional(PORT_TOKEN_POSITION, "Token Position", PortDataType::Number),
            ],
            outputs: vec![
                PortMetadata::required(PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::required(PORT_METADATA, "Metadata", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(KvCacheTruncateTask::descriptor));

#[async_trait]
impl Task for KvCacheTruncateTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "kv-cache-truncate requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_expected_ports() {
        let meta = KvCacheTruncateTask::descriptor();
        assert_eq!(meta.node_type, "kv-cache-truncate");
        assert_eq!(meta.inputs.len(), 3);
        assert_eq!(meta.outputs.len(), 2);
        assert!(meta.inputs.iter().any(|port| port.id == "cache_id"));
        assert!(meta.inputs.iter().any(|port| port.id == "marker_name"));
        assert!(meta.inputs.iter().any(|port| port.id == "token_position"));
        assert!(meta.outputs.iter().any(|port| port.id == "cache_id"));
        assert!(meta.outputs.iter().any(|port| port.id == "metadata"));
    }

    #[tokio::test]
    async fn test_run_returns_core_executor_error() {
        let task = KvCacheTruncateTask::new("test-truncate");
        let result = task.run(Context::new()).await;

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("CoreTaskExecutor"),
            "error should mention CoreTaskExecutor, got: {error}"
        );
    }
}
