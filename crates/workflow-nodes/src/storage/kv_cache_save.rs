//! KV Cache Save Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `kv-cache-save` node type. Actual execution is delegated to
//! `CoreTaskExecutor`, so `run()` always returns an error directing callers to
//! that backend-owned path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_CACHE_DATA: &str = "cache_data";
const PORT_MODEL_FINGERPRINT: &str = "model_fingerprint";
const PORT_LABEL: &str = "label";
const PORT_MARKERS: &str = "markers";
const PORT_STORAGE_POLICY: &str = "storage_policy";
const PORT_CACHE_DIR: &str = "cache_dir";
const PORT_COMPRESSED: &str = "compressed";
const PORT_CACHE_ID: &str = "cache_id";
const PORT_METADATA: &str = "metadata";

/// Stub descriptor for the KV-cache save node.
#[derive(Clone)]
pub struct KvCacheSaveTask {
    task_id: String,
}

impl KvCacheSaveTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for KvCacheSaveTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "kv-cache-save".to_string(),
            category: NodeCategory::Tool,
            label: "KV Cache Save".to_string(),
            description: "Save KV cache to memory or disk".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_CACHE_DATA, "Cache Data", PortDataType::KvCache),
                PortMetadata::required(
                    PORT_MODEL_FINGERPRINT,
                    "Model Fingerprint",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_LABEL, "Label", PortDataType::String),
                PortMetadata::optional(PORT_MARKERS, "Markers", PortDataType::Json),
                PortMetadata::optional(PORT_STORAGE_POLICY, "Storage Policy", PortDataType::String),
                PortMetadata::optional(PORT_CACHE_DIR, "Cache Dir", PortDataType::String),
                PortMetadata::optional(PORT_COMPRESSED, "Compressed", PortDataType::Boolean),
            ],
            outputs: vec![
                PortMetadata::required(PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::required(PORT_METADATA, "Metadata", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(KvCacheSaveTask::descriptor));

#[async_trait]
impl Task for KvCacheSaveTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "kv-cache-save requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_expected_ports() {
        let meta = KvCacheSaveTask::descriptor();
        assert_eq!(meta.node_type, "kv-cache-save");
        assert_eq!(meta.inputs.len(), 7);
        assert_eq!(meta.outputs.len(), 2);
        assert!(meta.inputs.iter().any(|port| port.id == "cache_data"));
        assert!(meta
            .inputs
            .iter()
            .any(|port| port.id == "model_fingerprint"));
        assert!(meta.inputs.iter().any(|port| port.id == "markers"));
        assert!(meta.inputs.iter().any(|port| port.id == "storage_policy"));
        assert!(meta.outputs.iter().any(|port| port.id == "cache_id"));
        assert!(meta.outputs.iter().any(|port| port.id == "metadata"));
    }

    #[tokio::test]
    async fn test_run_returns_core_executor_error() {
        let task = KvCacheSaveTask::new("test-save");
        let result = task.run(Context::new()).await;

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("CoreTaskExecutor"),
            "error should mention CoreTaskExecutor, got: {error}"
        );
    }
}
