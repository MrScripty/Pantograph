//! KV Cache Load Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `kv-cache-load` node type. Actual execution is delegated to
//! `CoreTaskExecutor`, so `run()` always returns an error directing callers to
//! that backend-owned path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_CACHE_ID: &str = "cache_id";
const PORT_MODEL_FINGERPRINT: &str = "model_fingerprint";
const PORT_CACHE_DATA: &str = "cache_data";
const PORT_METADATA: &str = "metadata";
const PORT_VALID: &str = "valid";

/// Stub descriptor for the KV-cache load node.
#[derive(Clone)]
pub struct KvCacheLoadTask {
    task_id: String,
}

impl KvCacheLoadTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for KvCacheLoadTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "kv-cache-load".to_string(),
            category: NodeCategory::Tool,
            label: "KV Cache Load".to_string(),
            description: "Load KV cache from storage".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::required(
                    PORT_MODEL_FINGERPRINT,
                    "Model Fingerprint",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::required(PORT_CACHE_DATA, "Cache Data", PortDataType::KvCache),
                PortMetadata::required(PORT_METADATA, "Metadata", PortDataType::Json),
                PortMetadata::required(PORT_VALID, "Valid", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(KvCacheLoadTask::descriptor));

#[async_trait]
impl Task for KvCacheLoadTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "kv-cache-load requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_expected_ports() {
        let meta = KvCacheLoadTask::descriptor();
        assert_eq!(meta.node_type, "kv-cache-load");
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.outputs.len(), 3);
        assert!(meta.inputs.iter().any(|port| port.id == "cache_id"));
        assert!(meta
            .inputs
            .iter()
            .any(|port| port.id == "model_fingerprint"));
        assert!(meta.outputs.iter().any(|port| port.id == "cache_data"));
        assert!(meta.outputs.iter().any(|port| port.id == "metadata"));
        assert!(meta.outputs.iter().any(|port| port.id == "valid"));
    }

    #[tokio::test]
    async fn test_run_returns_core_executor_error() {
        let task = KvCacheLoadTask::new("test-load");
        let result = task.run(Context::new()).await;

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("CoreTaskExecutor"),
            "error should mention CoreTaskExecutor, got: {error}"
        );
    }
}
