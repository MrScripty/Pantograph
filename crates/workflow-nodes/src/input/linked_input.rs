//! Linked Input Task
//!
//! An input node that reads its value from a linked GUI element.
//! The linking is managed by the frontend (linkStore); this task
//! simply reads the current linked value from node data during execution.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Linked Input Task
///
/// Provides input from a linked GUI element (button, text input, checkbox, etc.).
/// The frontend injects the current linked value into node data before execution.
///
/// # Node Data (from frontend)
/// - `linked_value` - The current value from the linked GUI element
///
/// # Outputs (to context)
/// - `{task_id}.output.value` - The linked value (empty string if not linked)
#[derive(Clone)]
pub struct LinkedInputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl LinkedInputTask {
    /// Port ID for value output
    pub const PORT_VALUE: &'static str = "value";

    /// Create a new linked input task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl TaskDescriptor for LinkedInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "linked-input".to_string(),
            category: NodeCategory::Input,
            label: "Linked Input".to_string(),
            description: "Input that links to GUI elements for reactive binding".to_string(),
            inputs: vec![], // No graph inputs - value comes from GUI link
            outputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::String,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Task for LinkedInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // The linked value is passed from frontend via context
        // Frontend injects it before execution using the linked_value key
        let linked_value_key = format!("{}.linked_value", self.task_id);
        let value: String = context.get(&linked_value_key).await.unwrap_or_default();

        // Store output in context
        let output_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
        context.set(&output_key, value.clone()).await;

        log::debug!(
            "LinkedInputTask {}: read {} chars from linked element",
            self.task_id,
            value.len()
        );

        Ok(TaskResult::new(Some(value), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = LinkedInputTask::new("my_linked_input");
        assert_eq!(task.id(), "my_linked_input");
    }

    #[test]
    fn test_descriptor() {
        let meta = LinkedInputTask::descriptor();
        assert_eq!(meta.node_type, "linked-input");
        assert_eq!(meta.category, NodeCategory::Input);
        assert!(meta.inputs.is_empty());
        assert_eq!(meta.outputs.len(), 1);
        assert_eq!(meta.outputs[0].id, "value");
    }

    #[tokio::test]
    async fn test_linked_value() {
        let task = LinkedInputTask::new("test_linked");
        let context = Context::new();

        // Set linked value (as frontend would)
        context
            .set(&"test_linked.linked_value".to_string(), "Hello from GUI!".to_string())
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("Hello from GUI!"));

        // Verify output was stored
        let output_key = ContextKeys::output("test_linked", "value");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("Hello from GUI!".to_string()));
    }

    #[tokio::test]
    async fn test_no_linked_value() {
        let task = LinkedInputTask::new("test_linked");
        let context = Context::new();

        // Run without setting linked value - should default to empty string
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some(""));

        // Verify empty output was stored
        let output_key = ContextKeys::output("test_linked", "value");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some(String::new()));
    }
}
