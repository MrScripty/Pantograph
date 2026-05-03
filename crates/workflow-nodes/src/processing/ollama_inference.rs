//! Retired Ollama Inference Task
//!
//! Pantograph no longer registers `ollama-inference` as a graph-visible node.
//! The descriptor remains in this module only as a migration reference for old
//! saved workflows while canonical inference nodes replace backend-specific
//! workflow shapes.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const OLLAMA_RETIRED_MESSAGE: &str = "Ollama is no longer supported as a first-party Pantograph inference backend. Migrate this saved workflow node to the canonical inference node with a Pumas model reference.";

/// Retired Ollama inference task.
///
/// Preserves the old node shape as a migration reference without preserving
/// Ollama execution support.
///
/// # Inputs (from context)
/// - `{task_id}.input.prompt` - The prompt to send (required)
/// - `{task_id}.input.model` - The model name to use (required)
/// - `{task_id}.input.system_prompt` (optional) - System prompt
/// - `{task_id}.input.temperature` (optional) - Sampling temperature
/// - `{task_id}.input.max_tokens` (optional) - Maximum tokens to generate
///
/// # Outputs (to context)
/// - `{task_id}.output.response` - The model's response text
/// - `{task_id}.output.model` - The model that was used
#[derive(Clone)]
pub struct OllamaInferenceTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl OllamaInferenceTask {
    /// Port ID for prompt input
    pub const PORT_PROMPT: &'static str = "prompt";
    /// Port ID for model input
    pub const PORT_MODEL: &'static str = "model";
    /// Port ID for system prompt input
    pub const PORT_SYSTEM_PROMPT: &'static str = "system_prompt";
    /// Port ID for temperature input
    pub const PORT_TEMPERATURE: &'static str = "temperature";
    /// Port ID for max tokens input
    pub const PORT_MAX_TOKENS: &'static str = "max_tokens";
    /// Port ID for response output
    pub const PORT_RESPONSE: &'static str = "response";
    /// Port ID for model output (echo back which model was used)
    pub const PORT_MODEL_OUT: &'static str = "model_used";
    /// Port ID for stream output
    pub const PORT_STREAM: &'static str = "stream";
    /// Port ID for model reference output (engine + model_id for unload node)
    pub const PORT_MODEL_REF: &'static str = "model_ref";

    /// Create a new Ollama inference task with the given ID
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

impl TaskDescriptor for OllamaInferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "ollama-inference".to_string(),
            category: NodeCategory::Processing,
            label: "Retired Ollama Inference".to_string(),
            description:
                "Retired legacy node shape; migrate to canonical inference with a Pumas model ref"
                    .to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::required(Self::PORT_MODEL, "Model", PortDataType::String),
                PortMetadata::optional(
                    Self::PORT_SYSTEM_PROMPT,
                    "System Prompt",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_TEMPERATURE, "Temperature", PortDataType::Number),
                PortMetadata::optional(Self::PORT_MAX_TOKENS, "Max Tokens", PortDataType::Number),
                PortMetadata::optional(
                    "inference_settings",
                    "Inference Settings",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::required(Self::PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(Self::PORT_MODEL_OUT, "Model Used", PortDataType::String),
                PortMetadata::optional(Self::PORT_MODEL_REF, "Model Reference", PortDataType::Json),
                PortMetadata::optional(Self::PORT_STREAM, "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

#[async_trait]
impl Task for OllamaInferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            OLLAMA_RETIRED_MESSAGE.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = OllamaInferenceTask::new("ollama_task");
        assert_eq!(task.id(), "ollama_task");
    }

    #[test]
    fn test_descriptor_has_model_port() {
        let meta = OllamaInferenceTask::descriptor();

        // Check for model input (required)
        let model_input = meta.inputs.iter().find(|p| p.id == "model");
        assert!(model_input.is_some());
        assert!(model_input.unwrap().required);

        // Check for response output
        assert!(meta.outputs.iter().any(|p| p.id == "response"));
    }

    #[tokio::test]
    async fn run_returns_retired_backend_error_without_http_execution() {
        let task = OllamaInferenceTask::new("retired-ollama");
        let error = task
            .run(Context::new())
            .await
            .expect_err("retired task must not execute");

        assert!(error.to_string().contains("Ollama is no longer supported"));
    }
}
