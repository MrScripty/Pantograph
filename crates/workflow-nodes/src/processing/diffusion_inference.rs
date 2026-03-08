//! Diffusion Inference Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `diffusion-inference` node type for image generation workflows.
//! Actual execution is delegated to `CoreTaskExecutor`, so `run()`
//! always returns an error directing callers to that path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_PROMPT: &str = "prompt";
const PORT_NEGATIVE_PROMPT: &str = "negative_prompt";
const PORT_INFERENCE_SETTINGS: &str = "inference_settings";
const PORT_STEPS: &str = "steps";
const PORT_CFG_SCALE: &str = "cfg_scale";
const PORT_SEED: &str = "seed";
const PORT_WIDTH: &str = "width";
const PORT_HEIGHT: &str = "height";
const PORT_ENVIRONMENT_REF: &str = "environment_ref";
const PORT_IMAGE: &str = "image";
const PORT_SEED_USED: &str = "seed_used";
const PORT_STREAM: &str = "stream";

/// Stub descriptor for the diffusion inference node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections, but all inference work is
/// performed by `CoreTaskExecutor` once the Python worker is implemented.
#[derive(Clone)]
pub struct DiffusionInferenceTask {
    task_id: String,
}

impl DiffusionInferenceTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for DiffusionInferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "diffusion-inference".to_string(),
            category: NodeCategory::Processing,
            label: "Diffusion Inference".to_string(),
            description: "Generate images via diffusion models (Stable Diffusion, SDXL, Flux)"
                .to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::required(PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(
                    PORT_NEGATIVE_PROMPT,
                    "Negative Prompt",
                    PortDataType::String,
                ),
                PortMetadata::optional(
                    PORT_INFERENCE_SETTINGS,
                    "Inference Settings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_STEPS, "Steps", PortDataType::Number),
                PortMetadata::optional(PORT_CFG_SCALE, "CFG Scale", PortDataType::Number),
                PortMetadata::optional(PORT_SEED, "Seed", PortDataType::Number),
                PortMetadata::optional(PORT_WIDTH, "Width", PortDataType::Number),
                PortMetadata::optional(PORT_HEIGHT, "Height", PortDataType::Number),
                PortMetadata::optional(PORT_ENVIRONMENT_REF, "Environment Ref", PortDataType::Json),
            ],
            outputs: vec![
                PortMetadata::required(PORT_IMAGE, "Image", PortDataType::Image),
                PortMetadata::optional(PORT_SEED_USED, "Seed Used", PortDataType::Number),
                PortMetadata::optional(PORT_STREAM, "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(
    DiffusionInferenceTask::descriptor
));

#[async_trait]
impl Task for DiffusionInferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "diffusion-inference requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = DiffusionInferenceTask::descriptor();
        assert_eq!(meta.node_type, "diffusion-inference");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = DiffusionInferenceTask::descriptor();

        // 10 inputs: model_path, prompt, negative_prompt, inference_settings,
        //            steps, cfg_scale, seed, width, height, environment_ref
        assert_eq!(meta.inputs.len(), 10);
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "negative_prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "inference_settings"));
        assert!(meta.inputs.iter().any(|p| p.id == "steps"));
        assert!(meta.inputs.iter().any(|p| p.id == "cfg_scale"));
        assert!(meta.inputs.iter().any(|p| p.id == "seed"));
        assert!(meta.inputs.iter().any(|p| p.id == "width"));
        assert!(meta.inputs.iter().any(|p| p.id == "height"));
        assert!(meta.inputs.iter().any(|p| p.id == "environment_ref"));

        // 3 outputs: image, seed_used, stream
        assert_eq!(meta.outputs.len(), 3);
        assert!(meta.outputs.iter().any(|p| p.id == "image"));
        assert!(meta.outputs.iter().any(|p| p.id == "seed_used"));
        assert!(meta.outputs.iter().any(|p| p.id == "stream"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = DiffusionInferenceTask::new("test-diffusion");
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
