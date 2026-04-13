//! Event adapter for converting node-engine events to Tauri channel events
//!
//! This module bridges the gap between node-engine's generic EventSink trait
//! and Tauri's Channel-based event streaming to the frontend.

use std::collections::HashMap;

use node_engine::{EventError, EventSink};
use tauri::ipc::Channel;

use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::events::WorkflowEvent as TauriWorkflowEvent;

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

impl EventSink for TauriEventAdapter {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), EventError> {
        let tauri_event = match event {
            node_engine::WorkflowEvent::WorkflowStarted { .. } => {
                // Note: node_engine doesn't have node_count, but we send 0
                // The frontend can ignore this if needed
                TauriWorkflowEvent::Started {
                    workflow_id: self.workflow_id.clone(),
                    node_count: 0,
                    execution_id: self.workflow_id.clone(),
                }
            }

            node_engine::WorkflowEvent::WorkflowCompleted { .. } => {
                // For now, send empty outputs - the actual outputs are retrieved separately
                TauriWorkflowEvent::Completed {
                    workflow_id: self.workflow_id.clone(),
                    outputs: HashMap::new(),
                    execution_id: self.workflow_id.clone(),
                }
            }

            node_engine::WorkflowEvent::WorkflowFailed { error, .. } => {
                TauriWorkflowEvent::Failed {
                    workflow_id: self.workflow_id.clone(),
                    error,
                    execution_id: self.workflow_id.clone(),
                }
            }

            node_engine::WorkflowEvent::TaskStarted { task_id, .. } => {
                TauriWorkflowEvent::NodeStarted {
                    node_id: task_id.clone(),
                    node_type: String::new(), // We don't have the type here
                    execution_id: self.workflow_id.clone(),
                }
            }

            node_engine::WorkflowEvent::TaskCompleted {
                task_id, output, ..
            } => {
                // Convert the output to HashMap<String, PortValue>
                let outputs: HashMap<String, PortValue> = output
                    .and_then(|v| v.as_object().cloned())
                    .map(|obj| obj.into_iter().collect())
                    .unwrap_or_default();

                TauriWorkflowEvent::NodeCompleted {
                    node_id: task_id,
                    outputs,
                    execution_id: self.workflow_id.clone(),
                }
            }

            node_engine::WorkflowEvent::TaskFailed { task_id, error, .. } => {
                TauriWorkflowEvent::NodeError {
                    node_id: task_id,
                    error,
                    execution_id: self.workflow_id.clone(),
                }
            }

            node_engine::WorkflowEvent::TaskProgress {
                task_id,
                progress,
                message,
                ..
            } => TauriWorkflowEvent::NodeProgress {
                node_id: task_id,
                progress,
                message,
                execution_id: self.workflow_id.clone(),
            },

            node_engine::WorkflowEvent::TaskStream {
                task_id,
                port,
                data,
                ..
            } => TauriWorkflowEvent::NodeStream {
                node_id: task_id,
                port,
                chunk: data,
                execution_id: self.workflow_id.clone(),
            },

            node_engine::WorkflowEvent::WaitingForInput {
                task_id, prompt, ..
            } => TauriWorkflowEvent::WaitingForInput {
                workflow_id: self.workflow_id.clone(),
                execution_id: self.workflow_id.clone(),
                node_id: task_id,
                message: prompt,
            },

            node_engine::WorkflowEvent::GraphModified { dirty_tasks, .. } => {
                TauriWorkflowEvent::GraphModified {
                    workflow_id: self.workflow_id.clone(),
                    execution_id: self.workflow_id.clone(),
                    graph: None,
                    dirty_tasks,
                }
            }

            node_engine::WorkflowEvent::IncrementalExecutionStarted { tasks, .. } => {
                TauriWorkflowEvent::IncrementalExecutionStarted {
                    workflow_id: self.workflow_id.clone(),
                    execution_id: self.workflow_id.clone(),
                    task_ids: tasks,
                }
            }
        };

        let diagnostics_snapshot = self.diagnostics_store.record_workflow_event(
            &tauri_event,
            super::workflow_execution_commands::unix_timestamp_ms(),
        );

        self.channel
            .send(tauri_event)
            .map_err(|_| EventError::channel_closed())
            .and_then(|_| {
                self.channel
                    .send(TauriWorkflowEvent::diagnostics_snapshot(
                        self.workflow_id.clone(),
                        diagnostics_snapshot,
                    ))
                    .map_err(|_| EventError::channel_closed())
            })
    }
}

#[cfg(test)]
mod tests {
    // Note: Testing requires mocking Tauri's Channel, which is complex.
    // These tests would be integration tests in practice.
}
