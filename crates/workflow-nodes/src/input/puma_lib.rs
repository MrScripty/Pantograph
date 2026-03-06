//! Puma-Lib Node
//!
//! This module registers a stub node descriptor for `puma-lib` so that
//! `register_builtins()` discovers the node via `inventory`. Actual execution
//! is handled by the host application through the callback bridge — the host
//! provides the model file path from its local pumas-core library.
//!
//! When the `model-library` feature is enabled, this module also registers
//! a `PortOptionsProvider` for the `model_path` port, enabling hosts to
//! query available models from the pumas-library.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

const PORT_MODEL_PATH: &str = "model_path";
const PORT_INFERENCE_SETTINGS: &str = "inference_settings";
const PORT_DEPENDENCY_REQUIREMENTS: &str = "dependency_requirements";

/// Stub task for the puma-lib node.
///
/// The node is discoverable by all consumers (including puma-bot NIF) but
/// always fails at runtime — the host must intercept execution via the
/// callback bridge and supply the model file path itself.
#[derive(Clone)]
pub struct PumaLibTask {
    task_id: String,
}

impl PumaLibTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for PumaLibTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "puma-lib".to_string(),
            category: NodeCategory::Input,
            label: "Puma-Lib".to_string(),
            description: "Provides AI model file path".to_string(),
            inputs: vec![],
            outputs: vec![
                PortMetadata::optional(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(
                    PORT_INFERENCE_SETTINGS,
                    "Inference Settings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(
                    PORT_DEPENDENCY_REQUIREMENTS,
                    "Dependency Requirements",
                    PortDataType::Json,
                ),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(PumaLibTask::descriptor));

// ---------------------------------------------------------------------------
// Port options provider (model-library feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "model-library")]
mod options_provider {
    use async_trait::async_trait;
    use node_engine::{
        extension_keys, ExecutorExtensions, NodeEngineError, PortOption, PortOptionsProvider,
        PortOptionsQuery, PortOptionsResult,
    };
    use std::sync::Arc;

    /// Provides available models from pumas-library for the `model_path` port.
    pub struct PumaLibOptionsProvider;

    /// Extract inference settings from a model record, falling back to computed
    /// defaults when the stored metadata has no settings (pre-existing models).
    fn resolve_inference_settings(record: &pumas_library::ModelRecord) -> serde_json::Value {
        // Try the stored value first
        if let Some(stored) = record.metadata.get("inference_settings") {
            if stored.is_array() && !stored.as_array().map_or(true, |a| a.is_empty()) {
                return stored.clone();
            }
        }

        // Lazy fallback: compute from model_type + file format.
        // record.path is a directory; try to infer format from files in metadata.
        let file_format = record
            .metadata
            .get("files")
            .and_then(|f| f.as_array())
            .and_then(|files| {
                files.iter().find_map(|f| {
                    let name = f.get("name")?.as_str()?;
                    std::path::Path::new(name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase())
                })
            })
            .unwrap_or_default();
        let subtype = record
            .metadata
            .get("subtype")
            .and_then(|v| v.as_str())
            .map(String::from);

        pumas_library::models::default_inference_settings(
            &record.model_type,
            &file_format,
            subtype.as_deref(),
        )
        .map(|s| serde_json::to_value(s).unwrap_or_default())
        .unwrap_or(serde_json::Value::Null)
    }

    fn metadata_string(record: &pumas_library::ModelRecord, keys: &[&str]) -> Option<String> {
        let obj = record.metadata.as_object()?;
        keys.iter()
            .find_map(|k| obj.get(*k).and_then(|v| v.as_str()).map(|s| s.to_string()))
    }

    fn pipeline_tag_to_task(pipeline_tag: &str) -> String {
        match pipeline_tag.to_lowercase().as_str() {
            "text-to-audio" | "text-to-speech" => "text-to-audio".to_string(),
            "automatic-speech-recognition" => "audio-to-text".to_string(),
            "text-to-image" | "image-to-image" => "text-to-image".to_string(),
            "image-classification" | "object-detection" | "image-to-text" => {
                "image-to-text".to_string()
            }
            "feature-extraction" | "sentence-similarity" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    #[async_trait]
    impl PortOptionsProvider for PumaLibOptionsProvider {
        async fn query_options(
            &self,
            query: &PortOptionsQuery,
            extensions: &ExecutorExtensions,
        ) -> node_engine::Result<PortOptionsResult> {
            let api = extensions
                .get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
                .ok_or_else(|| {
                    NodeEngineError::ExecutionFailed("Model library not available".to_string())
                })?;

            let records = if let Some(ref search) = query.search {
                let result = api
                    .search_models(search, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
                    .await
                    .map_err(|e| NodeEngineError::ExecutionFailed(e.to_string()))?;
                result.models
            } else {
                api.list_models()
                    .await
                    .map_err(|e| NodeEngineError::ExecutionFailed(e.to_string()))?
            };

            let options: Vec<PortOption> = records
                .iter()
                .map(|m| {
                    let pipeline_tag = metadata_string(m, &["pipeline_tag", "pipelineTag"]);
                    let task_type_primary = metadata_string(
                        m,
                        &[
                            "task_type_primary",
                            "taskTypePrimary",
                            "task_type",
                            "taskType",
                        ],
                    )
                    .or_else(|| pipeline_tag.as_deref().map(pipeline_tag_to_task))
                    .unwrap_or_else(|| {
                        if m.model_type.eq_ignore_ascii_case("audio") {
                            "text-to-audio".to_string()
                        } else {
                            "text-generation".to_string()
                        }
                    });
                    let dependency_bindings = m
                        .metadata
                        .get("dependency_bindings")
                        .cloned()
                        .unwrap_or(serde_json::Value::Array(Vec::new()));
                    let recommended_backend =
                        metadata_string(m, &["recommended_backend", "recommendedBackend"]);
                    let runtime_engine_hints = m
                        .metadata
                        .get("runtime_engine_hints")
                        .cloned()
                        .unwrap_or(serde_json::Value::Array(Vec::new()));
                    let requires_custom_code = m
                        .metadata
                        .get("requires_custom_code")
                        .cloned()
                        .unwrap_or(serde_json::Value::Bool(false));
                    let custom_code_sources = m
                        .metadata
                        .get("custom_code_sources")
                        .cloned()
                        .unwrap_or(serde_json::Value::Array(Vec::new()));
                    let review_reasons =
                        m.metadata
                            .get("review_reasons")
                            .cloned()
                            .unwrap_or_else(|| {
                                metadata_string(m, &["review_reason", "reviewReason"])
                                    .map(|reason| serde_json::json!([reason]))
                                    .unwrap_or_else(|| serde_json::Value::Array(Vec::new()))
                            });

                    PortOption {
                        value: serde_json::json!(m.path),
                        label: m.official_name.clone(),
                        description: Some(format!("{} | {}", m.model_type, m.tags.join(", "))),
                        metadata: Some(serde_json::json!({
                            "id": m.id,
                            "model_type": m.model_type,
                            "cleaned_name": m.cleaned_name,
                            "pipeline_tag": pipeline_tag,
                            "task_type_primary": task_type_primary,
                            "recommended_backend": recommended_backend,
                            "runtime_engine_hints": runtime_engine_hints,
                            "requires_custom_code": requires_custom_code,
                            "custom_code_sources": custom_code_sources,
                            "dependency_bindings": dependency_bindings,
                            "review_reasons": review_reasons,
                            "inference_settings": resolve_inference_settings(m),
                        })),
                    }
                })
                .collect();

            let total = options.len();
            Ok(PortOptionsResult {
                options,
                total_count: total,
                searchable: true,
            })
        }
    }
}

#[cfg(feature = "model-library")]
inventory::submit!(node_engine::PortQueryFn {
    node_type: "puma-lib",
    port_id: "model_path",
    provider: || Box::new(options_provider::PumaLibOptionsProvider),
});

#[async_trait]
impl Task for PumaLibTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "puma-lib requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = PumaLibTask::descriptor();
        assert_eq!(meta.node_type, "puma-lib");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = PumaLibTask::descriptor();

        assert!(meta.inputs.is_empty());
        assert_eq!(meta.outputs.len(), 3);

        assert!(meta.outputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.outputs.iter().any(|p| p.id == "inference_settings"
            && p.data_type == PortDataType::Json
            && !p.required));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == "dependency_requirements"
                && p.data_type == PortDataType::Json
                && !p.required));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = PumaLibTask::new("test");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("callback bridge"),
            "expected callback bridge message, got: {err}"
        );
    }
}
