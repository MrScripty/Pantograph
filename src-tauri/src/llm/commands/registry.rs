//! Runtime-registry inspection and targeted reclaim commands.

#[path = "registry/debug.rs"]
mod debug;
#[path = "registry/request.rs"]
mod request;
#[cfg(test)]
#[path = "registry/tests.rs"]
mod tests;

use crate::llm::health_monitor::SharedHealthMonitor;
use crate::llm::recovery::SharedRecoveryManager;
use crate::workflow::commands::{
    SharedExtensions, SharedWorkflowDiagnosticsStore, SharedWorkflowService,
};
use crate::workflow::diagnostics::WorkflowDiagnosticsSnapshotRequest;
use crate::workflow::headless_diagnostics_transport::{
    workflow_diagnostics_snapshot_response, workflow_trace_snapshot_response,
};
use pantograph_workflow_service::{WorkflowServiceError, WorkflowTraceSnapshotRequest};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Manager, State};

use crate::llm::runtime_registry::reclaim_runtime_and_sync_runtime_registry;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
pub use debug::RuntimeDebugSnapshot;
pub(crate) use debug::RuntimeDebugTraceSelection;
pub(crate) use debug::{runtime_debug_snapshot_response, runtime_registry_snapshot_response};
pub use request::RuntimeDebugSnapshotRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRegistryReclaimResponse {
    pub reclaim: pantograph_runtime_registry::RuntimeReclaimDisposition,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
}

async fn reclaim_runtime(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    runtime_id: &str,
) -> Result<RuntimeRegistryReclaimResponse, String> {
    let reclaim = reclaim_runtime_and_sync_runtime_registry(gateway, runtime_registry, runtime_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(RuntimeRegistryReclaimResponse {
        reclaim,
        snapshot: runtime_registry.snapshot(),
    })
}

fn resolve_runtime_debug_trace_scope(
    diagnostics_store: Option<&SharedWorkflowDiagnosticsStore>,
    request: &WorkflowTraceSnapshotRequest,
) -> Result<Option<(WorkflowTraceSnapshotRequest, RuntimeDebugTraceSelection)>, WorkflowServiceError>
{
    let Some(diagnostics_store) = diagnostics_store else {
        return Ok(None);
    };

    let selection =
        RuntimeDebugTraceSelection::from(diagnostics_store.select_trace_runtime_metrics(request)?);
    let resolved_request = WorkflowTraceSnapshotRequest {
        execution_id: selection
            .execution_id
            .clone()
            .or_else(|| request.execution_id.clone()),
        session_id: request.session_id.clone(),
        workflow_id: request.workflow_id.clone(),
        workflow_name: request.workflow_name.clone(),
        include_completed: request.include_completed,
    };

    Ok(Some((resolved_request, selection)))
}

#[command]
pub async fn get_runtime_registry_snapshot(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<pantograph_runtime_registry::RuntimeRegistrySnapshot, String> {
    Ok(runtime_registry_snapshot_response(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn get_runtime_debug_snapshot(
    request: Option<RuntimeDebugSnapshotRequest>,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<RuntimeDebugSnapshot, String> {
    let health_monitor = app
        .try_state::<SharedHealthMonitor>()
        .map(|monitor| (*monitor).clone());
    let recovery_manager = app
        .try_state::<SharedRecoveryManager>()
        .map(|manager| (*manager).clone());
    let workflow_diagnostics_store = app
        .try_state::<SharedWorkflowDiagnosticsStore>()
        .map(|store| (*store).clone());
    let workflow_service = app
        .try_state::<SharedWorkflowService>()
        .map(|service| (*service).clone());
    let extensions = app
        .try_state::<SharedExtensions>()
        .map(|extensions| (*extensions).clone());
    let workflow_request = request.unwrap_or_default().normalized();
    workflow_request
        .validate()
        .map_err(|error| error.to_envelope_json())?;
    let execution_id_filter = workflow_request.execution_id.clone();
    let session_id_filter = workflow_request.session_id.clone();
    let workflow_id_filter = workflow_request.workflow_id.clone();
    let workflow_name_filter = workflow_request.workflow_name.clone();
    let include_trace = workflow_request.include_trace.unwrap_or(false);
    let include_completed = workflow_request.include_completed;
    let has_workflow_filter = session_id_filter.is_some()
        || workflow_id_filter.is_some()
        || workflow_name_filter.is_some();
    let workflow_diagnostics = match (
        workflow_diagnostics_store.clone(),
        workflow_service,
        extensions,
        has_workflow_filter,
    ) {
        (Some(store), Some(service), Some(extensions), true) => Some(
            workflow_diagnostics_snapshot_response(
                &app,
                gateway.inner(),
                runtime_registry.inner(),
                &extensions,
                &service,
                &store,
                WorkflowDiagnosticsSnapshotRequest {
                    session_id: session_id_filter.clone(),
                    workflow_id: workflow_id_filter.clone(),
                    workflow_name: workflow_name_filter.clone(),
                    workflow_graph: None,
                },
            )
            .await?,
        ),
        (Some(store), _, _, _) => Some(store.snapshot()),
        _ => None,
    };
    let trace_request = WorkflowTraceSnapshotRequest {
        execution_id: execution_id_filter,
        session_id: session_id_filter,
        workflow_id: workflow_id_filter,
        workflow_name: workflow_name_filter,
        include_completed,
    };
    let (workflow_trace, workflow_trace_selection) = if include_trace {
        let selection =
            resolve_runtime_debug_trace_scope(workflow_diagnostics_store.as_ref(), &trace_request)
                .map_err(|error| error.to_envelope_json())?;
        let workflow_trace = match (workflow_diagnostics_store, selection.as_ref()) {
            (Some(store), Some((_, selection))) if selection.ambiguous => None,
            (Some(store), Some((resolved_request, _))) => Some(workflow_trace_snapshot_response(
                &store,
                resolved_request.clone(),
            )?),
            _ => None,
        };
        (workflow_trace, selection.map(|(_, selection)| selection))
    } else {
        (None, None)
    };

    Ok(runtime_debug_snapshot_response(
        gateway.inner(),
        runtime_registry.inner(),
        health_monitor.as_ref(),
        recovery_manager.as_ref(),
        workflow_diagnostics,
        workflow_trace,
        workflow_trace_selection,
    )
    .await)
}

#[command]
pub async fn reclaim_runtime_registry_runtime(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    runtime_id: String,
) -> Result<RuntimeRegistryReclaimResponse, String> {
    reclaim_runtime(gateway.inner(), runtime_registry.inner(), &runtime_id).await
}
