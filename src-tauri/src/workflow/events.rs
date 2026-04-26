//! Workflow events for streaming updates to the frontend
//!
//! These events are sent via Tauri channels to provide real-time
//! feedback on workflow execution progress.

use serde::{Serialize, Serializer, ser::Error as _};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

use pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionSummary, WorkflowGraph, WorkflowSchedulerSnapshotDiagnostics,
    WorkflowTraceRuntimeMetrics,
};

use super::diagnostics::{DiagnosticsRuntimeLifecycleSnapshot, WorkflowDiagnosticsProjection};

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEventOwnershipProjection {
    pub event_workflow_run_id: String,
    pub active_workflow_run_id: String,
    pub relevant: bool,
}

impl WorkflowEventOwnershipProjection {
    fn from_workflow_run_id(workflow_run_id: &str) -> Self {
        Self {
            event_workflow_run_id: workflow_run_id.to_string(),
            active_workflow_run_id: workflow_run_id.to_string(),
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
        Some(WorkflowEventOwnershipProjection::from_workflow_run_id(
            self.workflow_run_id(),
        ))
    }

    fn workflow_run_id(&self) -> &str {
        match self {
            Self::Started {
                workflow_run_id, ..
            }
            | Self::NodeStarted {
                workflow_run_id, ..
            }
            | Self::NodeProgress {
                workflow_run_id, ..
            }
            | Self::NodeStream {
                workflow_run_id, ..
            }
            | Self::NodeCompleted {
                workflow_run_id, ..
            }
            | Self::NodeError {
                workflow_run_id, ..
            }
            | Self::Completed {
                workflow_run_id, ..
            }
            | Self::Failed {
                workflow_run_id, ..
            }
            | Self::Cancelled {
                workflow_run_id, ..
            }
            | Self::GraphModified {
                workflow_run_id, ..
            }
            | Self::WaitingForInput {
                workflow_run_id, ..
            }
            | Self::IncrementalExecutionStarted {
                workflow_run_id, ..
            }
            | Self::RuntimeSnapshot {
                workflow_run_id, ..
            }
            | Self::SchedulerSnapshot {
                workflow_run_id, ..
            }
            | Self::DiagnosticsSnapshot {
                workflow_run_id, ..
            } => workflow_run_id,
        }
    }

    fn serialized_parts(&self) -> (&'static str, Value) {
        match self {
            Self::Started {
                workflow_id,
                node_count,
                workflow_run_id,
            } => (
                "Started",
                json!({
                    "workflow_id": workflow_id,
                    "node_count": node_count,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::NodeStarted {
                node_id,
                node_type,
                workflow_run_id,
            } => (
                "NodeStarted",
                json!({
                    "node_id": node_id,
                    "node_type": node_type,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::NodeProgress {
                node_id,
                progress,
                message,
                detail,
                workflow_run_id,
            } => (
                "NodeProgress",
                json!({
                    "node_id": node_id,
                    "progress": progress,
                    "message": message,
                    "detail": detail,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::NodeStream {
                node_id,
                port,
                chunk,
                workflow_run_id,
            } => (
                "NodeStream",
                json!({
                    "node_id": node_id,
                    "port": port,
                    "chunk": chunk,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::NodeCompleted {
                node_id,
                outputs,
                workflow_run_id,
            } => (
                "NodeCompleted",
                json!({
                    "node_id": node_id,
                    "outputs": outputs,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::NodeError {
                node_id,
                error,
                workflow_run_id,
            } => (
                "NodeError",
                json!({
                    "node_id": node_id,
                    "error": error,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::Completed {
                workflow_id,
                outputs,
                workflow_run_id,
            } => (
                "Completed",
                json!({
                    "workflow_id": workflow_id,
                    "outputs": outputs,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::Failed {
                workflow_id,
                error,
                workflow_run_id,
            } => (
                "Failed",
                json!({
                    "workflow_id": workflow_id,
                    "error": error,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::Cancelled {
                workflow_id,
                error,
                workflow_run_id,
            } => (
                "Cancelled",
                json!({
                    "workflow_id": workflow_id,
                    "error": error,
                    "workflow_run_id": workflow_run_id,
                }),
            ),
            Self::GraphModified {
                workflow_id,
                workflow_run_id,
                graph,
                dirty_tasks,
                memory_impact,
            } => (
                "GraphModified",
                json!({
                    "workflow_id": workflow_id,
                    "workflow_run_id": workflow_run_id,
                    "graph": graph,
                    "dirty_tasks": dirty_tasks,
                    "memory_impact": memory_impact,
                }),
            ),
            Self::WaitingForInput {
                workflow_id,
                workflow_run_id,
                node_id,
                message,
            } => (
                "WaitingForInput",
                json!({
                    "workflow_id": workflow_id,
                    "workflow_run_id": workflow_run_id,
                    "node_id": node_id,
                    "message": message,
                }),
            ),
            Self::IncrementalExecutionStarted {
                workflow_id,
                workflow_run_id,
                task_ids,
            } => (
                "IncrementalExecutionStarted",
                json!({
                    "workflow_id": workflow_id,
                    "workflow_run_id": workflow_run_id,
                    "task_ids": task_ids,
                }),
            ),
            Self::RuntimeSnapshot {
                workflow_id,
                workflow_run_id,
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
                    "workflow_run_id": workflow_run_id,
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
                workflow_run_id,
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
                    "workflow_run_id": workflow_run_id,
                    "session_id": session_id,
                    "captured_at_ms": captured_at_ms,
                    "session": session,
                    "items": items,
                    "diagnostics": diagnostics,
                    "error": error,
                }),
            ),
            Self::DiagnosticsSnapshot {
                workflow_run_id,
                snapshot,
            } => (
                "DiagnosticsSnapshot",
                json!({
                    "workflow_run_id": workflow_run_id,
                    "snapshot": snapshot,
                }),
            ),
        }
    }

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
