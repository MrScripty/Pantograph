//! Workflow events for streaming updates to the frontend
//!
//! These events are sent via Tauri channels to provide real-time
//! feedback on workflow execution progress.

use serde::Serialize;
use std::collections::HashMap;

use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowSessionQueueItem, WorkflowSessionSummary,
    WorkflowTraceRuntimeMetrics,
};

use super::diagnostics::WorkflowDiagnosticsProjection;

/// A value that flows through a port (alias for serde_json::Value)
pub type PortValue = serde_json::Value;

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
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// A node has begun executing
    NodeStarted {
        /// ID of the node that started
        node_id: String,
        /// Type of the node (for UI display)
        node_type: String,
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// Progress update from a node (for long-running operations)
    NodeProgress {
        /// ID of the node reporting progress
        node_id: String,
        /// Progress percentage (0.0 to 1.0)
        progress: f32,
        /// Optional status message
        message: Option<String>,
        /// Unique identifier for this execution
        execution_id: String,
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
        execution_id: String,
    },

    /// A node has completed successfully
    NodeCompleted {
        /// ID of the completed node
        node_id: String,
        /// Output values produced by the node
        outputs: HashMap<String, PortValue>,
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// A node has failed
    NodeError {
        /// ID of the failed node
        node_id: String,
        /// Error message
        error: String,
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// Workflow has completed successfully
    Completed {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// All outputs from all nodes
        outputs: HashMap<String, HashMap<String, PortValue>>,
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// Workflow has failed
    Failed {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Error message describing the failure
        error: String,
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// Workflow was cancelled before completing successfully
    Cancelled {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Cancellation reason when one is available
        error: String,
        /// Unique identifier for this execution
        execution_id: String,
    },

    /// Graph was modified (edge/node added/removed)
    GraphModified {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        execution_id: String,
        /// The updated graph when a full snapshot is available
        graph: Option<super::types::WorkflowGraph>,
        /// Nodes invalidated by the graph change
        dirty_tasks: Vec<String>,
    },

    /// Workflow execution is waiting for input before it can continue
    WaitingForInput {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        execution_id: String,
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
        execution_id: String,
        /// Task ids that are being re-executed
        task_ids: Vec<String>,
    },

    /// Runtime capabilities snapshot captured during execution
    RuntimeSnapshot {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        execution_id: String,
        /// Millisecond unix timestamp for when the snapshot was captured
        captured_at_ms: u64,
        /// Runtime capabilities and requirements when available
        capabilities: Option<WorkflowCapabilitiesResponse>,
        /// Backend-owned runtime lifecycle metrics captured alongside the snapshot
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
        /// Error encountered while collecting the runtime snapshot
        error: Option<String>,
    },

    /// Scheduler/session snapshot captured during execution
    SchedulerSnapshot {
        /// Workflow identifier associated with this run
        workflow_id: Option<String>,
        /// Unique identifier for this execution
        execution_id: String,
        /// Session identifier the snapshot belongs to
        session_id: String,
        /// Millisecond unix timestamp for when the snapshot was captured
        captured_at_ms: u64,
        /// Session summary when available
        session: Option<WorkflowSessionSummary>,
        /// Queue items visible at capture time
        items: Vec<WorkflowSessionQueueItem>,
        /// Error encountered while collecting the scheduler snapshot
        error: Option<String>,
    },

    /// Backend-owned diagnostics projection captured after a workflow event.
    DiagnosticsSnapshot {
        /// Unique identifier for this execution
        execution_id: String,
        /// Canonical diagnostics projection for the workflow UI
        snapshot: WorkflowDiagnosticsProjection,
    },
}

impl WorkflowEvent {
    /// Create a Started event
    pub fn started(
        workflow_id: impl Into<String>,
        node_count: usize,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::Started {
            workflow_id: workflow_id.into(),
            node_count,
            execution_id: execution_id.into(),
        }
    }

    /// Create a NodeStarted event
    pub fn node_started(
        node_id: impl Into<String>,
        node_type: impl Into<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::NodeStarted {
            node_id: node_id.into(),
            node_type: node_type.into(),
            execution_id: execution_id.into(),
        }
    }

    /// Create a NodeProgress event
    pub fn node_progress(
        node_id: impl Into<String>,
        progress: f32,
        message: Option<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::NodeProgress {
            node_id: node_id.into(),
            progress,
            message,
            execution_id: execution_id.into(),
        }
    }

    /// Create a NodeStream event
    pub fn node_stream(
        node_id: impl Into<String>,
        port: impl Into<String>,
        chunk: serde_json::Value,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::NodeStream {
            node_id: node_id.into(),
            port: port.into(),
            chunk,
            execution_id: execution_id.into(),
        }
    }

    /// Create a NodeCompleted event
    pub fn node_completed(
        node_id: impl Into<String>,
        outputs: HashMap<String, PortValue>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::NodeCompleted {
            node_id: node_id.into(),
            outputs,
            execution_id: execution_id.into(),
        }
    }

    /// Create a NodeError event
    pub fn node_error(
        node_id: impl Into<String>,
        error: impl Into<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::NodeError {
            node_id: node_id.into(),
            error: error.into(),
            execution_id: execution_id.into(),
        }
    }

    /// Create a Completed event
    pub fn completed(
        workflow_id: impl Into<String>,
        outputs: HashMap<String, HashMap<String, PortValue>>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::Completed {
            workflow_id: workflow_id.into(),
            outputs,
            execution_id: execution_id.into(),
        }
    }

    /// Create a Failed event
    pub fn failed(
        workflow_id: impl Into<String>,
        error: impl Into<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::Failed {
            workflow_id: workflow_id.into(),
            error: error.into(),
            execution_id: execution_id.into(),
        }
    }

    /// Create a Cancelled event
    pub fn cancelled(
        workflow_id: impl Into<String>,
        error: impl Into<String>,
        execution_id: impl Into<String>,
    ) -> Self {
        Self::Cancelled {
            workflow_id: workflow_id.into(),
            error: error.into(),
            execution_id: execution_id.into(),
        }
    }

    /// Create a RuntimeSnapshot event
    pub fn runtime_snapshot(
        workflow_id: impl Into<String>,
        execution_id: impl Into<String>,
        captured_at_ms: u64,
        capabilities: Option<WorkflowCapabilitiesResponse>,
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
        error: Option<String>,
    ) -> Self {
        Self::RuntimeSnapshot {
            workflow_id: workflow_id.into(),
            execution_id: execution_id.into(),
            captured_at_ms,
            capabilities,
            trace_runtime_metrics,
            error,
        }
    }

    /// Create a SchedulerSnapshot event
    pub fn scheduler_snapshot(
        workflow_id: Option<String>,
        execution_id: impl Into<String>,
        session_id: impl Into<String>,
        captured_at_ms: u64,
        session: Option<WorkflowSessionSummary>,
        items: Vec<WorkflowSessionQueueItem>,
        error: Option<String>,
    ) -> Self {
        Self::SchedulerSnapshot {
            workflow_id,
            execution_id: execution_id.into(),
            session_id: session_id.into(),
            captured_at_ms,
            session,
            items,
            error,
        }
    }

    /// Create a DiagnosticsSnapshot event
    pub fn diagnostics_snapshot(
        execution_id: impl Into<String>,
        snapshot: WorkflowDiagnosticsProjection,
    ) -> Self {
        Self::DiagnosticsSnapshot {
            execution_id: execution_id.into(),
            snapshot,
        }
    }
}

pub(super) fn is_cancelled_error_message(error: &str) -> bool {
    let normalized = error.trim().to_ascii_lowercase();
    normalized.contains("cancelled") || normalized.contains("canceled")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = WorkflowEvent::started("test-123", 5, "exec-123");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Started"));
        assert!(json.contains("test-123"));
        assert!(json.contains("5"));
        assert!(json.contains("exec-123"));
    }

    #[test]
    fn test_node_stream_event() {
        let event = WorkflowEvent::node_stream(
            "node1",
            "output",
            serde_json::json!({"text": "hello"}),
            "exec-123",
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("NodeStream"));
        assert!(json.contains("hello"));
        assert!(json.contains("exec-123"));
    }

    #[test]
    fn test_runtime_snapshot_event() {
        let event = WorkflowEvent::runtime_snapshot(
            "workflow-123",
            "exec-123",
            1234,
            None,
            WorkflowTraceRuntimeMetrics::default(),
            Some("capability unavailable".to_string()),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RuntimeSnapshot"));
        assert!(json.contains("workflow-123"));
        assert!(json.contains("1234"));
        assert!(json.contains("capability unavailable"));
    }

    #[test]
    fn cancelled_error_message_helper_matches_common_cancelled_forms() {
        assert!(is_cancelled_error_message("Workflow cancelled"));
        assert!(is_cancelled_error_message(
            "workflow run cancelled during execution"
        ));
        assert!(is_cancelled_error_message(
            "workflow canceled before dispatch"
        ));
        assert!(!is_cancelled_error_message("runtime unavailable"));
    }
}
