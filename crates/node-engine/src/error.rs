//! Error types for the node engine

use thiserror::Error;

/// Result type alias using NodeEngineError
pub type Result<T> = std::result::Result<T, NodeEngineError>;

/// Errors that can occur in the node engine
#[derive(Debug, Error)]
pub enum NodeEngineError {
    /// Error from graph-flow execution
    #[error("Graph execution error: {0}")]
    GraphFlow(String),

    /// Missing required input
    #[error("Missing required input: {0}")]
    MissingInput(String),

    /// Invalid input type
    #[error("Invalid input type for '{port}': expected {expected}")]
    InvalidInputType { port: String, expected: String },

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Context value not found
    #[error("Context value not found: {0}")]
    ContextNotFound(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Workflow was cancelled
    #[error("Workflow cancelled")]
    Cancelled,

    /// Workflow is waiting for interactive input
    #[error("Workflow is waiting for input at task '{task_id}'")]
    WaitingForInput {
        task_id: String,
        prompt: Option<String>,
    },

    /// Gateway/inference error
    #[error("Gateway error: {0}")]
    Gateway(String),

    /// RAG/vector search error
    #[error("RAG error: {0}")]
    Rag(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl NodeEngineError {
    /// Create an execution failed error with a message
    pub fn failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed(msg.into())
    }

    /// Create a waiting-for-input error for interactive tasks.
    pub fn waiting_for_input(task_id: impl Into<String>, prompt: Option<String>) -> Self {
        Self::WaitingForInput {
            task_id: task_id.into(),
            prompt,
        }
    }

    /// Create from a graph-flow error
    pub fn from_graph_flow(err: graph_flow::GraphError) -> Self {
        Self::GraphFlow(err.to_string())
    }
}
