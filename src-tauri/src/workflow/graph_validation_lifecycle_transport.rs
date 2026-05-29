use pantograph_workflow_service::WorkflowGraphValidationLifecycleEvent;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT: &str =
    "workflow://graph-validation/lifecycle-event";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphValidationLifecycleTransportEvent {
    pub event: WorkflowGraphValidationLifecycleEvent,
}

pub fn emit_validation_lifecycle_event(
    app: &AppHandle,
    event: WorkflowGraphValidationLifecycleEvent,
) -> Result<(), tauri::Error> {
    app.emit(
        WORKFLOW_GRAPH_VALIDATION_LIFECYCLE_EVENT,
        WorkflowGraphValidationLifecycleTransportEvent { event },
    )
}
