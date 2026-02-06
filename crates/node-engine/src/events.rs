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

/// Event sink that broadcasts events via a tokio broadcast channel
///
/// Allows multiple consumers to receive events concurrently. Useful for
/// fanout to multiple listeners (UI, logging, metrics, etc.).
///
/// If no receivers are listening, events are silently dropped.
pub struct BroadcastEventSink {
    sender: tokio::sync::broadcast::Sender<WorkflowEvent>,
}

impl BroadcastEventSink {
    /// Create a new broadcast event sink with the given channel capacity
    pub fn new(capacity: usize) -> (Self, tokio::sync::broadcast::Receiver<WorkflowEvent>) {
        let (sender, receiver) = tokio::sync::broadcast::channel(capacity);
        (Self { sender }, receiver)
    }

    /// Subscribe to this broadcast sink
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<WorkflowEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active receivers
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl EventSink for BroadcastEventSink {
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError> {
        let _ = self.sender.send(event);
        Ok(())
    }
}

/// Event sink that calls a user-provided callback for each event
///
/// Critical for NIF bridging: the callback can be a closure that sends
/// events to an Elixir process or any other foreign runtime.
pub struct CallbackEventSink {
    callback: Box<dyn Fn(WorkflowEvent) + Send + Sync>,
}

impl CallbackEventSink {
    /// Create a new callback event sink
    pub fn new(callback: impl Fn(WorkflowEvent) + Send + Sync + 'static) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl EventSink for CallbackEventSink {
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError> {
        (self.callback)(event);
        Ok(())
    }
}

/// Event sink that fans out events to multiple child sinks
///
/// All child sinks receive every event. If one child fails, the error
/// is logged but other children still receive the event.
pub struct CompositeEventSink {
    sinks: Vec<Box<dyn EventSink>>,
}

impl CompositeEventSink {
    /// Create a new composite event sink with no children
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Create with pre-built sinks
    pub fn with_sinks(sinks: Vec<Box<dyn EventSink>>) -> Self {
        Self { sinks }
    }

    /// Add a child sink
    pub fn add(&mut self, sink: Box<dyn EventSink>) {
        self.sinks.push(sink);
    }

    /// Get the number of child sinks
    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    /// Check if there are no child sinks
    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}

impl Default for CompositeEventSink {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSink for CompositeEventSink {
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError> {
        let mut last_error = None;
        for sink in &self.sinks {
            if let Err(e) = sink.send(event.clone()) {
                last_error = Some(e);
            }
        }
        match last_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
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

    #[test]
    fn test_broadcast_event_sink() {
        let (sink, mut rx) = BroadcastEventSink::new(16);
        let mut rx2 = sink.subscribe();

        sink.send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
            .unwrap();

        let event = rx.try_recv().unwrap();
        assert!(matches!(event, WorkflowEvent::TaskProgress { .. }));

        let event2 = rx2.try_recv().unwrap();
        assert!(matches!(event2, WorkflowEvent::TaskProgress { .. }));
    }

    #[test]
    fn test_broadcast_no_receivers() {
        let (sink, rx) = BroadcastEventSink::new(16);
        drop(rx);
        // Should not error even with no receivers
        let result = sink.send(WorkflowEvent::task_progress("task1", "exec1", 1.0, None));
        assert!(result.is_ok());
    }

    #[test]
    fn test_callback_event_sink() {
        let collected = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let collected_clone = collected.clone();

        let sink = CallbackEventSink::new(move |event| {
            collected_clone.lock().unwrap().push(event);
        });

        sink.send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
            .unwrap();
        sink.send(WorkflowEvent::task_progress("task1", "exec1", 1.0, None))
            .unwrap();

        assert_eq!(collected.lock().unwrap().len(), 2);
    }

    #[test]
    fn test_composite_event_sink() {
        let mut composite = CompositeEventSink::new();
        let collector = std::sync::Arc::new(VecEventSink::new());

        // Use a callback sink to verify events are fanned out
        let collector_clone = collector.clone();
        composite.add(Box::new(CallbackEventSink::new(move |event| {
            collector_clone.events.lock().unwrap().push(event);
        })));
        composite.add(Box::new(NullEventSink));
        assert_eq!(composite.len(), 2);

        composite
            .send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
            .unwrap();

        assert_eq!(collector.events().len(), 1);
    }

    #[test]
    fn test_composite_empty() {
        let composite = CompositeEventSink::new();
        assert!(composite.is_empty());
        // Should succeed silently
        composite
            .send(WorkflowEvent::task_progress("task1", "exec1", 0.5, None))
            .unwrap();
    }
}
