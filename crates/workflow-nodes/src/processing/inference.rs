//! Canonical LLM inference descriptor.
//!
//! Execution is owned by the host typed inference gateway. This module only
//! defines the graph-visible `llm-inference` contract.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};
use serde::{Deserialize, Serialize};

/// A tool definition for the LLM (reused from tool_loop)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
}

/// A tool call returned by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments for the tool as JSON
    pub arguments: serde_json::Value,
}

/// Canonical LLM inference descriptor.
///
/// Hosts execute this node through the typed inference gateway. The descriptor
/// keeps the graph-visible task/model/option/result ports stable across
/// frontend, workflow-service, and node-engine consumers.
///
/// # Inputs (from context)
/// - `{task_id}.input.prompt` - The prompt to send
/// - `{task_id}.input.system_prompt` (optional) - System prompt
/// - `{task_id}.input.context` (optional) - Additional context to append to prompt
/// - `{task_id}.input.tools` (optional) - Array of ToolDefinition for tool calling
///
/// # Outputs (to context)
/// - `{task_id}.output.response` - The LLM's response text
/// - `{task_id}.output.tool_calls` - Array of ToolCall if the LLM requested tools
/// - `{task_id}.output.has_tool_calls` - Boolean indicating if tool calls were made
///
#[derive(Clone)]
pub struct InferenceTask {
    /// Unique identifier for this task instance
    task_id: String,
}

impl InferenceTask {
    /// Port ID for canonical task registry id input
    pub const PORT_TASK_KIND: &'static str = "task_kind";
    /// Port ID for optional graph-authored scheduler runtime requirement
    pub const PORT_RUNTIME: &'static str = "runtime";
    /// Port ID for canonical Pumas model reference input
    pub const PORT_PUMAS_MODEL_REF: &'static str = "pumas_model_ref";
    /// Port ID for canonical generation option input
    pub const PORT_GENERATION_OPTIONS: &'static str = "generation_options";
    /// Port ID for canonical task option input
    pub const PORT_TASK_OPTIONS: &'static str = "task_options";
    /// Port ID for optional image denoising scheduler selection
    pub const PORT_DENOISING_SCHEDULER: &'static str = "denoising_scheduler";
    /// Port ID for text input used by embedding/scoring tasks
    pub const PORT_TEXT: &'static str = "text";
    /// Port ID for query input used by rerank tasks
    pub const PORT_QUERY: &'static str = "query";
    /// Port ID for structured candidate documents input
    pub const PORT_DOCUMENTS: &'static str = "documents";
    /// Port ID for string-encoded candidate documents input
    pub const PORT_DOCUMENTS_JSON: &'static str = "documents_json";
    /// Port ID for prompt input
    pub const PORT_PROMPT: &'static str = "prompt";
    /// Port ID for audio input used by transcription tasks
    pub const PORT_AUDIO: &'static str = "audio";
    /// Port ID for system prompt input
    pub const PORT_SYSTEM_PROMPT: &'static str = "system_prompt";
    /// Port ID for context input (additional context to append)
    pub const PORT_CONTEXT: &'static str = "context";
    /// Port ID for tools input
    pub const PORT_TOOLS: &'static str = "tools";
    /// Port ID for optional reusable KV-cache input
    pub const PORT_KV_CACHE_IN: &'static str = "kv_cache_in";
    /// Port ID for response output
    pub const PORT_RESPONSE: &'static str = "response";
    /// Port ID for structured rerank results output
    pub const PORT_RESULTS: &'static str = "results";
    /// Port ID for rerank score output
    pub const PORT_SCORES: &'static str = "scores";
    /// Port ID for top reranked document output
    pub const PORT_TOP_DOCUMENT: &'static str = "top_document";
    /// Port ID for top rerank score output
    pub const PORT_TOP_SCORE: &'static str = "top_score";
    /// Port ID for embedding vector output
    pub const PORT_EMBEDDING: &'static str = "embedding";
    /// Port ID for first generated image output
    pub const PORT_IMAGE: &'static str = "image";
    /// Port ID for task metadata output
    pub const PORT_METADATA: &'static str = "metadata";
    /// Port ID for canonical model reference output
    pub const PORT_MODEL_REF: &'static str = "model_ref";
    /// Port ID for tool calls output
    pub const PORT_TOOL_CALLS: &'static str = "tool_calls";
    /// Port ID for has_tool_calls output
    pub const PORT_HAS_TOOL_CALLS: &'static str = "has_tool_calls";
    /// Port ID for optional reusable KV-cache output
    pub const PORT_KV_CACHE_OUT: &'static str = "kv_cache_out";
    /// Port ID for stream output
    pub const PORT_STREAM: &'static str = "stream";
    /// Port ID for bounded execution diagnostics output
    pub const PORT_DIAGNOSTICS: &'static str = "diagnostics";
    /// Port ID for bounded usage summary output
    pub const PORT_USAGE: &'static str = "usage";

    /// Create a new inference task with the given ID
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

impl TaskDescriptor for InferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "llm-inference".to_string(),
            category: NodeCategory::Processing,
            label: "LLM Inference".to_string(),
            description: "Runs text through a language model with optional tool calling"
                .to_string(),
            inputs: vec![
                PortMetadata::optional(Self::PORT_TASK_KIND, "Task Kind", PortDataType::String),
                PortMetadata::optional(Self::PORT_RUNTIME, "Runtime", PortDataType::String),
                PortMetadata::optional(
                    Self::PORT_PUMAS_MODEL_REF,
                    "Pumas Model Ref",
                    PortDataType::Json,
                ),
                PortMetadata::optional(Self::PORT_TEXT, "Text", PortDataType::String),
                PortMetadata::optional(Self::PORT_QUERY, "Query", PortDataType::String),
                PortMetadata::optional(Self::PORT_DOCUMENTS, "Documents", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_DOCUMENTS_JSON,
                    "Documents JSON",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(Self::PORT_AUDIO, "Audio", PortDataType::Audio),
                PortMetadata::optional(
                    Self::PORT_SYSTEM_PROMPT,
                    "System Prompt",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_CONTEXT, "Context", PortDataType::String),
                PortMetadata::optional(Self::PORT_TOOLS, "Tools", PortDataType::Tools).multiple(),
                PortMetadata::optional(
                    Self::PORT_KV_CACHE_IN,
                    "KV Cache In",
                    PortDataType::KvCache,
                ),
                PortMetadata::optional(
                    Self::PORT_GENERATION_OPTIONS,
                    "Generation Options",
                    PortDataType::Json,
                ),
                PortMetadata::optional(Self::PORT_TASK_OPTIONS, "Task Options", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_DENOISING_SCHEDULER,
                    "Denoising Scheduler",
                    PortDataType::String,
                ),
                PortMetadata::optional(
                    "inference_settings",
                    "Inference Settings",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![
                PortMetadata::optional(Self::PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(Self::PORT_RESULTS, "Results", PortDataType::Json),
                PortMetadata::optional(Self::PORT_SCORES, "Scores", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_TOP_DOCUMENT,
                    "Top Document",
                    PortDataType::String,
                ),
                PortMetadata::optional(Self::PORT_TOP_SCORE, "Top Score", PortDataType::Number),
                PortMetadata::optional(Self::PORT_EMBEDDING, "Embedding", PortDataType::Embedding),
                PortMetadata::optional(Self::PORT_IMAGE, "Image", PortDataType::Image),
                PortMetadata::optional(Self::PORT_METADATA, "Metadata", PortDataType::Json),
                PortMetadata::optional(Self::PORT_MODEL_REF, "Model Ref", PortDataType::Json),
                PortMetadata::optional(Self::PORT_TOOL_CALLS, "Tool Calls", PortDataType::Json),
                PortMetadata::optional(
                    Self::PORT_HAS_TOOL_CALLS,
                    "Has Tool Calls",
                    PortDataType::Boolean,
                ),
                PortMetadata::optional(
                    Self::PORT_KV_CACHE_OUT,
                    "KV Cache Out",
                    PortDataType::KvCache,
                ),
                PortMetadata::optional(Self::PORT_STREAM, "Stream", PortDataType::Stream),
                PortMetadata::optional(Self::PORT_DIAGNOSTICS, "Diagnostics", PortDataType::Json),
                PortMetadata::optional(Self::PORT_USAGE, "Usage", PortDataType::Json),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(InferenceTask::descriptor));

#[async_trait]
impl Task for InferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "llm-inference requires host execution through the typed inference gateway".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let task = InferenceTask::new("my_inference");
        assert_eq!(task.id(), "my_inference");
    }

    #[test]
    fn test_descriptor_has_tool_ports() {
        let meta = InferenceTask::descriptor();

        // Check for tools input
        assert!(meta.inputs.iter().any(|p| p.id == "tools"));
        assert!(meta.inputs.iter().any(|p| p.id == "kv_cache_in"));
        assert!(meta.inputs.iter().any(|p| p.id == "inference_settings"));

        // Check for tool_calls output
        assert!(meta.outputs.iter().any(|p| p.id == "tool_calls"));

        // Check for has_tool_calls output
        assert!(meta.outputs.iter().any(|p| p.id == "has_tool_calls"));
        assert!(meta.outputs.iter().any(|p| p.id == "kv_cache_out"));
    }

    #[test]
    fn test_descriptor_has_canonical_inference_contract_ports() {
        let meta = InferenceTask::descriptor();

        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_TASK_KIND
                && p.data_type == PortDataType::String
                && !p.required));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_RUNTIME
                && p.data_type == PortDataType::String
                && !p.required));
        assert!(!meta.inputs.iter().any(|p| p.id == "backend_key"));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_PUMAS_MODEL_REF
                && p.data_type == PortDataType::Json));
        assert!(meta.inputs.iter().any(|p| p.id == InferenceTask::PORT_TEXT
            && p.data_type == PortDataType::String
            && !p.required));
        assert!(meta.inputs.iter().any(|p| p.id == InferenceTask::PORT_QUERY
            && p.data_type == PortDataType::String
            && !p.required));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_DOCUMENTS
                && p.data_type == PortDataType::Json
                && !p.required));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_DOCUMENTS_JSON
                && p.data_type == PortDataType::String
                && !p.required));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_PROMPT
                && p.data_type == PortDataType::Prompt
                && !p.required));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_AUDIO && p.data_type == PortDataType::Audio));
        assert!(!meta.inputs.iter().any(|p| p.id == "resolved_model_source"));
        assert!(!meta
            .inputs
            .iter()
            .any(|p| p.id == "resolved_model_package_facts"));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_GENERATION_OPTIONS
                && p.data_type == PortDataType::Json));
        assert!(
            meta.inputs
                .iter()
                .any(|p| p.id == InferenceTask::PORT_TASK_OPTIONS
                    && p.data_type == PortDataType::Json)
        );
        assert!(meta.inputs.iter().any(|p| {
            p.id == InferenceTask::PORT_DENOISING_SCHEDULER
                && p.data_type == PortDataType::String
                && !p.required
        }));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_MODEL_REF && p.data_type == PortDataType::Json));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_RESULTS && p.data_type == PortDataType::Json));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_SCORES && p.data_type == PortDataType::Json));
        assert!(meta.outputs.iter().any(|p| {
            p.id == InferenceTask::PORT_TOP_DOCUMENT && p.data_type == PortDataType::String
        }));
        assert!(meta.outputs.iter().any(|p| {
            p.id == InferenceTask::PORT_TOP_SCORE && p.data_type == PortDataType::Number
        }));
        assert!(meta.outputs.iter().any(|p| {
            p.id == InferenceTask::PORT_EMBEDDING && p.data_type == PortDataType::Embedding
        }));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_IMAGE && p.data_type == PortDataType::Image));
        assert!(meta.outputs.iter().any(|p| {
            p.id == InferenceTask::PORT_METADATA && p.data_type == PortDataType::Json
        }));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_DIAGNOSTICS && p.data_type == PortDataType::Json));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_USAGE && p.data_type == PortDataType::Json));
    }

    #[tokio::test]
    async fn test_run_returns_host_gateway_error() {
        let task = InferenceTask::new("test-llm");
        let result = task.run(Context::new()).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("typed inference gateway"),
            "error should point callers at the host typed gateway, got: {err}"
        );
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
            id: "call_123".to_string(),
            name: "search".to_string(),
            arguments: serde_json::json!({"query": "rust programming"}),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("call_123"));
        assert!(json.contains("search"));
        assert!(json.contains("rust programming"));
    }
}
