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
const PORT_PUMAS_MODEL_REF: &str = "pumas_model_ref";
const PORT_RESOLVED_MODEL_PACKAGE_FACTS: &str = "resolved_model_package_facts";
const PORT_MODEL_ID: &str = "model_id";
const PORT_MODEL_TYPE: &str = "model_type";
const PORT_TASK_TYPE_PRIMARY: &str = "task_type_primary";
const PORT_BACKEND_KEY: &str = "backend_key";
const PORT_RECOMMENDED_BACKEND: &str = "recommended_backend";
const PORT_PLATFORM_CONTEXT: &str = "platform_context";
const PORT_SELECTED_BINDING_IDS: &str = "selected_binding_ids";
const PORT_DEPENDENCY_BINDINGS: &str = "dependency_bindings";
const PORT_DEPENDENCY_REQUIREMENTS_ID: &str = "dependency_requirements_id";
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
                PortMetadata::optional(PORT_PUMAS_MODEL_REF, "Pumas Model Ref", PortDataType::Json),
                PortMetadata::optional(
                    PORT_RESOLVED_MODEL_PACKAGE_FACTS,
                    "Resolved Model Package Facts",
                    PortDataType::Json,
                ),
                PortMetadata::optional(PORT_MODEL_ID, "Model ID", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_TYPE, "Model Type", PortDataType::String),
                PortMetadata::optional(PORT_TASK_TYPE_PRIMARY, "Task Type", PortDataType::String),
                PortMetadata::optional(PORT_BACKEND_KEY, "Backend Key", PortDataType::String),
                PortMetadata::optional(
                    PORT_RECOMMENDED_BACKEND,
                    "Recommended Backend",
                    PortDataType::String,
                ),
                PortMetadata::optional(
                    PORT_PLATFORM_CONTEXT,
                    "Platform Context",
                    PortDataType::Json,
                ),
                PortMetadata::optional(
                    PORT_SELECTED_BINDING_IDS,
                    "Selected Bindings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(
                    PORT_DEPENDENCY_BINDINGS,
                    "Dependency Bindings",
                    PortDataType::Json,
                ),
                PortMetadata::optional(
                    PORT_DEPENDENCY_REQUIREMENTS_ID,
                    "Dependency Requirements ID",
                    PortDataType::String,
                ),
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
    use pumas_library::models::{
        ModelExecutionDescriptor, ModelLibraryChangeKind, ModelLibraryRefreshScope,
        ModelLibraryUpdateFeed, ModelPackageFactsSummaryResult, ModelPackageFactsSummaryStatus,
    };
    use std::{collections::HashMap, sync::Arc};

    /// Provides available models from pumas-library for the `model_path` port.
    pub struct PumaLibOptionsProvider;

    /// Extract inference settings from a model record, falling back to computed
    /// defaults when the API-backed settings lookup is unavailable.
    fn resolve_inference_settings_fallback(
        record: &pumas_library::ModelRecord,
    ) -> serde_json::Value {
        // Try the stored value first
        if let Some(stored) = record.metadata.get("inference_settings") {
            if stored.as_array().is_some_and(|a| !a.is_empty()) {
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

    pub(crate) fn task_type_primary_from_descriptor_or_record(
        execution_descriptor: Option<&ModelExecutionDescriptor>,
        record: &pumas_library::ModelRecord,
    ) -> String {
        execution_descriptor
            .map(|descriptor| descriptor.task_type_primary.trim())
            .filter(|task| !task.is_empty() && !task.eq_ignore_ascii_case("unknown"))
            .map(ToOwned::to_owned)
            .or_else(|| {
                metadata_string(
                    record,
                    &[
                        "task_type_primary",
                        "taskTypePrimary",
                        "task_type",
                        "taskType",
                    ],
                )
            })
            .or_else(|| {
                metadata_string(record, &["pipeline_tag", "pipelineTag"])
                    .as_deref()
                    .map(pipeline_tag_to_task)
            })
            .unwrap_or_else(|| {
                if record.model_type.eq_ignore_ascii_case("audio") {
                    "text-to-audio".to_string()
                } else if record.model_type.eq_ignore_ascii_case("diffusion") {
                    "text-to-image".to_string()
                } else {
                    "text-generation".to_string()
                }
            })
    }

    pub(crate) async fn resolve_execution_descriptor(
        api: &Arc<pumas_library::PumasApi>,
        record: &pumas_library::ModelRecord,
    ) -> Option<ModelExecutionDescriptor> {
        if record.id.trim().is_empty() {
            return None;
        }

        api.resolve_model_execution_descriptor(&record.id)
            .await
            .ok()
    }

    pub(crate) struct PackageFactsSummaryCache {
        pub cursor: Option<String>,
        pub summaries: HashMap<String, ModelPackageFactsSummaryResult>,
    }

    impl PackageFactsSummaryCache {
        pub(crate) fn apply_update_feed(&mut self, feed: &ModelLibraryUpdateFeed) {
            self.cursor = Some(feed.cursor.clone());

            if feed.stale_cursor || feed.snapshot_required {
                self.summaries.clear();
                return;
            }

            for event in &feed.events {
                if event.change_kind == ModelLibraryChangeKind::ModelRemoved
                    || matches!(
                        event.refresh_scope,
                        ModelLibraryRefreshScope::Summary
                            | ModelLibraryRefreshScope::SummaryAndDetail
                    )
                {
                    self.summaries.remove(&event.model_id);
                }
            }
        }

        pub(crate) fn insert_summary(&mut self, summary: ModelPackageFactsSummaryResult) {
            self.summaries.insert(summary.model_id.clone(), summary);
        }

        pub(crate) fn needs_resolution(&self, model_id: &str) -> bool {
            self.summaries.get(model_id).map_or(true, |result| {
                result.summary.is_none()
                    || matches!(
                        result.status,
                        ModelPackageFactsSummaryStatus::Missing
                            | ModelPackageFactsSummaryStatus::Invalid
                    )
            })
        }
    }

    async fn poll_package_facts_summary_updates(
        api: &Arc<pumas_library::PumasApi>,
        cache: &mut PackageFactsSummaryCache,
        limit: usize,
    ) {
        if let Some(cursor) = cache.cursor.clone() {
            if let Ok(feed) = api
                .list_model_library_updates_since(Some(&cursor), limit)
                .await
            {
                cache.apply_update_feed(&feed);
            }
        }
    }

    async fn resolve_missing_package_facts_summaries(
        api: &Arc<pumas_library::PumasApi>,
        records: &[pumas_library::ModelRecord],
        cache: &mut PackageFactsSummaryCache,
    ) {
        for record in records {
            if cache.needs_resolution(&record.id) {
                if let Ok(summary) = api.resolve_model_package_facts_summary(&record.id).await {
                    cache.insert_summary(summary);
                }
            }
        }
    }

    pub(crate) async fn load_package_facts_summary_cache(
        api: &Arc<pumas_library::PumasApi>,
        records: &[pumas_library::ModelRecord],
        limit: usize,
        offset: usize,
    ) -> PackageFactsSummaryCache {
        let mut cache = PackageFactsSummaryCache {
            cursor: None,
            summaries: HashMap::new(),
        };

        if let Ok(snapshot) = api
            .model_package_facts_summary_snapshot(limit, offset)
            .await
        {
            cache.cursor = Some(snapshot.cursor);
            for item in snapshot.items {
                cache.summaries.insert(
                    item.model_id.clone(),
                    ModelPackageFactsSummaryResult {
                        model_id: item.model_id,
                        status: item.status,
                        summary: item.summary,
                    },
                );
            }
        }

        poll_package_facts_summary_updates(api, &mut cache, limit).await;
        resolve_missing_package_facts_summaries(api, records, &mut cache).await;
        poll_package_facts_summary_updates(api, &mut cache, limit).await;
        resolve_missing_package_facts_summaries(api, records, &mut cache).await;

        cache
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

            let summary_cache = load_package_facts_summary_cache(
                api,
                &records,
                query.limit.unwrap_or(records.len()).max(records.len()),
                query.offset.unwrap_or(0),
            )
            .await;

            let mut options = Vec::with_capacity(records.len());
            for m in &records {
                // Prefer the Pumas execution descriptor whenever the record can
                // resolve one so runtime-facing paths come from the executable
                // contract rather than projected metadata.
                let execution_descriptor = resolve_execution_descriptor(api, m).await;
                let inference_settings = api
                    .get_inference_settings(&m.id)
                    .await
                    .map(|settings| serde_json::to_value(settings).unwrap_or_default())
                    .unwrap_or_else(|_| resolve_inference_settings_fallback(m));
                let pipeline_tag = metadata_string(m, &["pipeline_tag", "pipelineTag"]);
                let task_type_primary =
                    task_type_primary_from_descriptor_or_record(execution_descriptor.as_ref(), m);
                let dependency_bindings = m
                    .metadata
                    .get("dependency_bindings")
                    .cloned()
                    .unwrap_or(serde_json::Value::Array(Vec::new()));
                let recommended_backend = execution_descriptor
                    .as_ref()
                    .and_then(|descriptor| descriptor.recommended_backend.clone())
                    .or_else(|| metadata_string(m, &["recommended_backend", "recommendedBackend"]));
                let runtime_engine_hints = execution_descriptor
                    .as_ref()
                    .map(|descriptor| {
                        serde_json::to_value(&descriptor.runtime_engine_hints)
                            .unwrap_or(serde_json::Value::Array(Vec::new()))
                    })
                    .unwrap_or_else(|| {
                        m.metadata
                            .get("runtime_engine_hints")
                            .cloned()
                            .unwrap_or(serde_json::Value::Array(Vec::new()))
                    });
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
                let execution_path = execution_descriptor
                    .as_ref()
                    .map(|descriptor| descriptor.entry_path.clone())
                    .unwrap_or_else(|| m.path.clone());
                let package_facts_summary = summary_cache.summaries.get(&m.id).and_then(|result| {
                    result
                        .summary
                        .as_ref()
                        .and_then(|summary| serde_json::to_value(summary).ok())
                });
                let package_facts_summary_status =
                    summary_cache.summaries.get(&m.id).and_then(|result| {
                        serde_json::to_value(result.status)
                            .ok()
                            .and_then(|value| value.as_str().map(ToOwned::to_owned))
                    });

                options.push(PortOption {
                    value: serde_json::json!(execution_path),
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
                        "entry_path": execution_descriptor.as_ref().map(|descriptor| descriptor.entry_path.clone()),
                        "execution_contract_version": execution_descriptor.as_ref().map(|descriptor| descriptor.execution_contract_version),
                        "storage_kind": execution_descriptor.as_ref().map(|descriptor| descriptor.storage_kind),
                        "validation_state": execution_descriptor.as_ref().map(|descriptor| descriptor.validation_state),
                        "dependency_resolution": execution_descriptor.as_ref().and_then(|descriptor| descriptor.dependency_resolution.clone()),
                        "requires_custom_code": requires_custom_code,
                        "custom_code_sources": custom_code_sources,
                        "dependency_bindings": dependency_bindings,
                        "review_reasons": review_reasons,
                        "inference_settings": inference_settings,
                        "package_facts_summary_cursor": summary_cache.cursor.clone(),
                        "package_facts_summary_status": package_facts_summary_status,
                        "package_facts_summary": package_facts_summary,
                    })),
                });
            }

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
        assert_eq!(meta.outputs.len(), 14);

        assert!(meta.outputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.outputs.iter().any(|p| p.id == "pumas_model_ref"
            && p.data_type == PortDataType::Json
            && !p.required));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == "resolved_model_package_facts"
                && p.data_type == PortDataType::Json
                && !p.required));
        assert!(meta.outputs.iter().any(|p| p.id == "model_id"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_type"));
        assert!(meta.outputs.iter().any(|p| p.id == "task_type_primary"));
        assert!(meta.outputs.iter().any(|p| p.id == "backend_key"));
        assert!(meta.outputs.iter().any(|p| p.id == "recommended_backend"));
        assert!(meta.outputs.iter().any(|p| p.id == "platform_context"
            && p.data_type == PortDataType::Json
            && !p.required));
        assert!(meta.outputs.iter().any(|p| p.id == "selected_binding_ids"
            && p.data_type == PortDataType::Json
            && !p.required));
        assert!(meta.outputs.iter().any(|p| p.id == "dependency_bindings"
            && p.data_type == PortDataType::Json
            && !p.required));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == "dependency_requirements_id"
                && p.data_type == PortDataType::String
                && !p.required));
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

#[cfg(all(test, feature = "model-library"))]
mod model_library_tests {
    use super::options_provider::{
        load_package_facts_summary_cache, resolve_execution_descriptor,
        task_type_primary_from_descriptor_or_record, PackageFactsSummaryCache,
    };
    use pumas_library::models::{
        ModelExecutionDescriptor, ModelFactFamily, ModelLibraryChangeKind,
        ModelLibraryRefreshScope, ModelLibraryUpdateEvent, ModelLibraryUpdateFeed,
        ModelPackageFactsSummaryResult, ModelPackageFactsSummaryStatus,
    };
    use pumas_library::{ModelRecord, PumasApi};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_env() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        std::fs::create_dir_all(temp_dir.path().join("launcher-data")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
        temp_dir
    }

    fn package_summary_result(model_id: &str, status: &str) -> ModelPackageFactsSummaryResult {
        serde_json::from_value(serde_json::json!({
            "model_id": model_id,
            "status": status,
            "summary": {
                "package_facts_contract_version": 1,
                "model_ref": {
                    "model_id": model_id,
                    "revision": null,
                    "selected_artifact_id": "main",
                    "selected_artifact_path": model_id
                },
                "artifact_kind": "gguf",
                "entry_path": format!("{model_id}/model.gguf"),
                "storage_kind": "library_owned",
                "validation_state": "valid",
                "task": {
                    "pipeline_tag": "text-generation",
                    "task_type_primary": "text-generation",
                    "input_modalities": ["text"],
                    "output_modalities": ["text"]
                },
                "backend_hints": {
                    "accepted": ["llama.cpp"],
                    "raw": ["llama.cpp"]
                },
                "requires_custom_code": false,
                "config_status": "missing",
                "tokenizer_status": "missing",
                "processor_status": "missing",
                "generation_config_status": "missing",
                "generation_defaults_status": "missing"
            }
        }))
        .expect("summary fixture should decode")
    }

    fn model_record_with_metadata(metadata: serde_json::Value) -> ModelRecord {
        ModelRecord {
            id: "llm/imported/test-model".to_string(),
            path: "/models/test-model".to_string(),
            cleaned_name: "test-model".to_string(),
            official_name: "test-model".to_string(),
            model_type: "llm".to_string(),
            tags: Vec::new(),
            hashes: HashMap::new(),
            metadata,
            updated_at: "2026-05-04T00:00:00Z".to_string(),
        }
    }

    fn model_execution_descriptor_with_task(task_type_primary: &str) -> ModelExecutionDescriptor {
        serde_json::from_value(serde_json::json!({
            "execution_contract_version": 1,
            "model_id": "llm/imported/test-model",
            "entry_path": "/models/test-model/model.safetensors",
            "model_type": "llm",
            "task_type_primary": task_type_primary,
            "recommended_backend": "pytorch",
            "runtime_engine_hints": ["transformers", "pytorch"],
            "storage_kind": "library_owned",
            "validation_state": "valid",
            "dependency_resolution": null
        }))
        .expect("execution descriptor fixture should decode")
    }

    fn write_test_diffusers_bundle(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("scheduler")).unwrap();
        std::fs::create_dir_all(root.join("text_encoder")).unwrap();
        std::fs::create_dir_all(root.join("tokenizer")).unwrap();
        std::fs::create_dir_all(root.join("unet")).unwrap();
        std::fs::create_dir_all(root.join("vae")).unwrap();
        std::fs::write(
            root.join("model_index.json"),
            serde_json::json!({
                "_class_name": "StableDiffusionPipeline",
                "scheduler": ["diffusers", "EulerDiscreteScheduler"],
                "text_encoder": ["transformers", "CLIPTextModel"],
                "tokenizer": ["transformers", "CLIPTokenizer"],
                "unet": ["diffusers", "UNet2DConditionModel"],
                "vae": ["diffusers", "AutoencoderKL"]
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_imported_diffusion_metadata(
        model_dir: &std::path::Path,
        entry_path: &std::path::Path,
    ) {
        std::fs::create_dir_all(model_dir).unwrap();
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": "diffusion/imported/test-bundle",
                "family": "imported",
                "model_type": "diffusion",
                "official_name": "test-bundle",
                "cleaned_name": "test-bundle",
                "source_path": entry_path.display().to_string(),
                "entry_path": entry_path.display().to_string(),
                "storage_kind": "external_reference",
                "bundle_format": "diffusers_directory",
                "pipeline_class": "StableDiffusionPipeline",
                "import_state": "ready",
                "validation_state": "valid",
                "pipeline_tag": "text-to-image",
                "task_type_primary": "text-to-image",
                "input_modalities": ["text"],
                "output_modalities": ["image"],
                "task_classification_source": "external-diffusers-import",
                "task_classification_confidence": 1.0,
                "model_type_resolution_source": "external-diffusers-import",
                "model_type_resolution_confidence": 1.0,
                "recommended_backend": "diffusers",
                "runtime_engine_hints": ["diffusers", "pytorch"]
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_library_owned_file_model(
        model_dir: &std::path::Path,
        file_name: &str,
        file_size_bytes: usize,
    ) -> std::path::PathBuf {
        std::fs::create_dir_all(model_dir).unwrap();
        let model_file = model_dir.join(file_name);
        std::fs::write(&model_file, vec![0_u8; file_size_bytes]).unwrap();
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": "llm/imported/test-gguf",
                "family": "imported",
                "model_type": "llm",
                "official_name": "test-gguf",
                "cleaned_name": "test-gguf",
                "source_path": model_dir.display().to_string(),
                "storage_kind": "library_owned",
                "import_state": "ready",
                "validation_state": "valid",
                "task_type_primary": "text-generation",
                "recommended_backend": "llamacpp",
                "runtime_engine_hints": ["llamacpp"]
            })
            .to_string(),
        )
        .unwrap();
        model_file
    }

    #[tokio::test]
    async fn test_bundle_models_resolve_execution_descriptor_entry_path() {
        let temp_dir = create_test_env();
        let bundle_root = temp_dir.path().join("external/tiny-sd-turbo");
        write_test_diffusers_bundle(&bundle_root);

        let model_dir = temp_dir
            .path()
            .join("shared-resources/models/diffusion/imported/test-bundle");
        write_imported_diffusion_metadata(&model_dir, &bundle_root);

        let api = PumasApi::builder(temp_dir.path()).build().await.unwrap();
        api.rebuild_model_index().await.unwrap();

        let record = api
            .get_model("diffusion/imported/test-bundle")
            .await
            .unwrap()
            .expect("model record should exist");

        let descriptor = resolve_execution_descriptor(&Arc::new(api), &record)
            .await
            .expect("execution descriptor should resolve");
        assert_eq!(descriptor.entry_path, bundle_root.display().to_string());
        assert_eq!(descriptor.task_type_primary, "text-to-image");
    }

    #[tokio::test]
    async fn test_file_models_resolve_execution_descriptor_primary_file_path() {
        let temp_dir = create_test_env();
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models/llm/imported/test-gguf");
        let model_file = write_library_owned_file_model(&model_dir, "model.gguf", 256);

        let api = PumasApi::builder(temp_dir.path()).build().await.unwrap();
        api.rebuild_model_index().await.unwrap();

        let record = api
            .get_model("llm/imported/test-gguf")
            .await
            .unwrap()
            .expect("model record should exist");

        let descriptor = resolve_execution_descriptor(&Arc::new(api), &record)
            .await
            .expect("execution descriptor should resolve");
        assert_eq!(descriptor.entry_path, model_file.display().to_string());
        assert_eq!(descriptor.task_type_primary, "text-generation");
    }

    #[test]
    fn test_task_type_primary_prefers_execution_descriptor_over_metadata() {
        let record = model_record_with_metadata(serde_json::json!({
            "task_type_primary": "text-generation",
            "pipeline_tag": "text-generation"
        }));
        let descriptor = model_execution_descriptor_with_task("image-to-text");

        let task_type = task_type_primary_from_descriptor_or_record(Some(&descriptor), &record);

        assert_eq!(task_type, "image-to-text");
    }

    #[test]
    fn test_task_type_primary_uses_metadata_when_execution_descriptor_task_is_unknown() {
        let record = model_record_with_metadata(serde_json::json!({
            "task_type_primary": "text-generation",
            "pipeline_tag": "text-generation"
        }));
        let descriptor = model_execution_descriptor_with_task("unknown");

        let task_type = task_type_primary_from_descriptor_or_record(Some(&descriptor), &record);

        assert_eq!(task_type, "text-generation");
    }

    #[tokio::test]
    async fn test_model_options_populate_package_facts_summary_cache() {
        let temp_dir = create_test_env();
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models/llm/imported/test-gguf");
        write_library_owned_file_model(&model_dir, "model.gguf", 256);

        let api = Arc::new(PumasApi::builder(temp_dir.path()).build().await.unwrap());
        api.rebuild_model_index().await.unwrap();

        let record = api
            .get_model("llm/imported/test-gguf")
            .await
            .unwrap()
            .expect("model record should exist");

        let cache = load_package_facts_summary_cache(&api, &[record], 100, 0).await;
        let summary = cache
            .summaries
            .get("llm/imported/test-gguf")
            .expect("summary should be populated for listed model");

        assert!(cache
            .cursor
            .as_deref()
            .is_some_and(|cursor| { cursor.starts_with("model-library-updates:") }));
        assert!(
            summary.summary.is_some(),
            "missing summary rows should be regenerated through Pumas API"
        );
    }

    #[test]
    fn test_package_facts_summary_cache_applies_update_feed_invalidation() {
        let mut cache = PackageFactsSummaryCache {
            cursor: Some("model-library-updates:1".to_string()),
            summaries: HashMap::from([
                (
                    "model-a".to_string(),
                    ModelPackageFactsSummaryResult {
                        model_id: "model-a".to_string(),
                        status: ModelPackageFactsSummaryStatus::Cached,
                        summary: None,
                    },
                ),
                (
                    "model-b".to_string(),
                    ModelPackageFactsSummaryResult {
                        model_id: "model-b".to_string(),
                        status: ModelPackageFactsSummaryStatus::Cached,
                        summary: None,
                    },
                ),
            ]),
        };
        let feed = ModelLibraryUpdateFeed {
            cursor: "model-library-updates:2".to_string(),
            events: vec![ModelLibraryUpdateEvent {
                cursor: "model-library-updates:2".to_string(),
                model_id: "model-a".to_string(),
                change_kind: ModelLibraryChangeKind::PackageFactsModified,
                fact_family: ModelFactFamily::PackageFacts,
                refresh_scope: ModelLibraryRefreshScope::SummaryAndDetail,
                selected_artifact_id: None,
                producer_revision: Some("rev-2".to_string()),
            }],
            stale_cursor: false,
            snapshot_required: false,
        };

        cache.apply_update_feed(&feed);

        assert_eq!(cache.cursor.as_deref(), Some("model-library-updates:2"));
        assert!(!cache.summaries.contains_key("model-a"));
        assert!(cache.summaries.contains_key("model-b"));
    }

    #[test]
    fn test_package_facts_summary_cache_regenerates_after_snapshot_update_invalidation() {
        let mut cache = PackageFactsSummaryCache {
            cursor: Some("model-library-updates:1".to_string()),
            summaries: HashMap::from([
                (
                    "model-a".to_string(),
                    package_summary_result("model-a", "cached"),
                ),
                (
                    "model-b".to_string(),
                    package_summary_result("model-b", "cached"),
                ),
            ]),
        };
        let feed = ModelLibraryUpdateFeed {
            cursor: "model-library-updates:2".to_string(),
            events: vec![ModelLibraryUpdateEvent {
                cursor: "model-library-updates:2".to_string(),
                model_id: "model-a".to_string(),
                change_kind: ModelLibraryChangeKind::PackageFactsModified,
                fact_family: ModelFactFamily::PackageFacts,
                refresh_scope: ModelLibraryRefreshScope::SummaryAndDetail,
                selected_artifact_id: Some("main".to_string()),
                producer_revision: Some("rev-2".to_string()),
            }],
            stale_cursor: false,
            snapshot_required: false,
        };

        cache.apply_update_feed(&feed);

        assert_eq!(cache.cursor.as_deref(), Some("model-library-updates:2"));
        assert!(
            cache.needs_resolution("model-a"),
            "updated snapshot rows must be regenerated after feed invalidation"
        );
        assert!(
            !cache.needs_resolution("model-b"),
            "unaffected snapshot rows should remain usable"
        );

        cache.insert_summary(package_summary_result("model-a", "regenerated"));

        assert!(!cache.needs_resolution("model-a"));
        assert_eq!(
            cache.summaries.get("model-a").map(|summary| summary.status),
            Some(ModelPackageFactsSummaryStatus::Regenerated)
        );
    }

    #[test]
    fn test_package_facts_summary_cache_invalidates_regenerated_rows_from_later_feed() {
        let mut cache = PackageFactsSummaryCache {
            cursor: Some("model-library-updates:2".to_string()),
            summaries: HashMap::from([
                (
                    "model-a".to_string(),
                    package_summary_result("model-a", "regenerated"),
                ),
                (
                    "model-b".to_string(),
                    package_summary_result("model-b", "regenerated"),
                ),
            ]),
        };
        let feed = ModelLibraryUpdateFeed {
            cursor: "model-library-updates:3".to_string(),
            events: vec![ModelLibraryUpdateEvent {
                cursor: "model-library-updates:3".to_string(),
                model_id: "model-a".to_string(),
                change_kind: ModelLibraryChangeKind::PackageFactsModified,
                fact_family: ModelFactFamily::PackageFacts,
                refresh_scope: ModelLibraryRefreshScope::SummaryAndDetail,
                selected_artifact_id: Some("main".to_string()),
                producer_revision: Some("rev-3".to_string()),
            }],
            stale_cursor: false,
            snapshot_required: false,
        };

        cache.apply_update_feed(&feed);

        assert_eq!(cache.cursor.as_deref(), Some("model-library-updates:3"));
        assert!(
            cache.needs_resolution("model-a"),
            "post-regeneration update feeds must invalidate changed summaries"
        );
        assert!(
            !cache.needs_resolution("model-b"),
            "unaffected regenerated rows should remain usable"
        );
    }

    #[test]
    fn test_package_facts_summary_cache_keeps_regenerated_rows_when_later_feed_is_empty() {
        let mut cache = PackageFactsSummaryCache {
            cursor: Some("model-library-updates:2".to_string()),
            summaries: HashMap::from([(
                "model-a".to_string(),
                package_summary_result("model-a", "regenerated"),
            )]),
        };
        let feed = ModelLibraryUpdateFeed {
            cursor: "model-library-updates:2".to_string(),
            events: Vec::new(),
            stale_cursor: false,
            snapshot_required: false,
        };

        cache.apply_update_feed(&feed);

        assert_eq!(cache.cursor.as_deref(), Some("model-library-updates:2"));
        assert!(
            !cache.needs_resolution("model-a"),
            "empty post-regeneration feeds should not discard fresh summaries"
        );
    }

    #[test]
    fn test_package_facts_summary_cache_invalidates_removed_model_for_detail_scope() {
        let mut cache = PackageFactsSummaryCache {
            cursor: Some("model-library-updates:1".to_string()),
            summaries: HashMap::from([(
                "model-a".to_string(),
                ModelPackageFactsSummaryResult {
                    model_id: "model-a".to_string(),
                    status: ModelPackageFactsSummaryStatus::Cached,
                    summary: None,
                },
            )]),
        };
        let feed = ModelLibraryUpdateFeed {
            cursor: "model-library-updates:2".to_string(),
            events: vec![ModelLibraryUpdateEvent {
                cursor: "model-library-updates:2".to_string(),
                model_id: "model-a".to_string(),
                change_kind: ModelLibraryChangeKind::ModelRemoved,
                fact_family: ModelFactFamily::ModelRecord,
                refresh_scope: ModelLibraryRefreshScope::Detail,
                selected_artifact_id: None,
                producer_revision: None,
            }],
            stale_cursor: false,
            snapshot_required: false,
        };

        cache.apply_update_feed(&feed);

        assert!(!cache.summaries.contains_key("model-a"));
    }

    #[test]
    fn test_package_facts_summary_cache_clears_on_stale_update_cursor() {
        let mut cache = PackageFactsSummaryCache {
            cursor: Some("model-library-updates:1".to_string()),
            summaries: HashMap::from([(
                "model-a".to_string(),
                ModelPackageFactsSummaryResult {
                    model_id: "model-a".to_string(),
                    status: ModelPackageFactsSummaryStatus::Cached,
                    summary: None,
                },
            )]),
        };
        let feed = ModelLibraryUpdateFeed {
            cursor: "model-library-updates:latest".to_string(),
            events: Vec::new(),
            stale_cursor: true,
            snapshot_required: true,
        };

        cache.apply_update_feed(&feed);

        assert_eq!(
            cache.cursor.as_deref(),
            Some("model-library-updates:latest")
        );
        assert!(cache.summaries.is_empty());
    }
}
