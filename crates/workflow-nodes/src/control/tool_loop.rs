//! Tool Loop Task
//!
//! Declares the composed tool-loop authoring contract. Runtime execution is
//! owned by the composed primitive graph, not this descriptor task.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// A tool definition for the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
}

/// A tool call made by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Arguments as JSON
    pub arguments: serde_json::Value,
    /// Optional call ID for response matching
    pub id: Option<String>,
}

/// Tool loop authoring descriptor.
///
/// Composed contract metadata maps this stable external node onto canonical
/// `llm-inference`, `tool-executor`, and turn-state primitives.
///
/// # Inputs (from context)
/// - `{task_id}.input.prompt` (required) - The initial user prompt
/// - `{task_id}.input.system_prompt` (optional) - System prompt
/// - `{task_id}.input.context` (optional) - Additional context
/// - `{task_id}.input.tools` (optional) - Array of ToolDefinition
/// - `{task_id}.input.max_turns` (optional) - Override default max turns
///
/// # Outputs (to context)
/// - `{task_id}.output.response` - The final LLM response
/// - `{task_id}.output.tool_calls` - Array of all tool calls made
/// - `{task_id}.output.turns` - Number of turns executed
///
/// # Streaming
/// - `{task_id}.stream.turn` - Stream data for each turn
#[derive(Clone)]
pub struct ToolLoopTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl ToolLoopTask {
    /// Port ID for prompt input
    pub const PORT_PROMPT: &'static str = "prompt";
    /// Port ID for system prompt input
    pub const PORT_SYSTEM_PROMPT: &'static str = "system_prompt";
    /// Port ID for context input
    pub const PORT_CONTEXT: &'static str = "context";
    /// Port ID for tools input
    pub const PORT_TOOLS: &'static str = "tools";
    /// Port ID for max turns input
    pub const PORT_MAX_TURNS: &'static str = "max_turns";
    /// Port ID for response output
    pub const PORT_RESPONSE: &'static str = "response";
    /// Port ID for tool calls output
    pub const PORT_TOOL_CALLS: &'static str = "tool_calls";
    /// Port ID for turns output
    pub const PORT_TURNS: &'static str = "turns";

    /// Create a new tool loop task
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

impl TaskDescriptor for ToolLoopTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "tool-loop".to_string(),
            category: NodeCategory::Control,
            label: "Tool Loop".to_string(),
            description: "Composed tool-loop authoring contract backed by canonical inference and tool primitives".to_string(),
            inputs: vec![
                PortMetadata::required(Self::PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(
                    Self::PORT_SYSTEM_PROMPT,
                    "System Prompt",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_CONTEXT, "Context", PortDataType::String),
                PortMetadata::optional(Self::PORT_TOOLS, "Tools", PortDataType::Tools).multiple(),
                PortMetadata::optional(Self::PORT_MAX_TURNS, "Max Turns", PortDataType::Number),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(Self::PORT_TOOL_CALLS, "Tool Calls", PortDataType::Json),
                PortMetadata::optional(Self::PORT_TURNS, "Turns", PortDataType::Number),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(ToolLoopTask::descriptor));

#[async_trait]
impl Task for ToolLoopTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "tool-loop requires composed execution through canonical llm-inference and tool-executor primitives".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = ToolLoopTask::new("my_loop");
        assert_eq!(task.id(), "my_loop");
    }

    #[test]
    fn test_tool_definition_serialize() {
        let tool = ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get current weather".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("get_weather"));
        assert!(json.contains("location"));
    }

    #[test]
    fn test_tool_call_serialize() {
        let call = ToolCall {
            name: "search".to_string(),
            arguments: serde_json::json!({"query": "rust programming"}),
            id: Some("call_123".to_string()),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("search"));
        assert!(json.contains("rust programming"));
    }

    #[tokio::test]
    async fn test_run_requires_composed_execution() {
        let task = ToolLoopTask::new("test_loop");
        let result = task.run(Context::new()).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("composed execution"),
            "error should point callers at composed execution, got: {err}"
        );
    }
}
