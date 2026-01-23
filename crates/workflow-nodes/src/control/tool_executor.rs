//! Tool Executor Task
//!
//! Executes tool calls returned by an LLM inference node.
//! This task takes tool call definitions and tool implementations,
//! executes each tool call, and returns the results.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use node_engine::{
    ContextKeys, ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor,
    TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// A tool call to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments for the tool as JSON
    pub arguments: serde_json::Value,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// ID matching the original tool call
    pub tool_call_id: String,
    /// Result of the tool execution
    pub result: serde_json::Value,
    /// Whether the execution was successful
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
}

/// Tool Executor Task
///
/// Executes tool calls from an LLM and returns the results.
/// In the context of a workflow, this task receives tool calls
/// from an inference node and executes them using the provided
/// tool definitions.
///
/// # Inputs (from context)
/// - `{task_id}.input.tool_calls` (required) - Array of ToolCallRequest
/// - `{task_id}.input.tools` (required) - Tool definitions with implementations
///
/// # Outputs (to context)
/// - `{task_id}.output.results` - Array of ToolCallResult
/// - `{task_id}.output.all_success` - Boolean indicating all tools succeeded
#[derive(Clone)]
pub struct ToolExecutorTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ToolExecutorTask {
    /// Port ID for tool calls input
    pub const PORT_TOOL_CALLS: &'static str = "tool_calls";
    /// Port ID for tools input
    pub const PORT_TOOLS: &'static str = "tools";
    /// Port ID for results output
    pub const PORT_RESULTS: &'static str = "results";
    /// Port ID for all_success output
    pub const PORT_ALL_SUCCESS: &'static str = "all_success";

    /// Create a new tool executor task
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

impl TaskDescriptor for ToolExecutorTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "tool-executor".to_string(),
            category: NodeCategory::Control,
            label: "Tool Executor".to_string(),
            description: "Executes tool calls from LLM and returns results".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_TOOL_CALLS, "Tool Calls", PortDataType::Json),
                PortMetadata::required(Self::PORT_TOOLS, "Tools", PortDataType::Tools),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_RESULTS, "Results", PortDataType::Json),
                PortMetadata::optional(Self::PORT_ALL_SUCCESS, "All Success", PortDataType::Boolean),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Task for ToolExecutorTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: tool_calls
        let tool_calls_key = ContextKeys::input(&self.task_id, Self::PORT_TOOL_CALLS);
        let tool_calls: Vec<ToolCallRequest> = context.get(&tool_calls_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'tool_calls' at key '{}'",
                tool_calls_key
            ))
        })?;

        // Get required input: tools (for now we just validate they exist)
        let tools_key = ContextKeys::input(&self.task_id, Self::PORT_TOOLS);
        let _tools: serde_json::Value = context.get(&tools_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'tools' at key '{}'",
                tools_key
            ))
        })?;

        log::debug!(
            "ToolExecutorTask {}: executing {} tool calls",
            self.task_id,
            tool_calls.len()
        );

        let mut results: Vec<ToolCallResult> = Vec::new();
        let all_success = true;

        for call in &tool_calls {
            log::debug!(
                "ToolExecutorTask {}: executing tool '{}' with id '{}'",
                self.task_id,
                call.name,
                call.id
            );

            // For now, tool execution is a placeholder.
            // In a full implementation, this would:
            // 1. Look up the tool definition by name
            // 2. Validate arguments against the tool's parameter schema
            // 3. Execute the tool's implementation
            // 4. Capture the result or error
            //
            // Currently, we return a placeholder result indicating
            // that actual tool execution requires external implementation.
            let result = ToolCallResult {
                tool_call_id: call.id.clone(),
                result: serde_json::json!({
                    "status": "pending",
                    "message": format!("Tool '{}' execution requires external implementation", call.name),
                    "arguments_received": call.arguments
                }),
                success: true,
                error: None,
            };

            results.push(result);
        }

        // Store outputs in context
        let results_key = ContextKeys::output(&self.task_id, Self::PORT_RESULTS);
        context.set(&results_key, results.clone()).await;

        let all_success_key = ContextKeys::output(&self.task_id, Self::PORT_ALL_SUCCESS);
        context.set(&all_success_key, all_success).await;

        log::debug!(
            "ToolExecutorTask {}: completed with {} results, all_success={}",
            self.task_id,
            results.len(),
            all_success
        );

        Ok(TaskResult::new(
            Some(serde_json::to_string(&results).unwrap_or_default()),
            NextAction::Continue,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ToolExecutorTask::new("my_executor");
        assert_eq!(task.id(), "my_executor");
    }

    #[test]
    fn test_descriptor() {
        let meta = ToolExecutorTask::descriptor();
        assert_eq!(meta.node_type, "tool-executor");
        assert_eq!(meta.category, NodeCategory::Control);
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.outputs.len(), 2);
    }

    #[test]
    fn test_tool_call_request_serialize() {
        let call = ToolCallRequest {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"location": "San Francisco"}),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("call_123"));
        assert!(json.contains("get_weather"));
        assert!(json.contains("San Francisco"));
    }

    #[test]
    fn test_tool_call_result_serialize() {
        let result = ToolCallResult {
            tool_call_id: "call_123".to_string(),
            result: serde_json::json!({"temperature": 72}),
            success: true,
            error: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("call_123"));
        assert!(json.contains("temperature"));
    }

    #[tokio::test]
    async fn test_missing_tool_calls_error() {
        let task = ToolExecutorTask::new("test_executor");
        let context = Context::new();

        // Run without setting tool_calls - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
