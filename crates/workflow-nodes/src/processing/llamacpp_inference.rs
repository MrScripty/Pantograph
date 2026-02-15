//! LlamaCpp Inference Task — Stub Descriptor
//!
//! Provides metadata so that `register_builtins()` discovers the
//! `llamacpp-inference` node type. Actual execution is delegated to
//! the host application via the callback bridge, so `run()` always
//! returns an error directing callers to that path.

use async_trait::async_trait;
use graph_flow::{Context, GraphError, Task, TaskResult};
use node_engine::{
    ExecutionMode, NodeCategory, PortDataType, PortMetadata, TaskDescriptor, TaskMetadata,
};

// Port name constants
const PORT_MODEL_PATH: &str = "model_path";
const PORT_PROMPT: &str = "prompt";
const PORT_SYSTEM_PROMPT: &str = "system_prompt";
const PORT_TEMPERATURE: &str = "temperature";
const PORT_MAX_TOKENS: &str = "max_tokens";
const PORT_TOOLS: &str = "tools";
const PORT_RESPONSE: &str = "response";
const PORT_TOOL_CALLS: &str = "tool_calls";
const PORT_HAS_TOOL_CALLS: &str = "has_tool_calls";
const PORT_STREAM: &str = "stream";
const PORT_MODEL_REF: &str = "model_ref";

/// Stub descriptor for the llama.cpp inference node.
///
/// The node metadata is registered via `inventory` so the frontend can
/// render the node and validate connections, but all inference work is
/// performed by the host through the callback bridge.
#[derive(Clone)]
pub struct LlamaCppInferenceTask {
    task_id: String,
}

impl LlamaCppInferenceTask {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
        }
    }
}

impl TaskDescriptor for LlamaCppInferenceTask {
    fn descriptor() -> TaskMetadata {
        TaskMetadata {
            node_type: "llamacpp-inference".to_string(),
            category: NodeCategory::Processing,
            label: "LlamaCpp Inference".to_string(),
            description: "Run inference via llama.cpp server (no model duplication)".to_string(),
            inputs: vec![
                PortMetadata::required(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::required(PORT_PROMPT, "Prompt", PortDataType::Prompt),
                PortMetadata::optional(
                    PORT_SYSTEM_PROMPT,
                    "System Prompt",
                    PortDataType::String,
                ),
                PortMetadata::optional(PORT_TEMPERATURE, "Temperature", PortDataType::Number),
                PortMetadata::optional(PORT_MAX_TOKENS, "Max Tokens", PortDataType::Number),
                PortMetadata::optional(PORT_TOOLS, "Tools", PortDataType::Tools).multiple(),
            ],
            outputs: vec![
                PortMetadata::required(PORT_RESPONSE, "Response", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_PATH, "Model Path", PortDataType::String),
                PortMetadata::optional(PORT_MODEL_REF, "Model Reference", PortDataType::Json),
                PortMetadata::optional(PORT_TOOL_CALLS, "Tool Calls", PortDataType::Json),
                PortMetadata::optional(PORT_HAS_TOOL_CALLS, "Has Tool Calls", PortDataType::Boolean),
                PortMetadata::optional(PORT_STREAM, "Stream", PortDataType::Stream),
            ],
            execution_mode: ExecutionMode::Stream,
        }
    }
}

inventory::submit!(node_engine::DescriptorFn(LlamaCppInferenceTask::descriptor));

#[async_trait]
impl Task for LlamaCppInferenceTask {
    fn id(&self) -> &str {
        &self.task_id
    }

    async fn run(&self, _context: Context) -> graph_flow::Result<TaskResult> {
        Err(GraphError::TaskExecutionFailed(
            "llamacpp-inference requires host-specific execution via the callback bridge".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_has_correct_node_type() {
        let meta = LlamaCppInferenceTask::descriptor();

        assert_eq!(meta.node_type, "llamacpp-inference");
    }

    #[test]
    fn test_descriptor_has_correct_ports() {
        let meta = LlamaCppInferenceTask::descriptor();

        // 6 inputs: model_path, prompt, system_prompt, temperature, max_tokens, tools
        assert_eq!(meta.inputs.len(), 6);
        assert!(meta.inputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.inputs.iter().any(|p| p.id == "prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "system_prompt"));
        assert!(meta.inputs.iter().any(|p| p.id == "temperature"));
        assert!(meta.inputs.iter().any(|p| p.id == "max_tokens"));
        assert!(meta.inputs.iter().any(|p| p.id == "tools"));

        // 6 outputs: response, model_path, model_ref, tool_calls, has_tool_calls, stream
        assert_eq!(meta.outputs.len(), 6);
        assert!(meta.outputs.iter().any(|p| p.id == "model_ref"));
        assert!(meta.outputs.iter().any(|p| p.id == "response"));
        assert!(meta.outputs.iter().any(|p| p.id == "model_path"));
        assert!(meta.outputs.iter().any(|p| p.id == "tool_calls"));
        assert!(meta.outputs.iter().any(|p| p.id == "has_tool_calls"));
        assert!(meta.outputs.iter().any(|p| p.id == "stream"));
    }

    #[tokio::test]
    async fn test_run_returns_error() {
        let task = LlamaCppInferenceTask::new("test-llamacpp");
        let context = Context::new();

        let result = task.run(context).await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("callback bridge"),
            "error should mention callback bridge, got: {err}"
        );
    }
}
