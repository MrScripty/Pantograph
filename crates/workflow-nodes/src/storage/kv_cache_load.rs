//! KV Cache Load Task
//!
//! Loads KV cache data from storage, validating against the model fingerprint.
//!
//! # Inputs (from context)
//! - `{task_id}.input.cache_id` - Which cache to load
//! - `{task_id}.input.model_fingerprint` - For model validation
//!
//! # Outputs (to context)
//! - `{task_id}.output.cache_data` - Opaque cache bytes (JSON)
//! - `{task_id}.output.metadata` - Full KvCacheMetadata (JSON)
//! - `{task_id}.output.valid` - Whether load succeeded (Boolean)

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// KV Cache Load Task
///
/// Retrieves KV cache data from storage by cache ID. Validates the
/// model fingerprint to ensure cache compatibility before returning data.
#[derive(Clone)]
pub struct KvCacheLoadTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl KvCacheLoadTask {
    // Input port IDs
    /// Port ID for cache ID input
    pub const PORT_CACHE_ID: &'static str = "cache_id";
    /// Port ID for model fingerprint input
    pub const PORT_MODEL_FINGERPRINT: &'static str = "model_fingerprint";

    // Output port IDs
    /// Port ID for cache data output
    pub const PORT_CACHE_DATA: &'static str = "cache_data";
    /// Port ID for metadata output
    pub const PORT_METADATA: &'static str = "metadata";
    /// Port ID for validity flag output
    pub const PORT_VALID: &'static str = "valid";

    /// Create a new KV Cache Load task
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

impl TaskDescriptor for KvCacheLoadTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "kv-cache-load".to_string(),
            category: NodeCategory::Tool,
            label: "KV Cache Load".to_string(),
            description: "Load KV cache from storage".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::required(
                    Self::PORT_MODEL_FINGERPRINT,
                    "Model Fingerprint",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::required(Self::PORT_CACHE_DATA, "Cache Data", PortDataType::Json),
                PortMetadata::required(Self::PORT_METADATA, "Metadata", PortDataType::Json),
                PortMetadata::required(Self::PORT_VALID, "Valid", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(KvCacheLoadTask::descriptor));

#[async_trait]
impl Task for KvCacheLoadTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Read required inputs
        let cache_id_key = ContextKeys::input(&self.task_id, Self::PORT_CACHE_ID);
        let cache_id: Option<String> = context.get(&cache_id_key).await;

        let fingerprint_key = ContextKeys::input(&self.task_id, Self::PORT_MODEL_FINGERPRINT);
        let model_fingerprint: Option<serde_json::Value> = context.get(&fingerprint_key).await;

        let cache_id_str = cache_id.unwrap_or_default();

        log::info!(
            "KvCacheLoadTask {}: loading cache '{}', fingerprint present={}",
            self.task_id,
            cache_id_str,
            model_fingerprint.is_some(),
        );

        // TODO: Integrate with actual KV cache store
        // This would:
        // 1. Look up the cache entry by cache_id
        // 2. Validate the model fingerprint matches the stored fingerprint
        // 3. Deserialize and return the cache data
        // 4. Set valid=true if everything succeeded

        // Stub: no actual store yet, so mark as invalid
        let valid = false;

        log::debug!(
            "KvCacheLoadTask {}: cache load not implemented - returning valid=false",
            self.task_id,
        );

        // Store outputs in context
        let cache_data_key = ContextKeys::output(&self.task_id, Self::PORT_CACHE_DATA);
        context
            .set(&cache_data_key, serde_json::Value::Null)
            .await;

        let metadata_key = ContextKeys::output(&self.task_id, Self::PORT_METADATA);
        context
            .set(
                &metadata_key,
                serde_json::json!({
                    "cache_id": cache_id_str,
                    "valid": valid,
                }),
            )
            .await;

        let valid_key = ContextKeys::output(&self.task_id, Self::PORT_VALID);
        context.set(&valid_key, valid).await;

        Ok(TaskResult::new(
            Some(format!(
                "KV cache load '{}': valid={}",
                cache_id_str, valid
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
        let task = KvCacheLoadTask::new("load_1");
        assert_eq!(task.id(), "load_1");
    }

    #[test]
    fn test_descriptor_ports() {
        let meta = KvCacheLoadTask::descriptor();
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.outputs.len(), 3);

        // Verify input port IDs
        let input_ids: Vec<&str> = meta.inputs.iter().map(|p| p.id.as_str()).collect();
        assert!(input_ids.contains(&"cache_id"));
        assert!(input_ids.contains(&"model_fingerprint"));

        // Verify output port IDs
        let output_ids: Vec<&str> = meta.outputs.iter().map(|p| p.id.as_str()).collect();
        assert!(output_ids.contains(&"cache_data"));
        assert!(output_ids.contains(&"metadata"));
        assert!(output_ids.contains(&"valid"));
    }

    #[test]
    fn test_descriptor_category() {
        let meta = KvCacheLoadTask::descriptor();
        assert_eq!(meta.category, NodeCategory::Tool);
    }

    #[tokio::test]
    async fn test_run_returns_invalid() {
        let task = KvCacheLoadTask::new("test_load");
        let context = Context::new();

        // Set required inputs
        let cache_id_key = ContextKeys::input("test_load", "cache_id");
        context
            .set(&cache_id_key, "some-cache-id".to_string())
            .await;

        let fingerprint_key = ContextKeys::input("test_load", "model_fingerprint");
        context
            .set(
                &fingerprint_key,
                serde_json::json!({"model": "test-model", "hash": "abc123"}),
            )
            .await;

        // Run the task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify valid output is false (stub behavior)
        let valid_key = ContextKeys::output("test_load", "valid");
        let valid: Option<bool> = context.get(&valid_key).await;
        assert_eq!(valid, Some(false));
    }
}
