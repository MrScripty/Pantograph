//! Tauri command entrypoints for workflow graph persistence operations.

use tauri::{command, State};

use super::commands::{SharedWorkflowGraphStore, SharedWorkflowService};

#[command]
pub fn delete_workflow(
    name: String,
    workflow_service: State<'_, SharedWorkflowService>,
    workflow_graph_store: State<'_, SharedWorkflowGraphStore>,
) -> Result<(), String> {
    workflow_service
        .workflow_graph_delete(
            workflow_graph_store.inner().as_ref(),
            pantograph_workflow_service::WorkflowGraphDeleteRequest { name },
        )
        .map(|_| ())
        .map_err(|e| e.to_envelope_json())
}
