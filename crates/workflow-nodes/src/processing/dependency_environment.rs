//! Dependency Environment Task - Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `dependency-environment` node type. Actual execution is delegated to
//! the host task executor, where dependency resolution/check/install and
//! environment materialization are implemented.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_DEPENDENCY_REQUIREMENTS: &str = "dependency_requirements";
const PORT_MODEL_ID: &str = "model_id";
const PORT_MODEL_TYPE: &str = "model_type";
const PORT_TASK_TYPE_PRIMARY: &str = "task_type_primary";
const PORT_BACKEND_KEY: &str = "backend_key";
const PORT_PLATFORM_CONTEXT: &str = "platform_context";
const PORT_SELECTED_BINDING_IDS: &str = "selected_binding_ids";
const PORT_MODE: &str = "mode";
const PORT_MANUAL_OVERRIDES: &str = "manual_overrides";

const PORT_ENVIRONMENT_REF: &str = "environment_ref";
const PORT_DEPENDENCY_STATUS: &str = "dependency_status";

#[derive(Clone)]
pub struct DependencyEnvironmentTask {
    task_id: String,
}

impl DependencyEnvironmentTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for DependencyEnvironmentTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "dependency-environment".to_string(),
            category: NodeCategory::Processing,
            label: "Dependency Environment".to_string(),
            description:
                "Resolve/check/install model dependencies and output an environment reference"
                    .to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(
                    PORT_DEPENDENCY_REQUIREMENTS,
                    "Dependency Requirements",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_MODEL_ID, "Model ID", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_TYPE, "Model Type", PortDataType::String),
                PortMetadata::optional(PORT_TASK_TYPE_PRIMARY, "Task Type", PortDataType::String),
                PortMetadata::optional(PORT_BACKEND_KEY, "Backend Key", PortDataType::String),
                PortMetadata::optional(PORT_PLATFORM_CONTEXT, "Platform", PortDataType::Json),
                PortMetadata::optional(
                    PORT_SELECTED_BINDING_IDS,
                    "Selected Bindings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_MODE, "Mode", PortDataType::String),
                PortMetadata::optional(
                    PORT_MANUAL_OVERRIDES,
                    "Manual Overrides",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::optional(PORT_ENVIRONMENT_REF, "Environment Ref", PortDataType::Json),
                PortMetadata::optional(
                    PORT_DEPENDENCY_REQUIREMENTS,
                    "Dependency Requirements",
                    PortDataType::Json,
                ),
                PortMetadata::optional(
                    PORT_DEPENDENCY_STATUS,
                    "Dependency Status",
                    PortDataType::Json,
                ),
            ],
            execution_mode: ExecutionMode::Batch,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(
    DependencyEnvironmentTask::descriptor
));

#[async_trait]
impl Task for DependencyEnvironmentTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "dependency-environment requires execution via host TaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = DependencyEnvironmentTask::descriptor();
        assert_eq!(meta.node_type, "dependency-environment");
    }

    #[test]
    fn test_descriptor_has_required_ports() {
        let meta = DependencyEnvironmentTask::descriptor();
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == "dependency_requirements"));
        assert!(meta.inputs.iter().any(|p| p.id == "mode"));
        assert!(meta.inputs.iter().any(|p| p.id == "manual_overrides"));
        assert!(meta.outputs.iter().any(|p| p.id == "environment_ref"));
        assert!(meta.outputs.iter().any(|p| p.id == "dependency_status"));
    }
}
