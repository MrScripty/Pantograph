//! Model Provider Task
//!
//! This task provides model information to inference nodes.
//! It can be configured with a model name which is then passed
//! to downstream inference nodes (like Ollama Inference).

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
    /// Pumas model UUID (if resolved from library)
    pub id: Option<String>,
    /// Canonical name from pumas-core
    pub official_name: Option<String>,
    /// Inference parameter schema from model metadata
    #[serde(default)]
    pub inference_settings: Option<Vec<serde_json::Value>>,
}

// Port constants
const PORT_MODEL_NAME: &str = "model_name";
const PORT_SEARCH_QUERY: &str = "search_query";
const PORT_MODEL_NAME_OUT: &str = "model_name";
const PORT_MODEL_PATH: &str = "model_path";
const PORT_MODEL_INFO: &str = "model_info";
const PORT_SEARCH_RESULTS: &str = "search_results";
const PORT_INFERENCE_SETTINGS: &str = "inference_settings";

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
/// - `{task_id}.output.search_results` - Search results as JSON (when search_query provided)
#[derive(Clone)]
pub struct ModelProviderTask {
    task_id: String,
}

impl ModelProviderTask {
    pub const PORT_MODEL_NAME: &'static str = PORT_MODEL_NAME;
    pub const PORT_SEARCH_QUERY: &'static str = PORT_SEARCH_QUERY;
    pub const PORT_MODEL_NAME_OUT: &'static str = PORT_MODEL_NAME_OUT;
    pub const PORT_MODEL_PATH: &'static str = PORT_MODEL_PATH;
    pub const PORT_MODEL_INFO: &'static str = PORT_MODEL_INFO;
    pub const PORT_SEARCH_RESULTS: &'static str = PORT_SEARCH_RESULTS;
    pub const PORT_INFERENCE_SETTINGS: &'static str = PORT_INFERENCE_SETTINGS;

    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }

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
                PortMetadata::optional(PORT_MODEL_NAME, "Model Name", PortDataType::String),
                PortMetadata::optional(PORT_SEARCH_QUERY, "Search Query", PortDataType::String),
            ],
            outputs: vec![
                PortMetadata::required(PORT_MODEL_NAME_OUT, "Model Name", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_INFO, "Model Info", PortDataType::Json),
                PortMetadata::optional(PORT_SEARCH_RESULTS, "Search Results", PortDataType::Json),
                PortMetadata::optional(
                    PORT_INFERENCE_SETTINGS,
                    "Inference Settings",
                    PortDataType::Json,
                ),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ModelProviderTask::descriptor));

#[async_trait]
impl Task for ModelProviderTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input_key = ContextKeys::input(&self.task_id, PORT_MODEL_NAME);
        let model_name: String = context
            .get(&input_key)
            .await
            .unwrap_or_else(|| "llama2".to_string());

        let model_info = ModelInfo {
            name: model_name.clone(),
            path: None,
            model_type: Some("llm".to_string()),
            id: None,
            official_name: None,
            inference_settings: None,
        };

        let name_out_key = ContextKeys::output(&self.task_id, PORT_MODEL_NAME_OUT);
        context.set(&name_out_key, model_name.clone()).await;

        let info_key = ContextKeys::output(&self.task_id, PORT_MODEL_INFO);
        context
            .set(&info_key, serde_json::to_value(&model_info).unwrap())
            .await;

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

        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response.as_deref(), Some("llama2"));

        let output_key = ContextKeys::output("test", "model_name");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("llama2".to_string()));
    }

    #[tokio::test]
    async fn test_custom_model() {
        let task = ModelProviderTask::new("test");
        let context = Context::new();

        let input_key = ContextKeys::input("test", "model_name");
        context.set(&input_key, "codellama:7b".to_string()).await;

        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response.as_deref(), Some("codellama:7b"));
    }

    #[test]
    fn test_descriptor() {
        let meta = ModelProviderTask::descriptor();

        assert_eq!(meta.node_type, "model-provider");
        assert_eq!(meta.outputs.len(), 5);
        assert!(meta.outputs.iter().any(|p| p.id == "model_name"));
        assert!(meta.outputs.iter().any(|p| p.id == "search_results"));
        assert!(meta.outputs.iter().any(|p| p.id == "inference_settings"));
    }

    #[test]
    fn test_model_info_serialization() {
        let info = ModelInfo {
            name: "test-model".to_string(),
            path: Some("/path/to/model".to_string()),
            model_type: Some("llm".to_string()),
            id: Some("uuid-123".to_string()),
            official_name: Some("Test Model".to_string()),
            inference_settings: None,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["name"], "test-model");
        assert_eq!(json["id"], "uuid-123");
        assert_eq!(json["official_name"], "Test Model");

        let deserialized: ModelInfo = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, info.name);
        assert_eq!(deserialized.id, info.id);
    }
}
