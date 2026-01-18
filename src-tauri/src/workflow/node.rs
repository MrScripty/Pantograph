//! Node trait and execution context
//!
//! Defines the core Node trait that all workflow nodes must implement,
//! along with the ExecutionContext that provides access to shared resources.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use async_trait::async_trait;
use tauri::ipc::Channel;
use tokio::sync::RwLock;

use crate::agent::rag::RagManager;
use crate::llm::gateway::InferenceGateway;

use super::events::WorkflowEvent;
use super::types::NodeDefinition;

/// A value that flows through a port
pub type PortValue = serde_json::Value;

/// Resolved inputs for node execution
pub type NodeInputs = HashMap<String, PortValue>;

/// Outputs produced by node execution
pub type NodeOutputs = HashMap<String, PortValue>;

/// Errors that can occur during node execution
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Missing required input: {0}")]
    MissingInput(String),

    #[error("Invalid input type for '{port}': expected {expected}")]
    InvalidInputType { port: String, expected: String },

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Execution cancelled")]
    Cancelled,

    #[error("Gateway error: {0}")]
    Gateway(String),

    #[error("RAG error: {0}")]
    Rag(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl NodeError {
    /// Create an execution failed error with a message
    pub fn failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed(msg.into())
    }
}

/// Context available to all nodes during execution
///
/// Provides access to shared resources like the inference gateway,
/// RAG manager, and project configuration.
pub struct ExecutionContext {
    /// Root directory of the project being worked on
    pub project_root: PathBuf,

    /// Signal to abort execution (set to true to cancel)
    pub abort_signal: Arc<AtomicBool>,

    /// Gateway for LLM inference operations
    pub gateway: Arc<InferenceGateway>,

    /// Manager for RAG/vector search operations
    pub rag_manager: Arc<RwLock<RagManager>>,

    /// Unique identifier for this execution run
    pub execution_id: String,
}

impl ExecutionContext {
    /// Check if execution has been aborted
    pub fn is_aborted(&self) -> bool {
        self.abort_signal.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the base URL of the LLM server
    pub async fn llm_base_url(&self) -> Option<String> {
        self.gateway.base_url().await
    }

    /// Check if the LLM server is ready
    pub async fn is_llm_ready(&self) -> bool {
        self.gateway.is_ready().await
    }
}

/// The core trait that all workflow nodes must implement
///
/// Nodes are the building blocks of workflows. Each node:
/// - Has a definition describing its ports and metadata
/// - Executes asynchronously with resolved inputs
/// - Produces outputs that flow to downstream nodes
/// - Can emit streaming events via the channel
#[async_trait]
pub trait Node: Send + Sync {
    /// Returns the node's type definition
    ///
    /// This defines the node's ports, category, and other metadata.
    fn definition(&self) -> &NodeDefinition;

    /// Returns the node instance ID
    ///
    /// This is the unique identifier for this specific node instance
    /// in the workflow graph.
    fn id(&self) -> &str;

    /// Execute the node with resolved inputs
    ///
    /// # Arguments
    /// * `inputs` - Map of port ID to resolved input values
    /// * `context` - Shared execution context with access to resources
    /// * `channel` - Channel for emitting streaming events to the frontend
    ///
    /// # Returns
    /// Map of port ID to output values, or an error
    async fn execute(
        &self,
        inputs: NodeInputs,
        context: &ExecutionContext,
        channel: &Channel<WorkflowEvent>,
    ) -> Result<NodeOutputs, NodeError>;
}

/// Helper trait for extracting typed values from NodeInputs
pub trait InputsExt {
    /// Get a required string input
    fn get_string(&self, key: &str) -> Result<&str, NodeError>;

    /// Get an optional string input
    fn get_string_opt(&self, key: &str) -> Option<&str>;

    /// Get a required number input
    fn get_number(&self, key: &str) -> Result<f64, NodeError>;

    /// Get an optional number input with default
    fn get_number_or(&self, key: &str, default: f64) -> f64;

    /// Get a required boolean input
    fn get_bool(&self, key: &str) -> Result<bool, NodeError>;

    /// Get an optional boolean input with default
    fn get_bool_or(&self, key: &str, default: bool) -> bool;

    /// Get a required JSON object input
    fn get_object(&self, key: &str) -> Result<&serde_json::Map<String, serde_json::Value>, NodeError>;
}

impl InputsExt for NodeInputs {
    fn get_string(&self, key: &str) -> Result<&str, NodeError> {
        self.get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::MissingInput(key.to_string()))
    }

    fn get_string_opt(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(|v| v.as_str())
    }

    fn get_number(&self, key: &str) -> Result<f64, NodeError> {
        self.get(key)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| NodeError::MissingInput(key.to_string()))
    }

    fn get_number_or(&self, key: &str, default: f64) -> f64 {
        self.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    }

    fn get_bool(&self, key: &str) -> Result<bool, NodeError> {
        self.get(key)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| NodeError::MissingInput(key.to_string()))
    }

    fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    fn get_object(&self, key: &str) -> Result<&serde_json::Map<String, serde_json::Value>, NodeError> {
        self.get(key)
            .and_then(|v| v.as_object())
            .ok_or_else(|| NodeError::MissingInput(key.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_inputs_get_string() {
        let mut inputs = NodeInputs::new();
        inputs.insert("text".into(), json!("hello"));

        assert_eq!(inputs.get_string("text").unwrap(), "hello");
        assert!(inputs.get_string("missing").is_err());
    }

    #[test]
    fn test_inputs_get_number() {
        let mut inputs = NodeInputs::new();
        inputs.insert("count".into(), json!(42.0));

        assert_eq!(inputs.get_number("count").unwrap(), 42.0);
        assert_eq!(inputs.get_number_or("missing", 10.0), 10.0);
    }

    #[test]
    fn test_inputs_get_bool() {
        let mut inputs = NodeInputs::new();
        inputs.insert("flag".into(), json!(true));

        assert!(inputs.get_bool("flag").unwrap());
        assert!(!inputs.get_bool_or("missing", false));
    }
}
