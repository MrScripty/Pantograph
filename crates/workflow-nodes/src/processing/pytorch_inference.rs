//! PyTorch Inference Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `pytorch-inference` node type. Actual execution is delegated to
//! `CoreTaskExecutor`, so `run()` always returns an error directing
//! callers to that path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_PROMPT: &str = "prompt";
const PORT_AUDIO: &str = "audio";
const PORT_SYSTEM_PROMPT: &str = "system_prompt";
const PORT_TEMPERATURE: &str = "temperature";
const PORT_MAX_TOKENS: &str = "max_tokens";
const PORT_DEVICE: &str = "device";
const PORT_MODEL_TYPE: &str = "model_type";
const PORT_ENVIRONMENT_REF: &str = "environment_ref";
const PORT_RESPONSE: &str = "response";
const PORT_MODEL_REF: &str = "model_ref";
const PORT_STREAM: &str = "stream";

/// Stub descriptor for the PyTorch inference node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections, but all inference work is
/// performed by `CoreTaskExecutor`.
#[derive(Clone)]
pub struct PyTorchInferenceTask {
    task_id: String,
}

impl PyTorchInferenceTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for PyTorchInferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "pytorch-inference".to_string(),
            category: NodeCategory::Processing,
            label: "PyTorch Inference".to_string(),
            description:
                "Run inference via PyTorch (text generation, ASR, and HF-backed pipelines)"
                    .to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(PORT_AUDIO, "Audio", PortDataType::Audio),
                PortMetadata::optional(PORT_SYSTEM_PROMPT, "System Prompt", PortDataType::String),
                PortMetadata::optional(PORT_TEMPERATURE, "Temperature", PortDataType::Number),
                PortMetadata::optional(PORT_MAX_TOKENS, "Max Tokens", PortDataType::Number),
                PortMetadata::optional(PORT_DEVICE, "Device", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_TYPE, "Model Type", PortDataType::String),
                PortMetadata::optional(
                    "inference_settings",
                    "Inference Settings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_ENVIRONMENT_REF, "Environment Ref", PortDataType::Json),
            ],
            outputs: vec![
                PortMetadata::required(PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_REF, "Model Reference", PortDataType::Json),
                PortMetadata::optional(PORT_STREAM, "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(PyTorchInferenceTask::descriptor));

#[async_trait]
impl Task for PyTorchInferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "pytorch-inference requires execution via CoreTaskExecutor".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = PyTorchInferenceTask::descriptor();
        assert_eq!(meta.node_type, "pytorch-inference");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = PyTorchInferenceTask::descriptor();

        // 10 inputs: model_path, prompt, audio, system_prompt, temperature, max_tokens, device,
        // model_type, inference_settings, environment_ref
        assert_eq!(meta.inputs.len(), 10);
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "audio"));
        assert!(meta.inputs.iter().any(|p| p.id == "system_prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "temperature"));
        assert!(meta.inputs.iter().any(|p| p.id == "max_tokens"));
        assert!(meta.inputs.iter().any(|p| p.id == "device"));
        assert!(meta.inputs.iter().any(|p| p.id == "model_type"));
        assert!(meta.inputs.iter().any(|p| p.id == "inference_settings"));
        assert!(meta.inputs.iter().any(|p| p.id == "environment_ref"));

        // 3 outputs: response, model_ref, stream
        assert_eq!(meta.outputs.len(), 3);
        assert!(meta.outputs.iter().any(|p| p.id == "response"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_ref"));
        assert!(meta.outputs.iter().any(|p| p.id == "stream"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = PyTorchInferenceTask::new("test-pytorch");
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
