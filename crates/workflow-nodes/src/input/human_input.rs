//! Human Input Task
//!
//! This task demonstrates the WaitForInput pattern for human-in-the-loop workflows.
//! It pauses execution and waits for user input before continuing.

use async_trait::async_trait;
use graph_flow::{Context, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// Human Input Task
///
/// Pauses workflow execution to wait for user input.
///
/// # Inputs (from context)
/// - `{task_id}.input.prompt` (optional) - Prompt to display to user
/// - `{task_id}.input.default` (optional) - Default value
/// - `{task_id}.input.auto_accept` (optional) - If true, auto-accept default value
/// - `{task_id}.input.user_response` (optional) - The user's response (set when resuming)
///
/// # Outputs (to context)
/// - `{task_id}.output.value` - The user's input value
///
/// # Control Flow
/// Returns `NextAction::WaitForInput` to pause execution.
/// Resume by setting `{task_id}.input.user_response` in context.
#[derive(Clone)]
pub struct HumanInputTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl HumanInputTask {
    /// Port ID for prompt input
    pub const PORT_PROMPT: &'static str = "prompt";
    /// Port ID for default value input
    pub const PORT_DEFAULT: &'static str = "default";
    /// Port ID for auto-accept flag
    pub const PORT_AUTO_ACCEPT: &'static str = "auto_accept";
    /// Port ID for user response (set when resuming)
    pub const PORT_USER_RESPONSE: &'static str = "user_response";
    /// Port ID for value output
    pub const PORT_VALUE: &'static str = "value";

    /// Create a new human input task
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

impl TaskDescriptor for HumanInputTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "human-input".to_string(),
            category: NodeCategory::Input,
            label: "Human Input".to_string(),
            description: "Pauses workflow to wait for user input".to_string(),
            inputs: vec![
                PortMetadata::optional(Self::PORT_PROMPT, "Prompt", PortDataType::String),
                PortMetadata::optional(Self::PORT_DEFAULT, "Default Value", PortDataType::String),
                PortMetadata::optional(Self::PORT_AUTO_ACCEPT, "Auto Accept", PortDataType::Boolean),
                PortMetadata::optional(
                    Self::PORT_USER_RESPONSE,
                    "User Response",
                    PortDataType::String,
                ),
            ],
            outputs: vec![PortMetadata::optional(
                Self::PORT_VALUE,
                "Value",
                PortDataType::String,
            )],
            execution_mode: ExecutionMode::Manual,
        }
    }
}

/// State tracking for human input task
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HumanInputState {
    /// Whether we're waiting for input
    waiting: bool,
    /// The prompt shown to user
    prompt: Option<String>,
}

#[async_trait]
impl Task for HumanInputTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        let state_key = ContextKeys::meta(&self.task_id, "state");
        let response_key = ContextKeys::input(&self.task_id, Self::PORT_USER_RESPONSE);

        // Check if we already have a response (resuming after wait)
        if let Some(response) = context.get::<String>(&response_key).await {
            // Clear state and output the response
            context.set(&state_key, HumanInputState::default()).await;

            // Store output
            let output_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
            context.set(&output_key, response.clone()).await;

            log::debug!("HumanInputTask {}: received input, continuing", self.task_id);
            return Ok(TaskResult::new(Some(response), NextAction::Continue));
        }

        // Check if we already set a default value
        let default_key = ContextKeys::input(&self.task_id, Self::PORT_DEFAULT);
        if let Some(default_value) = context.get::<String>(&default_key).await {
            // Check if user wants to use default (auto_accept flag)
            let auto_key = ContextKeys::input(&self.task_id, Self::PORT_AUTO_ACCEPT);
            if let Some(true) = context.get::<bool>(&auto_key).await {
                let output_key = ContextKeys::output(&self.task_id, Self::PORT_VALUE);
                context.set(&output_key, default_value.clone()).await;
                return Ok(TaskResult::new(Some(default_value), NextAction::Continue));
            }
        }

        // Get prompt to display
        let prompt_key = ContextKeys::input(&self.task_id, Self::PORT_PROMPT);
        let prompt: Option<String> = context.get(&prompt_key).await;

        // Store state indicating we're waiting
        let state = HumanInputState {
            waiting: true,
            prompt: prompt.clone(),
        };
        context.set(&state_key, state).await;

        log::debug!(
            "HumanInputTask {}: waiting for input (prompt: {:?})",
            self.task_id,
            prompt
        );

        // Return WaitForInput to pause execution
        Ok(TaskResult::new(prompt, NextAction::WaitForInput))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = HumanInputTask::new("approval");
        assert_eq!(task.id(), "approval");
    }

    #[tokio::test]
    async fn test_wait_for_input() {
        let task = HumanInputTask::new("test_input");
        let context = Context::new();

        // Set a prompt
        let prompt_key = ContextKeys::input("test_input", "prompt");
        context
            .set(&prompt_key, "Please enter your name:".to_string())
            .await;

        // First run should return WaitForInput
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::WaitForInput));
    }

    #[tokio::test]
    async fn test_resume_with_input() {
        let task = HumanInputTask::new("test_input");
        let context = Context::new();

        // Simulate user providing input
        let response_key = ContextKeys::input("test_input", "user_response");
        context.set(&response_key, "John Doe".to_string()).await;

        // Should continue with the input
        let result = task.run(context.clone()).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("John Doe"));

        // Output should be stored
        let output_key = ContextKeys::output("test_input", "value");
        let output: Option<String> = context.get(&output_key).await;
        assert_eq!(output, Some("John Doe".to_string()));
    }

    #[tokio::test]
    async fn test_auto_accept_default() {
        let task = HumanInputTask::new("test_input");
        let context = Context::new();

        // Set default and auto_accept
        let default_key = ContextKeys::input("test_input", "default");
        let auto_key = ContextKeys::input("test_input", "auto_accept");
        context.set(&default_key, "default_value".to_string()).await;
        context.set(&auto_key, true).await;

        // Should continue with default value
        let result = task.run(context).await.unwrap();
        assert!(matches!(result.next_action, NextAction::Continue));
        assert_eq!(result.response.as_deref(), Some("default_value"));
    }
}
