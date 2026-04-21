//! Workflow events for streaming updates to the frontend
//!
//! These events are sent via Tauri channels to provide real-time
//! feedback on workflow execution progress.

use serde::{Serialize, Serializer, ser::Error as _};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

use pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowSchedulerSnapshotDiagnostics, WorkflowSessionQueueItem,
    WorkflowSessionSummary, WorkflowTraceRuntimeMetrics,
};

use super::diagnostics::{DiagnosticsRuntimeLifecycleSnapshot, WorkflowDiagnosticsProjection};

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
        /// Optional backend-owned structured progress detail
        detail: Option<node_engine::TaskProgressDetail>,
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
        /// Backend-owned mutation impact for preserved vs invalidated node memory
        memory_impact: Option<node_engine::GraphMemoryImpactSummary>,
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
        execution_id: String,
        /// Session identifier the snapshot belongs to
        session_id: String,
        /// Millisecond unix timestamp for when the snapshot was captured
        captured_at_ms: u64,
        /// Session summary when available
        session: Option<WorkflowSessionSummary>,
        /// Queue items visible at capture time
        items: Vec<WorkflowSessionQueueItem>,
        /// Additive backend-owned scheduler diagnostics
        diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEventOwnershipProjection {
    pub event_execution_id: String,
    pub active_execution_id: String,
    pub relevant: bool,
}

impl WorkflowEventOwnershipProjection {
    fn from_execution_id(execution_id: &str) -> Self {
        Self {
            event_execution_id: execution_id.to_string(),
            active_execution_id: execution_id.to_string(),
            relevant: true,
        }
    }
}

impl Serialize for WorkflowEvent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (event_type, mut data) = self.serialized_parts();
        if let Some(ownership) = self.ownership_projection() {
            let ownership = serde_json::to_value(ownership).map_err(S::Error::custom)?;
            if let Value::Object(fields) = &mut data {
                fields.insert("ownership".to_string(), ownership);
            }
        }

        let mut envelope = Map::new();
        envelope.insert("type".to_string(), Value::String(event_type.to_string()));
        envelope.insert("data".to_string(), data);
        envelope.serialize(serializer)
    }
}

impl WorkflowEvent {
    pub fn ownership_projection(&self) -> Option<WorkflowEventOwnershipProjection> {
        Some(WorkflowEventOwnershipProjection::from_execution_id(
            self.execution_id(),
        ))
    }

    fn execution_id(&self) -> &str {
        match self {
            Self::Started { execution_id, .. }
            | Self::NodeStarted { execution_id, .. }
            | Self::NodeProgress { execution_id, .. }
            | Self::NodeStream { execution_id, .. }
            | Self::NodeCompleted { execution_id, .. }
            | Self::NodeError { execution_id, .. }
            | Self::Completed { execution_id, .. }
            | Self::Failed { execution_id, .. }
            | Self::Cancelled { execution_id, .. }
            | Self::GraphModified { execution_id, .. }
            | Self::WaitingForInput { execution_id, .. }
            | Self::IncrementalExecutionStarted { execution_id, .. }
            | Self::RuntimeSnapshot { execution_id, .. }
            | Self::SchedulerSnapshot { execution_id, .. }
            | Self::DiagnosticsSnapshot { execution_id, .. } => execution_id,
        }
    }

    fn serialized_parts(&self) -> (&'static str, Value) {
        match self {
            Self::Started {
                workflow_id,
                node_count,
                execution_id,
            } => (
                "Started",
                json!({
                    "workflow_id": workflow_id,
                    "node_count": node_count,
                    "execution_id": execution_id,
                }),
            ),
            Self::NodeStarted {
                node_id,
                node_type,
                execution_id,
            } => (
                "NodeStarted",
                json!({
                    "node_id": node_id,
                    "node_type": node_type,
                    "execution_id": execution_id,
                }),
            ),
            Self::NodeProgress {
                node_id,
                progress,
                message,
                detail,
                execution_id,
            } => (
                "NodeProgress",
                json!({
                    "node_id": node_id,
                    "progress": progress,
                    "message": message,
                    "detail": detail,
                    "execution_id": execution_id,
                }),
            ),
            Self::NodeStream {
                node_id,
                port,
                chunk,
                execution_id,
            } => (
                "NodeStream",
                json!({
                    "node_id": node_id,
                    "port": port,
                    "chunk": chunk,
                    "execution_id": execution_id,
                }),
            ),
            Self::NodeCompleted {
                node_id,
                outputs,
                execution_id,
            } => (
                "NodeCompleted",
                json!({
                    "node_id": node_id,
                    "outputs": outputs,
                    "execution_id": execution_id,
                }),
            ),
            Self::NodeError {
                node_id,
                error,
                execution_id,
            } => (
                "NodeError",
                json!({
                    "node_id": node_id,
                    "error": error,
                    "execution_id": execution_id,
                }),
            ),
            Self::Completed {
                workflow_id,
                outputs,
                execution_id,
            } => (
                "Completed",
                json!({
                    "workflow_id": workflow_id,
                    "outputs": outputs,
                    "execution_id": execution_id,
                }),
            ),
            Self::Failed {
                workflow_id,
                error,
                execution_id,
            } => (
                "Failed",
                json!({
                    "workflow_id": workflow_id,
                    "error": error,
                    "execution_id": execution_id,
                }),
            ),
            Self::Cancelled {
                workflow_id,
                error,
                execution_id,
            } => (
                "Cancelled",
                json!({
                    "workflow_id": workflow_id,
                    "error": error,
                    "execution_id": execution_id,
                }),
            ),
            Self::GraphModified {
                workflow_id,
                execution_id,
                graph,
                dirty_tasks,
                memory_impact,
            } => (
                "GraphModified",
                json!({
                    "workflow_id": workflow_id,
                    "execution_id": execution_id,
                    "graph": graph,
                    "dirty_tasks": dirty_tasks,
                    "memory_impact": memory_impact,
                }),
            ),
            Self::WaitingForInput {
                workflow_id,
                execution_id,
                node_id,
                message,
            } => (
                "WaitingForInput",
                json!({
                    "workflow_id": workflow_id,
                    "execution_id": execution_id,
                    "node_id": node_id,
                    "message": message,
                }),
            ),
            Self::IncrementalExecutionStarted {
                workflow_id,
                execution_id,
                task_ids,
            } => (
                "IncrementalExecutionStarted",
                json!({
                    "workflow_id": workflow_id,
                    "execution_id": execution_id,
                    "task_ids": task_ids,
                }),
            ),
            Self::RuntimeSnapshot {
                workflow_id,
                execution_id,
                captured_at_ms,
                capabilities,
                trace_runtime_metrics,
                active_model_target,
                embedding_model_target,
                active_runtime_snapshot,
                embedding_runtime_snapshot,
                managed_runtimes,
                error,
            } => (
                "RuntimeSnapshot",
                json!({
                    "workflow_id": workflow_id,
                    "execution_id": execution_id,
                    "captured_at_ms": captured_at_ms,
                    "capabilities": capabilities,
                    "trace_runtime_metrics": trace_runtime_metrics,
                    "active_model_target": active_model_target,
                    "embedding_model_target": embedding_model_target,
                    "active_runtime_snapshot": active_runtime_snapshot,
                    "embedding_runtime_snapshot": embedding_runtime_snapshot,
                    "managed_runtimes": managed_runtimes,
                    "error": error,
                }),
            ),
            Self::SchedulerSnapshot {
                workflow_id,
                execution_id,
                session_id,
                captured_at_ms,
                session,
                items,
                diagnostics,
                error,
            } => (
                "SchedulerSnapshot",
                json!({
                    "workflow_id": workflow_id,
                    "execution_id": execution_id,
                    "session_id": session_id,
                    "captured_at_ms": captured_at_ms,
                    "session": session,
                    "items": items,
                    "diagnostics": diagnostics,
                    "error": error,
                }),
            ),
            Self::DiagnosticsSnapshot {
                execution_id,
                snapshot,
            } => (
                "DiagnosticsSnapshot",
                json!({
                    "execution_id": execution_id,
                    "snapshot": snapshot,
                }),
            ),
        }
    }

    /// Create a RuntimeSnapshot event
    pub fn runtime_snapshot(
        workflow_id: impl Into<String>,
        execution_id: impl Into<String>,
        captured_at_ms: u64,
        capabilities: Option<WorkflowCapabilitiesResponse>,
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
        active_model_target: Option<String>,
        embedding_model_target: Option<String>,
        active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
        embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
        managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
        error: Option<String>,
    ) -> Self {
        Self::RuntimeSnapshot {
            workflow_id: workflow_id.into(),
            execution_id: execution_id.into(),
            captured_at_ms,
            capabilities,
            trace_runtime_metrics,
            active_model_target,
            embedding_model_target,
            active_runtime_snapshot: active_runtime_snapshot
                .as_ref()
                .map(DiagnosticsRuntimeLifecycleSnapshot::from),
            embedding_runtime_snapshot: embedding_runtime_snapshot
                .as_ref()
                .map(DiagnosticsRuntimeLifecycleSnapshot::from),
            managed_runtimes,
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
        diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
        error: Option<String>,
    ) -> Self {
        Self::SchedulerSnapshot {
            workflow_id,
            execution_id: execution_id.into(),
            session_id: session_id.into(),
            captured_at_ms,
            session,
            items,
            diagnostics,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = WorkflowEvent::Started {
            workflow_id: "test-123".to_string(),
            node_count: 5,
            execution_id: "exec-123".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Started"));
        assert!(json.contains("test-123"));
        assert!(json.contains("5"));
        assert!(json.contains("exec-123"));
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(
            value["data"]["ownership"]["eventExecutionId"].as_str(),
            Some("exec-123")
        );
        assert_eq!(
            value["data"]["ownership"]["activeExecutionId"].as_str(),
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
            execution_id: "exec-123".to_string(),
        };
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
            Some("/models/main.gguf".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            Vec::new(),
            Some("capability unavailable".to_string()),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("RuntimeSnapshot"));
        assert!(json.contains("workflow-123"));
        assert!(json.contains("1234"));
        assert!(json.contains("/models/main.gguf"));
        assert!(json.contains("/models/embed.gguf"));
        assert!(json.contains("capability unavailable"));
    }
}
