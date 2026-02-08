//! Text Output Task
//!
//! Displays text result in the workflow output.
//! Stores the text in context and can optionally pass it through for chaining.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Text Output Task
///
/// Displays text result in the workflow output.
/// The text is stored in context for display and optionally passed through.
///
/// # Inputs (from context)
/// - `{task_id}.input.text` (required) - The text to display
///
/// # Outputs (to context)
/// - `{task_id}.output.text` - The same text (for chaining)
///
/// # Streaming
/// - `{task_id}.stream.text` - Stream event with the text content
#[derive(Clone)]
pub struct TextOutputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl TextOutputTask {
    /// Port ID for text input/output
    pub const PORT_TEXT: &'static str = "text";

    /// Create a new text output task
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

impl TaskDescriptor for TextOutputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "text-output".to_string(),
            category: NodeCategory::Output,
            label: "Text Output".to_string(),
            description: "Displays text output from the workflow".to_string(),
            inputs: vec![PortMetadata::required(
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

inventory::submit!(node_engine::DescriptorFn(TextOutputTask::descriptor));

#[async_trait]
impl Task for TextOutputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: text
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_TEXT);
        let text: String = context.get(&input_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'text' at key '{}'",
                input_key
            ))
        })?;

        // Store output in context (for chaining)
        let output_key = ContextKeys::output(&self.task_id, Self::PORT_TEXT);
        context.set(&output_key, text.clone()).await;

        // Store stream data for frontend display
        // The frontend can watch for this key to display the output
        let stream_key = ContextKeys::stream(&self.task_id, Self::PORT_TEXT);
        context
            .set(
                &stream_key,
                serde_json::json!({
                    "type": "text",
                    "content": &text
                }),
            )
            .await;

        log::debug!(
            "TextOutputTask {}: outputting {} chars",
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
        let task = TextOutputTask::new("my_output");
        assert_eq!(task.id(), "my_output");
    }

    #[tokio::test]
    async fn test_text_output() {
        let task = TextOutputTask::new("test_output");
        let context = Context::new();

        // Set input text
        let input_key = ContextKeys::input("test_output", "text");
        context
            .set(&input_key, "Hello, world!".to_string())
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("Hello, world!"));

        // Verify output was stored
        let output_key = ContextKeys::output("test_output", "text");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("Hello, world!".to_string()));

        // Verify stream data was stored
        let stream_key = ContextKeys::stream("test_output", "text");
        let stream: Option<serde_json::Value> = context.get(&stream_key).await;
        assert!(stream.is_some());
        let stream_data = stream.unwrap();
        assert_eq!(stream_data["type"], "text");
        assert_eq!(stream_data["content"], "Hello, world!");
    }

    #[tokio::test]
    async fn test_missing_text_error() {
        let task = TextOutputTask::new("test_output");
        let context = Context::new();

        // Run without setting text - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
