//! Model Provider Task
//!
//! This task provides model information to inference nodes.
//! It can be configured with a model name which is then passed
//! to downstream inference nodes (like Ollama Inference).
//!
//! Future enhancement: integrate with pumas-core to list and search
//! available models from the model library.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Model information output by the Model Provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name/identifier (e.g., "llama2", "codellama:7b")
    pub name: String,
    /// Optional model path (for local models)
    pub path: Option<String>,
    /// Model type (e.g., "llm", "embedding")
    pub model_type: Option<String>,
}

/// Model Provider Task
///
/// Provides model selection for inference nodes. The model name can be
/// set via the input port or configured in the node's UI.
///
/// # Inputs (from context)
/// - `{task_id}.input.model_name` - The model name/identifier
/// - `{task_id}.input.search_query` (optional) - Search query for finding models
///
/// # Outputs (to context)
/// - `{task_id}.output.model_name` - The selected model name
/// - `{task_id}.output.model_path` - The model's path (if known)
/// - `{task_id}.output.model_info` - Full model info as JSON
#[derive(Clone)]
pub struct ModelProviderTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ModelProviderTask {
    /// Port ID for model name input
    pub const PORT_MODEL_NAME: &'static str = "model_name";
    /// Port ID for search query input
    pub const PORT_SEARCH_QUERY: &'static str = "search_query";
    /// Port ID for model name output
    pub const PORT_MODEL_NAME_OUT: &'static str = "model_name";
    /// Port ID for model path output
    pub const PORT_MODEL_PATH: &'static str = "model_path";
    /// Port ID for full model info output
    pub const PORT_MODEL_INFO: &'static str = "model_info";

    /// Create a new model provider task
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

impl TaskDescriptor for ModelProviderTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "model-provider".to_string(),
            category: NodeCategory::Input,
            label: "Model Provider".to_string(),
            description: "Provides model selection for inference nodes".to_string(),
            inputs: vec![
                PortMetadata::optional(
                    Self::PORT_MODEL_NAME,
                    "Model Name",
                    PortDataType::String,
                ),
                PortMetadata::optional(
                    Self::PORT_SEARCH_QUERY,
                    "Search Query",
                    PortDataType::String,
                ),
            ],
            outputs: vec![
                PortMetadata::required(
                    Self::PORT_MODEL_NAME_OUT,
                    "Model Name",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(Self::PORT_MODEL_INFO, "Model Info", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Task for ModelProviderTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get model name from context (can be set via UI or input connection)
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_MODEL_NAME);
        let model_name: String = context.get(&input_key).await.unwrap_or_else(|| {
            // Default to a common Ollama model
            "llama2".to_string()
        });

        // For now, just pass through the model name
        // TODO: In future, use pumas-core to:
        // 1. Validate model exists
        // 2. Get model path and metadata
        // 3. Search models if search_query is provided

        let model_info = ModelInfo {
            name: model_name.clone(),
            path: None,
            model_type: Some("llm".to_string()),
        };

        // Store outputs in context
        let name_out_key = ContextKeys::output(&self.task_id, Self::PORT_MODEL_NAME_OUT);
        context.set(&name_out_key, model_name.clone()).await;

        let info_key = ContextKeys::output(&self.task_id, Self::PORT_MODEL_INFO);
        context.set(&info_key, serde_json::to_value(&model_info).unwrap()).await;

        log::debug!(
            "ModelProviderTask {}: providing model '{}'",
            self.task_id,
            model_name
        );

        Ok(TaskResult::new(Some(model_name), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ModelProviderTask::new("model_provider");
        assert_eq!(task.id(), "model_provider");
    }

    #[tokio::test]
    async fn test_default_model() {
        let task = ModelProviderTask::new("test");
        let context = Context::new();

        // Run without setting input - should default to llama2
        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response.as_deref(), Some("llama2"));

        // Verify output was stored
        let output_key = ContextKeys::output("test", "model_name");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("llama2".to_string()));
    }

    #[tokio::test]
    async fn test_custom_model() {
        let task = ModelProviderTask::new("test");
        let context = Context::new();

        // Set custom model name
        let input_key = ContextKeys::input("test", "model_name");
        context.set(&input_key, "codellama:7b".to_string()).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response.as_deref(), Some("codellama:7b"));
    }

    #[test]
    fn test_descriptor() {
        let meta = ModelProviderTask::descriptor();

        assert_eq!(meta.node_type, "model-provider");
        assert!(meta.outputs.iter().any(|p| p.id == "model_name"));
    }
}
