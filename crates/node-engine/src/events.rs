//! Event types for streaming workflow progress
//!
//! Events are sent from the engine to the frontend (or any consumer)
//! to report progress, streaming output, errors, and state changes.

use serde::{Deserialize, Serialize};

/// Trait for sending workflow events
///
/// This abstracts over the transport mechanism (Tauri channel, mpsc, etc.)
/// allowing the engine to be used in different contexts.
pub trait EventSink: Send + Sync {
    /// Send an event
    ///
    /// Returns an error if the event could not be sent (e.g., channel closed)
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError>;
}

/// Error when sending events fails
#[derive(Debug, Clone)]
pub struct EventError {
    pub message: String,
}

impl std::fmt::Display for EventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event error: {}", self.message)
    }
}

impl std::error::Error for EventError {}

impl EventError {
    pub fn channel_closed() -> Self {
        Self {
            message: "Channel closed".to_string(),
        }
    }
}

/// Events emitted during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowEvent {
    /// Workflow execution started
    #[serde(rename_all = "camelCase")]
    WorkflowStarted {
        workflow_id: String,
        execution_id: String,
    },

    /// Workflow execution completed successfully
    #[serde(rename_all = "camelCase")]
    WorkflowCompleted {
        workflow_id: String,
        execution_id: String,
    },

    /// Workflow execution failed
    #[serde(rename_all = "camelCase")]
    WorkflowFailed {
        workflow_id: String,
        execution_id: String,
        error: String,
    },

    /// Workflow is waiting for user input
    #[serde(rename_all = "camelCase")]
    WaitingForInput {
        workflow_id: String,
        execution_id: String,
        task_id: String,
        prompt: Option<String>,
    },

    /// A task started executing
    #[serde(rename_all = "camelCase")]
    TaskStarted {
        task_id: String,
        execution_id: String,
    },

    /// A task completed successfully
    #[serde(rename_all = "camelCase")]
    TaskCompleted {
        task_id: String,
        execution_id: String,
        output: Option<serde_json::Value>,
    },

    /// A task failed
    #[serde(rename_all = "camelCase")]
    TaskFailed {
        task_id: String,
        execution_id: String,
        error: String,
    },

    /// Progress update for a task
    #[serde(rename_all = "camelCase")]
    TaskProgress {
        task_id: String,
        execution_id: String,
        progress: f32,
        message: Option<String>,
    },

    /// Streaming output from a task
    #[serde(rename_all = "camelCase")]
    TaskStream {
        task_id: String,
        execution_id: String,
        port: String,
        data: serde_json::Value,
    },

    /// Graph was modified during execution
    #[serde(rename_all = "camelCase")]
    GraphModified {
        workflow_id: String,
        dirty_tasks: Vec<String>,
    },

    /// Incremental re-execution started
    #[serde(rename_all = "camelCase")]
    IncrementalExecutionStarted {
        workflow_id: String,
        execution_id: String,
        tasks: Vec<String>,
    },
}

impl WorkflowEvent {
    /// Create a task progress event
    pub fn task_progress(task_id: &str, execution_id: &str, progress: f32, message: Option<String>) -> Self {
        Self::TaskProgress {
            task_id: task_id.to_string(),
            execution_id: execution_id.to_string(),
            progress,
            message,
        }
    }

    /// Create a task stream event
    pub fn task_stream(task_id: &str, execution_id: &str, port: &str, data: serde_json::Value) -> Self {
        Self::TaskStream {
            task_id: task_id.to_string(),
            execution_id: execution_id.to_string(),
            port: port.to_string(),
            data,
        }
    }
}

/// A no-op event sink that discards all events
///
/// Useful for testing or when events aren't needed.
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn send(&self, _event: WorkflowEvent) -> Result<(), EventError> {
        Ok(())
    }
}

/// A vector-based event sink that collects events
///
/// Useful for testing to verify events were emitted correctly.
pub struct VecEventSink {
    events: std::sync::Mutex<Vec<WorkflowEvent>>,
}

impl VecEventSink {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Get all collected events
    pub fn events(&self) -> Vec<WorkflowEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear all collected events
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl Default for VecEventSink {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSink for VecEventSink {
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_event_sink() {
        let sink = VecEventSink::new();

        sink.send(WorkflowEvent::task_progress("task1", "exec1", 0.5, Some("halfway".to_string())))
            .unwrap();

        let events = sink.events();
        assert_eq!(events.len(), 1);

        match &events[0] {
            WorkflowEvent::TaskProgress { task_id, progress, .. } => {
                assert_eq!(task_id, "task1");
                assert_eq!(*progress, 0.5);
            }
            _ => panic!("Expected TaskProgress event"),
        }
    }

    #[test]
    fn test_null_event_sink() {
        let sink = NullEventSink;
        // Should not panic
        sink.send(WorkflowEvent::task_progress("task1", "exec1", 1.0, None))
            .unwrap();
    }
}
