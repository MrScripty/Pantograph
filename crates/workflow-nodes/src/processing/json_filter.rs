//! JSON Filter Task
//!
//! Extracts values from JSON data using path expressions.
//! Supports simple dot notation and array indexing.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Configuration for the JSON filter task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonFilterConfig {
    /// JSON path expression (e.g., "data.items[0].name" or "[0].arguments.content")
    pub path: String,
    /// Default value if path doesn't exist
    pub default_value: Option<serde_json::Value>,
}

impl Default for JsonFilterConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            default_value: None,
        }
    }
}

/// JSON Filter Task
///
/// Extracts a value from JSON input using a path expression.
/// The path supports dot notation for object access and bracket
/// notation for array indexing.
///
/// # Path Syntax Examples
/// - `"name"` - Get the "name" field
/// - `"data.items"` - Get nested field
/// - `"[0]"` - Get first array element
/// - `"items[0].name"` - Combined access
/// - `"[0].arguments.content"` - Array then object access
///
/// # Inputs (from context)
/// - `{task_id}.input.json` (required) - JSON data to filter
///
/// # Node Data
/// - `path` - JSON path expression (configured in node data)
///
/// # Outputs (to context)
/// - `{task_id}.output.value` - Extracted value
/// - `{task_id}.output.found` - Whether the path was found
#[derive(Clone)]
pub struct JsonFilterTask {
    /// Unique identifier for this task instance
    task_id: String,
    /// Configuration containing the path
    config: Option<JsonFilterConfig>,
}

impl JsonFilterTask {
    /// Port ID for json input
    pub const PORT_JSON: &'static str = "json";
    /// Port ID for value output
    pub const PORT_VALUE: &'static str = "value";
    /// Port ID for found output
    pub const PORT_FOUND: &'static str = "found";

    /// Create a new JSON filter task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: JsonFilterConfig) -> Self {
        Self {
            task_id: task_id.into(),
            config: Some(config),
        }
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Extract a value from JSON using a path expression.
    ///
    /// Supports:
    /// - Dot notation: `field.nested.value`
    /// - Array indexing: `[0]`, `items[1]`
    /// - Combined: `data.items[0].name`
    fn extract_path(json: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
        if path.is_empty() {
            return Some(json.clone());
        }

        let mut current = json;
        let mut remaining = path;

        while !remaining.is_empty() {
            // Handle array indexing at start: [0]
            if remaining.starts_with('[') {
                if let Some(end) = remaining.find(']') {
                    let index_str = &remaining[1..end];
                    if let Ok(index) = index_str.parse::<usize>() {
                        current = current.get(index)?;
                        remaining = &remaining[end + 1..];
                        // Skip leading dot after array index
                        if remaining.starts_with('.') {
                            remaining = &remaining[1..];
                        }
                        continue;
                    }
                }
                return None;
            }

            // Handle object field access
            let (field, rest) = if let Some(dot_pos) = remaining.find('.') {
                let bracket_pos = remaining.find('[').unwrap_or(remaining.len());
                if dot_pos < bracket_pos {
                    (&remaining[..dot_pos], &remaining[dot_pos + 1..])
                } else {
                    (&remaining[..bracket_pos], &remaining[bracket_pos..])
                }
            } else if let Some(bracket_pos) = remaining.find('[') {
                (&remaining[..bracket_pos], &remaining[bracket_pos..])
            } else {
                (remaining, "")
            };

            if !field.is_empty() {
                current = current.get(field)?;
            }
            remaining = rest;
        }

        Some(current.clone())
    }
}

impl TaskDescriptor for JsonFilterTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "json-filter".to_string(),
            category: NodeCategory::Processing,
            label: "JSON Filter".to_string(),
            description: "Extracts values from JSON using path expressions".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_JSON, "JSON", PortDataType::Json),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_VALUE, "Value", PortDataType::Any),
                PortMetadata::optional(Self::PORT_FOUND, "Found", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(JsonFilterTask::descriptor));

#[async_trait]
impl Task for JsonFilterTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: json
        let json_key = ContextKeys::input(&self.task_id, Self::PORT_JSON);
        let json: serde_json::Value = context.get(&json_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'json' at key '{}'",
                json_key
            ))
        })?;

        // Get configuration (path is stored in node data)
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<JsonFilterConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        log::debug!(
            "JsonFilterTask {}: extracting path '{}' from JSON",
            self.task_id,
            config.path
        );

        // Extract value using path
        let (value, found) = match Self::extract_path(&json, &config.path) {
            Some(v) => (v, true),
            None => {
                // Use default value if provided
                let default = config.default_value.unwrap_or(serde_json::Value::Null);
                (default, false)
            }
        };

        // Store outputs in context
        let value_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
        context.set(&value_key, value.clone()).await;

        let found_key = ContextKeys::output(&self.task_id, Self::PORT_FOUND);
        context.set(&found_key, found).await;

        log::debug!(
            "JsonFilterTask {}: extracted value, found={}",
            self.task_id,
            found
        );

        Ok(TaskResult::new(Some(serde_json::to_string(&value).unwrap_or_default()), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_task_id() {
        let task = JsonFilterTask::new("my_filter");
        assert_eq!(task.id(), "my_filter");
    }

    #[test]
    fn test_with_config() {
        let config = JsonFilterConfig {
            path: "data.name".to_string(),
            default_value: Some(json!("default")),
        };
        let task = JsonFilterTask::with_config("task1", config);
        assert_eq!(task.config.as_ref().unwrap().path, "data.name");
    }

    #[test]
    fn test_descriptor() {
        let meta = JsonFilterTask::descriptor();
        assert_eq!(meta.node_type, "json-filter");
        assert_eq!(meta.category, NodeCategory::Processing);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.outputs.len(), 2);
    }

    #[test]
    fn test_extract_simple_field() {
        let json = json!({"name": "test", "value": 42});
        let result = JsonFilterTask::extract_path(&json, "name");
        assert_eq!(result, Some(json!("test")));
    }

    #[test]
    fn test_extract_nested_field() {
        let json = json!({"data": {"items": [1, 2, 3], "name": "nested"}});
        let result = JsonFilterTask::extract_path(&json, "data.name");
        assert_eq!(result, Some(json!("nested")));
    }

    #[test]
    fn test_extract_array_index() {
        let json = json!([{"name": "first"}, {"name": "second"}]);
        let result = JsonFilterTask::extract_path(&json, "[0]");
        assert_eq!(result, Some(json!({"name": "first"})));
    }

    #[test]
    fn test_extract_array_then_field() {
        let json = json!([{"name": "first"}, {"name": "second"}]);
        let result = JsonFilterTask::extract_path(&json, "[1].name");
        assert_eq!(result, Some(json!("second")));
    }

    #[test]
    fn test_extract_field_then_array() {
        let json = json!({"items": [10, 20, 30]});
        let result = JsonFilterTask::extract_path(&json, "items[2]");
        assert_eq!(result, Some(json!(30)));
    }

    #[test]
    fn test_extract_complex_path() {
        let json = json!({
            "response": {
                "choices": [
                    {
                        "message": {
                            "tool_calls": [
                                {"id": "call_1", "arguments": {"content": "hello"}}
                            ]
                        }
                    }
                ]
            }
        });
        let result = JsonFilterTask::extract_path(
            &json,
            "response.choices[0].message.tool_calls[0].arguments.content",
        );
        assert_eq!(result, Some(json!("hello")));
    }

    #[test]
    fn test_extract_missing_field() {
        let json = json!({"name": "test"});
        let result = JsonFilterTask::extract_path(&json, "missing");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_empty_path() {
        let json = json!({"name": "test"});
        let result = JsonFilterTask::extract_path(&json, "");
        assert_eq!(result, Some(json!({"name": "test"})));
    }

    #[test]
    fn test_extract_array_out_of_bounds() {
        let json = json!([1, 2, 3]);
        let result = JsonFilterTask::extract_path(&json, "[10]");
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_filter_execution() {
        let config = JsonFilterConfig {
            path: "data.value".to_string(),
            default_value: None,
        };
        let task = JsonFilterTask::with_config("test_filter", config);
        let context = Context::new();

        let json_key = ContextKeys::input("test_filter", "json");
        context.set(&json_key, json!({"data": {"value": 42}})).await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        let value_key = ContextKeys::output("test_filter", "value");
        let value: Option<serde_json::Value> = context.get(&value_key).await;
        assert_eq!(value, Some(json!(42)));

        let found_key = ContextKeys::output("test_filter", "found");
        let found: Option<bool> = context.get(&found_key).await;
        assert_eq!(found, Some(true));
    }

    #[tokio::test]
    async fn test_filter_with_default() {
        let config = JsonFilterConfig {
            path: "missing.path".to_string(),
            default_value: Some(json!("default_value")),
        };
        let task = JsonFilterTask::with_config("test_filter", config);
        let context = Context::new();

        let json_key = ContextKeys::input("test_filter", "json");
        context.set(&json_key, json!({"other": "data"})).await;

        task.run(context.clone()).await.unwrap();

        let value_key = ContextKeys::output("test_filter", "value");
        let value: Option<serde_json::Value> = context.get(&value_key).await;
        assert_eq!(value, Some(json!("default_value")));

        let found_key = ContextKeys::output("test_filter", "found");
        let found: Option<bool> = context.get(&found_key).await;
        assert_eq!(found, Some(false));
    }

    #[tokio::test]
    async fn test_missing_json_error() {
        let task = JsonFilterTask::new("test_filter");
        let context = Context::new();

        // Run without setting json - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
