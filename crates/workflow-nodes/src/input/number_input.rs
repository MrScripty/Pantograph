//! Number Input Task
//!
//! Provides a numeric value to the workflow without requiring a setting-specific
//! node type for every numeric inference option.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Number Input Task
#[derive(Clone)]
pub struct NumberInputTask {
    task_id: String,
}

impl NumberInputTask {
    /// Port ID for numeric value input/output.
    pub const PORT_VALUE: &'static str = "value";

    /// Create a new number input task.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for NumberInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "number-input".to_string(),
            category: NodeCategory::Input,
            label: "Number Input".to_string(),
            description: "Provides a numeric value to the workflow".to_string(),
            inputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::Number,
            )],
            outputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::Number,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(NumberInputTask::descriptor));

fn parse_number_value(value: &serde_json::Value) -> Option<f64> {
    if let Some(number) = value.as_f64() {
        return number.is_finite().then_some(number);
    }

    value
        .as_str()
        .and_then(|raw| raw.parse::<f64>().ok())
        .and_then(|number| number.is_finite().then_some(number))
}

#[async_trait]
impl Task for NumberInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_VALUE);
        let value: Option<serde_json::Value> = context.get(&input_key).await;

        let Some(number) = value.as_ref().and_then(parse_number_value) else {
            return Ok(TaskResult::new(None, NextAction::Continue));
        };

        let output_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
        context.set(&output_key, serde_json::json!(number)).await;

        Ok(TaskResult::new(
            Some(number.to_string()),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let meta = NumberInputTask::descriptor();
        assert_eq!(meta.node_type, "number-input");
        assert_eq!(meta.category, NodeCategory::Input);
        assert_eq!(meta.inputs[0].data_type, PortDataType::Number);
        assert_eq!(meta.outputs[0].data_type, PortDataType::Number);
    }

    #[tokio::test]
    async fn test_passthrough_number() {
        let task = NumberInputTask::new("test_number_input");
        let context = Context::new();

        let input_key = ContextKeys::input("test_number_input", "value");
        context.set(&input_key, serde_json::json!(1.25)).await;

        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response.as_deref(), Some("1.25"));

        let output_key = ContextKeys::output("test_number_input", "value");
        let output: Option<serde_json::Value> = context.get(&output_key).await;
        assert_eq!(output, Some(serde_json::json!(1.25)));
    }

    #[tokio::test]
    async fn test_missing_number_yields_no_output() {
        let task = NumberInputTask::new("test_number_input");
        let context = Context::new();

        let result = task.run(context.clone()).await.unwrap();
        assert_eq!(result.response, None);

        let output_key = ContextKeys::output("test_number_input", "value");
        let output: Option<serde_json::Value> = context.get(&output_key).await;
        assert_eq!(output, None);
    }
}
