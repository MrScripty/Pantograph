//! Ollama Inference Task
//!
//! This task sends a prompt to an Ollama server and returns the response.
//! The model is specified via an input port (typically from a Model Provider node).

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Response structure from Ollama API
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: String,
    response: String,
    done: bool,
    #[serde(default)]
    context: Vec<i64>,
}

/// Ollama Inference Task
///
/// Sends a prompt to an Ollama server using the specified model.
/// The model name is received as an input, allowing dynamic model selection
/// from a Model Provider node.
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
    /// Base URL of the Ollama server (default: http://localhost:11434)
    base_url: String,
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

    /// Create a new Ollama inference task with the given ID
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            base_url: "http://localhost:11434".to_string(),
        }
    }

    /// Create with a custom base URL
    pub fn with_base_url(task_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            base_url: base_url.into(),
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
            label: "Ollama Inference".to_string(),
            description: "Runs inference using a local Ollama server".to_string(),
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
            ],
            outputs: vec![
                PortMetadata::required(Self::PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(Self::PORT_MODEL_OUT, "Model Used", PortDataType::String),
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

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required inputs
        let prompt_key = ContextKeys::input(&self.task_id, Self::PORT_PROMPT);
        let prompt: String = context.get(&prompt_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'prompt' at key '{}'",
                prompt_key
            ))
        })?;

        let model_key = ContextKeys::input(&self.task_id, Self::PORT_MODEL);
        let model: String = context.get(&model_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'model' at key '{}'. Connect a Model Provider node.",
                model_key
            ))
        })?;

        // Get optional inputs
        let system_prompt_key = ContextKeys::input(&self.task_id, Self::PORT_SYSTEM_PROMPT);
        let system_prompt: Option<String> = context.get(&system_prompt_key).await;

        let temp_key = ContextKeys::input(&self.task_id, Self::PORT_TEMPERATURE);
        let temperature: Option<f64> = context.get(&temp_key).await;

        let max_tokens_key = ContextKeys::input(&self.task_id, Self::PORT_MAX_TOKENS);
        let max_tokens: Option<i64> = context.get(&max_tokens_key).await;

        // Build Ollama API request
        let mut request_body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        });

        if let Some(sys) = &system_prompt {
            request_body["system"] = serde_json::json!(sys);
        }

        // Add options if provided
        let mut options = serde_json::Map::new();
        if let Some(temp) = temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(max) = max_tokens {
            options.insert("num_predict".to_string(), serde_json::json!(max));
        }
        if !options.is_empty() {
            request_body["options"] = serde_json::Value::Object(options);
        }

        // Make the HTTP request to Ollama
        let client = reqwest::Client::new();
        let url = format!("{}/api/generate", self.base_url);

        log::debug!(
            "OllamaInferenceTask {}: sending request to {} with model '{}'",
            self.task_id,
            url,
            model
        );

        let http_response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                GraphError::TaskExecutionFailed(format!(
                    "Failed to connect to Ollama server at {}: {}. Is Ollama running?",
                    self.base_url, e
                ))
            })?;

        if !http_response.status().is_success() {
            let status = http_response.status();
            let error_body = http_response.text().await.unwrap_or_default();
            return Err(GraphError::TaskExecutionFailed(format!(
                "Ollama API error ({}): {}",
                status, error_body
            )));
        }

        let response_data: OllamaResponse = http_response.json().await.map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to parse Ollama response: {}", e))
        })?;

        // Store outputs in context
        let output_key = ContextKeys::output(&self.task_id, Self::PORT_RESPONSE);
        context.set(&output_key, response_data.response.clone()).await;

        let model_out_key = ContextKeys::output(&self.task_id, Self::PORT_MODEL_OUT);
        context.set(&model_out_key, response_data.model.clone()).await;

        log::debug!(
            "OllamaInferenceTask {}: completed with {} chars response using model '{}'",
            self.task_id,
            response_data.response.len(),
            response_data.model
        );

        Ok(TaskResult::new(Some(response_data.response), NextAction::Continue))
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
    fn test_default_base_url() {
        let task = OllamaInferenceTask::new("test");
        assert_eq!(task.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_custom_base_url() {
        let task = OllamaInferenceTask::with_base_url("test", "http://custom:8080");
        assert_eq!(task.base_url, "http://custom:8080");
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
}
