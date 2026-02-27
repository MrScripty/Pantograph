//! Point Cloud Output Task
//!
//! Displays a 3D point cloud visualization in the workflow output.
//! Desktop-only — requires Three.js/Threlte for rendering.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Point Cloud Output Task
///
/// Displays a 3D point cloud visualization from depth estimation data.
/// The point cloud JSON (positions + colors) is stored in context for
/// the frontend Threlte component to render.
///
/// # Inputs (from context)
/// - `{task_id}.input.point_cloud` (required) - JSON with positions and colors
/// - `{task_id}.input.source_image` (optional) - Original image for reference
///
/// # Streaming
/// - `{task_id}.stream.point_cloud` - Stream event with the point cloud data
#[derive(Clone)]
pub struct PointCloudOutputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl PointCloudOutputTask {
    /// Port ID for point cloud data input
    pub const PORT_POINT_CLOUD: &'static str = "point_cloud";
    /// Port ID for optional source image
    pub const PORT_SOURCE_IMAGE: &'static str = "source_image";

    /// Create a new point cloud output task
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

impl TaskDescriptor for PointCloudOutputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "point-cloud-output".to_string(),
            category: NodeCategory::Output,
            label: "Point Cloud Output".to_string(),
            description: "Displays a 3D point cloud visualization".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_POINT_CLOUD, "Point Cloud", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_SOURCE_IMAGE,
                    "Source Image",
                    PortDataType::Image,
                ),
            ],
            outputs: vec![],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[cfg(feature = "desktop")]
inventory::submit!(node_engine::DescriptorFn(PointCloudOutputTask::descriptor));

#[async_trait]
impl Task for PointCloudOutputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required point cloud data
        let pc_key = ContextKeys::input(&self.task_id, Self::PORT_POINT_CLOUD);
        let point_cloud: Option<serde_json::Value> = context.get(&pc_key).await;

        if let Some(ref pc_data) = point_cloud {
            // Store stream data for frontend 3D rendering
            let stream_key = ContextKeys::stream(&self.task_id, Self::PORT_POINT_CLOUD);
            context
                .set(
                    &stream_key,
                    serde_json::json!({
                        "type": "point_cloud",
                        "data": pc_data
                    }),
                )
                .await;

            log::debug!(
                "PointCloudOutputTask {}: point cloud data stored for rendering",
                self.task_id,
            );
        } else {
            log::debug!(
                "PointCloudOutputTask {}: no point cloud data received",
                self.task_id,
            );
        }

        Ok(TaskResult::new(
            point_cloud.map(|v| v.to_string()),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = PointCloudOutputTask::new("my_pc");
        assert_eq!(task.id(), "my_pc");
    }

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = PointCloudOutputTask::descriptor();
        assert_eq!(meta.node_type, "point-cloud-output");
    }

    #[tokio::test]
    async fn test_point_cloud_stores_stream_data() {
        // Arrange
        let task = PointCloudOutputTask::new("test_pc");
        let context = Context::new();
        let input_key = ContextKeys::input("test_pc", "point_cloud");
        let pc_data = serde_json::json!({
            "positions": [[0.0, 0.0, 1.0], [1.0, 0.0, 2.0]],
            "colors": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        });
        context.set(&input_key, pc_data).await;

        // Act
        let result = task.run(context.clone()).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        assert!(result.response.is_some());

        let stream_key = ContextKeys::stream("test_pc", "point_cloud");
        let stream: Option<serde_json::Value> = context.get(&stream_key).await;
        assert!(stream.is_some());
        let stream_data = stream.unwrap();
        assert_eq!(stream_data["type"], "point_cloud");
        assert!(stream_data["data"]["positions"].is_array());
    }

    #[tokio::test]
    async fn test_missing_point_cloud_ok() {
        // Arrange
        let task = PointCloudOutputTask::new("test_pc");
        let context = Context::new();

        // Act
        let result = task.run(context).await.unwrap();

        // Assert
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response, None);
    }
}
