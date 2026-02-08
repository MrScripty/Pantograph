//! Model Provider Task
//!
//! This task provides model information to inference nodes.
//! It can be configured with a model name which is then passed
//! to downstream inference nodes (like Ollama Inference).
//!
//! When the `model-library` feature is enabled and a `PumasApi` is
//! available via `ExecutorExtensions`, the provider resolves models
//! against the local pumas-core library and can search HuggingFace.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, NodeExecutor, NodeExecutorFactory, PortDataType,
    PortMetadata, TaskDescriptor, TaskMetadata,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
}

// Port constants
const PORT_MODEL_NAME: &str = "model_name";
const PORT_SEARCH_QUERY: &str = "search_query";
const PORT_MODEL_NAME_OUT: &str = "model_name";
const PORT_MODEL_PATH: &str = "model_path";
const PORT_MODEL_INFO: &str = "model_info";
const PORT_SEARCH_RESULTS: &str = "search_results";

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
                PortMetadata::optional(
                    PORT_SEARCH_RESULTS,
                    "Search Results",
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

// ============================================================================
// NodeExecutor implementation (used via NodeRegistry + ExecutorExtensions)
// ============================================================================

/// NodeExecutor for ModelProvider that uses ExecutorExtensions
/// to access pumas-core when available.
pub struct ModelProviderExecutor;

impl ModelProviderExecutor {
    pub fn factory() -> Arc<dyn NodeExecutorFactory> {
        Arc::new(ModelProviderExecutorFactory)
    }
}

struct ModelProviderExecutorFactory;

impl NodeExecutorFactory for ModelProviderExecutorFactory {
    fn create_executor(&self) -> Arc<dyn NodeExecutor> {
        Arc::new(ModelProviderExecutor)
    }
}

#[async_trait]
impl NodeExecutor for ModelProviderExecutor {
    async fn execute(
        &self,
        _task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        extensions: &node_engine::ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let model_name = inputs
            .get(PORT_MODEL_NAME)
            .and_then(|v| v.as_str())
            .unwrap_or("llama2")
            .to_string();

        let search_query = inputs
            .get(PORT_SEARCH_QUERY)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut outputs = HashMap::new();

        // Try pumas-core resolution when available
        #[cfg(feature = "model-library")]
        {
            if let Some(resolved) =
                library_support::resolve_with_library(extensions, &model_name, search_query.as_deref()).await
            {
                outputs.insert(
                    PORT_MODEL_NAME_OUT.to_string(),
                    serde_json::Value::String(resolved.info.name.clone()),
                );
                if let Some(ref path) = resolved.info.path {
                    outputs.insert(
                        PORT_MODEL_PATH.to_string(),
                        serde_json::Value::String(path.clone()),
                    );
                }
                outputs.insert(
                    PORT_MODEL_INFO.to_string(),
                    serde_json::to_value(&resolved.info).unwrap(),
                );
                if let Some(search_results) = resolved.search_results {
                    outputs.insert(PORT_SEARCH_RESULTS.to_string(), search_results);
                }
                return Ok(outputs);
            }
        }

        // Suppress unused variable warnings when model-library is disabled
        let _ = (extensions, &search_query);

        // Fallback: passthrough mode
        let model_info = ModelInfo {
            name: model_name.clone(),
            path: None,
            model_type: Some("llm".to_string()),
            id: None,
            official_name: None,
        };

        outputs.insert(
            PORT_MODEL_NAME_OUT.to_string(),
            serde_json::Value::String(model_name),
        );
        outputs.insert(
            PORT_MODEL_INFO.to_string(),
            serde_json::to_value(&model_info).unwrap(),
        );

        Ok(outputs)
    }
}

// ============================================================================
// Pumas-core library integration (behind feature gate)
// ============================================================================

#[cfg(feature = "model-library")]
mod library_support {
    use super::*;
    use node_engine::extension_keys;

    pub struct ResolvedModel {
        pub info: ModelInfo,
        pub search_results: Option<serde_json::Value>,
    }

    pub async fn resolve_with_library(
        extensions: &node_engine::ExecutorExtensions,
        model_name: &str,
        search_query: Option<&str>,
    ) -> Option<ResolvedModel> {
        let api = extensions.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)?;

        let mut search_results_json = None;

        // If a search query is provided, search local + HF
        if let Some(query) = search_query {
            let mut combined_results = Vec::new();

            // Search local library
            if let Ok(local) = api.search_models(query, 10, 0).await {
                for record in &local.models {
                    combined_results.push(serde_json::json!({
                        "source": "local",
                        "name": &record.official_name,
                        "id": &record.id,
                        "path": &record.path,
                        "model_type": &record.model_type,
                    }));
                }
            }

            // Search HuggingFace (if HF client is available)
            if let Ok(hf_models) = api.search_hf_models(query, Some("llm"), 10).await {
                for hf in &hf_models {
                    combined_results.push(serde_json::json!({
                        "source": "huggingface",
                        "name": hf.name,
                        "repo_id": hf.repo_id,
                        "developer": hf.developer,
                        "kind": hf.kind,
                        "formats": hf.formats,
                        "downloads": hf.downloads,
                        "url": hf.url,
                    }));
                }
            }

            if !combined_results.is_empty() {
                search_results_json = Some(serde_json::Value::Array(combined_results));
            }
        }

        // Resolve the specified model name against the local library
        if let Ok(search) = api.search_models(model_name, 1, 0).await {
            if let Some(record) = search.models.first() {
                return Some(ResolvedModel {
                    info: ModelInfo {
                        name: record.official_name.clone(),
                        path: Some(record.path.clone()),
                        model_type: Some(record.model_type.clone()),
                        id: Some(record.id.clone()),
                        official_name: Some(record.official_name.clone()),
                    },
                    search_results: search_results_json,
                });
            }
        }

        // Model not found in library but we may still have search results
        if search_results_json.is_some() {
            return Some(ResolvedModel {
                info: ModelInfo {
                    name: model_name.to_string(),
                    path: None,
                    model_type: Some("llm".to_string()),
                    id: None,
                    official_name: None,
                },
                search_results: search_results_json,
            });
        }

        None
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
        assert!(meta.outputs.iter().any(|p| p.id == "model_name"));
        assert!(meta.outputs.iter().any(|p| p.id == "search_results"));
    }

    #[tokio::test]
    async fn test_executor_passthrough() {
        let executor = ModelProviderExecutor;
        let context = Context::new();
        let extensions = node_engine::ExecutorExtensions::new();

        let mut inputs = HashMap::new();
        inputs.insert(
            PORT_MODEL_NAME.to_string(),
            serde_json::Value::String("mistral".to_string()),
        );

        let result = executor
            .execute("test-1", inputs, &context, &extensions)
            .await
            .unwrap();

        assert_eq!(
            result.get(PORT_MODEL_NAME_OUT).unwrap().as_str().unwrap(),
            "mistral"
        );
        let info: ModelInfo =
            serde_json::from_value(result.get(PORT_MODEL_INFO).unwrap().clone()).unwrap();
        assert_eq!(info.name, "mistral");
        assert!(info.path.is_none());
        assert!(info.id.is_none());
    }

    #[tokio::test]
    async fn test_executor_default_model() {
        let executor = ModelProviderExecutor;
        let context = Context::new();
        let extensions = node_engine::ExecutorExtensions::new();

        let result = executor
            .execute("test-1", HashMap::new(), &context, &extensions)
            .await
            .unwrap();

        assert_eq!(
            result.get(PORT_MODEL_NAME_OUT).unwrap().as_str().unwrap(),
            "llama2"
        );
    }

    #[test]
    fn test_model_info_serialization() {
        let info = ModelInfo {
            name: "test-model".to_string(),
            path: Some("/path/to/model".to_string()),
            model_type: Some("llm".to_string()),
            id: Some("uuid-123".to_string()),
            official_name: Some("Test Model".to_string()),
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
