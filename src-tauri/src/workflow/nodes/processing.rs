//! Processing nodes - LLM, vision, and RAG operations
//!
//! These nodes perform AI/ML processing operations using the
//! inference gateway and RAG manager.

use std::collections::HashMap;

use async_trait::async_trait;
use tauri::ipc::Channel;

use crate::workflow::events::WorkflowEvent;
use crate::workflow::node::{ExecutionContext, InputsExt, Node, NodeError, NodeInputs, NodeOutputs};
use crate::workflow::types::{
    ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

/// LLM Inference node - text completion using the inference gateway
///
/// Sends a prompt to the LLM and returns the response.
/// Supports streaming output via the channel.
pub struct LLMInferenceNode {
    id: String,
    definition: NodeDefinition,
}

impl LLMInferenceNode {
    /// Create a new LLM inference node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "llm-inference".to_string(),
            category: NodeCategory::Processing,
            label: "LLM Inference".to_string(),
            description: "Send a prompt to the LLM and get a text response".to_string(),
            inputs: vec![
                PortDefinition::required("prompt", "Prompt", PortDataType::Prompt),
                PortDefinition::optional("system_prompt", "System Prompt", PortDataType::String),
                PortDefinition::optional("context", "Context", PortDataType::String).multiple(),
            ],
            outputs: vec![
                PortDefinition::required("response", "Response", PortDataType::String),
                PortDefinition::optional("stream", "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

#[async_trait]
impl Node for LLMInferenceNode {
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
        let extra_context = inputs.get_string_opt("context");

        // Check if LLM is ready
        if !context.is_llm_ready().await {
            return Err(NodeError::Gateway("LLM server is not ready".to_string()));
        }

        let base_url = context
            .llm_base_url()
            .await
            .ok_or_else(|| NodeError::Gateway("No LLM server URL available".to_string()))?;

        // Build the full prompt with context if provided
        let full_prompt = if let Some(ctx) = extra_context {
            format!("{}\n\nContext:\n{}", prompt, ctx)
        } else {
            prompt.to_string()
        };

        // Send progress event
        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            0.1,
            Some("Sending request to LLM...".to_string()),
        ));

        // Build messages for OpenAI-compatible API
        let mut messages = Vec::new();
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

        // Make the completion request using reqwest directly
        let client = reqwest::Client::new();
        let http_response = client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&serde_json::json!({
                "model": "gpt-4",
                "messages": messages,
                "stream": false
            }))
            .send()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("LLM request failed: {}", e)))?;

        if !http_response.status().is_success() {
            let error = http_response.text().await.unwrap_or_default();
            return Err(NodeError::ExecutionFailed(format!("LLM error: {}", error)));
        }

        let json: serde_json::Value = http_response
            .json()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;

        let response = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        // Stream the response
        let _ = channel.send(WorkflowEvent::node_stream(
            &self.id,
            "stream",
            serde_json::json!({
                "type": "text",
                "content": &response
            }),
        ));

        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            1.0,
            Some("Complete".to_string()),
        ));

        let mut outputs = HashMap::new();
        outputs.insert("response".to_string(), serde_json::json!(response));
        outputs.insert("stream".to_string(), serde_json::Value::Null);

        Ok(outputs)
    }
}

/// Vision analysis node - analyze images using the vision API
///
/// Sends an image to the vision model for analysis.
pub struct VisionAnalysisNode {
    id: String,
    definition: NodeDefinition,
}

impl VisionAnalysisNode {
    /// Create a new vision analysis node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "vision-analysis".to_string(),
            category: NodeCategory::Processing,
            label: "Vision Analysis".to_string(),
            description: "Analyze an image using the vision model".to_string(),
            inputs: vec![
                PortDefinition::required("image", "Image", PortDataType::Image),
                PortDefinition::required("prompt", "Prompt", PortDataType::Prompt),
            ],
            outputs: vec![PortDefinition::required(
                "analysis",
                "Analysis",
                PortDataType::String,
            )],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

#[async_trait]
impl Node for VisionAnalysisNode {
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
        let image_base64 = inputs.get_string("image")?;
        let prompt = inputs.get_string("prompt")?;

        // Check if LLM is ready
        if !context.is_llm_ready().await {
            return Err(NodeError::Gateway("Vision server is not ready".to_string()));
        }

        let base_url = context
            .llm_base_url()
            .await
            .ok_or_else(|| NodeError::Gateway("No vision server URL available".to_string()))?;

        // Send progress
        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            0.1,
            Some("Analyzing image...".to_string()),
        ));

        // Build vision request with image
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&serde_json::json!({
                "model": "gpt-4-vision-preview",
                "messages": [{
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": prompt
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/png;base64,{}", image_base64)
                            }
                        }
                    ]
                }],
                "max_tokens": 4096
            }))
            .send()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("Vision request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(NodeError::ExecutionFailed(format!(
                "Vision API error: {}",
                error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| NodeError::ExecutionFailed(format!("Failed to parse response: {}", e)))?;

        let analysis = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            1.0,
            Some("Complete".to_string()),
        ));

        let mut outputs = HashMap::new();
        outputs.insert("analysis".to_string(), serde_json::json!(analysis));

        Ok(outputs)
    }
}

/// RAG search node - semantic search using the RAG manager
///
/// Searches indexed documents for relevant content.
pub struct RAGSearchNode {
    id: String,
    definition: NodeDefinition,
}

impl RAGSearchNode {
    /// Create a new RAG search node instance
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            definition: Self::definition(),
        }
    }

    /// Get the node type definition
    pub fn definition() -> NodeDefinition {
        NodeDefinition {
            node_type: "rag-search".to_string(),
            category: NodeCategory::Processing,
            label: "RAG Search".to_string(),
            description: "Search indexed documents for relevant content".to_string(),
            inputs: vec![
                PortDefinition::required("query", "Query", PortDataType::String),
                PortDefinition::optional("limit", "Result Limit", PortDataType::Number),
            ],
            outputs: vec![
                PortDefinition::required("documents", "Documents", PortDataType::Document),
                PortDefinition::optional("context", "Context String", PortDataType::String),
            ],
            execution_mode: ExecutionMode::Reactive,
        }
    }
}

#[async_trait]
impl Node for RAGSearchNode {
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
        let query = inputs.get_string("query")?;
        let limit = inputs.get_number_or("limit", 5.0) as usize;

        // Send progress
        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            0.1,
            Some("Searching documents...".to_string()),
        ));

        // Perform RAG search
        let rag_manager = context.rag_manager.read().await;
        let docs = rag_manager
            .search_as_docs(&query, limit)
            .await
            .map_err(|e| NodeError::Rag(e.to_string()))?;

        // Build context string from documents
        let context_str = docs
            .iter()
            .map(|doc| format!("## {}\n{}", doc.title, doc.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let _ = channel.send(WorkflowEvent::node_progress(
            &self.id,
            1.0,
            Some(format!("Found {} documents", docs.len())),
        ));

        let mut outputs = HashMap::new();
        outputs.insert("documents".to_string(), serde_json::to_value(&docs).unwrap());
        outputs.insert("context".to_string(), serde_json::json!(context_str));

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_inference_definition() {
        let def = LLMInferenceNode::definition();
        assert_eq!(def.node_type, "llm-inference");
        assert_eq!(def.category, NodeCategory::Processing);
        assert!(def.inputs.iter().any(|p| p.id == "prompt" && p.required));
    }

    #[test]
    fn test_vision_analysis_definition() {
        let def = VisionAnalysisNode::definition();
        assert_eq!(def.node_type, "vision-analysis");
        assert!(def.inputs.iter().any(|p| p.id == "image"));
        assert!(def.inputs.iter().any(|p| p.id == "prompt"));
    }

    #[test]
    fn test_rag_search_definition() {
        let def = RAGSearchNode::definition();
        assert_eq!(def.node_type, "rag-search");
        assert!(def.inputs.iter().any(|p| p.id == "query" && p.required));
        assert!(def.outputs.iter().any(|p| p.id == "documents"));
    }
}
