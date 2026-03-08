//! Selection Input Task
//!
//! Provides a generic enum-style input source for workflow ports whose
//! valid values come from downstream port metadata.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Selection Input Task
///
/// Passes through a metadata-driven selected value.
#[derive(Clone)]
pub struct SelectionInputTask {
    task_id: String,
}

impl SelectionInputTask {
    /// Port ID for selection input/output.
    pub const PORT_VALUE: &'static str = "value";

    /// Create a new selection input task.
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for SelectionInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "selection-input".to_string(),
            category: NodeCategory::Input,
            label: "Selection Input".to_string(),
            description: "Provides a metadata-driven selected value to the workflow".to_string(),
            inputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::Any,
            )],
            outputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::Any,
            )],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(SelectionInputTask::descriptor));

#[async_trait]
impl Task for SelectionInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let input_key = ContextKeys::input(&self.task_id, Self::PORT_VALUE);
        let value: serde_json::Value = context
            .get(&input_key)
            .await
            .unwrap_or(serde_json::Value::Null);

        let output_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
        context.set(&output_key, value.clone()).await;

        Ok(TaskResult::new(
            Some(value.to_string()),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor() {
        let meta = SelectionInputTask::descriptor();
        assert_eq!(meta.node_type, "selection-input");
        assert_eq!(meta.category, NodeCategory::Input);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.outputs.len(), 1);
        assert_eq!(meta.inputs[0].data_type, PortDataType::Any);
    }

    #[tokio::test]
    async fn test_passthrough_value() {
        let task = SelectionInputTask::new("test_selection_input");
        let context = Context::new();

        let input_key = ContextKeys::input("test_selection_input", "value");
        context
            .set(&input_key, serde_json::json!("expr-voice-5-m"))
            .await;

        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        let output_key = ContextKeys::output("test_selection_input", "value");
        let output: Option<serde_json::Value> = context.get(&output_key).await;
        assert_eq!(output, Some(serde_json::json!("expr-voice-5-m")));
    }
}
