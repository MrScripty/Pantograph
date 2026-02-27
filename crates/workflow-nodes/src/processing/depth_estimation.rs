//! Depth Estimation Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `depth-estimation` node type. Actual execution is delegated to
//! `CoreTaskExecutor` via PyO3/DepthPro, so `run()` always returns
//! an error directing callers to that path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_IMAGE: &str = "image";
const PORT_MODEL_PATH: &str = "model_path";
const PORT_DEVICE: &str = "device";
const PORT_DEPTH_MAP: &str = "depth_map";
const PORT_POINT_CLOUD: &str = "point_cloud";
const PORT_FOCAL_LENGTH: &str = "focal_length";

/// Stub descriptor for the depth estimation node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections. Depth estimation is performed
/// by `CoreTaskExecutor` via the DepthPro Python worker.
#[derive(Clone)]
pub struct DepthEstimationTask {
    task_id: String,
}

impl DepthEstimationTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for DepthEstimationTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "depth-estimation".to_string(),
            category: NodeCategory::Processing,
            label: "Depth Estimation".to_string(),
            description: "Estimate depth from images using Apple DepthPro".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_IMAGE, "Image", PortDataType::Image),
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(PORT_DEVICE, "Device", PortDataType::String),
            ],
            outputs: vec![
                PortMetadata::required(PORT_DEPTH_MAP, "Depth Map", PortDataType::Image),
                PortMetadata::optional(PORT_POINT_CLOUD, "Point Cloud", PortDataType::Json),
                PortMetadata::optional(PORT_FOCAL_LENGTH, "Focal Length", PortDataType::Number),
            ],
            execution_mode: ExecutionMode::Batch,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(DepthEstimationTask::descriptor));

#[async_trait]
impl Task for DepthEstimationTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "depth-estimation requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = DepthEstimationTask::descriptor();
        assert_eq!(meta.node_type, "depth-estimation");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = DepthEstimationTask::descriptor();

        // 3 inputs: image, model_path, device
        assert_eq!(meta.inputs.len(), 3);
        assert!(meta.inputs.iter().any(|p| p.id == "image"));
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "device"));

        // 3 outputs: depth_map, point_cloud, focal_length
        assert_eq!(meta.outputs.len(), 3);
        assert!(meta.outputs.iter().any(|p| p.id == "depth_map"));
        assert!(meta.outputs.iter().any(|p| p.id == "point_cloud"));
        assert!(meta.outputs.iter().any(|p| p.id == "focal_length"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = DepthEstimationTask::new("test-depth");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("CoreTaskExecutor"),
            "error should mention CoreTaskExecutor, got: {err}"
        );
    }
}
