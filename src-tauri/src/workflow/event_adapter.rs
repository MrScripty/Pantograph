//! Event adapter for converting node-engine events to Tauri channel events
//!
//! This module bridges the gap between node-engine's generic EventSink trait
//! and Tauri's Channel-based event streaming to the frontend.

use std::collections::HashMap;

use node_engine::{EventError, EventSink};
use tauri::ipc::Channel;

use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::events::{is_cancelled_error_message, WorkflowEvent as TauriWorkflowEvent};

/// A value that flows through a port (alias for serde_json::Value)
type PortValue = serde_json::Value;

/// Adapter that converts node-engine WorkflowEvents to Tauri WorkflowEvents
/// and sends them through a Tauri channel to the frontend.
pub struct TauriEventAdapter {
    channel: Channel<TauriWorkflowEvent>,
    diagnostics_store: SharedWorkflowDiagnosticsStore,
}

impl TauriEventAdapter {
    /// Create a new adapter with the given Tauri channel and workflow ID
    pub fn new(
        channel: Channel<TauriWorkflowEvent>,
        workflow_id: impl Into<String>,
        diagnostics_store: SharedWorkflowDiagnosticsStore,
    ) -> Self {
        let _ = workflow_id.into();
        Self {
            channel,
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

fn translate_node_event(event: node_engine::WorkflowEvent) -> TauriWorkflowEvent {
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
            execution_id,
            dirty_tasks,
        } => TauriWorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
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
    event: node_engine::WorkflowEvent,
) -> (TauriWorkflowEvent, TauriWorkflowEvent) {
    let tauri_event = translate_node_event(event);
    let diagnostics_snapshot = diagnostics_store.record_workflow_event_now(&tauri_event);
    let diagnostics_event = TauriWorkflowEvent::diagnostics_snapshot(
        translated_execution_id(&tauri_event).to_string(),
        diagnostics_snapshot,
    );

    (tauri_event, diagnostics_event)
}

impl EventSink for TauriEventAdapter {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), EventError> {
        let (tauri_event, diagnostics_event) =
            translate_node_event_with_diagnostics(&self.diagnostics_store, event);

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
    use std::sync::{Arc, Mutex};

    use node_engine::EventSink;
    use serde_json::Value;
    use tauri::ipc::{Channel, InvokeResponseBody};

    use super::{translate_node_event_with_diagnostics, translated_execution_id};
    use crate::workflow::WorkflowDiagnosticsStore;

    #[test]
    fn translated_workflow_started_event_preserves_engine_execution_id() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let (event, diagnostics_event) = translate_node_event_with_diagnostics(
            &diagnostics_store,
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
            },
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
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
            },
        );

        let (_, diagnostics_event) = translate_node_event_with_diagnostics(
            &diagnostics_store,
            node_engine::WorkflowEvent::TaskProgress {
                task_id: "node-a".to_string(),
                execution_id: "exec-1".to_string(),
                progress: 0.5,
                message: Some("working".to_string()),
            },
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
            node_engine::WorkflowEvent::WorkflowFailed {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
                error: "workflow run cancelled during execution".to_string(),
            },
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

    #[test]
    fn translated_graph_modified_event_preserves_engine_execution_id() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let (event, diagnostics_event) = translate_node_event_with_diagnostics(
            &diagnostics_store,
            node_engine::WorkflowEvent::GraphModified {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-graph".to_string(),
                dirty_tasks: vec!["node-a".to_string(), "node-b".to_string()],
            },
        );

        match &event {
            super::TauriWorkflowEvent::GraphModified {
                workflow_id,
                execution_id,
                dirty_tasks,
                ..
            } => {
                assert_eq!(workflow_id, "wf-1");
                assert_eq!(execution_id, "exec-graph");
                assert_eq!(
                    dirty_tasks,
                    &vec!["node-a".to_string(), "node-b".to_string()]
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }

        assert_eq!(translated_execution_id(&event), "exec-graph");
        match diagnostics_event {
            super::TauriWorkflowEvent::DiagnosticsSnapshot {
                execution_id,
                snapshot,
            } => {
                assert_eq!(execution_id, "exec-graph");
                assert_eq!(snapshot.run_order, vec!["exec-graph".to_string()]);
            }
            other => panic!("unexpected diagnostics event: {other:?}"),
        }
    }

    #[test]
    fn adapter_send_emits_primary_and_diagnostics_events() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let emitted = Arc::new(Mutex::new(Vec::<Value>::new()));
        let captured = emitted.clone();
        let channel: Channel<super::TauriWorkflowEvent> = Channel::new(move |body| {
            let value = match body {
                InvokeResponseBody::Json(json) => {
                    serde_json::from_str::<Value>(&json).expect("channel event json")
                }
                InvokeResponseBody::Raw(bytes) => {
                    serde_json::from_slice::<Value>(&bytes).expect("channel event raw json")
                }
            };
            captured.lock().expect("captured events lock").push(value);
            Ok(())
        });
        let adapter = super::TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store);

        EventSink::send(
            &adapter,
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: "wf-1".to_string(),
                execution_id: "exec-1".to_string(),
            },
        )
        .expect("send should succeed");

        let events = emitted.lock().expect("captured events lock");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["type"], "Started");
        assert_eq!(events[0]["data"]["execution_id"], "exec-1");
        assert_eq!(events[1]["type"], "DiagnosticsSnapshot");
        assert_eq!(events[1]["data"]["execution_id"], "exec-1");
        assert_eq!(events[1]["data"]["snapshot"]["runOrder"][0], "exec-1");
    }
}
