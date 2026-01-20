//! Tool Loop Task
//!
//! Runs an LLM in a multi-turn loop with tool calling capability.
//! This is the composable replacement for monolithic agent loops.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, NextAction, Task, TaskResult};
use serde::{Deserialize, Serialize};

use super::ContextKeys;

/// Configuration for the tool loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolLoopConfig {
    /// Base URL of the LLM server
    pub base_url: String,
    /// Model name
    pub model: String,
    /// Maximum number of turns before stopping
    pub max_turns: usize,
    /// Whether to include tool definitions in requests
    pub enable_tools: bool,
}

impl Default for ToolLoopConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            model: "gpt-4".to_string(),
            max_turns: 5,
            enable_tools: true,
        }
    }
}

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

/// Tool Loop Task
///
/// Runs an LLM in a loop, allowing it to call tools until it produces
/// a final response. This is the composable replacement for the
/// monolithic agent loop.
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
    /// Configuration
    config: Option<ToolLoopConfig>,
}

impl ToolLoopTask {
    /// Create a new tool loop task
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            config: None,
        }
    }

    /// Create with configuration
    pub fn with_config(task_id: impl Into<String>, config: ToolLoopConfig) -> Self {
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

#[async_trait]
impl Task for ToolLoopTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, context: Context) -> graph_flow::Result<TaskResult> {
        // Get required input: prompt
        let prompt_key = ContextKeys::input(&self.task_id, "prompt");
        let prompt: String = context.get(&prompt_key).await.ok_or_else(|| {
            GraphError::TaskExecutionFailed(format!(
                "Missing required input 'prompt' at key '{}'",
                prompt_key
            ))
        })?;

        // Get optional inputs
        let system_prompt_key = ContextKeys::input(&self.task_id, "system_prompt");
        let system_prompt: Option<String> = context.get(&system_prompt_key).await;

        let context_key = ContextKeys::input(&self.task_id, "context");
        let extra_context: Option<String> = context.get(&context_key).await;

        let tools_key = ContextKeys::input(&self.task_id, "tools");
        let tools: Vec<ToolDefinition> = context.get(&tools_key).await.unwrap_or_default();

        // Get configuration
        let config = if let Some(ref cfg) = self.config {
            cfg.clone()
        } else {
            let config_key = ContextKeys::meta(&self.task_id, "config");
            context
                .get::<ToolLoopConfig>(&config_key)
                .await
                .unwrap_or_default()
        };

        // Check for max_turns override in input
        let max_turns_key = ContextKeys::input(&self.task_id, "max_turns");
        let max_turns: usize = context
            .get::<f64>(&max_turns_key)
            .await
            .map(|n| n as usize)
            .unwrap_or(config.max_turns);

        // Build the initial prompt with context
        let full_prompt = if let Some(ctx) = extra_context {
            format!("{}\n\nContext:\n{}", prompt, ctx)
        } else {
            prompt
        };

        // Build conversation messages
        let mut messages: Vec<serde_json::Value> = Vec::new();

        if let Some(sys) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": full_prompt
        }));

        let mut all_tool_calls: Vec<ToolCall> = Vec::new();
        let mut final_response = String::new();
        let mut turns_executed = 0;

        let client = reqwest::Client::new();
        let url = format!("{}/v1/chat/completions", config.base_url);

        log::debug!(
            "ToolLoopTask {}: starting loop with {} tools, max {} turns",
            self.task_id,
            tools.len(),
            max_turns
        );

        for turn in 0..max_turns {
            turns_executed = turn + 1;

            // Build request body
            let mut request_body = serde_json::json!({
                "model": config.model,
                "messages": messages,
                "stream": false
            });

            // Add tools if available and enabled
            if config.enable_tools && !tools.is_empty() {
                let tools_json: Vec<serde_json::Value> = tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": t.name,
                                "description": t.description,
                                "parameters": t.parameters
                            }
                        })
                    })
                    .collect();
                request_body["tools"] = serde_json::json!(tools_json);
            }

            log::debug!(
                "ToolLoopTask {}: turn {}/{}",
                self.task_id,
                turn + 1,
                max_turns
            );

            // Make LLM request
            let http_response = client
                .post(&url)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| {
                    GraphError::TaskExecutionFailed(format!("LLM request failed: {}", e))
                })?;

            if !http_response.status().is_success() {
                let status = http_response.status();
                let error_body = http_response.text().await.unwrap_or_default();
                return Err(GraphError::TaskExecutionFailed(format!(
                    "LLM API error ({}): {}",
                    status, error_body
                )));
            }

            let json: serde_json::Value = http_response.json().await.map_err(|e| {
                GraphError::TaskExecutionFailed(format!("Parse error: {}", e))
            })?;

            let message = &json["choices"][0]["message"];
            let content = message["content"].as_str().unwrap_or("").to_string();

            // Store stream data for this turn
            let stream_key = ContextKeys::stream(&self.task_id, "turn");
            context
                .set(
                    &stream_key,
                    serde_json::json!({
                        "type": "turn",
                        "turn": turn,
                        "content": &content
                    }),
                )
                .await;

            // Check for tool calls
            let tool_calls_json = message.get("tool_calls");
            let has_tool_calls = tool_calls_json
                .and_then(|t| t.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false);

            if !has_tool_calls {
                // No tool calls - we're done
                final_response = content;
                log::debug!(
                    "ToolLoopTask {}: completed after {} turns",
                    self.task_id,
                    turns_executed
                );
                break;
            }

            // Process tool calls
            if let Some(calls) = tool_calls_json.and_then(|t| t.as_array()) {
                for call in calls {
                    let tool_name = call["function"]["name"].as_str().unwrap_or("unknown");
                    let tool_args_str = call["function"]["arguments"]
                        .as_str()
                        .unwrap_or("{}");
                    let tool_args: serde_json::Value =
                        serde_json::from_str(tool_args_str).unwrap_or(serde_json::json!({}));
                    let call_id = call["id"].as_str().map(String::from);

                    let tool_call = ToolCall {
                        name: tool_name.to_string(),
                        arguments: tool_args,
                        id: call_id.clone(),
                    };

                    all_tool_calls.push(tool_call);

                    log::debug!(
                        "ToolLoopTask {}: tool call '{}' with args",
                        self.task_id,
                        tool_name
                    );
                }
            }

            // Add assistant message to conversation
            messages.push(message.clone());

            // Note: In a full implementation, we would:
            // 1. Execute the tools (via separate tool executor tasks)
            // 2. Add tool results to messages
            // 3. Continue the loop
            //
            // For now, we simulate tool execution by adding a placeholder response
            // and break after first tool call since we don't have actual tool execution
            messages.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": all_tool_calls.last().and_then(|c| c.id.clone()).unwrap_or_default(),
                "content": "Tool execution not implemented in this task. Please provide a final response."
            }));

            // Continue loop to get final response after tool call
            // In production, this would have actual tool results
        }

        // If we hit max turns without a final response, use the last content
        if final_response.is_empty() && !messages.is_empty() {
            if let Some(last_assistant) = messages
                .iter()
                .rev()
                .find(|m| m["role"] == "assistant")
            {
                final_response = last_assistant["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
            }
        }

        // Store outputs in context
        let response_key = ContextKeys::output(&self.task_id, "response");
        context.set(&response_key, final_response.clone()).await;

        let tool_calls_key = ContextKeys::output(&self.task_id, "tool_calls");
        context.set(&tool_calls_key, all_tool_calls.clone()).await;

        let turns_key = ContextKeys::output(&self.task_id, "turns");
        context.set(&turns_key, turns_executed as f64).await;

        log::debug!(
            "ToolLoopTask {}: completed with {} tool calls in {} turns",
            self.task_id,
            all_tool_calls.len(),
            turns_executed
        );

        Ok(TaskResult::new(Some(final_response), NextAction::Continue))
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
    fn test_with_config() {
        let config = ToolLoopConfig {
            base_url: "http://localhost:1234".to_string(),
            model: "gpt-3.5".to_string(),
            max_turns: 10,
            enable_tools: false,
        };
        let task = ToolLoopTask::with_config("task1", config);
        assert_eq!(
            task.config.as_ref().unwrap().base_url,
            "http://localhost:1234"
        );
        assert_eq!(task.config.as_ref().unwrap().max_turns, 10);
    }

    #[test]
    fn test_default_config() {
        let config = ToolLoopConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.max_turns, 5);
        assert!(config.enable_tools);
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
    async fn test_missing_prompt_error() {
        let task = ToolLoopTask::new("test_loop");
        let context = Context::new();

        // Run without setting prompt - should error
        let result = task.run(context).await;
        assert!(result.is_err());
    }
}
