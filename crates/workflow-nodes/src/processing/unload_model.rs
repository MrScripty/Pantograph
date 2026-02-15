//! Unload Model Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `unload-model` node type. Actual execution is delegated to the
//! host application via the callback bridge.
//!
//! # Usage
//!
//! Connect the `model_ref` input to an inference node's `model_ref`
//! output to identify which engine and model to unload. Connect the
//! `trigger` input to any upstream node's output — the unload will
//! execute only after that node completes (pull-based dependency).

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_REF: &str = "model_ref";
const PORT_TRIGGER: &str = "trigger";
const PORT_STATUS: &str = "status";
const PORT_TRIGGER_PASSTHROUGH: &str = "trigger_passthrough";

/// Stub descriptor for the unload-model node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections, but the actual unload work
/// is performed by the host through the callback bridge.
#[derive(Clone)]
pub struct UnloadModelTask {
    task_id: String,
}

impl UnloadModelTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for UnloadModelTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "unload-model".to_string(),
            category: NodeCategory::Processing,
            label: "Unload Model".to_string(),
            description: "Unloads a model from an inference engine when triggered".to_string(),
            inputs: vec![
                PortMetadata::required(
                    PORT_MODEL_REF,
                    "Model Reference",
                    PortDataType::Json,
                ),
                PortMetadata::required(PORT_TRIGGER, "Trigger", PortDataType::Any),
            ],
            outputs: vec![
                PortMetadata::optional(PORT_STATUS, "Status", PortDataType::String),
                PortMetadata::optional(
                    PORT_TRIGGER_PASSTHROUGH,
                    "Trigger Data",
                    PortDataType::Any,
                ),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(UnloadModelTask::descriptor));

#[async_trait]
impl Task for UnloadModelTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "unload-model requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = UnloadModelTask::descriptor();
        assert_eq!(meta.node_type, "unload-model");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = UnloadModelTask::descriptor();

        // 2 inputs: model_ref, trigger
        assert_eq!(meta.inputs.len(), 2);
        assert!(meta.inputs.iter().any(|p| p.id == "model_ref"));
        assert!(meta.inputs.iter().any(|p| p.id == "trigger"));

        // 2 outputs: status, trigger_passthrough
        assert_eq!(meta.outputs.len(), 2);
        assert!(meta.outputs.iter().any(|p| p.id == "status"));
        assert!(meta.outputs.iter().any(|p| p.id == "trigger_passthrough"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = UnloadModelTask::new("test-unload");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("callback bridge"),
            "error should mention callback bridge, got: {err}"
        );
    }
}
