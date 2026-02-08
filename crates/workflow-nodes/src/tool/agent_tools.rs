//! Agent Tools stub descriptor
//!
//! Registers the `agent-tools` node type so the frontend can render it.
//! Execution is delegated to the host via the callback bridge, so `run()`
//! always returns an error directing callers to use that path instead.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_TOOLS: &str = "tools";

/// Stub task for the agent-tools node type.
///
/// This node is rendered by the frontend but execution is handled
/// by the host-specific callback bridge, not by `graph-flow`.
#[derive(Clone)]
pub struct AgentToolsTask {
    task_id: String,
}

impl AgentToolsTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for AgentToolsTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "agent-tools".to_string(),
            category: NodeCategory::Tool,
            label: "Agent Tools".to_string(),
            description: "Configures available tools for agent".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata::optional(PORT_TOOLS, "Tools", PortDataType::Tools)],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(AgentToolsTask::descriptor));

#[async_trait]
impl Task for AgentToolsTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "agent-tools requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = AgentToolsTask::descriptor();
        assert_eq!(meta.node_type, "agent-tools");
        assert_eq!(meta.label, "Agent Tools");
        assert!(matches!(meta.category, NodeCategory::Tool));
        assert!(matches!(meta.execution_mode, ExecutionMode::Reactive));
        assert!(meta.inputs.is_empty());
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = AgentToolsTask::descriptor();
        assert_eq!(meta.outputs.len(), 1);
        let port = &meta.outputs[0];
        assert_eq!(port.id, "tools");
        assert!(matches!(port.data_type, PortDataType::Tools));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = AgentToolsTask::new("test-agent-tools");
        let context = Context::new();
        let result = task.run(context).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("callback bridge"));
    }
}
