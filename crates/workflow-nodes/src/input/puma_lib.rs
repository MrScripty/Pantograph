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
    use crate::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};
    use async_trait::async_trait;
    use node_engine::{
        extension_keys, ExecutorExtensions, NodeEngineError, PortOption, PortOptionsProvider,
        PortOptionsQuery, PortOptionsResult,
    };
    #[cfg(test)]
    use pumas_library::models::{
        ModelExecutionDescriptor, ModelLibraryChangeKind, ModelLibraryRefreshScope,
        ModelLibraryUpdateFeed, ModelPackageFactsSummaryResult, ModelPackageFactsSummaryStatus,
    };
    use pumas_library::models::{
        ModelLibrarySelectorSnapshotRequest, ModelLibrarySelectorSnapshotRow,
    };
    #[cfg(test)]
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Provides available models from pumas-library for the `model_path` port.
    pub struct PumaLibOptionsProvider;

    /// Compute conservative inference settings when the API-backed settings
    /// lookup is unavailable.
    #[cfg(test)]
    pub(crate) fn resolve_inference_settings_fallback(
        record: &pumas_library::ModelRecord,
    ) -> serde_json::Value {
        pumas_library::models::default_inference_settings(&record.model_type, "", None)
            .map(|s| serde_json::to_value(s).unwrap_or_default())
            .unwrap_or(serde_json::Value::Null)
    }

    #[cfg(test)]
    pub(crate) fn runtime_engine_hints_from_summary(
        summary_result: Option<&ModelPackageFactsSummaryResult>,
    ) -> Option<serde_json::Value> {
        let summary_result = summary_result?;
        let Some(summary) = summary_result.summary.as_ref() else {
            return Some(serde_json::Value::Array(Vec::new()));
        };
        if !summary.backend_hints.accepted.is_empty() {
            return serde_json::to_value(&summary.backend_hints.accepted).ok();
        }
        if !summary.backend_hints.raw.is_empty() {
            return serde_json::to_value(&summary.backend_hints.raw).ok();
        }
        None
    }

    #[cfg(test)]
    pub(crate) fn requires_custom_code_from_summary(
        summary_result: Option<&ModelPackageFactsSummaryResult>,
    ) -> Option<serde_json::Value> {
        summary_result.map(|result| {
            result
                .summary
                .as_ref()
                .map(|summary| serde_json::Value::Bool(summary.requires_custom_code))
                .unwrap_or(serde_json::Value::Bool(false))
        })
    }

    #[cfg(test)]
    pub(crate) fn custom_code_sources_for_option_metadata(
        _summary_result: Option<&ModelPackageFactsSummaryResult>,
        _record: &pumas_library::ModelRecord,
    ) -> serde_json::Value {
        serde_json::Value::Array(Vec::new())
    }

    #[cfg(test)]
    pub(crate) fn review_reasons_for_option_metadata(
        summary_result: Option<&ModelPackageFactsSummaryResult>,
        _record: &pumas_library::ModelRecord,
    ) -> serde_json::Value {
        if let Some(result) = summary_result {
            let Some(summary) = result.summary.as_ref() else {
                return serde_json::Value::Array(Vec::new());
            };
            return serde_json::to_value(&summary.diagnostic_codes)
                .unwrap_or(serde_json::Value::Array(Vec::new()));
        }
        serde_json::Value::Array(Vec::new())
    }

    #[cfg(test)]
    pub(crate) fn dependency_bindings_for_option_metadata(
        execution_descriptor: Option<&ModelExecutionDescriptor>,
        _record: &pumas_library::ModelRecord,
    ) -> serde_json::Value {
        if let Some(descriptor) = execution_descriptor {
            return descriptor
                .dependency_resolution
                .as_ref()
                .and_then(|resolution| resolution.get("bindings").cloned())
                .unwrap_or(serde_json::Value::Array(Vec::new()));
        }
        serde_json::Value::Array(Vec::new())
    }

    #[cfg(test)]
    fn pipeline_tag_from_summary(
        summary_result: Option<&ModelPackageFactsSummaryResult>,
    ) -> Option<String> {
        summary_result
            .and_then(|result| result.summary.as_ref())
            .and_then(|summary| summary.task.pipeline_tag.as_deref())
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(ToOwned::to_owned)
    }

    #[cfg(test)]
    fn task_type_primary_from_summary(
        summary_result: Option<&ModelPackageFactsSummaryResult>,
    ) -> Option<String> {
        summary_result
            .and_then(|result| result.summary.as_ref())
            .and_then(|summary| summary.task.task_type_primary.as_deref())
            .map(str::trim)
            .filter(|task| !task.is_empty() && !task.eq_ignore_ascii_case("unknown"))
            .map(ToOwned::to_owned)
            .or_else(|| {
                pipeline_tag_from_summary(summary_result)
                    .as_deref()
                    .map(pipeline_tag_to_task)
            })
    }

    #[cfg(test)]
    fn default_task_type_primary_from_record(record: &pumas_library::ModelRecord) -> String {
        if record.model_type.eq_ignore_ascii_case("audio") {
            "text-to-audio".to_string()
        } else if record.model_type.eq_ignore_ascii_case("diffusion") {
            "text-to-image".to_string()
        } else {
            "text-generation".to_string()
        }
    }

    #[cfg(test)]
    pub(crate) fn task_type_primary_from_descriptor_or_record(
        execution_descriptor: Option<&ModelExecutionDescriptor>,
        summary_result: Option<&ModelPackageFactsSummaryResult>,
        record: &pumas_library::ModelRecord,
    ) -> String {
        execution_descriptor
            .map(|descriptor| descriptor.task_type_primary.trim())
            .filter(|task| !task.is_empty() && !task.eq_ignore_ascii_case("unknown"))
            .map(ToOwned::to_owned)
            .or_else(|| task_type_primary_from_summary(summary_result))
            .unwrap_or_else(|| default_task_type_primary_from_record(record))
    }

    #[cfg(test)]
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

    #[cfg(test)]
    pub(crate) struct PackageFactsSummaryCache {
        pub cursor: Option<String>,
        pub summaries: HashMap<String, ModelPackageFactsSummaryResult>,
    }

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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

    fn selector_snapshot_request(query: &PortOptionsQuery) -> ModelLibrarySelectorSnapshotRequest {
        ModelLibrarySelectorSnapshotRequest {
            offset: query
                .offset
                .map(|offset| offset.min(u32::MAX as usize) as u32),
            limit: query.limit.map(|limit| limit.min(u32::MAX as usize) as u32),
            search: query
                .search
                .as_deref()
                .map(str::trim)
                .filter(|search| !search.is_empty())
                .map(ToOwned::to_owned),
            model_type: None,
            task_type_primary: None,
        }
    }

    fn selector_row_description(row: &ModelLibrarySelectorSnapshotRow) -> String {
        let model_type = row.model_type.as_deref().unwrap_or("unknown");
        if row.tags.is_empty() {
            model_type.to_string()
        } else {
            format!("{} | {}", model_type, row.tags.join(", "))
        }
    }

    fn selector_row_value(row: &ModelLibrarySelectorSnapshotRow) -> serde_json::Value {
        row.executable_entry_path()
            .map(|path| serde_json::json!(path))
            .unwrap_or_else(|| serde_json::json!(row.model_ref.model_id))
    }

    fn selector_row_option_metadata(
        row: &ModelLibrarySelectorSnapshotRow,
        cursor: &str,
    ) -> serde_json::Value {
        let package_facts_summary = row
            .package_facts_summary
            .as_ref()
            .and_then(|summary| serde_json::to_value(summary).ok());
        let runtime_engine_hints = serde_json::to_value(&row.runtime_engine_hints)
            .unwrap_or(serde_json::Value::Array(Vec::new()));
        let model_ref = serde_json::to_value(&row.model_ref).unwrap_or(serde_json::Value::Null);

        serde_json::json!({
            "id": row.model_ref.model_id,
            "model_ref": model_ref,
            "pumas_model_ref": model_ref,
            "repo_id": row.repo_id,
            "model_type": row.model_type,
            "cleaned_name": row.display_name,
            "pipeline_tag": row.pipeline_tag,
            "task_type_primary": row.task_type_primary,
            "recommended_backend": row.recommended_backend,
            "runtime_engine_hints": runtime_engine_hints,
            "entry_path": row.executable_entry_path(),
            "indexed_path": row.indexed_path,
            "selected_artifact_id": row.selected_artifact_id,
            "selected_artifact_path": row.selected_artifact_path,
            "entry_path_state": row.entry_path_state,
            "artifact_state": row.artifact_state,
            "selector_detail_state": row.detail_state,
            "storage_kind": row.storage_kind,
            "validation_state": row.validation_state,
            "requires_custom_code": row
                .package_facts_summary
                .as_ref()
                .map(|summary| serde_json::Value::Bool(summary.requires_custom_code))
                .unwrap_or(serde_json::Value::Bool(false)),
            "custom_code_sources": serde_json::Value::Array(Vec::new()),
            "dependency_bindings": serde_json::Value::Array(Vec::new()),
            "review_reasons": row
                .package_facts_summary
                .as_ref()
                .map(|summary| {
                    serde_json::to_value(&summary.diagnostic_codes)
                        .unwrap_or(serde_json::Value::Array(Vec::new()))
                })
                .unwrap_or(serde_json::Value::Array(Vec::new())),
            "inference_settings": serde_json::Value::Array(Vec::new()),
            "package_facts_summary_cursor": cursor,
            "package_facts_summary_status": row.package_facts_summary_status,
            "package_facts_summary": package_facts_summary,
            "selector_snapshot_contract_version": 1,
            "selector_row_executable": row.is_executable_reference_ready(),
        })
    }

    pub(crate) fn port_option_from_selector_row(
        row: &ModelLibrarySelectorSnapshotRow,
        cursor: &str,
    ) -> PortOption {
        PortOption {
            value: selector_row_value(row),
            label: row.display_name.clone(),
            description: Some(selector_row_description(row)),
            metadata: Some(selector_row_option_metadata(row, cursor)),
        }
    }

    #[async_trait]
    impl PortOptionsProvider for PumaLibOptionsProvider {
        async fn query_options(
            &self,
            query: &PortOptionsQuery,
            extensions: &ExecutorExtensions,
        ) -> node_engine::Result<PortOptionsResult> {
            let selector_access = extensions
                .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
                .cloned()
                .or_else(|| {
                    extensions
                        .get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API)
                        .cloned()
                        .map(|api| Arc::new(PumasSelectorAccess::Owner(api)))
                })
                .ok_or_else(|| {
                    NodeEngineError::ExecutionFailed(
                        "Pumas model selector access not available".to_string(),
                    )
                })?;

            let snapshot = selector_access
                .model_library_selector_snapshot(selector_snapshot_request(query))
                .await
                .map_err(|e| NodeEngineError::ExecutionFailed(e.to_string()))?;
            let cursor = snapshot.cursor.clone();
            let total = snapshot
                .total_count
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or(snapshot.rows.len());
            let options = snapshot
                .rows
                .iter()
                .map(|row| port_option_from_selector_row(row, &cursor))
                .collect();

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
        custom_code_sources_for_option_metadata, dependency_bindings_for_option_metadata,
        load_package_facts_summary_cache, port_option_from_selector_row,
        requires_custom_code_from_summary, resolve_execution_descriptor,
        resolve_inference_settings_fallback, review_reasons_for_option_metadata,
        runtime_engine_hints_from_summary, task_type_primary_from_descriptor_or_record,
        PackageFactsSummaryCache, PumaLibOptionsProvider,
    };
    use crate::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};
    use node_engine::{extension_keys, ExecutorExtensions, PortOptionsProvider, PortOptionsQuery};
    use pumas_library::models::{
        ModelArtifactState, ModelEntryPathState, ModelExecutionDescriptor, ModelFactFamily,
        ModelLibraryChangeKind, ModelLibraryRefreshScope, ModelLibrarySelectorSnapshotRow,
        ModelLibraryUpdateEvent, ModelLibraryUpdateFeed, ModelPackageFactsSummaryResult,
        ModelPackageFactsSummaryStatus,
    };
    use pumas_library::{ModelIndex, ModelRecord, PumasApi, PumasReadOnlyLibrary};
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

    fn sparse_package_summary_result(
        model_id: &str,
        status: ModelPackageFactsSummaryStatus,
    ) -> ModelPackageFactsSummaryResult {
        ModelPackageFactsSummaryResult {
            model_id: model_id.to_string(),
            status,
            summary: None,
        }
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

    fn model_execution_descriptor_with_dependency_resolution() -> ModelExecutionDescriptor {
        serde_json::from_value(serde_json::json!({
            "execution_contract_version": 1,
            "model_id": "llm/imported/test-model",
            "entry_path": "/models/test-model/model.safetensors",
            "model_type": "llm",
            "task_type_primary": "text-generation",
            "recommended_backend": "pytorch",
            "runtime_engine_hints": ["transformers", "pytorch"],
            "storage_kind": "library_owned",
            "validation_state": "valid",
            "dependency_resolution": {
                "dependency_contract_version": 1,
                "bindings": [
                    {
                        "binding_id": "binding-public",
                        "profile_id": "profile-public",
                        "profile_version": 1,
                        "backend_key": "pytorch",
                        "validation_state": "valid",
                        "validation_errors": [],
                        "requirements": []
                    }
                ]
            }
        }))
        .expect("execution descriptor fixture should decode")
    }

    fn selector_snapshot_row(
        model_id: &str,
        entry_path_state: ModelEntryPathState,
        artifact_state: ModelArtifactState,
    ) -> ModelLibrarySelectorSnapshotRow {
        serde_json::from_value(serde_json::json!({
            "model_id": model_id,
            "model_ref": {
                "model_ref_contract_version": 1,
                "model_id": model_id,
                "selected_artifact_id": "model.gguf",
                "selected_artifact_path": format!("{model_id}/model.gguf")
            },
            "selected_artifact_id": "model.gguf",
            "selected_artifact_path": format!("{model_id}/model.gguf"),
            "entry_path": format!("/models/{model_id}/model.gguf"),
            "entry_path_state": entry_path_state,
            "artifact_state": artifact_state,
            "display_name": "Selector Model",
            "model_type": "llm",
            "tags": ["gguf"],
            "indexed_path": format!("indexed/{model_id}"),
            "task_type_primary": "text-generation",
            "pipeline_tag": "text-generation",
            "recommended_backend": "llama.cpp",
            "runtime_engine_hints": ["llama.cpp"],
            "package_facts_summary_status": "missing",
            "detail_state": "needs_package_facts"
        }))
        .expect("selector row fixture should decode")
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
                "entry_path": model_file.display().to_string(),
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
        let summary = package_summary_result("llm/imported/test-model", "cached");
        let descriptor = model_execution_descriptor_with_task("image-to-text");

        let task_type =
            task_type_primary_from_descriptor_or_record(Some(&descriptor), Some(&summary), &record);

        assert_eq!(task_type, "image-to-text");
    }

    #[test]
    fn test_task_type_primary_uses_summary_when_execution_descriptor_task_is_unknown() {
        let record = model_record_with_metadata(serde_json::json!({
            "task_type_primary": "stale-metadata-task",
            "pipeline_tag": "image-to-text"
        }));
        let summary = package_summary_result("llm/imported/test-model", "cached");
        let descriptor = model_execution_descriptor_with_task("unknown");

        let task_type =
            task_type_primary_from_descriptor_or_record(Some(&descriptor), Some(&summary), &record);

        assert_eq!(task_type, "text-generation");
    }

    #[test]
    fn test_task_type_primary_uses_record_type_default_when_versioned_facts_absent() {
        let record = model_record_with_metadata(serde_json::json!({
            "task_type_primary": "stale-metadata-task",
            "pipeline_tag": "image-to-text"
        }));
        let descriptor = model_execution_descriptor_with_task("unknown");

        let task_type =
            task_type_primary_from_descriptor_or_record(Some(&descriptor), None, &record);

        assert_eq!(task_type, "text-generation");
    }

    #[test]
    fn test_option_metadata_uses_package_summary_backend_hints() {
        let summary = package_summary_result("llm/imported/test-model", "cached");

        let hints = runtime_engine_hints_from_summary(Some(&summary));

        assert_eq!(hints, Some(serde_json::json!(["llama.cpp"])));
    }

    #[test]
    fn test_option_metadata_uses_package_summary_custom_code_flag() {
        let summary = package_summary_result("llm/imported/test-model", "cached");

        let requires_custom_code = requires_custom_code_from_summary(Some(&summary));

        assert_eq!(requires_custom_code, Some(serde_json::json!(false)));
    }

    #[test]
    fn test_option_metadata_uses_package_summary_diagnostics_as_review_reasons() {
        let mut summary = package_summary_result("llm/imported/test-model", "cached");
        summary.summary.as_mut().unwrap().diagnostic_codes = vec![
            "missing_tokenizer".to_string(),
            "custom_code_required".to_string(),
        ];
        let record = model_record_with_metadata(serde_json::json!({
            "review_reasons": ["stale-record-review"]
        }));

        let review_reasons = review_reasons_for_option_metadata(Some(&summary), &record);

        assert_eq!(
            review_reasons,
            serde_json::json!(["missing_tokenizer", "custom_code_required"])
        );
    }

    #[test]
    fn test_option_metadata_omits_raw_custom_code_sources_when_summary_exists() {
        let summary = package_summary_result("llm/imported/test-model", "cached");
        let record = model_record_with_metadata(serde_json::json!({
            "custom_code_sources": ["metadata.json:trust_remote_code"]
        }));

        let custom_code_sources = custom_code_sources_for_option_metadata(Some(&summary), &record);

        assert_eq!(custom_code_sources, serde_json::json!([]));
    }

    #[test]
    fn test_option_metadata_does_not_fall_back_to_record_metadata_for_sparse_summary_result() {
        let summary = sparse_package_summary_result(
            "llm/imported/test-model",
            ModelPackageFactsSummaryStatus::Missing,
        );
        let record = model_record_with_metadata(serde_json::json!({
            "runtime_engine_hints": ["stale-metadata-engine"],
            "requires_custom_code": true,
            "custom_code_sources": ["metadata.json:trust_remote_code"],
            "review_reasons": ["stale-record-review"]
        }));

        assert_eq!(
            runtime_engine_hints_from_summary(Some(&summary)),
            Some(serde_json::json!([]))
        );
        assert_eq!(
            requires_custom_code_from_summary(Some(&summary)),
            Some(serde_json::json!(false))
        );
        assert_eq!(
            custom_code_sources_for_option_metadata(Some(&summary), &record),
            serde_json::json!([])
        );
        assert_eq!(
            review_reasons_for_option_metadata(Some(&summary), &record),
            serde_json::json!([])
        );
    }

    #[test]
    fn test_option_metadata_uses_execution_descriptor_dependency_bindings() {
        let descriptor = model_execution_descriptor_with_dependency_resolution();
        let record = model_record_with_metadata(serde_json::json!({
            "dependency_bindings": [{"binding_id": "stale-record-binding"}]
        }));

        let dependency_bindings =
            dependency_bindings_for_option_metadata(Some(&descriptor), &record);

        assert_eq!(
            dependency_bindings,
            serde_json::json!([{
                "binding_id": "binding-public",
                "profile_id": "profile-public",
                "profile_version": 1,
                "backend_key": "pytorch",
                "validation_state": "valid",
                "validation_errors": [],
                "requirements": []
            }])
        );
    }

    #[test]
    fn test_selector_row_option_uses_entry_path_only_when_ready() {
        let ready = selector_snapshot_row(
            "llm/imported/ready",
            ModelEntryPathState::Ready,
            ModelArtifactState::Ready,
        );
        let ready_option = port_option_from_selector_row(&ready, "model-library-updates:1");
        assert_eq!(
            ready_option.value,
            serde_json::json!("/models/llm/imported/ready/model.gguf")
        );
        let ready_metadata = ready_option
            .metadata
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .expect("selector option metadata should be an object");
        assert_eq!(
            ready_metadata["id"],
            serde_json::json!("llm/imported/ready")
        );
        assert_eq!(
            ready_metadata["indexed_path"],
            serde_json::json!("indexed/llm/imported/ready")
        );
        assert_eq!(
            ready_metadata["selector_row_executable"],
            serde_json::json!(true)
        );

        let partial = selector_snapshot_row(
            "llm/imported/partial",
            ModelEntryPathState::Partial,
            ModelArtifactState::Ready,
        );
        let partial_option = port_option_from_selector_row(&partial, "model-library-updates:1");
        assert_eq!(
            partial_option.value,
            serde_json::json!("llm/imported/partial")
        );
        let partial_metadata = partial_option
            .metadata
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .expect("selector option metadata should be an object");
        assert!(partial_metadata["entry_path"].is_null());
        assert_eq!(
            partial_metadata["selector_row_executable"],
            serde_json::json!(false)
        );
    }

    #[tokio::test]
    async fn test_model_options_use_selector_snapshot_without_detail_hydration() {
        let temp_dir = create_test_env();
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models/llm/imported/test-gguf");
        let model_file = write_library_owned_file_model(&model_dir, "model.gguf", 256);

        let api = Arc::new(PumasApi::builder(temp_dir.path()).build().await.unwrap());
        api.rebuild_model_index().await.unwrap();

        let mut extensions = ExecutorExtensions::new();
        extensions.set(extension_keys::PUMAS_API, api);
        let provider = PumaLibOptionsProvider;
        let result = provider
            .query_options(
                &PortOptionsQuery {
                    limit: Some(25),
                    ..PortOptionsQuery::default()
                },
                &extensions,
            )
            .await
            .expect("selector options should load");

        assert_eq!(result.options.len(), 1);
        let option = &result.options[0];
        assert_eq!(
            option.value,
            serde_json::json!(model_file.display().to_string())
        );
        let metadata = option
            .metadata
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .expect("selector option metadata should be an object");
        assert_eq!(metadata["id"], serde_json::json!("llm/imported/test-gguf"));
        assert_eq!(metadata["selector_row_executable"], serde_json::json!(true));
        assert_eq!(metadata["inference_settings"], serde_json::json!([]));
        assert!(metadata.get("execution_contract_version").is_none());
    }

    #[tokio::test]
    async fn test_model_options_use_read_only_selector_snapshot_without_pumas_api() {
        let temp_dir = TempDir::new().unwrap();
        let model_root = temp_dir.path();
        let writer = ModelIndex::new(model_root.join("models.db")).unwrap();
        writer
            .upsert(&ModelRecord {
                id: "llm/imported/read-only".to_string(),
                path: "llm/imported/read-only".to_string(),
                cleaned_name: "read-only".to_string(),
                official_name: "read-only".to_string(),
                model_type: "llm".to_string(),
                tags: vec!["gguf".to_string()],
                hashes: HashMap::new(),
                metadata: serde_json::json!({
                    "entry_path": "/models/read-only/model.gguf",
                    "validation_state": "valid",
                    "task_type_primary": "text-generation",
                    "recommended_backend": "llama.cpp",
                    "runtime_engine_hints": ["llama.cpp"]
                }),
                updated_at: "2026-05-06T00:00:00Z".to_string(),
            })
            .unwrap();
        drop(writer);

        let read_only = PumasReadOnlyLibrary::open(model_root).unwrap();
        let mut extensions = ExecutorExtensions::new();
        extensions.set(
            PUMAS_SELECTOR_ACCESS,
            Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
        );

        let provider = PumaLibOptionsProvider;
        let result = provider
            .query_options(
                &PortOptionsQuery {
                    limit: Some(25),
                    ..PortOptionsQuery::default()
                },
                &extensions,
            )
            .await
            .expect("read-only selector options should load");

        assert_eq!(result.options.len(), 1);
        let option = &result.options[0];
        assert_eq!(
            option.value,
            serde_json::json!("/models/read-only/model.gguf")
        );
        let metadata = option
            .metadata
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .expect("selector option metadata should be an object");
        assert_eq!(metadata["id"], serde_json::json!("llm/imported/read-only"));
        assert_eq!(metadata["selector_row_executable"], serde_json::json!(true));
        assert_eq!(metadata["inference_settings"], serde_json::json!([]));
    }

    #[test]
    fn test_inference_settings_fallback_ignores_record_metadata() {
        let record = model_record_with_metadata(serde_json::json!({
            "inference_settings": [{
                "key": "stale_metadata_setting",
                "label": "Stale Metadata Setting",
                "param_type": "number",
                "default": 1
            }],
            "files": [{"name": "model.gguf"}],
            "subtype": "dllm"
        }));

        let settings = resolve_inference_settings_fallback(&record);

        assert!(settings
            .as_array()
            .expect("fallback settings should be an array")
            .iter()
            .all(|setting| setting.get("key").and_then(|key| key.as_str())
                != Some("stale_metadata_setting")));
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
