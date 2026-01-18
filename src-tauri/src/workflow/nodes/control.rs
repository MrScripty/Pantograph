//! Control flow nodes - loops, conditionals, and agent loops
//!
//! These nodes provide control flow capabilities within workflows,
//! including the ToolLoopNode for multi-turn agent conversations.

use std::collections::HashMap;

use async_trait::async_trait;
use tauri::ipc::Channel;

use crate::workflow::events::WorkflowEvent;
use crate::workflow::node::{ExecutionContext, InputsExt, Node, NodeError, NodeInputs, NodeOutputs};
use crate::workflow::types::{
    ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

/// Tool loop node - multi-turn agent loop with tool calling
///
/// This node runs an LLM in a loop, allowing it to call tools
/// until it produces a final response. This is the composable
/// replacement for the monolithic agent loop.
pub struct ToolLoopNode {
    id: String,
    definition: NodeDefinition,
}

impl ToolLoopNode {
    /// Create a new tool loop node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "tool-loop".to_string(),
            category: NodeCategory::Control,
            label: "Tool Loop".to_string(),
            description: "Run an LLM with tools in a loop until task completion".to_string(),
            inputs: vec![
                PortDefinition::required("prompt", "Prompt", PortDataType::Prompt),
                PortDefinition::optional("system_prompt", "System Prompt", PortDataType::String),
                PortDefinition::optional("tools", "Tools", PortDataType::Tools).multiple(),
                PortDefinition::optional("max_turns", "Max Turns", PortDataType::Number),
                PortDefinition::optional("context", "Context", PortDataType::String).multiple(),
            ],
            outputs: vec![
                PortDefinition::required("response", "Response", PortDataType::String),
                PortDefinition::optional("tool_calls", "Tool Calls", PortDataType::Json),
                PortDefinition::optional("stream", "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

#[async_trait]
impl Node for ToolLoopNode {
    fn definition(&self) -> &NodeDefinition {
        &self.definition
    }

    fn id(&self) -> &str {
        &self.id
    }

    async fn execute(
        &self,
        inputs: NodeInputs,
        context: &ExecutionContext,
        channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError> {
        let prompt = inputs.get_string("prompt")?;
        let system_prompt = inputs.get_string_opt("system_prompt");
        let max_turns = inputs.get_number_or("max_turns", 5.0) as usize;
        let extra_context = inputs.get_string_opt("context");

        // Check if LLM is ready
        if !context.is_llm_ready().await {
            return Err(NodeError::Gateway("LLM server is not ready".to_string()));
        }

        let base_url = context
            .llm_base_url()
            .await
            .ok_or_else(|| NodeError::Gateway("No LLM server URL available".to_string()))?;

        // Build the initial prompt with context
        let full_prompt = if let Some(ctx) = extra_context {
            format!("{}\n\nContext:\n{}", prompt, ctx)
        } else {
            prompt.to_string()
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

        let mut all_tool_calls: Vec<serde_json::Value> = Vec::new();
        let mut final_response = String::new();

        let client = reqwest::Client::new();

        for turn in 0..max_turns {
            // Check abort signal
            if context.is_aborted() {
                return Err(NodeError::Cancelled);
            }

            // Send progress
            let _ = channel.send(WorkflowEvent::node_progress(
                &self.id,
                turn as f32 / max_turns as f32,
                Some(format!("Turn {}/{}", turn + 1, max_turns)),
            ));

            // Make LLM request
            // Note: In a full implementation, we would include tool definitions here
            // For now, we do a simple completion without tools
            let response = client
                .post(format!("{}/v1/chat/completions", base_url))
                .json(&serde_json::json!({
                    "model": "gpt-4",
                    "messages": messages,
                    "stream": false
                }))
                .send()
                .await
                .map_err(|e| NodeError::ExecutionFailed(format!("LLM request failed: {}", e)))?;

            if !response.status().is_success() {
                let error = response.text().await.unwrap_or_default();
                return Err(NodeError::ExecutionFailed(format!("LLM error: {}", error)));
            }

            let json: serde_json::Value = response
                .json()
                .await
                .map_err(|e| NodeError::ExecutionFailed(format!("Parse error: {}", e)))?;

            let message = &json["choices"][0]["message"];
            let content = message["content"].as_str().unwrap_or("").to_string();

            // Stream the response
            let _ = channel.send(WorkflowEvent::node_stream(
                &self.id,
                "stream",
                serde_json::json!({
                    "type": "text",
                    "content": &content,
                    "turn": turn
                }),
            ));

            // Check for tool calls
            let tool_calls = message.get("tool_calls");

            if tool_calls.is_none() || tool_calls.unwrap().as_array().map_or(true, |a| a.is_empty())
            {
                // No tool calls - we're done
                final_response = content;
                break;
            }

            // Process tool calls (placeholder - would need actual tool execution)
            if let Some(calls) = tool_calls.and_then(|t| t.as_array()) {
                for call in calls {
                    let tool_name = call["function"]["name"].as_str().unwrap_or("unknown");
                    let tool_args = &call["function"]["arguments"];

                    // Stream tool call event
                    let _ = channel.send(WorkflowEvent::node_stream(
                        &self.id,
                        "stream",
                        serde_json::json!({
                            "type": "tool_call",
                            "name": tool_name,
                            "arguments": tool_args
                        }),
                    ));

                    all_tool_calls.push(serde_json::json!({
                        "name": tool_name,
                        "arguments": tool_args
                    }));
                }
            }

            // Add assistant message to conversation
            messages.push(message.clone());

            // In a full implementation, we would:
            // 1. Execute the tools
            // 2. Add tool results to messages
            // 3. Continue the loop

            // For now, break after first response since we don't have tool execution
            final_response = content;
            break;
        }

        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            1.0,
            Some("Complete".to_string()),
        ));

        let mut outputs = HashMap::new();
        outputs.insert("response".to_string(), serde_json::json!(final_response));
        outputs.insert("tool_calls".to_string(), serde_json::json!(all_tool_calls));
        outputs.insert("stream".to_string(), serde_json::Value::Null);

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_loop_definition() {
        let def = ToolLoopNode::definition();
        assert_eq!(def.node_type, "tool-loop");
        assert_eq!(def.category, NodeCategory::Control);
        assert!(def.inputs.iter().any(|p| p.id == "prompt" && p.required));
        assert!(def.inputs.iter().any(|p| p.id == "max_turns"));
        assert!(def.outputs.iter().any(|p| p.id == "response"));
        assert!(def.outputs.iter().any(|p| p.id == "tool_calls"));
    }
}
