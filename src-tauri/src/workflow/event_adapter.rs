//! Event adapter for converting node-engine events to Tauri channel events
//!
//! This module bridges the gap between node-engine's generic EventSink trait
//! and Tauri's Channel-based event streaming to the frontend.

use std::collections::HashMap;

use node_engine::{EventError, EventSink};
use tauri::ipc::Channel;

use super::events::WorkflowEvent as TauriWorkflowEvent;

/// A value that flows through a port (alias for serde_json::Value)
type PortValue = serde_json::Value;

/// Adapter that converts node-engine WorkflowEvents to Tauri WorkflowEvents
/// and sends them through a Tauri channel to the frontend.
pub struct TauriEventAdapter {
    channel: Channel<TauriWorkflowEvent>,
    workflow_id: String,
}

impl TauriEventAdapter {
    /// Create a new adapter with the given Tauri channel and workflow ID
    pub fn new(channel: Channel<TauriWorkflowEvent>, workflow_id: impl Into<String>) -> Self {
        Self {
            channel,
            workflow_id: workflow_id.into(),
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
                }
            }

            node_engine::WorkflowEvent::WorkflowCompleted { .. } => {
                // For now, send empty outputs - the actual outputs are retrieved separately
                TauriWorkflowEvent::Completed {
                    outputs: HashMap::new(),
                }
            }

            node_engine::WorkflowEvent::WorkflowFailed { error, .. } => {
                TauriWorkflowEvent::Failed { error }
            }

            node_engine::WorkflowEvent::TaskStarted { task_id, .. } => {
                TauriWorkflowEvent::NodeStarted {
                    node_id: task_id.clone(),
                    node_type: String::new(), // We don't have the type here
                }
            }

            node_engine::WorkflowEvent::TaskCompleted { task_id, output, .. } => {
                // Convert the output to HashMap<String, PortValue>
                let outputs: HashMap<String, PortValue> = output
                    .and_then(|v| v.as_object().cloned())
                    .map(|obj| obj.into_iter().collect())
                    .unwrap_or_default();

                TauriWorkflowEvent::NodeCompleted {
                    node_id: task_id,
                    outputs,
                }
            }

            node_engine::WorkflowEvent::TaskFailed { task_id, error, .. } => {
                TauriWorkflowEvent::NodeError {
                    node_id: task_id,
                    error,
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
            },

            node_engine::WorkflowEvent::WaitingForInput { task_id, prompt, .. } => {
                // Map to NodeProgress with a special message for now
                // TODO: Add proper WaitingForInput event to Tauri events
                TauriWorkflowEvent::NodeProgress {
                    node_id: task_id,
                    progress: 0.0,
                    message: Some(prompt.unwrap_or_else(|| "Waiting for input...".to_string())),
                }
            }

            node_engine::WorkflowEvent::GraphModified { dirty_tasks, .. } => {
                // Map to a progress event for now
                // TODO: Add proper GraphModified event to frontend
                log::debug!("Graph modified, dirty tasks: {:?}", dirty_tasks);
                return Ok(()); // Don't send this event to frontend yet
            }

            node_engine::WorkflowEvent::IncrementalExecutionStarted { tasks, .. } => {
                // Log but don't send to frontend
                log::debug!("Incremental execution started for tasks: {:?}", tasks);
                return Ok(());
            }
        };

        self.channel
            .send(tauri_event)
            .map_err(|_| EventError::channel_closed())
    }
}

#[cfg(test)]
mod tests {
    // Note: Testing requires mocking Tauri's Channel, which is complex.
    // These tests would be integration tests in practice.
}
