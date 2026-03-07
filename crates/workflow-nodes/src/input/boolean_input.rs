//! Boolean Input Task
//!
//! Provides a boolean value to the workflow without requiring a
//! setting-specific node type for every true/false inference option.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Boolean Input Task
#[derive(Clone)]
pub struct BooleanInputTask {
    task_id: String,
}

impl BooleanInputTask {
    /// Port ID for boolean value input/output.
    pub const PORT_VALUE: &'static str = "value";

    /// Create a new boolean input task.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for BooleanInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "boolean-input".to_string(),
            category: NodeCategory::Input,
            label: "Boolean Input".to_string(),
            description: "Provides a true/false value to the workflow".to_string(),
            inputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::Boolean,
            )],
            outputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::Boolean,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(BooleanInputTask::descriptor));

fn parse_boolean_value(value: &serde_json::Value) -> Option<bool> {
    value.as_bool().or_else(|| match value.as_str()? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    })
}

#[async_trait]
impl Task for BooleanInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_VALUE);
        let value: Option<serde_json::Value> = context.get(&input_key).await;

        let Some(boolean) = value.as_ref().and_then(parse_boolean_value) else {
            return Ok(TaskResult::new(None, NextAction::Continue));
        };

        let output_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
        context.set(&output_key, serde_json::json!(boolean)).await;

        Ok(TaskResult::new(
            Some(boolean.to_string()),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let meta = BooleanInputTask::descriptor();
        assert_eq!(meta.node_type, "boolean-input");
        assert_eq!(meta.category, NodeCategory::Input);
        assert_eq!(meta.inputs[0].data_type, PortDataType::Boolean);
        assert_eq!(meta.outputs[0].data_type, PortDataType::Boolean);
    }

    #[tokio::test]
    async fn test_passthrough_boolean() {
        let task = BooleanInputTask::new("test_boolean_input");
        let context = Context::new();

        let input_key = ContextKeys::input("test_boolean_input", "value");
        context.set(&input_key, serde_json::json!(true)).await;

        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response.as_deref(), Some("true"));

        let output_key = ContextKeys::output("test_boolean_input", "value");
        let output: Option<serde_json::Value> = context.get(&output_key).await;
        assert_eq!(output, Some(serde_json::json!(true)));
    }

    #[tokio::test]
    async fn test_missing_boolean_yields_no_output() {
        let task = BooleanInputTask::new("test_boolean_input");
        let context = Context::new();

        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response, None);

        let output_key = ContextKeys::output("test_boolean_input", "value");
        let output: Option<serde_json::Value> = context.get(&output_key).await;
        assert_eq!(output, None);
    }
}
