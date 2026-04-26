use std::sync::Arc;

use node_engine::{EventSink, WorkflowEvent};
use tokio::sync::RwLock;

use crate::FfiWorkflowEvent;

/// Callback EventSink that buffers events for polling.
pub(crate) struct BufferedEventSink {
    buffer: Arc<RwLock<Vec<FfiWorkflowEvent>>>,
}

impl BufferedEventSink {
    pub(crate) fn new(buffer: Arc<RwLock<Vec<FfiWorkflowEvent>>>) -> Self {
        Self { buffer }
    }
}

impl EventSink for BufferedEventSink {
    fn send(&self, event: WorkflowEvent) -> std::result::Result<(), node_engine::EventError> {
        let event_type = ffi_workflow_event_type(&event).to_string();
        let mut event_value =
            serde_json::to_value(&event).map_err(|e| node_engine::EventError {
                message: e.to_string(),
            })?;
        rename_execution_id_to_workflow_run_id(&mut event_value);
        let event_json =
            serde_json::to_string(&event_value).map_err(|e| node_engine::EventError {
                message: e.to_string(),
            })?;

        if let Ok(mut buf) = self.buffer.try_write() {
            buf.push(FfiWorkflowEvent {
                event_type,
                event_json,
            });
        }
        Ok(())
    }
}

fn rename_execution_id_to_workflow_run_id(value: &mut serde_json::Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    if let Some(execution_id) = object.remove("executionId") {
        object.insert("workflowRunId".to_string(), execution_id);
    }
}

fn ffi_workflow_event_type(event: &WorkflowEvent) -> &'static str {
    match event {
        WorkflowEvent::WorkflowStarted { .. } => "WorkflowStarted",
        WorkflowEvent::WorkflowCompleted { .. } => "WorkflowCompleted",
        WorkflowEvent::WorkflowFailed { .. } => "WorkflowFailed",
        WorkflowEvent::WorkflowCancelled { .. } => "WorkflowCancelled",
        WorkflowEvent::WaitingForInput { .. } => "WaitingForInput",
        WorkflowEvent::TaskStarted { .. } => "TaskStarted",
        WorkflowEvent::TaskCompleted { .. } => "TaskCompleted",
        WorkflowEvent::TaskFailed { .. } => "TaskFailed",
        WorkflowEvent::TaskProgress { .. } => "TaskProgress",
        WorkflowEvent::TaskStream { .. } => "TaskStream",
        WorkflowEvent::GraphModified { .. } => "GraphModified",
        WorkflowEvent::IncrementalExecutionStarted { .. } => "IncrementalExecutionStarted",
    }
}
