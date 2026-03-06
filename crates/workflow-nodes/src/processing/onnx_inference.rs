//! ONNX Inference Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `onnx-inference` node type. Actual execution is delegated to
//! `CoreTaskExecutor`, so `run()` always returns an error directing
//! callers to that path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_PROMPT: &str = "prompt";
const PORT_VOICE: &str = "voice";
const PORT_LANGUAGE: &str = "language";
const PORT_SPEED: &str = "speed";
const PORT_SEED: &str = "seed";
const PORT_INFERENCE_SETTINGS: &str = "inference_settings";
const PORT_ENVIRONMENT_REF: &str = "environment_ref";
const PORT_AUDIO: &str = "audio";
const PORT_DURATION_SECONDS: &str = "duration_seconds";
const PORT_SAMPLE_RATE: &str = "sample_rate";
const PORT_MODEL_REF: &str = "model_ref";
const PORT_STREAM: &str = "stream";

/// Stub descriptor for the ONNX inference node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections, but all inference work is
/// performed by `CoreTaskExecutor`.
#[derive(Clone)]
pub struct OnnxInferenceTask {
    task_id: String,
}

impl OnnxInferenceTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for OnnxInferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "onnx-inference".to_string(),
            category: NodeCategory::Processing,
            label: "ONNX Runtime Inference (Python Sidecar)".to_string(),
            description: "Run ONNX model inference (including text-to-audio pipelines)".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::required(PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(PORT_VOICE, "Voice", PortDataType::String),
                PortMetadata::optional(PORT_LANGUAGE, "Language", PortDataType::String),
                PortMetadata::optional(PORT_SPEED, "Speed", PortDataType::Number),
                PortMetadata::optional(PORT_SEED, "Seed", PortDataType::Number),
                PortMetadata::optional(
                    PORT_INFERENCE_SETTINGS,
                    "Inference Settings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_ENVIRONMENT_REF, "Environment Ref", PortDataType::Json),
            ],
            outputs: vec![
                PortMetadata::optional(PORT_AUDIO, "Audio", PortDataType::Audio),
                PortMetadata::optional(PORT_DURATION_SECONDS, "Duration", PortDataType::Number),
                PortMetadata::optional(PORT_SAMPLE_RATE, "Sample Rate", PortDataType::Number),
                PortMetadata::optional(PORT_MODEL_REF, "Model Reference", PortDataType::Json),
                PortMetadata::optional(PORT_STREAM, "Audio Stream", PortDataType::AudioStream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(OnnxInferenceTask::descriptor));

#[async_trait]
impl Task for OnnxInferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "onnx-inference requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = OnnxInferenceTask::descriptor();
        assert_eq!(meta.node_type, "onnx-inference");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = OnnxInferenceTask::descriptor();

        // 8 inputs: model_path, prompt, voice, language, speed, seed,
        // inference_settings, environment_ref
        assert_eq!(meta.inputs.len(), 8);
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "voice"));
        assert!(meta.inputs.iter().any(|p| p.id == "language"));
        assert!(meta.inputs.iter().any(|p| p.id == "speed"));
        assert!(meta.inputs.iter().any(|p| p.id == "seed"));
        assert!(meta.inputs.iter().any(|p| p.id == "inference_settings"));
        assert!(meta.inputs.iter().any(|p| p.id == "environment_ref"));

        // 5 outputs: audio, duration_seconds, sample_rate, model_ref, stream
        assert_eq!(meta.outputs.len(), 5);
        assert!(meta.outputs.iter().any(|p| p.id == "audio"));
        assert!(meta.outputs.iter().any(|p| p.id == "duration_seconds"));
        assert!(meta.outputs.iter().any(|p| p.id == "sample_rate"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_ref"));
        assert!(meta.outputs.iter().any(|p| p.id == "stream"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = OnnxInferenceTask::new("test-onnx");
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
