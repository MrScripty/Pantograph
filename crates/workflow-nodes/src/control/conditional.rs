//! Conditional Task
//!
//! Routes data based on a boolean condition.
//! This task enables branching in workflow graphs by directing
//! input values to different output ports based on a condition.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};

/// Conditional Task
///
/// Routes a value to one of two outputs based on a boolean condition.
/// When the condition is true, the value flows to `true_out`.
/// When the condition is false, the value flows to `false_out`.
///
/// # Inputs (from context)
/// - `{task_id}.input.condition` (required) - Boolean condition
/// - `{task_id}.input.value` (required) - Value to route
///
/// # Outputs (to context)
/// - `{task_id}.output.true_out` - Output when condition is true
/// - `{task_id}.output.false_out` - Output when condition is false
#[derive(Clone)]
pub struct ConditionalTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ConditionalTask {
    /// Port ID for condition input
    pub const PORT_CONDITION: &'static str = "condition";
    /// Port ID for value input
    pub const PORT_VALUE: &'static str = "value";
    /// Port ID for true output
    pub const PORT_TRUE_OUT: &'static str = "true_out";
    /// Port ID for false output
    pub const PORT_FALSE_OUT: &'static str = "false_out";

    /// Create a new conditional task
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

impl TaskDescriptor for ConditionalTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "conditional".to_string(),
            category: NodeCategory::Control,
            label: "Conditional".to_string(),
            description: "Routes data based on a boolean condition".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_CONDITION, "Condition", PortDataType::Boolean),
                PortMetadata::required(Self::PORT_VALUE, "Value", PortDataType::Any),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_TRUE_OUT, "True", PortDataType::Any),
                PortMetadata::optional(Self::PORT_FALSE_OUT, "False", PortDataType::Any),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Task for ConditionalTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: condition
        let condition_key = ContextKeys::input(&self.task_id, Self::PORT_CONDITION);
        let condition: bool = context.get(&condition_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'condition' at key '{}'",
                condition_key
            ))
        })?;

        // Get required input: value
        let value_key = ContextKeys::input(&self.task_id, Self::PORT_VALUE);
        let value: serde_json::Value = context.get(&value_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'value' at key '{}'",
                value_key
            ))
        })?;

        log::debug!(
            "ConditionalTask {}: condition={}, routing value",
            self.task_id,
            condition
        );

        // Route value based on condition
        if condition {
            let true_out_key = ContextKeys::output(&self.task_id, Self::PORT_TRUE_OUT);
            context.set(&true_out_key, value.clone()).await;
            log::debug!("ConditionalTask {}: routed to true_out", self.task_id);
        } else {
            let false_out_key = ContextKeys::output(&self.task_id, Self::PORT_FALSE_OUT);
            context.set(&false_out_key, value.clone()).await;
            log::debug!("ConditionalTask {}: routed to false_out", self.task_id);
        }

        Ok(TaskResult::new(Some(serde_json::to_string(&value).unwrap_or_default()), NextAction::Continue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ConditionalTask::new("my_conditional");
        assert_eq!(task.id(), "my_conditional");
    }

    #[test]
    fn test_descriptor() {
        let meta = ConditionalTask::descriptor();
        assert_eq!(meta.node_type, "conditional");
        assert_eq!(meta.category, NodeCategory::Control);
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.outputs.len(), 2);

        // Check input ports
        assert!(meta.inputs.iter().any(|p| p.id == "condition"));
        assert!(meta.inputs.iter().any(|p| p.id == "value"));

        // Check output ports
        assert!(meta.outputs.iter().any(|p| p.id == "true_out"));
        assert!(meta.outputs.iter().any(|p| p.id == "false_out"));
    }

    #[tokio::test]
    async fn test_route_to_true() {
        let task = ConditionalTask::new("test_cond");
        let context = Context::new();

        // Set inputs
        let condition_key = ContextKeys::input("test_cond", "condition");
        context.set(&condition_key, true).await;

        let value_key = ContextKeys::input("test_cond", "value");
        context.set(&value_key, serde_json::json!("test_value")).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify output
        let true_out_key = ContextKeys::output("test_cond", "true_out");
        let output: Option<serde_json::Value> = context.get(&true_out_key).await;
        assert_eq!(output, Some(serde_json::json!("test_value")));

        // False output should not be set
        let false_out_key = ContextKeys::output("test_cond", "false_out");
        let false_output: Option<serde_json::Value> = context.get(&false_out_key).await;
        assert!(false_output.is_none());
    }

    #[tokio::test]
    async fn test_route_to_false() {
        let task = ConditionalTask::new("test_cond");
        let context = Context::new();

        // Set inputs
        let condition_key = ContextKeys::input("test_cond", "condition");
        context.set(&condition_key, false).await;

        let value_key = ContextKeys::input("test_cond", "value");
        context.set(&value_key, serde_json::json!({"data": 123})).await;

        // Run task
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));

        // Verify output
        let false_out_key = ContextKeys::output("test_cond", "false_out");
        let output: Option<serde_json::Value> = context.get(&false_out_key).await;
        assert_eq!(output, Some(serde_json::json!({"data": 123})));

        // True output should not be set
        let true_out_key = ContextKeys::output("test_cond", "true_out");
        let true_output: Option<serde_json::Value> = context.get(&true_out_key).await;
        assert!(true_output.is_none());
    }

    #[tokio::test]
    async fn test_missing_condition_error() {
        let task = ConditionalTask::new("test_cond");
        let context = Context::new();

        // Only set value, not condition
        let value_key = ContextKeys::input("test_cond", "value");
        context.set(&value_key, serde_json::json!("test")).await;

        // Run task - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_value_error() {
        let task = ConditionalTask::new("test_cond");
        let context = Context::new();

        // Only set condition, not value
        let condition_key = ContextKeys::input("test_cond", "condition");
        context.set(&condition_key, true).await;

        // Run task - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
