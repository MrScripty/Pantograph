//! Canonical LLM inference bootstrap descriptor.
//!
//! Execution is owned by the host typed inference gateway. This module only
//! defines the graph-visible `llm-inference` bootstrap contract. Model and
//! task specific ports are resolved by workflow-service descriptors and
//! persisted as authored inference interface snapshots.

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

/// Canonical LLM inference bootstrap descriptor.
///
/// Hosts execute this node through the typed inference gateway. The descriptor
/// keeps only pre-resolution control ports stable across frontend,
/// workflow-service, and node-engine consumers.
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
    /// Port ID for optional graph-authored scheduler device requirement
    pub const PORT_DEVICE: &'static str = "device";
    /// Port ID for canonical Pumas model reference input
    pub const PORT_PUMAS_MODEL_REF: &'static str = "pumas_model_ref";
    /// Port ID for bounded execution diagnostics output
    pub const PORT_DIAGNOSTICS: &'static str = "diagnostics";

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
            description:
                "Schedules a model-specific inference task through backend descriptor resolution"
                    .to_string(),
            inputs: vec![
                PortMetadata::optional(Self::PORT_TASK_KIND, "Task Kind", PortDataType::String),
                PortMetadata::optional(Self::PORT_RUNTIME, "Runtime", PortDataType::String),
                PortMetadata::optional(Self::PORT_DEVICE, "Device", PortDataType::String),
                PortMetadata::optional(
                    Self::PORT_PUMAS_MODEL_REF,
                    "Pumas Model Ref",
                    PortDataType::Json,
                ),
            ],
            outputs: vec![PortMetadata::optional(
                Self::PORT_DIAGNOSTICS,
                "Diagnostics",
                PortDataType::Json,
            )],
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
    fn test_descriptor_is_bootstrap_only() {
        let meta = InferenceTask::descriptor();

        assert_eq!(
            meta.inputs
                .iter()
                .map(|port| port.id.as_str())
                .collect::<Vec<_>>(),
            vec!["task_kind", "runtime", "device", "pumas_model_ref"]
        );
        assert_eq!(
            meta.outputs
                .iter()
                .map(|port| port.id.as_str())
                .collect::<Vec<_>>(),
            vec!["diagnostics"]
        );
        for retired_port in [
            "prompt",
            "text",
            "query",
            "documents",
            "documents_json",
            "audio",
            "tools",
            "kv_cache_in",
            "generation_options",
            "task_options",
            "denoising_scheduler",
            "inference_settings",
            "response",
            "results",
            "scores",
            "top_document",
            "top_score",
            "embedding",
            "image",
            "metadata",
            "model_ref",
            "tool_calls",
            "has_tool_calls",
            "kv_cache_out",
            "stream",
            "usage",
        ] {
            assert!(
                meta.inputs.iter().all(|port| port.id != retired_port)
                    && meta.outputs.iter().all(|port| port.id != retired_port),
                "static llm-inference descriptor must not expose retired port {retired_port}"
            );
        }
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
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_DEVICE
                && p.data_type == PortDataType::String
                && !p.required));
        assert!(!meta.inputs.iter().any(|p| p.id == "backend_key"));
        assert!(meta
            .inputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_PUMAS_MODEL_REF
                && p.data_type == PortDataType::Json));
        assert!(!meta.inputs.iter().any(|p| p.id == "resolved_model_source"));
        assert!(!meta
            .inputs
            .iter()
            .any(|p| p.id == "resolved_model_package_facts"));
        assert!(meta
            .outputs
            .iter()
            .any(|p| p.id == InferenceTask::PORT_DIAGNOSTICS && p.data_type == PortDataType::Json));
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
