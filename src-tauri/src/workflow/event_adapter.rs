//! Event adapter for converting node-engine events to Tauri channel events
//!
//! This module bridges the gap between node-engine's generic EventSink trait
//! and Tauri's Channel-based event streaming to the frontend.

use std::collections::HashMap;

use node_engine::{EventError, EventSink};
use tauri::ipc::Channel;

use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::events::{WorkflowEvent as TauriWorkflowEvent, is_cancelled_error_message};

/// A value that flows through a port (alias for serde_json::Value)
type PortValue = serde_json::Value;

/// Adapter that converts node-engine WorkflowEvents to Tauri WorkflowEvents
/// and sends them through a Tauri channel to the frontend.
pub struct TauriEventAdapter {
    channel: Channel<TauriWorkflowEvent>,
    workflow_id: String,
    diagnostics_store: SharedWorkflowDiagnosticsStore,
}

impl TauriEventAdapter {
    /// Create a new adapter with the given Tauri channel and workflow ID
    pub fn new(
        channel: Channel<TauriWorkflowEvent>,
        workflow_id: impl Into<String>,
        diagnostics_store: SharedWorkflowDiagnosticsStore,
    ) -> Self {
        Self {
            channel,
            workflow_id: workflow_id.into(),
            diagnostics_store,
        }
    }
}

fn translated_execution_id(event: &TauriWorkflowEvent) -> &str {
    match event {
        TauriWorkflowEvent::Started { execution_id, .. }
        | TauriWorkflowEvent::NodeStarted { execution_id, .. }
        | TauriWorkflowEvent::NodeProgress { execution_id, .. }
        | TauriWorkflowEvent::NodeStream { execution_id, .. }
        | TauriWorkflowEvent::NodeCompleted { execution_id, .. }
        | TauriWorkflowEvent::NodeError { execution_id, .. }
        | TauriWorkflowEvent::Completed { execution_id, .. }
        | TauriWorkflowEvent::Failed { execution_id, .. }
        | TauriWorkflowEvent::Cancelled { execution_id, .. }
        | TauriWorkflowEvent::GraphModified { execution_id, .. }
        | TauriWorkflowEvent::WaitingForInput { execution_id, .. }
        | TauriWorkflowEvent::IncrementalExecutionStarted { execution_id, .. }
        | TauriWorkflowEvent::RuntimeSnapshot { execution_id, .. }
        | TauriWorkflowEvent::SchedulerSnapshot { execution_id, .. }
        | TauriWorkflowEvent::DiagnosticsSnapshot { execution_id, .. } => execution_id,
    }
}

fn translate_node_event(
    adapter_workflow_id: &str,
    event: node_engine::WorkflowEvent,
) -> TauriWorkflowEvent {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            execution_id,
        } => {
            // Note: node_engine doesn't have node_count, so we send 0.
            TauriWorkflowEvent::Started {
                workflow_id,
                node_count: 0,
                execution_id,
            }
        }

        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            execution_id,
        } => {
            // Actual outputs are retrieved separately.
            TauriWorkflowEvent::Completed {
                workflow_id,
                outputs: HashMap::new(),
                execution_id,
            }
        }

        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id,
            error,
        } => {
            if is_cancelled_error_message(&error) {
                TauriWorkflowEvent::Cancelled {
                    workflow_id,
                    error,
                    execution_id,
                }
            } else {
                TauriWorkflowEvent::Failed {
                    workflow_id,
                    error,
                    execution_id,
                }
            }
        }

        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id,
        } => TauriWorkflowEvent::NodeStarted {
            node_id: task_id,
            node_type: String::new(),
            execution_id,
        },

        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id,
            output,
        } => {
            let outputs: HashMap<String, PortValue> = output
                .and_then(|value| value.as_object().cloned())
                .map(|object| object.into_iter().collect())
                .unwrap_or_default();

            TauriWorkflowEvent::NodeCompleted {
                node_id: task_id,
                outputs,
                execution_id,
            }
        }

        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id,
            error,
        } => TauriWorkflowEvent::NodeError {
            node_id: task_id,
            error,
            execution_id,
        },

        node_engine::WorkflowEvent::TaskProgress {
            task_id,
            execution_id,
            progress,
            message,
        } => TauriWorkflowEvent::NodeProgress {
            node_id: task_id,
            progress,
            message,
            execution_id,
        },

        node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id,
            port,
            data,
        } => TauriWorkflowEvent::NodeStream {
            node_id: task_id,
            port,
            chunk: data,
            execution_id,
        },

        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            task_id,
            prompt,
        } => TauriWorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            node_id: task_id,
            message: prompt,
        },

        node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            dirty_tasks,
        } => TauriWorkflowEvent::GraphModified {
            workflow_id,
            execution_id: adapter_workflow_id.to_string(),
            graph: None,
            dirty_tasks,
        },

        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            tasks,
        } => TauriWorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            task_ids: tasks,
        },
    }
}

fn translate_node_event_with_diagnostics(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    adapter_workflow_id: &str,
    event: node_engine::WorkflowEvent,
    timestamp_ms: u64,
) -> (TauriWorkflowEvent, TauriWorkflowEvent) {
    let tauri_event = translate_node_event(adapter_workflow_id, event);
    let diagnostics_snapshot = diagnostics_store.record_workflow_event(&tauri_event, timestamp_ms);
    let diagnostics_event = TauriWorkflowEvent::diagnostics_snapshot(
        translated_execution_id(&tauri_event).to_string(),
        diagnostics_snapshot,
    );

    (tauri_event, diagnostics_event)
}

impl EventSink for TauriEventAdapter {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), EventError> {
        let (tauri_event, diagnostics_event) = translate_node_event_with_diagnostics(
            &self.diagnostics_store,
            &self.workflow_id,
            event,
            super::workflow_execution_commands::unix_timestamp_ms(),
        );

        self.channel
            .send(tauri_event)
            .map_err(|_| EventError::channel_closed())
            .and_then(|_| {
                self.channel
                    .send(diagnostics_event)
                    .map_err(|_| EventError::channel_closed())
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{translate_node_event_with_diagnostics, translated_execution_id};
    use crate::workflow::WorkflowDiagnosticsStore;

    #[test]
    fn translated_workflow_started_event_preserves_engine_execution_id() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let (event, diagnostics_event) = translate_node_event_with_diagnostics(
            &diagnostics_store,
            "adapter-workflow",
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
            },
            100,
        );

        match &event {
            super::TauriWorkflowEvent::Started {
                workflow_id,
                execution_id,
                ..
            } => {
                assert_eq!(workflow_id, "wf-1");
                assert_eq!(execution_id, "exec-1");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        assert_eq!(translated_execution_id(&event), "exec-1");
        match diagnostics_event {
            super::TauriWorkflowEvent::DiagnosticsSnapshot {
                execution_id,
                snapshot,
            } => {
                assert_eq!(execution_id, "exec-1");
                assert_eq!(snapshot.run_order, vec!["exec-1".to_string()]);
            }
            other => panic!("unexpected diagnostics event: {other:?}"),
        }
    }

    #[test]
    fn translated_task_progress_event_updates_backend_diagnostics_projection() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let _ = translate_node_event_with_diagnostics(
            &diagnostics_store,
            "adapter-workflow",
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
            },
            100,
        );

        let (_, diagnostics_event) = translate_node_event_with_diagnostics(
            &diagnostics_store,
            "adapter-workflow",
            node_engine::WorkflowEvent::TaskProgress {
                task_id: "node-a".to_string(),
                execution_id: "exec-1".to_string(),
                progress: 0.5,
                message: Some("working".to_string()),
            },
            110,
        );

        match diagnostics_event {
            super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
                let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
                let node = trace.nodes.get("node-a").expect("node overlay");
                assert_eq!(node.last_progress, Some(0.5));
                assert_eq!(node.last_message.as_deref(), Some("working"));
            }
            other => panic!("unexpected diagnostics event: {other:?}"),
        }
    }

    #[test]
    fn translated_workflow_failed_event_maps_cancelled_errors_to_cancelled_event() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let (event, diagnostics_event) = translate_node_event_with_diagnostics(
            &diagnostics_store,
            "adapter-workflow",
            node_engine::WorkflowEvent::WorkflowFailed {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
                error: "workflow run cancelled during execution".to_string(),
            },
            120,
        );

        match event {
            super::TauriWorkflowEvent::Cancelled {
                workflow_id,
                execution_id,
                error,
            } => {
                assert_eq!(workflow_id, "wf-1");
                assert_eq!(execution_id, "exec-1");
                assert!(error.contains("cancelled"));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        match diagnostics_event {
            super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
                let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
                assert_eq!(
                    trace.status,
                    crate::workflow::diagnostics::DiagnosticsRunStatus::Cancelled
                );
            }
            other => panic!("unexpected diagnostics event: {other:?}"),
        }
    }
}
