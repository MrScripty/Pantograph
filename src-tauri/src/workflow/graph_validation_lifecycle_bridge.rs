use pantograph_workflow_service::{
    WorkflowGraphValidationLifecycleEvent, WorkflowGraphValidationLifecycleEventSink,
};
use tauri::AppHandle;

pub struct WorkflowGraphValidationLifecycleEventBridge {
    app: AppHandle,
}

impl WorkflowGraphValidationLifecycleEventBridge {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl WorkflowGraphValidationLifecycleEventSink for WorkflowGraphValidationLifecycleEventBridge {
    fn publish_validation_lifecycle_event(&self, event: WorkflowGraphValidationLifecycleEvent) {
        if let Err(error) =
            super::graph_validation_lifecycle_transport::emit_validation_lifecycle_event(
                &self.app, event,
            )
        {
            log::warn!("failed to emit graph validation lifecycle event: {error}");
        }
    }
}
