//! Embedding Task
//!
//! Generates vector embeddings from text using an embedding model.
//! Can use local models (via inference crate) or remote APIs.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Configuration for the embedding task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Base URL of the embedding server
    pub base_url: String,
    /// Model name for embeddings
    pub model: String,
    /// Embedding dimensions (for validation)
    pub dimensions: Option<usize>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            model: "nomic-embed-text".to_string(),
            dimensions: None,
        }
    }
}

/// Embedding Task
///
/// Generates vector embeddings from text input.
///
/// # Inputs (from context)
/// - `{task_id}.input.text` (required) - Text to embed
/// - `{task_id}.input.model` (optional) - Model name override
///
/// # Outputs (to context)
/// - `{task_id}.output.embedding` - The embedding vector (Vec<f32>)
/// - `{task_id}.output.dimensions` - Number of dimensions
#[derive(Clone)]
pub struct EmbeddingTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration
    config: Option<EmbeddingConfig>,
}

impl EmbeddingTask {
    /// Port ID for text input
    pub const PORT_TEXT: &'static str = "text";
    /// Port ID for model input
    pub const PORT_MODEL: &'static str = "model";
    /// Port ID for embedding output
    pub const PORT_EMBEDDING: &'static str = "embedding";
    /// Port ID for dimensions output
    pub const PORT_DIMENSIONS: &'static str = "dimensions";

    /// Create a new embedding task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: EmbeddingConfig) -> Self {
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

impl TaskDescriptor for EmbeddingTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "embedding".to_string(),
            category: NodeCategory::Processing,
            label: "Embedding".to_string(),
            description: "Generates vector embeddings from text".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_TEXT, "Text", PortDataType::String),
                PortMetadata::optional(Self::PORT_MODEL, "Model", PortDataType::String),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_EMBEDDING, "Embedding", PortDataType::Embedding),
                PortMetadata::optional(Self::PORT_DIMENSIONS, "Dimensions", PortDataType::Number),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(EmbeddingTask::descriptor));

#[async_trait]
impl Task for EmbeddingTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: text
        let text_key = ContextKeys::input(&self.task_id, Self::PORT_TEXT);
        let text: String = context.get(&text_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'text' at key '{}'",
                text_key
            ))
        })?;

        // Get configuration
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<EmbeddingConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        // Check for model override
        let model_key = ContextKeys::input(&self.task_id, Self::PORT_MODEL);
        let model = context
            .get::<String>(&model_key)
            .await
            .unwrap_or(config.model.clone());

        log::debug!(
            "EmbeddingTask {}: generating embedding for {} chars of text with model '{}'",
            self.task_id,
            text.len(),
            model
        );

        // Build embedding request (OpenAI-compatible API)
        let client = reqwest::Client::new();
        let url = format!("{}/v1/embeddings", config.base_url);

        let request_body = serde_json::json!({
            "model": model,
            "input": text
        });

        let http_response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                GraphError::TaskExecutionFailed(format!("Embedding request failed: {}", e))
            })?;

        if !http_response.status().is_success() {
            let status = http_response.status();
            let error_body = http_response.text().await.unwrap_or_default();
            return Err(GraphError::TaskExecutionFailed(format!(
                "Embedding API error ({}): {}",
                status, error_body
            )));
        }

        let json: serde_json::Value = http_response.json().await.map_err(|e| {
            GraphError::TaskExecutionFailed(format!("Failed to parse embedding response: {}", e))
        })?;

        // Extract embedding from response
        let embedding: Vec<f64> = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| {
                GraphError::TaskExecutionFailed("Invalid embedding response format".to_string())
            })?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();

        let dimensions = embedding.len();

        // Store outputs in context
        let embedding_key = ContextKeys::output(&self.task_id, Self::PORT_EMBEDDING);
        context.set(&embedding_key, embedding.clone()).await;

        let dimensions_key = ContextKeys::output(&self.task_id, Self::PORT_DIMENSIONS);
        context.set(&dimensions_key, dimensions as f64).await;

        log::debug!(
            "EmbeddingTask {}: generated {}-dimensional embedding",
            self.task_id,
            dimensions
        );

        Ok(TaskResult::new(
            Some(format!("Embedding: {} dimensions", dimensions)),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = EmbeddingTask::new("my_embedding");
        assert_eq!(task.id(), "my_embedding");
    }

    #[test]
    fn test_with_config() {
        let config = EmbeddingConfig {
            base_url: "http://localhost:1234".to_string(),
            model: "custom-embed".to_string(),
            dimensions: Some(384),
        };
        let task = EmbeddingTask::with_config("task1", config);
        assert_eq!(
            task.config.as_ref().unwrap().base_url,
            "http://localhost:1234"
        );
        assert_eq!(task.config.as_ref().unwrap().model, "custom-embed");
    }

    #[test]
    fn test_default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.model, "nomic-embed-text");
    }

    #[test]
    fn test_descriptor() {
        let meta = EmbeddingTask::descriptor();
        assert_eq!(meta.node_type, "embedding");
        assert_eq!(meta.category, NodeCategory::Processing);
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.outputs.len(), 2);
    }

    #[tokio::test]
    async fn test_missing_text_error() {
        let task = EmbeddingTask::new("test_embed");
        let context = Context::new();

        // Run without setting text - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
