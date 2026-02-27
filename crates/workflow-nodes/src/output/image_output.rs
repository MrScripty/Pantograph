//! Image Output Task
//!
//! Displays an image result in the workflow output.
//! Stores the image in context and can optionally pass it through for chaining.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Image Output Task
///
/// Displays an image result in the workflow output.
/// The image (base64-encoded) is stored in context for display and optionally
/// passed through for downstream chaining.
///
/// # Inputs (from context)
/// - `{task_id}.input.image` (optional) - Base64-encoded image data
///
/// # Outputs (to context)
/// - `{task_id}.output.image` - The same image (for chaining)
///
/// # Streaming
/// - `{task_id}.stream.image` - Stream event with the image content
#[derive(Clone)]
pub struct ImageOutputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ImageOutputTask {
    /// Port ID for image input/output
    pub const PORT_IMAGE: &'static str = "image";
    /// Port ID for streaming input
    pub const PORT_STREAM: &'static str = "stream";

    /// Create a new image output task
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

impl TaskDescriptor for ImageOutputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "image-output".to_string(),
            category: NodeCategory::Output,
            label: "Image Output".to_string(),
            description: "Displays image output from the workflow".to_string(),
            inputs: vec![
                PortMetadata::optional(Self::PORT_IMAGE, "Image", PortDataType::Image),
                PortMetadata::optional(Self::PORT_STREAM, "Stream", PortDataType::Stream),
            ],
            outputs: vec![PortMetadata::optional(
                Self::PORT_IMAGE,
                "Image",
                PortDataType::Image,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ImageOutputTask::descriptor));

#[async_trait]
impl Task for ImageOutputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get optional image input (base64-encoded)
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_IMAGE);
        let image: Option<String> = context.get(&input_key).await;

        if let Some(ref image_data) = image {
            // Store output in context (for chaining)
            let output_key = ContextKeys::output(&self.task_id, Self::PORT_IMAGE);
            context.set(&output_key, image_data.clone()).await;

            // Store stream data for frontend display
            let stream_key = ContextKeys::stream(&self.task_id, Self::PORT_IMAGE);
            context
                .set(
                    &stream_key,
                    serde_json::json!({
                        "type": "image",
                        "content": image_data
                    }),
                )
                .await;

            log::debug!(
                "ImageOutputTask {}: outputting image ({} bytes)",
                self.task_id,
                image_data.len()
            );
        } else {
            log::debug!(
                "ImageOutputTask {}: no image input (stream-only mode)",
                self.task_id,
            );
        }

        // Stream input is handled by the frontend event system (NodeStream events
        // propagate through edges), so no backend processing needed for it.

        Ok(TaskResult::new(image, NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ImageOutputTask::new("my_output");
        assert_eq!(task.id(), "my_output");
    }

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = ImageOutputTask::descriptor();
        assert_eq!(meta.node_type, "image-output");
    }

    #[tokio::test]
    async fn test_image_output_stores_in_context() {
        // Arrange
        let task = ImageOutputTask::new("test_output");
        let context = Context::new();
        let input_key = ContextKeys::input("test_output", "image");
        context
            .set(&input_key, "iVBORw0KGgoAAAA==".to_string())
            .await;

        // Act
        let result = task.run(context.clone()).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("iVBORw0KGgoAAAA=="));

        let output_key = ContextKeys::output("test_output", "image");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("iVBORw0KGgoAAAA==".to_string()));

        let stream_key = ContextKeys::stream("test_output", "image");
        let stream: Option<serde_json::Value> = context.get(&stream_key).await;
        assert!(stream.is_some());
        let stream_data = stream.unwrap();
        assert_eq!(stream_data["type"], "image");
        assert_eq!(stream_data["content"], "iVBORw0KGgoAAAA==");
    }

    #[tokio::test]
    async fn test_missing_image_ok() {
        // Arrange
        let task = ImageOutputTask::new("test_output");
        let context = Context::new();

        // Act — run without setting image (stream-only mode)
        let result = task.run(context).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response, None);
    }
}
