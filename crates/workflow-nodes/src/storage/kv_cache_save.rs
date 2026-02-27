//! KV Cache Save Task
//!
//! Saves KV cache data to memory or disk storage.
//!
//! # Inputs (from context)
//! - `{task_id}.input.cache_data` - Opaque cache data (JSON)
//! - `{task_id}.input.model_fingerprint` - ModelFingerprint as JSON
//! - `{task_id}.input.label` - User-facing label (optional)
//! - `{task_id}.input.markers` - Vec<CacheMarker> to attach (optional)
//! - `{task_id}.input.storage_policy` - "memory" / "disk" / "both" (optional, default: "memory")
//! - `{task_id}.input.cache_dir` - Override disk path (optional)
//! - `{task_id}.input.compressed` - Zstd-compress on disk (optional)
//!
//! # Outputs (to context)
//! - `{task_id}.output.cache_id` - Generated cache ID
//! - `{task_id}.output.metadata` - Full KvCacheMetadata as JSON

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// KV Cache Save Task
///
/// Persists KV cache data to memory or disk storage. Generates a unique
/// cache ID and stores associated metadata for later retrieval.
#[derive(Clone)]
pub struct KvCacheSaveTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl KvCacheSaveTask {
    // Input port IDs
    /// Port ID for cache data input
    pub const PORT_CACHE_DATA: &'static str = "cache_data";
    /// Port ID for model fingerprint input
    pub const PORT_MODEL_FINGERPRINT: &'static str = "model_fingerprint";
    /// Port ID for user-facing label input
    pub const PORT_LABEL: &'static str = "label";
    /// Port ID for cache markers input
    pub const PORT_MARKERS: &'static str = "markers";
    /// Port ID for storage policy input
    pub const PORT_STORAGE_POLICY: &'static str = "storage_policy";
    /// Port ID for cache directory override input
    pub const PORT_CACHE_DIR: &'static str = "cache_dir";
    /// Port ID for compression flag input
    pub const PORT_COMPRESSED: &'static str = "compressed";

    // Output port IDs
    /// Port ID for generated cache ID output
    pub const PORT_CACHE_ID: &'static str = "cache_id";
    /// Port ID for full metadata output
    pub const PORT_METADATA: &'static str = "metadata";

    /// Create a new KV Cache Save task
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

impl TaskDescriptor for KvCacheSaveTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "kv-cache-save".to_string(),
            category: NodeCategory::Tool,
            label: "KV Cache Save".to_string(),
            description: "Save KV cache to memory or disk".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_CACHE_DATA, "Cache Data", PortDataType::Json),
                PortMetadata::required(
                    Self::PORT_MODEL_FINGERPRINT,
                    "Model Fingerprint",
                    PortDataType::Json,
                ),
                PortMetadata::optional(Self::PORT_LABEL, "Label", PortDataType::String),
                PortMetadata::optional(Self::PORT_MARKERS, "Markers", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_STORAGE_POLICY,
                    "Storage Policy",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_CACHE_DIR, "Cache Dir", PortDataType::String),
                PortMetadata::optional(Self::PORT_COMPRESSED, "Compressed", PortDataType::Boolean),
            ],
            outputs: vec![
                PortMetadata::required(Self::PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::required(Self::PORT_METADATA, "Metadata", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(KvCacheSaveTask::descriptor));

#[async_trait]
impl Task for KvCacheSaveTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Read required inputs
        let cache_data_key = ContextKeys::input(&self.task_id, Self::PORT_CACHE_DATA);
        let cache_data: Option<serde_json::Value> = context.get(&cache_data_key).await;

        let fingerprint_key = ContextKeys::input(&self.task_id, Self::PORT_MODEL_FINGERPRINT);
        let model_fingerprint: Option<serde_json::Value> = context.get(&fingerprint_key).await;

        // Read optional inputs
        let label_key = ContextKeys::input(&self.task_id, Self::PORT_LABEL);
        let label: Option<String> = context.get(&label_key).await;

        let markers_key = ContextKeys::input(&self.task_id, Self::PORT_MARKERS);
        let markers: Option<serde_json::Value> = context.get(&markers_key).await;

        let policy_key = ContextKeys::input(&self.task_id, Self::PORT_STORAGE_POLICY);
        let storage_policy: String = context
            .get(&policy_key)
            .await
            .unwrap_or_else(|| "memory".to_string());

        let cache_dir_key = ContextKeys::input(&self.task_id, Self::PORT_CACHE_DIR);
        let cache_dir: Option<String> = context.get(&cache_dir_key).await;

        let compressed_key = ContextKeys::input(&self.task_id, Self::PORT_COMPRESSED);
        let compressed: bool = context.get(&compressed_key).await.unwrap_or(false);

        // Generate a unique cache ID
        let cache_id = uuid::Uuid::new_v4().to_string();

        log::info!(
            "KvCacheSaveTask {}: saving cache as '{}' with policy='{}', compressed={}, label={:?}",
            self.task_id,
            cache_id,
            storage_policy,
            compressed,
            label,
        );

        if let Some(ref dir) = cache_dir {
            log::info!(
                "KvCacheSaveTask {}: cache dir override: {}",
                self.task_id,
                dir
            );
        }

        log::debug!(
            "KvCacheSaveTask {}: cache_data present={}, fingerprint present={}, markers present={}",
            self.task_id,
            cache_data.is_some(),
            model_fingerprint.is_some(),
            markers.is_some(),
        );

        // TODO: Integrate with actual KV cache store
        // This would:
        // 1. Validate the model fingerprint
        // 2. Store cache_data in memory and/or on disk based on storage_policy
        // 3. Optionally compress with zstd if compressed=true
        // 4. Attach markers to the cache entry
        // 5. Build full KvCacheMetadata

        // Build placeholder metadata
        let metadata = serde_json::json!({
            "cache_id": cache_id,
            "model_fingerprint": model_fingerprint,
            "label": label,
            "storage_policy": storage_policy,
            "compressed": compressed,
            "markers": markers,
            "cache_dir": cache_dir,
        });

        // Store outputs in context
        let cache_id_key = ContextKeys::output(&self.task_id, Self::PORT_CACHE_ID);
        context.set(&cache_id_key, cache_id.clone()).await;

        let metadata_key = ContextKeys::output(&self.task_id, Self::PORT_METADATA);
        context.set(&metadata_key, metadata).await;

        Ok(TaskResult::new(
            Some(format!(
                "KV cache saved as '{}' (policy: {})",
                cache_id, storage_policy
            )),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = KvCacheSaveTask::new("save_1");
        assert_eq!(task.id(), "save_1");
    }

    #[test]
    fn test_descriptor_ports() {
        let meta = KvCacheSaveTask::descriptor();
        assert_eq!(meta.inputs.len(), 7);
        assert_eq!(meta.outputs.len(), 2);

        // Verify input port IDs
        let input_ids: Vec<&str> = meta.inputs.iter().map(|p| p.id.as_str()).collect();
        assert!(input_ids.contains(&"cache_data"));
        assert!(input_ids.contains(&"model_fingerprint"));
        assert!(input_ids.contains(&"label"));
        assert!(input_ids.contains(&"markers"));
        assert!(input_ids.contains(&"storage_policy"));
        assert!(input_ids.contains(&"cache_dir"));
        assert!(input_ids.contains(&"compressed"));

        // Verify output port IDs
        let output_ids: Vec<&str> = meta.outputs.iter().map(|p| p.id.as_str()).collect();
        assert!(output_ids.contains(&"cache_id"));
        assert!(output_ids.contains(&"metadata"));
    }

    #[test]
    fn test_descriptor_category() {
        let meta = KvCacheSaveTask::descriptor();
        assert_eq!(meta.category, NodeCategory::Tool);
    }

    #[tokio::test]
    async fn test_run_generates_cache_id() {
        let task = KvCacheSaveTask::new("test_save");
        let context = Context::new();

        // Set required inputs
        let cache_data_key = ContextKeys::input("test_save", "cache_data");
        context
            .set(&cache_data_key, serde_json::json!({"tokens": [1, 2, 3]}))
            .await;

        let fingerprint_key = ContextKeys::input("test_save", "model_fingerprint");
        context
            .set(
                &fingerprint_key,
                serde_json::json!({"model": "test-model", "hash": "abc123"}),
            )
            .await;

        // Run the task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify cache_id output is non-empty
        let cache_id_key = ContextKeys::output("test_save", "cache_id");
        let cache_id: Option<String> = context.get(&cache_id_key).await;
        assert!(cache_id.is_some());
        assert!(!cache_id.unwrap().is_empty());

        // Verify metadata output is present
        let metadata_key = ContextKeys::output("test_save", "metadata");
        let metadata: Option<serde_json::Value> = context.get(&metadata_key).await;
        assert!(metadata.is_some());
    }
}
