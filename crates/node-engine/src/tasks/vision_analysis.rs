//! Vision Analysis Task
//!
//! Analyzes images using a vision-capable LLM (e.g., GPT-4V, LLaVA).
//! Sends an image along with a prompt to get a text analysis.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use serde::{Deserialize, Serialize};

use super::ContextKeys;

/// Configuration for the vision analysis task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionConfig {
    /// Base URL of the LLM server
    pub base_url: String,
    /// Model name (for OpenAI-compatible APIs)
    pub model: String,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
}

impl Default for VisionConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            model: "gpt-4-vision-preview".to_string(),
            max_tokens: Some(4096),
        }
    }
}

/// Vision Analysis Task
///
/// Analyzes an image using a vision model and returns a text description.
///
/// # Inputs (from context)
/// - `{task_id}.input.image` (required) - Base64 encoded image data
/// - `{task_id}.input.prompt` (required) - Prompt describing what to analyze
///
/// # Outputs (to context)
/// - `{task_id}.output.analysis` - The vision model's analysis
///
/// # Configuration
/// - `{task_id}.meta.config` - VisionConfig with base_url and model
#[derive(Clone)]
pub struct VisionAnalysisTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration (optional, can also be set via context)
    config: Option<VisionConfig>,
}

impl VisionAnalysisTask {
    /// Create a new vision analysis task with the given ID
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: VisionConfig) -> Self {
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

#[async_trait]
impl Task for VisionAnalysisTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: image (base64)
        let image_key = ContextKeys::input(&self.task_id, "image");
        let image_base64: String = context.get(&image_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'image' at key '{}'",
                image_key
            ))
        })?;

        // Get required input: prompt
        let prompt_key = ContextKeys::input(&self.task_id, "prompt");
        let prompt: String = context.get(&prompt_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'prompt' at key '{}'",
                prompt_key
            ))
        })?;

        // Get configuration from context or use instance config
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<VisionConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        log::debug!(
            "VisionAnalysisTask {}: analyzing image with prompt '{}'",
            self.task_id,
            prompt.chars().take(50).collect::<String>()
        );

        // Build vision request with image
        let client = reqwest::Client::new();
        let url = format!("{}/v1/chat/completions", config.base_url);

        let mut request_body = serde_json::json!({
            "model": config.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", image_base64)
                        }
                    }
                ]
            }]
        });

        if let Some(max_tokens) = config.max_tokens {
            request_body["max_tokens"] = serde_json::json!(max_tokens);
        }

        let http_response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                GraphError::TaskExecutionFailed(format!("Vision request failed: {}", e))
            })?;

        if !http_response.status().is_success() {
            let status = http_response.status();
            let error_body = http_response.text().await.unwrap_or_default();
            return Err(GraphError::TaskExecutionFailed(format!(
                "Vision API error ({}): {}",
                status, error_body
            )));
        }

        let json: serde_json::Value = http_response.json().await.map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to parse response: {}", e))
        })?;

        let analysis = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Store output in context
        let output_key = ContextKeys::output(&self.task_id, "analysis");
        context.set(&output_key, analysis.clone()).await;

        log::debug!(
            "VisionAnalysisTask {}: completed with {} chars analysis",
            self.task_id,
            analysis.len()
        );

        Ok(TaskResult::new(Some(analysis), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = VisionAnalysisTask::new("my_vision");
        assert_eq!(task.id(), "my_vision");
    }

    #[test]
    fn test_with_config() {
        let config = VisionConfig {
            base_url: "http://localhost:1234".to_string(),
            model: "llava".to_string(),
            max_tokens: Some(1000),
        };
        let task = VisionAnalysisTask::with_config("task1", config);
        assert_eq!(
            task.config.as_ref().unwrap().base_url,
            "http://localhost:1234"
        );
        assert_eq!(task.config.as_ref().unwrap().model, "llava");
    }

    #[test]
    fn test_default_config() {
        let config = VisionConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.model, "gpt-4-vision-preview");
        assert_eq!(config.max_tokens, Some(4096));
    }

    #[tokio::test]
    async fn test_missing_image_error() {
        let task = VisionAnalysisTask::new("test_vision");
        let context = Context::new();

        // Set prompt but not image
        let prompt_key = ContextKeys::input("test_vision", "prompt");
        context
            .set(&prompt_key, "Describe this image".to_string())
            .await;

        // Should error due to missing image
        let result = task.run(context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_prompt_error() {
        let task = VisionAnalysisTask::new("test_vision");
        let context = Context::new();

        // Set image but not prompt
        let image_key = ContextKeys::input("test_vision", "image");
        context.set(&image_key, "base64data".to_string()).await;

        // Should error due to missing prompt
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
