//! Workflow events for streaming updates to the frontend
//!
//! These events are sent via Tauri channels to provide real-time
//! feedback on workflow execution progress.

use serde::Serialize;
use std::collections::HashMap;

use super::node::PortValue;

/// Events emitted during workflow execution
///
/// These are sent to the frontend via a Tauri channel to provide
/// real-time updates on execution progress.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum WorkflowEvent {
    /// Workflow execution has started
    Started {
        /// Unique identifier for this execution
        workflow_id: String,
        /// Total number of nodes to execute
        node_count: usize,
    },

    /// A node has begun executing
    NodeStarted {
        /// ID of the node that started
        node_id: String,
        /// Type of the node (for UI display)
        node_type: String,
    },

    /// Progress update from a node (for long-running operations)
    NodeProgress {
        /// ID of the node reporting progress
        node_id: String,
        /// Progress percentage (0.0 to 1.0)
        progress: f32,
        /// Optional status message
        message: Option<String>,
    },

    /// Streaming content from a node (for LLM output, etc.)
    NodeStream {
        /// ID of the node emitting the stream
        node_id: String,
        /// Output port the stream is for
        port: String,
        /// Chunk of streaming data
        chunk: serde_json::Value,
    },

    /// A node has completed successfully
    NodeCompleted {
        /// ID of the completed node
        node_id: String,
        /// Output values produced by the node
        outputs: HashMap<String, PortValue>,
    },

    /// A node has failed
    NodeError {
        /// ID of the failed node
        node_id: String,
        /// Error message
        error: String,
    },

    /// Workflow has completed successfully
    Completed {
        /// All outputs from all nodes
        outputs: HashMap<String, HashMap<String, PortValue>>,
    },

    /// Workflow has failed
    Failed {
        /// Error message describing the failure
        error: String,
    },
}

impl WorkflowEvent {
    /// Create a Started event
    pub fn started(workflow_id: impl Into<String>, node_count: usize) -> Self {
        Self::Started {
            workflow_id: workflow_id.into(),
            node_count,
        }
    }

    /// Create a NodeStarted event
    pub fn node_started(node_id: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self::NodeStarted {
            node_id: node_id.into(),
            node_type: node_type.into(),
        }
    }

    /// Create a NodeProgress event
    pub fn node_progress(
        node_id: impl Into<String>,
        progress: f32,
        message: Option<String>,
    ) -> Self {
        Self::NodeProgress {
            node_id: node_id.into(),
            progress,
            message,
        }
    }

    /// Create a NodeStream event
    pub fn node_stream(
        node_id: impl Into<String>,
        port: impl Into<String>,
        chunk: serde_json::Value,
    ) -> Self {
        Self::NodeStream {
            node_id: node_id.into(),
            port: port.into(),
            chunk,
        }
    }

    /// Create a NodeCompleted event
    pub fn node_completed(
        node_id: impl Into<String>,
        outputs: HashMap<String, PortValue>,
    ) -> Self {
        Self::NodeCompleted {
            node_id: node_id.into(),
            outputs,
        }
    }

    /// Create a NodeError event
    pub fn node_error(node_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::NodeError {
            node_id: node_id.into(),
            error: error.into(),
        }
    }

    /// Create a Completed event
    pub fn completed(outputs: HashMap<String, HashMap<String, PortValue>>) -> Self {
        Self::Completed { outputs }
    }

    /// Create a Failed event
    pub fn failed(error: impl Into<String>) -> Self {
        Self::Failed {
            error: error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = WorkflowEvent::started("test-123", 5);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Started"));
        assert!(json.contains("test-123"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_node_stream_event() {
        let event = WorkflowEvent::node_stream(
            "node1",
            "output",
            serde_json::json!({"text": "hello"}),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("NodeStream"));
        assert!(json.contains("hello"));
    }
}
