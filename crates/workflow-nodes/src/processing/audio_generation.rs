//! Audio Generation Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `audio-generation` node type. Actual execution is delegated to
//! `CoreTaskExecutor` via PyO3/Stable Audio, so `run()` always returns
//! an error directing callers to that path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_PROMPT: &str = "prompt";
const PORT_DURATION: &str = "duration";
const PORT_NUM_INFERENCE_STEPS: &str = "num_inference_steps";
const PORT_GUIDANCE_SCALE: &str = "guidance_scale";
const PORT_SEED: &str = "seed";
const PORT_INFERENCE_SETTINGS: &str = "inference_settings";
const PORT_AUDIO: &str = "audio";
const PORT_DURATION_SECONDS: &str = "duration_seconds";
const PORT_SAMPLE_RATE: &str = "sample_rate";
const PORT_MODEL_REF: &str = "model_ref";

/// Stub descriptor for the audio generation node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections. Audio generation is performed
/// by `CoreTaskExecutor` via the Stable Audio Python worker.
#[derive(Clone)]
pub struct AudioGenerationTask {
    task_id: String,
}

impl AudioGenerationTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for AudioGenerationTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "audio-generation".to_string(),
            category: NodeCategory::Processing,
            label: "Audio Generation".to_string(),
            description: "Generate audio from text prompts via Stable Audio".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::required(PORT_PROMPT, "Prompt", PortDataType::String),
                PortMetadata::optional(PORT_DURATION, "Duration (s)", PortDataType::Number),
                PortMetadata::optional(PORT_NUM_INFERENCE_STEPS, "Steps", PortDataType::Number),
                PortMetadata::optional(PORT_GUIDANCE_SCALE, "Guidance Scale", PortDataType::Number),
                PortMetadata::optional(PORT_SEED, "Seed", PortDataType::Number),
                PortMetadata::optional(
                    PORT_INFERENCE_SETTINGS,
                    "Inference Settings",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::required(PORT_AUDIO, "Audio", PortDataType::Audio),
                PortMetadata::optional(PORT_DURATION_SECONDS, "Duration", PortDataType::Number),
                PortMetadata::optional(PORT_SAMPLE_RATE, "Sample Rate", PortDataType::Number),
                PortMetadata::optional(PORT_MODEL_REF, "Model Reference", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Batch,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(AudioGenerationTask::descriptor));

#[async_trait]
impl Task for AudioGenerationTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "audio-generation requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = AudioGenerationTask::descriptor();
        assert_eq!(meta.node_type, "audio-generation");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = AudioGenerationTask::descriptor();

        // 7 inputs: model_path, prompt, duration, num_inference_steps,
        //           guidance_scale, seed, inference_settings
        assert_eq!(meta.inputs.len(), 7);
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "duration"));
        assert!(meta.inputs.iter().any(|p| p.id == "num_inference_steps"));
        assert!(meta.inputs.iter().any(|p| p.id == "guidance_scale"));
        assert!(meta.inputs.iter().any(|p| p.id == "seed"));
        assert!(meta.inputs.iter().any(|p| p.id == "inference_settings"));

        // 4 outputs: audio, duration_seconds, sample_rate, model_ref
        assert_eq!(meta.outputs.len(), 4);
        assert!(meta.outputs.iter().any(|p| p.id == "audio"));
        assert!(meta.outputs.iter().any(|p| p.id == "duration_seconds"));
        assert!(meta.outputs.iter().any(|p| p.id == "sample_rate"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_ref"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = AudioGenerationTask::new("test-audio-gen");
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
