//! Vector Input Task
//!
//! Provides an embedding/vector input source for workflows.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Vector Input Task
///
/// Provides a vector (embedding) as workflow input.
///
/// # Inputs (from context)
/// - `{task_id}.input.vector` (optional) - The vector value to pass through
///
/// # Outputs (to context)
/// - `{task_id}.output.vector` - The vector value
#[derive(Clone)]
pub struct VectorInputTask {
    task_id: String,
}

impl VectorInputTask {
    /// Port ID for vector input/output
    pub const PORT_VECTOR: &'static str = "vector";

    /// Create a new vector input task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for VectorInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "vector-input".to_string(),
            category: NodeCategory::Input,
            label: "Vector Input".to_string(),
            description: "Provides vector input to the workflow".to_string(),
            inputs: vec![PortMetadata::optional(
                Self::PORT_VECTOR,
                "Vector",
                PortDataType::Embedding,
            )],
            outputs: vec![PortMetadata::optional(
                Self::PORT_VECTOR,
                "Vector",
                PortDataType::Embedding,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(VectorInputTask::descriptor));

#[async_trait]
impl Task for VectorInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_VECTOR);
        let vector: Vec<f64> = context.get(&input_key).await.unwrap_or_default();

        let output_key = ContextKeys::output(&self.task_id, Self::PORT_VECTOR);
        context.set(&output_key, vector.clone()).await;

        Ok(TaskResult::new(
            Some(format!("Vector Input: {} dimensions", vector.len())),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let meta = VectorInputTask::descriptor();
        assert_eq!(meta.node_type, "vector-input");
        assert_eq!(meta.category, NodeCategory::Input);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.outputs.len(), 1);
        assert_eq!(meta.inputs[0].data_type, PortDataType::Embedding);
    }

    #[tokio::test]
    async fn test_passthrough_vector() {
        let task = VectorInputTask::new("test_vector_input");
        let context = Context::new();

        let input_key = ContextKeys::input("test_vector_input", "vector");
        context
            .set(&input_key, vec![0.1_f64, 0.2_f64, 0.3_f64])
            .await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        let output_key = ContextKeys::output("test_vector_input", "vector");
        let output: Option<Vec<f64>> = context.get(&output_key).await;
        assert_eq!(output, Some(vec![0.1_f64, 0.2_f64, 0.3_f64]));
    }
}
