use super::WorkflowEvent;

/// Trait for sending workflow events.
///
/// This abstracts over the transport mechanism (Tauri channel, mpsc, etc.),
/// allowing the engine to be used in different contexts.
pub trait EventSink: Send + Sync {
    /// Send an event.
    ///
    /// Returns an error if the event could not be sent.
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError>;
}

/// Error when sending events fails.
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

/// A no-op event sink that discards all events.
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn send(&self, _event: WorkflowEvent) -> Result<(), EventError> {
        Ok(())
    }
}

/// A vector-based event sink that collects events.
pub struct VecEventSink {
    events: std::sync::Mutex<Vec<WorkflowEvent>>,
}

impl VecEventSink {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Get all collected events.
    pub fn events(&self) -> Vec<WorkflowEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Clear all collected events.
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

/// Event sink that broadcasts events via a tokio broadcast channel.
pub struct BroadcastEventSink {
    sender: tokio::sync::broadcast::Sender<WorkflowEvent>,
}

impl BroadcastEventSink {
    /// Create a new broadcast event sink with the given channel capacity.
    pub fn new(capacity: usize) -> (Self, tokio::sync::broadcast::Receiver<WorkflowEvent>) {
        let (sender, receiver) = tokio::sync::broadcast::channel(capacity);
        (Self { sender }, receiver)
    }

    /// Subscribe to this broadcast sink.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<WorkflowEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active receivers.
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

/// Event sink that calls a user-provided callback for each event.
pub struct CallbackEventSink {
    callback: Box<dyn Fn(WorkflowEvent) + Send + Sync>,
}

impl CallbackEventSink {
    /// Create a new callback event sink.
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

/// Event sink that fans out events to multiple child sinks.
pub struct CompositeEventSink {
    sinks: Vec<Box<dyn EventSink>>,
}

impl CompositeEventSink {
    /// Create a new composite event sink with no children.
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Create with pre-built sinks.
    pub fn with_sinks(sinks: Vec<Box<dyn EventSink>>) -> Self {
        Self { sinks }
    }

    /// Add a child sink.
    pub fn add(&mut self, sink: Box<dyn EventSink>) {
        self.sinks.push(sink);
    }

    /// Get the number of child sinks.
    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    /// Check if there are no child sinks.
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
            if let Err(error) = sink.send(event.clone()) {
                last_error = Some(error);
            }
        }

        match last_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}
