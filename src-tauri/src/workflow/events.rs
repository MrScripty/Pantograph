//! Workflow events for streaming updates to the frontend
//!
//! These events are sent via Tauri channels to provide real-time
//! feedback on workflow execution progress.

use std::collections::HashMap;

use pantograph_workflow_service::WorkflowGraph;

use super::diagnostics::WorkflowDiagnosticsProjection;

#[path = "event_serialization.rs"]
pub(crate) mod event_serialization;

/// A value that flows through a port (alias for serde_json::Value)
pub type PortValue = serde_json::Value;

/// Events emitted during workflow execution
///
/// These are sent to the frontend via a Tauri channel to provide
/// real-time updates on execution progress.
#[derive(Debug, Clone)]
pub enum WorkflowEvent {
    /// Workflow execution has started
    Started {
        /// Unique identifier for this execution
        workflow_id: String,
        /// Total number of nodes to execute
        node_count: usize,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// A node has begun executing
    NodeStarted {
        /// ID of the node that started
        node_id: String,
        /// Type of the node (for UI display)
        node_type: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// A node's execution inputs have been resolved.
    NodeInputsResolved {
        /// ID of the node whose inputs were resolved
        node_id: String,
        /// Resolved input values by port
        inputs: HashMap<String, PortValue>,
        /// Fresh execution or cache-hit evidence for the input snapshot
        cache_status: Option<node_engine::TaskExecutionCacheStatus>,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// Progress update from a node (for long-running operations)
    NodeProgress {
        /// ID of the node reporting progress
        node_id: String,
        /// Progress percentage (0.0 to 1.0)
        progress: f32,
        /// Optional status message
        message: Option<String>,
        /// Optional backend-owned structured progress detail
        detail: Option<node_engine::TaskProgressDetail>,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// Streaming content from a node (for LLM output, etc.)
    NodeStream {
        /// ID of the node emitting the stream
        node_id: String,
        /// Output port the stream is for
        port: String,
        /// Chunk of streaming data
        chunk: serde_json::Value,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// A node has completed successfully
    NodeCompleted {
        /// ID of the completed node
        node_id: String,
        /// Output values produced by the node
        outputs: HashMap<String, PortValue>,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// A node has failed
    NodeError {
        /// ID of the failed node
        node_id: String,
        /// Error message
        error: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// Workflow has completed successfully
    Completed {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// All outputs from all nodes
        outputs: HashMap<String, HashMap<String, PortValue>>,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// Workflow has failed
    Failed {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Error message describing the failure
        error: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// Workflow was cancelled before completing successfully
    Cancelled {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Cancellation reason when one is available
        error: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
    },

    /// Graph was modified (edge/node added/removed)
    GraphModified {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
        /// The updated graph when a full snapshot is available
        graph: Option<WorkflowGraph>,
        /// Nodes invalidated by the graph change
        dirty_tasks: Vec<String>,
        /// Backend-owned mutation impact for preserved vs invalidated node memory
        memory_impact: Option<node_engine::GraphMemoryImpactSummary>,
    },

    /// Workflow execution is waiting for input before it can continue
    WaitingForInput {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
        /// Node or task waiting for input
        node_id: String,
        /// Optional prompt shown to the user
        message: Option<String>,
    },

    /// Incremental execution has started for a subset of tasks
    IncrementalExecutionStarted {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
        /// Task ids that are being re-executed
        task_ids: Vec<String>,
    },

    /// Backend-owned diagnostics projection captured after a workflow event.
    DiagnosticsSnapshot {
        /// Unique identifier for this execution
        workflow_run_id: String,
        /// Canonical diagnostics projection for the workflow UI
        snapshot: Box<WorkflowDiagnosticsProjection>,
    },
}

impl WorkflowEvent {
    /// Create a DiagnosticsSnapshot event
    pub fn diagnostics_snapshot(
        workflow_run_id: impl Into<String>,
        snapshot: WorkflowDiagnosticsProjection,
    ) -> Self {
        Self::DiagnosticsSnapshot {
            workflow_run_id: workflow_run_id.into(),
            snapshot: Box::new(snapshot),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = WorkflowEvent::Started {
            workflow_id: "test-123".to_string(),
            node_count: 5,
            workflow_run_id: "exec-123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Started"));
        assert!(json.contains("test-123"));
        assert!(json.contains("5"));
        assert!(json.contains("exec-123"));
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(
            value["data"]["ownership"]["eventWorkflowRunId"].as_str(),
            Some("exec-123")
        );
        assert_eq!(
            value["data"]["ownership"]["activeWorkflowRunId"].as_str(),
            Some("exec-123")
        );
        assert_eq!(value["data"]["ownership"]["relevant"].as_bool(), Some(true));
    }

    #[test]
    fn test_node_stream_event() {
        let event = WorkflowEvent::NodeStream {
            node_id: "node1".to_string(),
            port: "output".to_string(),
            chunk: serde_json::json!({"text": "hello"}),
            workflow_run_id: "exec-123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("NodeStream"));
        assert!(json.contains("hello"));
        assert!(json.contains("exec-123"));
    }
}
