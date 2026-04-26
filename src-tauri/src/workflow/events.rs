//! Workflow events for streaming updates to the frontend
//!
//! These events are sent via Tauri channels to provide real-time
//! feedback on workflow execution progress.

use std::collections::HashMap;

use pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionSummary, WorkflowGraph, WorkflowSchedulerSnapshotDiagnostics,
    WorkflowTraceRuntimeMetrics,
};

use super::diagnostics::{DiagnosticsRuntimeLifecycleSnapshot, WorkflowDiagnosticsProjection};

#[path = "event_serialization.rs"]
pub(crate) mod event_serialization;

/// A value that flows through a port (alias for serde_json::Value)
pub type PortValue = serde_json::Value;

#[derive(Debug, Clone)]
pub struct WorkflowRuntimeSnapshotEventInput {
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub captured_at_ms: u64,
    pub capabilities: Option<WorkflowCapabilitiesResponse>,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowSchedulerSnapshotEventInput {
    pub workflow_id: Option<String>,
    pub workflow_run_id: String,
    pub session_id: String,
    pub captured_at_ms: u64,
    pub session: Option<WorkflowExecutionSessionSummary>,
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
    pub diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
    pub error: Option<String>,
}

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

    /// Runtime capabilities snapshot captured during execution
    RuntimeSnapshot {
        /// Workflow identifier associated with this run
        workflow_id: String,
        /// Unique identifier for this execution
        workflow_run_id: String,
        /// Millisecond unix timestamp for when the snapshot was captured
        captured_at_ms: u64,
        /// Runtime capabilities and requirements when available
        capabilities: Box<Option<WorkflowCapabilitiesResponse>>,
        /// Backend-owned runtime lifecycle metrics captured alongside the snapshot
        trace_runtime_metrics: Box<WorkflowTraceRuntimeMetrics>,
        /// Backend-owned active runtime model target at capture time
        active_model_target: Option<String>,
        /// Backend-owned embedding runtime model target at capture time
        embedding_model_target: Option<String>,
        /// Backend-owned lifecycle snapshot for the active runtime at capture time
        active_runtime_snapshot: Option<DiagnosticsRuntimeLifecycleSnapshot>,
        /// Backend-owned lifecycle snapshot for the dedicated embedding runtime when available
        embedding_runtime_snapshot: Option<DiagnosticsRuntimeLifecycleSnapshot>,
        /// Backend-owned managed-runtime manager views captured alongside workflow runtime diagnostics
        managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
        /// Error encountered while collecting the runtime snapshot
        error: Option<String>,
    },

    /// Scheduler/session snapshot captured during execution
    SchedulerSnapshot {
        /// Workflow identifier associated with this run
        workflow_id: Option<String>,
        /// Unique identifier for this execution
        workflow_run_id: String,
        /// Session identifier the snapshot belongs to
        session_id: String,
        /// Millisecond unix timestamp for when the snapshot was captured
        captured_at_ms: u64,
        /// Session summary when available
        session: Option<WorkflowExecutionSessionSummary>,
        /// Queue items visible at capture time
        items: Vec<WorkflowExecutionSessionQueueItem>,
        /// Additive backend-owned scheduler diagnostics
        diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
        /// Error encountered while collecting the scheduler snapshot
        error: Option<String>,
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
    /// Create a RuntimeSnapshot event
    pub fn runtime_snapshot(input: WorkflowRuntimeSnapshotEventInput) -> Self {
        Self::RuntimeSnapshot {
            workflow_id: input.workflow_id,
            workflow_run_id: input.workflow_run_id,
            captured_at_ms: input.captured_at_ms,
            capabilities: Box::new(input.capabilities),
            trace_runtime_metrics: Box::new(input.trace_runtime_metrics),
            active_model_target: input.active_model_target,
            embedding_model_target: input.embedding_model_target,
            active_runtime_snapshot: input
                .active_runtime_snapshot
                .as_ref()
                .map(DiagnosticsRuntimeLifecycleSnapshot::from),
            embedding_runtime_snapshot: input
                .embedding_runtime_snapshot
                .as_ref()
                .map(DiagnosticsRuntimeLifecycleSnapshot::from),
            managed_runtimes: input.managed_runtimes,
            error: input.error,
        }
    }

    /// Create a SchedulerSnapshot event
    pub fn scheduler_snapshot(input: WorkflowSchedulerSnapshotEventInput) -> Self {
        Self::SchedulerSnapshot {
            workflow_id: input.workflow_id,
            workflow_run_id: input.workflow_run_id,
            session_id: input.session_id,
            captured_at_ms: input.captured_at_ms,
            session: input.session,
            items: input.items,
            diagnostics: input.diagnostics,
            error: input.error,
        }
    }

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

    #[test]
    fn test_runtime_snapshot_event() {
        let event = WorkflowEvent::runtime_snapshot(WorkflowRuntimeSnapshotEventInput {
            workflow_id: "workflow-123".to_string(),
            workflow_run_id: "exec-123".to_string(),
            captured_at_ms: 1234,
            capabilities: None,
            trace_runtime_metrics: WorkflowTraceRuntimeMetrics::default(),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime_snapshot: None,
            embedding_runtime_snapshot: None,
            managed_runtimes: Vec::new(),
            error: Some("capability unavailable".to_string()),
        });
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RuntimeSnapshot"));
        assert!(json.contains("workflow-123"));
        assert!(json.contains("1234"));
        assert!(json.contains("/models/main.gguf"));
        assert!(json.contains("/models/embed.gguf"));
        assert!(json.contains("capability unavailable"));
    }
}
