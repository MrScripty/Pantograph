//! Dependency Environment sidecar/control descriptor.
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `dependency-environment` node type. Dependency actions are resolved by
//! workflow-service from the typed sidecar association to one inference node.

use super::DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID;
use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_SELECTED_BINDING_IDS: &str = "selected_binding_ids";
const PORT_MODE: &str = "mode";
const PORT_MANUAL_OVERRIDES: &str = "manual_overrides";
const PORT_DEPENDENCY_ENVIRONMENT_SIDECAR: &str = DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID;

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
            category: NodeCategory::Control,
            label: "Dependency Environment".to_string(),
            description:
                "Controls dependency environment readiness for one associated inference node"
                    .to_string(),
            inputs: vec![
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
            outputs: vec![PortMetadata::optional(
                PORT_DEPENDENCY_ENVIRONMENT_SIDECAR,
                "Dependency Environment",
                PortDataType::DependencyEnvironmentSidecar,
            )],
            execution_mode: ExecutionMode::Manual,
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
            "dependency-environment actions must be resolved by workflow-service".into(),
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
        assert_eq!(meta.category, NodeCategory::Control);
        assert_eq!(meta.execution_mode, ExecutionMode::Manual);
        assert!(!meta.inputs.iter().any(|p| p.id == "pumas_model_ref"));
        assert!(!meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(!meta
            .inputs
            .iter()
            .any(|p| p.id == "dependency_requirements"));
        assert!(!meta.inputs.iter().any(|p| p.id == "model_id"));
        assert!(!meta.inputs.iter().any(|p| p.id == "model_type"));
        assert!(!meta.inputs.iter().any(|p| p.id == "task_type_primary"));
        assert!(!meta.inputs.iter().any(|p| p.id == "backend_key"));
        assert!(!meta.inputs.iter().any(|p| p.id == "platform_context"));
        assert!(meta.inputs.iter().any(|p| p.id == "mode"));
        assert!(meta.inputs.iter().any(|p| p.id == "manual_overrides"));
        assert!(!meta.outputs.iter().any(|p| p.id == "environment_ref"));
        assert!(!meta.outputs.iter().any(|p| p.id == "dependency_status"));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == PORT_DEPENDENCY_ENVIRONMENT_SIDECAR
                && p.data_type == PortDataType::DependencyEnvironmentSidecar));
    }
}
