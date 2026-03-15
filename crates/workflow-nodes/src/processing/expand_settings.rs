//! Expand Settings Task
//!
//! Decomposes an inference settings schema into individual output ports,
//! allowing users to see and override model-specific parameters in the
//! workflow graph. The schema is passed through unchanged for downstream
//! inference nodes.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

/// Expand Settings Task
///
/// Receives an `inference_settings` JSON array (from puma-lib or model-provider)
/// and exposes each parameter as a matching optional input/output port pair.
/// The schema is also passed through unchanged so downstream inference nodes can
/// consume the authoritative schema while override-capable graph wiring stays
/// visible.
///
/// Dynamic per-parameter ports are added by the frontend's `syncExpandPorts()`
/// when the upstream model selection changes.
#[derive(Clone)]
pub struct ExpandSettingsTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ExpandSettingsTask {
    /// Port ID for inference settings input (JSON array of InferenceParamSchema)
    pub const PORT_INFERENCE_SETTINGS: &'static str = "inference_settings";

    /// Create a new expand settings task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for ExpandSettingsTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "expand-settings".to_string(),
            category: NodeCategory::Processing,
            label: "Expand Settings".to_string(),
            description: "Expands inference parameter schema into individual visible ports"
                .to_string(),
            inputs: vec![PortMetadata::required(
                Self::PORT_INFERENCE_SETTINGS,
                "Inference Settings",
                PortDataType::Json,
            )],
            outputs: vec![
                PortMetadata::required(
                    Self::PORT_INFERENCE_SETTINGS,
                    "Inference Settings",
                    PortDataType::Json,
                ),
                // Dynamic per-parameter inputs/outputs added by frontend
                // syncExpandPorts()
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ExpandSettingsTask::descriptor));

#[async_trait]
impl Task for ExpandSettingsTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        // Execution handled by CoreTaskExecutor::execute_expand_settings
        Err(GraphError::TaskExecutionFailed(
            "expand-settings requires CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_node_type() {
        let meta = ExpandSettingsTask::descriptor();
        assert_eq!(meta.node_type, "expand-settings");
        assert_eq!(meta.category, NodeCategory::Processing);
        assert_eq!(meta.execution_mode, ExecutionMode::Reactive);
    }

    #[test]
    fn test_descriptor_ports() {
        let meta = ExpandSettingsTask::descriptor();
        assert_eq!(meta.inputs.len(), 1, "Expected 1 static input port");
        assert_eq!(meta.outputs.len(), 1, "Expected 1 static output port");
        assert_eq!(meta.inputs[0].id, "inference_settings");
        assert_eq!(meta.outputs[0].id, "inference_settings");
    }

    #[test]
    fn test_task_id() {
        let task = ExpandSettingsTask::new("expand-1");
        assert_eq!(task.id(), "expand-1");
    }
}
