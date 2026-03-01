//! Vector Output Task
//!
//! Displays vector (embedding) output in the workflow.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Vector Output Task
///
/// Displays vector output from the workflow.
///
/// # Inputs (from context)
/// - `{task_id}.input.vector` (optional) - The vector to display
///
/// # Outputs (to context)
/// - `{task_id}.output.vector` - The same vector (for chaining)
#[derive(Clone)]
pub struct VectorOutputTask {
    task_id: String,
}

impl VectorOutputTask {
    /// Port ID for vector input/output
    pub const PORT_VECTOR: &'static str = "vector";

    /// Create a new vector output task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for VectorOutputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "vector-output".to_string(),
            category: NodeCategory::Output,
            label: "Vector Output".to_string(),
            description: "Displays vector output from the workflow".to_string(),
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

inventory::submit!(node_engine::DescriptorFn(VectorOutputTask::descriptor));

#[async_trait]
impl Task for VectorOutputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_VECTOR);
        let vector: Option<Vec<f64>> = context.get(&input_key).await;

        if let Some(vector) = vector {
            let output_key = ContextKeys::output(&self.task_id, Self::PORT_VECTOR);
            context.set(&output_key, vector.clone()).await;
            return Ok(TaskResult::new(
                Some(format!("Vector Output: {} dimensions", vector.len())),
                NextAction::Continue,
            ));
        }

        Ok(TaskResult::new(None, NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let meta = VectorOutputTask::descriptor();
        assert_eq!(meta.node_type, "vector-output");
        assert_eq!(meta.category, NodeCategory::Output);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.outputs.len(), 1);
        assert_eq!(meta.inputs[0].data_type, PortDataType::Embedding);
    }

    #[tokio::test]
    async fn test_vector_output_passthrough() {
        let task = VectorOutputTask::new("test_vector_output");
        let context = Context::new();

        let input_key = ContextKeys::input("test_vector_output", "vector");
        context.set(&input_key, vec![1.0_f64, 2.0_f64]).await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        let output_key = ContextKeys::output("test_vector_output", "vector");
        let output: Option<Vec<f64>> = context.get(&output_key).await;
        assert_eq!(output, Some(vec![1.0_f64, 2.0_f64]));
    }
}
