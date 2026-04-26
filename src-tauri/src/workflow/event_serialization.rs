use serde::{Serialize, Serializer, ser::Error as _};
use serde_json::{Map, Value, json};

use super::WorkflowEvent;

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
        let ownership =
            serde_json::to_value(self.ownership_projection()).map_err(S::Error::custom)?;
        if let Value::Object(fields) = &mut data {
            fields.insert("ownership".to_string(), ownership);
        }

        let mut envelope = Map::new();
        envelope.insert("type".to_string(), Value::String(event_type.to_string()));
        envelope.insert("data".to_string(), data);
        envelope.serialize(serializer)
    }
}

impl WorkflowEvent {
    pub fn ownership_projection(&self) -> WorkflowEventOwnershipProjection {
        WorkflowEventOwnershipProjection::from_workflow_run_id(self.workflow_run_id())
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
}
