//! Merge Task
//!
//! Combines multiple string inputs into a single output.
//! This task is useful for aggregating results from parallel branches
//! or combining context from multiple sources.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Configuration for the merge task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConfig {
    /// Separator to use when joining inputs
    pub separator: String,
    /// Whether to filter out empty inputs
    pub filter_empty: bool,
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self {
            separator: "\n".to_string(),
            filter_empty: true,
        }
    }
}

/// Merge Task
///
/// Combines multiple string inputs into a single merged output.
/// The inputs are joined using a configurable separator (default: newline).
///
/// # Inputs (from context)
/// - `{task_id}.input.inputs` (multiple) - String inputs to merge
///
/// # Outputs (to context)
/// - `{task_id}.output.merged` - Combined string output
/// - `{task_id}.output.count` - Number of inputs merged
#[derive(Clone)]
pub struct MergeTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration
    config: Option<MergeConfig>,
}

impl MergeTask {
    /// Port ID for inputs (accepts multiple connections)
    pub const PORT_INPUTS: &'static str = "inputs";
    /// Port ID for merged output
    pub const PORT_MERGED: &'static str = "merged";
    /// Port ID for count output
    pub const PORT_COUNT: &'static str = "count";

    /// Create a new merge task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: MergeConfig) -> Self {
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

impl TaskDescriptor for MergeTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "merge".to_string(),
            category: NodeCategory::Control,
            label: "Merge".to_string(),
            description: "Combines multiple string inputs into one".to_string(),
            inputs: vec![
                PortMetadata::optional(Self::PORT_INPUTS, "Inputs", PortDataType::String).multiple(),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_MERGED, "Merged", PortDataType::String),
                PortMetadata::optional(Self::PORT_COUNT, "Count", PortDataType::Number),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(MergeTask::descriptor));

#[async_trait]
impl Task for MergeTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get configuration
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<MergeConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        // Get inputs - the input port accepts multiple connections
        // which may be stored as an array or as individual keyed values
        let inputs_key = ContextKeys::input(&self.task_id, Self::PORT_INPUTS);

        // Try to get as array first
        let inputs: Vec<String> = if let Some(arr) = context.get::<Vec<String>>(&inputs_key).await {
            arr
        } else if let Some(single) = context.get::<String>(&inputs_key).await {
            // Single input case
            vec![single]
        } else {
            // No inputs - return empty
            vec![]
        };

        log::debug!(
            "MergeTask {}: merging {} inputs",
            self.task_id,
            inputs.len()
        );

        // Filter and merge
        let filtered: Vec<&str> = if config.filter_empty {
            inputs.iter().map(|s| s.as_str()).filter(|s| !s.trim().is_empty()).collect()
        } else {
            inputs.iter().map(|s| s.as_str()).collect()
        };

        let merged = filtered.join(&config.separator);
        let count = filtered.len();

        // Store outputs in context
        let merged_key = ContextKeys::output(&self.task_id, Self::PORT_MERGED);
        context.set(&merged_key, merged.clone()).await;

        let count_key = ContextKeys::output(&self.task_id, Self::PORT_COUNT);
        context.set(&count_key, count as f64).await;

        log::debug!(
            "MergeTask {}: merged {} inputs into {} chars",
            self.task_id,
            count,
            merged.len()
        );

        Ok(TaskResult::new(Some(merged), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = MergeTask::new("my_merge");
        assert_eq!(task.id(), "my_merge");
    }

    #[test]
    fn test_with_config() {
        let config = MergeConfig {
            separator: " | ".to_string(),
            filter_empty: false,
        };
        let task = MergeTask::with_config("task1", config);
        assert_eq!(task.config.as_ref().unwrap().separator, " | ");
        assert!(!task.config.as_ref().unwrap().filter_empty);
    }

    #[test]
    fn test_default_config() {
        let config = MergeConfig::default();
        assert_eq!(config.separator, "\n");
        assert!(config.filter_empty);
    }

    #[test]
    fn test_descriptor() {
        let meta = MergeTask::descriptor();
        assert_eq!(meta.node_type, "merge");
        assert_eq!(meta.category, NodeCategory::Control);
        assert_eq!(meta.inputs.len(), 1);
        assert!(meta.inputs[0].multiple); // Should accept multiple connections
        assert_eq!(meta.outputs.len(), 2);
    }

    #[tokio::test]
    async fn test_merge_multiple_inputs() {
        let task = MergeTask::new("test_merge");
        let context = Context::new();

        // Set inputs as array
        let inputs_key = ContextKeys::input("test_merge", "inputs");
        context.set(&inputs_key, vec!["First".to_string(), "Second".to_string(), "Third".to_string()]).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify merged output
        let merged_key = ContextKeys::output("test_merge", "merged");
        let merged: Option<String> = context.get(&merged_key).await;
        assert_eq!(merged, Some("First\nSecond\nThird".to_string()));

        // Verify count
        let count_key = ContextKeys::output("test_merge", "count");
        let count: Option<f64> = context.get(&count_key).await;
        assert_eq!(count, Some(3.0));
    }

    #[tokio::test]
    async fn test_merge_filters_empty() {
        let task = MergeTask::new("test_merge");
        let context = Context::new();

        // Set inputs with empty strings
        let inputs_key = ContextKeys::input("test_merge", "inputs");
        context.set(&inputs_key, vec!["First".to_string(), "".to_string(), "Third".to_string()]).await;

        // Run task
        task.run(context.clone()).await.unwrap();

        // Verify merged output (empty filtered out)
        let merged_key = ContextKeys::output("test_merge", "merged");
        let merged: Option<String> = context.get(&merged_key).await;
        assert_eq!(merged, Some("First\nThird".to_string()));

        // Count should be 2 (empty filtered)
        let count_key = ContextKeys::output("test_merge", "count");
        let count: Option<f64> = context.get(&count_key).await;
        assert_eq!(count, Some(2.0));
    }

    #[tokio::test]
    async fn test_merge_single_input() {
        let task = MergeTask::new("test_merge");
        let context = Context::new();

        // Set single input
        let inputs_key = ContextKeys::input("test_merge", "inputs");
        context.set(&inputs_key, "Only one".to_string()).await;

        // Run task
        task.run(context.clone()).await.unwrap();

        // Verify output
        let merged_key = ContextKeys::output("test_merge", "merged");
        let merged: Option<String> = context.get(&merged_key).await;
        assert_eq!(merged, Some("Only one".to_string()));
    }

    #[tokio::test]
    async fn test_merge_no_inputs() {
        let task = MergeTask::new("test_merge");
        let context = Context::new();

        // Don't set any inputs
        // Run task
        task.run(context.clone()).await.unwrap();

        // Verify empty output
        let merged_key = ContextKeys::output("test_merge", "merged");
        let merged: Option<String> = context.get(&merged_key).await;
        assert_eq!(merged, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_merge_with_custom_separator() {
        let config = MergeConfig {
            separator: " | ".to_string(),
            filter_empty: true,
        };
        let task = MergeTask::with_config("test_merge", config);
        let context = Context::new();

        // Set inputs
        let inputs_key = ContextKeys::input("test_merge", "inputs");
        context.set(&inputs_key, vec!["A".to_string(), "B".to_string(), "C".to_string()]).await;

        // Run task
        task.run(context.clone()).await.unwrap();

        // Verify output with custom separator
        let merged_key = ContextKeys::output("test_merge", "merged");
        let merged: Option<String> = context.get(&merged_key).await;
        assert_eq!(merged, Some("A | B | C".to_string()));
    }
}
