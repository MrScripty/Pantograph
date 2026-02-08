//! Puma-Lib Stub Descriptor
//!
//! This module registers a stub node descriptor for `puma-lib` so that
//! `register_builtins()` discovers the node via `inventory`. Actual execution
//! is handled by the host application through the callback bridge — the host
//! provides the model file path from its local pumas-core library.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata};

const PORT_MODEL_PATH: &str = "model_path";

/// Stub task for the puma-lib node.
///
/// The node is discoverable by all consumers (including puma-bot NIF) but
/// always fails at runtime — the host must intercept execution via the
/// callback bridge and supply the model file path itself.
#[derive(Clone)]
pub struct PumaLibTask {
    task_id: String,
}

impl PumaLibTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for PumaLibTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "puma-lib".to_string(),
            category: NodeCategory::Input,
            label: "Puma-Lib".to_string(),
            description: "Provides AI model file path".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata::optional(
                PORT_MODEL_PATH,
                "Model Path",
                PortDataType::String,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(PumaLibTask::descriptor));

#[async_trait]
impl Task for PumaLibTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "puma-lib requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = PumaLibTask::descriptor();
        assert_eq!(meta.node_type, "puma-lib");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = PumaLibTask::descriptor();

        assert!(meta.inputs.is_empty());
        assert_eq!(meta.outputs.len(), 1);

        let port = &meta.outputs[0];
        assert_eq!(port.id, "model_path");
        assert_eq!(port.data_type, PortDataType::String);
        assert!(!port.required);
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = PumaLibTask::new("test");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("callback bridge"),
            "expected callback bridge message, got: {err}"
        );
    }
}
