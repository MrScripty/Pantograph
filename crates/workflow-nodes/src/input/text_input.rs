//! Text Input Task
//!
//! A simple passthrough task that provides text input to workflows.
//! Unlike HumanInputTask, this doesn't pause for user interaction -
//! it simply reads from context and passes the value through.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Text Input Task
///
/// Provides user-entered text as input to the workflow.
/// The text value is read from context and passed through as output.
///
/// # Inputs (from context)
/// - `{task_id}.input.text` (optional) - The text value to pass through
///
/// # Outputs (to context)
/// - `{task_id}.output.text` - The text value (empty string if not provided)
#[derive(Clone)]
pub struct TextInputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl TextInputTask {
    /// Port ID for text input
    pub const PORT_TEXT: &'static str = "text";

    /// Create a new text input task
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

impl TaskDescriptor for TextInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "text-input".to_string(),
            category: NodeCategory::Input,
            label: "Text Input".to_string(),
            description: "Provides text input to the workflow".to_string(),
            inputs: vec![PortMetadata::optional(
                Self::PORT_TEXT,
                "Text",
                PortDataType::String,
            )],
            outputs: vec![PortMetadata::optional(
                Self::PORT_TEXT,
                "Text",
                PortDataType::String,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(TextInputTask::descriptor));

#[async_trait]
impl Task for TextInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get text from context (optional - defaults to empty string)
        let input_key = ContextKeys::input(&self.task_id, "text");
        let text: String = context.get(&input_key).await.unwrap_or_default();

        // Store output in context
        let output_key = ContextKeys::output(&self.task_id, "text");
        context.set(&output_key, text.clone()).await;

        log::debug!(
            "TextInputTask {}: passing through {} chars",
            self.task_id,
            text.len()
        );

        Ok(TaskResult::new(Some(text), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = TextInputTask::new("my_input");
        assert_eq!(task.id(), "my_input");
    }

    #[tokio::test]
    async fn test_passthrough_text() {
        let task = TextInputTask::new("test_input");
        let context = Context::new();

        // Set input text
        let input_key = ContextKeys::input("test_input", "text");
        context.set(&input_key, "Hello, world!".to_string()).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("Hello, world!"));

        // Verify output was stored
        let output_key = ContextKeys::output("test_input", "text");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("Hello, world!".to_string()));
    }

    #[tokio::test]
    async fn test_empty_input() {
        let task = TextInputTask::new("test_input");
        let context = Context::new();

        // Run without setting input - should default to empty string
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some(""));

        // Verify empty output was stored
        let output_key = ContextKeys::output("test_input", "text");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some(String::new()));
    }
}
