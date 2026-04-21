//! Tool Executor Task
//!
//! Disabled tool execution boundary.
//!
//! The descriptor remains registered for saved workflow compatibility, but
//! invocation fails until a backend-owned tool runtime is available.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
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
/// Represents tool calls from an LLM. Runtime execution is disabled until
/// backend-owned tool execution contracts are implemented.
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
            description: "Disabled until backend-owned tool execution contracts are implemented"
                .to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_TOOL_CALLS, "Tool Calls", PortDataType::Json),
                PortMetadata::required(Self::PORT_TOOLS, "Tools", PortDataType::Tools),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_RESULTS, "Results", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_ALL_SUCCESS,
                    "All Success",
                    PortDataType::Boolean,
                ),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ToolExecutorTask::descriptor));

#[async_trait]
impl Task for ToolExecutorTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: tool_calls
        let tool_calls_key = ContextKeys::input(&self.task_id, Self::PORT_TOOL_CALLS);
        let tool_calls: Vec<ToolCallRequest> =
            context.get(&tool_calls_key).await.ok_or_else(|| {
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

        Err(GraphError::TaskExecutionFailed(format!(
            "tool-executor is disabled until backend-owned tool execution is implemented; received {} tool call(s)",
            tool_calls.len()
        )))
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

    #[tokio::test]
    async fn test_tool_execution_is_disabled() {
        let task = ToolExecutorTask::new("test_executor");
        let context = Context::new();
        context
            .set(
                &ContextKeys::input("test_executor", ToolExecutorTask::PORT_TOOL_CALLS),
                vec![ToolCallRequest {
                    id: "call_1".to_string(),
                    name: "get_weather".to_string(),
                    arguments: serde_json::json!({"location": "San Francisco"}),
                }],
            )
            .await;
        context
            .set(
                &ContextKeys::input("test_executor", ToolExecutorTask::PORT_TOOLS),
                serde_json::json!([]),
            )
            .await;

        let result = task.run(context).await;

        assert!(result.is_err());
        let message = result.unwrap_err().to_string();
        assert!(message.contains("tool-executor is disabled"));
    }
}
