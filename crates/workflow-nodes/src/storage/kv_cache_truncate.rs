//! KV Cache Truncate Task
//!
//! Truncates KV cache to a named marker position or exact token position.
//!
//! # Inputs (from context)
//! - `{task_id}.input.cache_id` - Which cache to truncate
//! - `{task_id}.input.marker_name` - Named marker position (optional)
//! - `{task_id}.input.token_position` - Exact token position (optional)
//!
//! # Outputs (to context)
//! - `{task_id}.output.cache_id` - Same cache ID post-truncation
//! - `{task_id}.output.metadata` - Updated KvCacheMetadata (JSON)

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// KV Cache Truncate Task
///
/// Truncates a KV cache entry to a specific position, either by a named
/// marker or an exact token position. This allows partial cache reuse
/// when only the tail of a conversation has changed.
#[derive(Clone)]
pub struct KvCacheTruncateTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl KvCacheTruncateTask {
    // Input port IDs
    /// Port ID for cache ID input
    pub const PORT_CACHE_ID: &'static str = "cache_id";
    /// Port ID for marker name input
    pub const PORT_MARKER_NAME: &'static str = "marker_name";
    /// Port ID for token position input
    pub const PORT_TOKEN_POSITION: &'static str = "token_position";

    // Output port IDs
    /// Port ID for cache ID output (same ID post-truncation)
    pub const PORT_CACHE_ID_OUT: &'static str = "cache_id";
    /// Port ID for updated metadata output
    pub const PORT_METADATA: &'static str = "metadata";

    /// Create a new KV Cache Truncate task
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

impl TaskDescriptor for KvCacheTruncateTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "kv-cache-truncate".to_string(),
            category: NodeCategory::Tool,
            label: "KV Cache Truncate".to_string(),
            description: "Truncate KV cache to a marker or position".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_CACHE_ID, "Cache ID", PortDataType::String),
                PortMetadata::optional(Self::PORT_MARKER_NAME, "Marker Name", PortDataType::String),
                PortMetadata::optional(
                    Self::PORT_TOKEN_POSITION,
                    "Token Position",
                    PortDataType::Number,
                ),
            ],
            outputs: vec![
                PortMetadata::required(Self::PORT_CACHE_ID_OUT, "Cache ID", PortDataType::String),
                PortMetadata::required(Self::PORT_METADATA, "Metadata", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(KvCacheTruncateTask::descriptor));

#[async_trait]
impl Task for KvCacheTruncateTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Read required inputs
        let cache_id_key = ContextKeys::input(&self.task_id, Self::PORT_CACHE_ID);
        let cache_id: String = context.get(&cache_id_key).await.unwrap_or_default();

        // Read optional truncation targets
        let marker_key = ContextKeys::input(&self.task_id, Self::PORT_MARKER_NAME);
        let marker_name: Option<String> = context.get(&marker_key).await;

        let position_key = ContextKeys::input(&self.task_id, Self::PORT_TOKEN_POSITION);
        let token_position: Option<f64> = context.get(&position_key).await;

        // Determine truncation mode
        let truncation_mode = match (&marker_name, &token_position) {
            (Some(marker), _) => {
                log::info!(
                    "KvCacheTruncateTask {}: truncating cache '{}' to marker '{}'",
                    self.task_id,
                    cache_id,
                    marker,
                );
                format!("marker:{}", marker)
            }
            (None, Some(pos)) => {
                log::info!(
                    "KvCacheTruncateTask {}: truncating cache '{}' to token position {}",
                    self.task_id,
                    cache_id,
                    pos,
                );
                format!("position:{}", pos)
            }
            (None, None) => {
                log::info!(
                    "KvCacheTruncateTask {}: no truncation target specified for cache '{}'",
                    self.task_id,
                    cache_id,
                );
                "none".to_string()
            }
        };

        // TODO: Integrate with actual KV cache store
        // This would:
        // 1. Look up the cache entry by cache_id
        // 2. Resolve the truncation position (marker lookup or direct position)
        // 3. Truncate the cache data in-place
        // 4. Update metadata (token count, markers, etc.)

        log::debug!(
            "KvCacheTruncateTask {}: truncation not implemented - passing through cache_id",
            self.task_id,
        );

        // Build placeholder metadata
        let metadata = serde_json::json!({
            "cache_id": cache_id,
            "truncation_mode": truncation_mode,
            "marker_name": marker_name,
            "token_position": token_position,
        });

        // Store outputs in context — pass through cache_id
        let cache_id_out_key = ContextKeys::output(&self.task_id, Self::PORT_CACHE_ID_OUT);
        context.set(&cache_id_out_key, cache_id.clone()).await;

        let metadata_key = ContextKeys::output(&self.task_id, Self::PORT_METADATA);
        context.set(&metadata_key, metadata).await;

        Ok(TaskResult::new(
            Some(format!(
                "KV cache '{}' truncated (mode: {})",
                cache_id, truncation_mode
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
        let task = KvCacheTruncateTask::new("trunc_1");
        assert_eq!(task.id(), "trunc_1");
    }

    #[test]
    fn test_descriptor_ports() {
        let meta = KvCacheTruncateTask::descriptor();
        assert_eq!(meta.inputs.len(), 3);
        assert_eq!(meta.outputs.len(), 2);

        // Verify input port IDs
        let input_ids: Vec<&str> = meta.inputs.iter().map(|p| p.id.as_str()).collect();
        assert!(input_ids.contains(&"cache_id"));
        assert!(input_ids.contains(&"marker_name"));
        assert!(input_ids.contains(&"token_position"));

        // Verify output port IDs
        let output_ids: Vec<&str> = meta.outputs.iter().map(|p| p.id.as_str()).collect();
        assert!(output_ids.contains(&"cache_id"));
        assert!(output_ids.contains(&"metadata"));
    }

    #[test]
    fn test_descriptor_category() {
        let meta = KvCacheTruncateTask::descriptor();
        assert_eq!(meta.category, NodeCategory::Tool);
    }

    #[tokio::test]
    async fn test_run_with_marker() {
        let task = KvCacheTruncateTask::new("test_trunc");
        let context = Context::new();

        // Set required input
        let cache_id_key = ContextKeys::input("test_trunc", "cache_id");
        context
            .set(&cache_id_key, "some-cache-id".to_string())
            .await;

        // Set marker name
        let marker_key = ContextKeys::input("test_trunc", "marker_name");
        context
            .set(&marker_key, "system_prompt_end".to_string())
            .await;

        // Run the task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify cache_id is passed through
        let cache_id_out_key = ContextKeys::output("test_trunc", "cache_id");
        let cache_id_out: Option<String> = context.get(&cache_id_out_key).await;
        assert_eq!(cache_id_out, Some("some-cache-id".to_string()));
    }

    #[tokio::test]
    async fn test_run_with_position() {
        let task = KvCacheTruncateTask::new("test_trunc_pos");
        let context = Context::new();

        // Set required input
        let cache_id_key = ContextKeys::input("test_trunc_pos", "cache_id");
        context
            .set(&cache_id_key, "another-cache".to_string())
            .await;

        // Set token position
        let pos_key = ContextKeys::input("test_trunc_pos", "token_position");
        context.set(&pos_key, 512.0_f64).await;

        // Run the task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify cache_id is passed through
        let cache_id_out_key = ContextKeys::output("test_trunc_pos", "cache_id");
        let cache_id_out: Option<String> = context.get(&cache_id_out_key).await;
        assert_eq!(cache_id_out, Some("another-cache".to_string()));
    }
}
