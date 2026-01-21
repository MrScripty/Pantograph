//! LLM Inference Task
//!
//! This task sends a prompt to an LLM and returns the response.
//! It demonstrates how to implement a graph-flow Task for Pantograph.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Configuration for the inference task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Base URL of the LLM server
    pub base_url: String,
    /// Model name (for OpenAI-compatible APIs)
    pub model: String,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature for sampling
    pub temperature: Option<f32>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            model: "gpt-4".to_string(),
            max_tokens: None,
            temperature: None,
        }
    }
}

/// LLM Inference Task
///
/// Sends a prompt to an LLM and stores the response in context.
///
/// # Inputs (from context)
/// - `{task_id}.input.prompt` - The prompt to send
/// - `{task_id}.input.system_prompt` (optional) - System prompt
/// - `{task_id}.input.context` (optional) - Additional context to append to prompt
///
/// # Outputs (to context)
/// - `{task_id}.output.response` - The LLM's response
///
/// # Configuration
/// - `config.base_url` - LLM server URL
/// - `config.model` - Model name
#[derive(Clone)]
pub struct InferenceTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration (optional, can also be set via context)
    config: Option<InferenceConfig>,
}

impl InferenceTask {
    /// Port ID for prompt input
    pub const PORT_PROMPT: &'static str = "prompt";
    /// Port ID for system prompt input
    pub const PORT_SYSTEM_PROMPT: &'static str = "system_prompt";
    /// Port ID for context input (additional context to append)
    pub const PORT_CONTEXT: &'static str = "context";
    /// Port ID for response output
    pub const PORT_RESPONSE: &'static str = "response";
    /// Port ID for stream output
    pub const PORT_STREAM: &'static str = "stream";

    /// Create a new inference task with the given ID
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: InferenceConfig) -> Self {
        Self {
            task_id: task_id.into(),
            config: Some(config),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

impl TaskDescriptor for InferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "llm-inference".to_string(),
            category: NodeCategory::Processing,
            label: "LLM Inference".to_string(),
            description: "Runs text through a language model".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(
                    Self::PORT_SYSTEM_PROMPT,
                    "System Prompt",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_CONTEXT, "Context", PortDataType::String),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(Self::PORT_STREAM, "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

#[async_trait]
impl Task for InferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: prompt
        let prompt_key = ContextKeys::input(&self.task_id, Self::PORT_PROMPT);
        let prompt: String = context.get(&prompt_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'prompt' at key '{}'",
                prompt_key
            ))
        })?;

        // Get optional inputs
        let system_prompt_key = ContextKeys::input(&self.task_id, Self::PORT_SYSTEM_PROMPT);
        let system_prompt: Option<String> = context.get(&system_prompt_key).await;

        let context_key = ContextKeys::input(&self.task_id, Self::PORT_CONTEXT);
        let extra_context: Option<String> = context.get(&context_key).await;

        // Get configuration from context or use instance config
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<InferenceConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        // Build the full prompt with context if provided
        let full_prompt = if let Some(ctx) = extra_context {
            format!("{}\n\nContext:\n{}", prompt, ctx)
        } else {
            prompt
        };

        // Build messages for OpenAI-compatible API
        let mut messages = Vec::new();
        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": full_prompt
        }));

        // Build request body
        let mut request_body = serde_json::json!({
            "model": config.model,
            "messages": messages,
            "stream": false
        });

        if let Some(max_tokens) = config.max_tokens {
            request_body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(temp) = config.temperature {
            request_body["temperature"] = serde_json::json!(temp);
        }

        // Make the HTTP request
        let client = reqwest::Client::new();
        let url = format!("{}/v1/chat/completions", config.base_url);

        log::debug!("InferenceTask {}: sending request to {}", self.task_id, url);

        let http_response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| GraphError::TaskExecutionFailed(format!("HTTP request failed: {}", e)))?;

        if !http_response.status().is_success() {
            let status = http_response.status();
            let error_body = http_response.text().await.unwrap_or_default();
            return Err(GraphError::TaskExecutionFailed(format!(
                "LLM API error ({}): {}",
                status, error_body
            )));
        }

        let json: serde_json::Value = http_response
            .json()
            .await
            .map_err(|e| GraphError::TaskExecutionFailed(format!("Failed to parse response: {}", e)))?;

        let response = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Store output in context
        let output_key = ContextKeys::output(&self.task_id, Self::PORT_RESPONSE);
        context.set(&output_key, response.clone()).await;

        log::debug!(
            "InferenceTask {}: completed with {} chars response",
            self.task_id,
            response.len()
        );

        Ok(TaskResult::new(Some(response), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = InferenceTask::new("my_inference");
        assert_eq!(task.id(), "my_inference");
    }

    #[test]
    fn test_with_config() {
        let config = InferenceConfig {
            base_url: "http://localhost:1234".to_string(),
            model: "llama".to_string(),
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let task = InferenceTask::with_config("task1", config);
        assert_eq!(
            task.config.as_ref().unwrap().base_url,
            "http://localhost:1234"
        );
    }

    #[test]
    fn test_default_config() {
        let config = InferenceConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.model, "gpt-4");
    }
}
