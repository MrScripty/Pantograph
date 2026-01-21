//! Image Input Task
//!
//! Provides image data (base64 encoded) from canvas capture or upstream nodes.
//! Outputs both the image data and optional capture bounds.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Capture bounds for image input
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Image Input Task
///
/// Provides image data (base64 encoded) from canvas capture.
/// Outputs both the image data and the capture bounds.
///
/// # Inputs (from context)
/// - `{task_id}.input.image_base64` (required) - The base64 encoded image data
/// - `{task_id}.input.bounds` (optional) - Capture bounds as JSON
///
/// # Outputs (to context)
/// - `{task_id}.output.image` - The base64 image data
/// - `{task_id}.output.bounds` - The capture bounds (null if not provided)
#[derive(Clone)]
pub struct ImageInputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ImageInputTask {
    /// Port ID for image input (base64 data)
    pub const PORT_IMAGE_BASE64: &'static str = "image_base64";
    /// Port ID for bounds input
    pub const PORT_BOUNDS: &'static str = "bounds";
    /// Port ID for image output
    pub const PORT_IMAGE: &'static str = "image";

    /// Create a new image input task
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

impl TaskDescriptor for ImageInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "image-input".to_string(),
            category: NodeCategory::Input,
            label: "Image Input".to_string(),
            description: "Provides image input to the workflow".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_IMAGE_BASE64, "Image Data", PortDataType::Image),
                PortMetadata::optional(Self::PORT_BOUNDS, "Bounds", PortDataType::Json),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_IMAGE, "Image", PortDataType::Image),
                PortMetadata::optional(Self::PORT_BOUNDS, "Bounds", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Task for ImageInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: image_base64
        let image_key = ContextKeys::input(&self.task_id, Self::PORT_IMAGE_BASE64);
        let image_base64: String = context.get(&image_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'image_base64' at key '{}'",
                image_key
            ))
        })?;

        // Get optional bounds
        let bounds_key = ContextKeys::input(&self.task_id, Self::PORT_BOUNDS);
        let bounds: Option<ImageBounds> = context.get(&bounds_key).await;

        // Store outputs in context
        let output_image_key = ContextKeys::output(&self.task_id, Self::PORT_IMAGE);
        context.set(&output_image_key, image_base64.clone()).await;

        let output_bounds_key = ContextKeys::output(&self.task_id, Self::PORT_BOUNDS);
        if let Some(ref b) = bounds {
            context.set(&output_bounds_key, b.clone()).await;
        }

        log::debug!(
            "ImageInputTask {}: passing through {} bytes of image data",
            self.task_id,
            image_base64.len()
        );

        Ok(TaskResult::new(Some(image_base64), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ImageInputTask::new("my_image");
        assert_eq!(task.id(), "my_image");
    }

    #[tokio::test]
    async fn test_image_passthrough() {
        let task = ImageInputTask::new("test_image");
        let context = Context::new();

        // Set image input
        let image_key = ContextKeys::input("test_image", "image_base64");
        context
            .set(&image_key, "iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB".to_string())
            .await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify output was stored
        let output_key = ContextKeys::output("test_image", "image");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(
            output,
            Some("iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB".to_string())
        );
    }

    #[tokio::test]
    async fn test_image_with_bounds() {
        let task = ImageInputTask::new("test_image");
        let context = Context::new();

        // Set image input
        let image_key = ContextKeys::input("test_image", "image_base64");
        context.set(&image_key, "base64data".to_string()).await;

        // Set bounds
        let bounds_key = ContextKeys::input("test_image", "bounds");
        let bounds = ImageBounds {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 200.0,
        };
        context.set(&bounds_key, bounds).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify bounds output was stored
        let output_bounds_key = ContextKeys::output("test_image", "bounds");
        let output_bounds: Option<ImageBounds> = context.get(&output_bounds_key).await;
        assert!(output_bounds.is_some());
        let b = output_bounds.unwrap();
        assert_eq!(b.x, 10.0);
        assert_eq!(b.width, 100.0);
    }

    #[tokio::test]
    async fn test_missing_image_error() {
        let task = ImageInputTask::new("test_image");
        let context = Context::new();

        // Run without setting image - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
