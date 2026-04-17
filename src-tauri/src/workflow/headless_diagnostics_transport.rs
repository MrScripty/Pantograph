//! Diagnostics and trace transport helpers for headless workflow entrypoints.
//!
//! This module owns the host-facing diagnostics snapshot path so debug and
//! workflow transport callers do not depend on the broader headless workflow
//! command adapter.

use pantograph_embedded_runtime::{
    workflow_runtime::{
        build_runtime_event_projection_with_registry_reconciliation, unix_timestamp_ms,
    },
    HostRuntimeModeSnapshot,
};
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowSchedulerSnapshotRequest, WorkflowTraceSnapshotRequest,
    WorkflowTraceSnapshotResponse,
};
use tauri::{AppHandle, State};

use crate::llm::runtime_registry::sync_runtime_registry_from_gateway;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

use super::commands::{SharedExtensions, SharedWorkflowDiagnosticsStore, SharedWorkflowService};
use super::diagnostics::{WorkflowDiagnosticsProjection, WorkflowDiagnosticsSnapshotRequest};
pub(crate) use super::headless_diagnostics::workflow_trace_snapshot_response;
use super::headless_diagnostics::{
    stored_runtime_model_targets, stored_runtime_snapshots, stored_runtime_trace_metrics,
    workflow_clear_diagnostics_history_response, workflow_diagnostics_snapshot_projection,
};
use super::headless_runtime::build_runtime;

pub async fn workflow_get_diagnostics_snapshot(
    request: WorkflowDiagnosticsSnapshotRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowDiagnosticsProjection, String> {
    workflow_diagnostics_snapshot_response(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        diagnostics_store.inner(),
        request,
    )
    .await
}

pub async fn workflow_diagnostics_snapshot_response(
    app: &AppHandle,
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    extensions: &SharedExtensions,
    workflow_service: &SharedWorkflowService,
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    request: WorkflowDiagnosticsSnapshotRequest,
) -> Result<WorkflowDiagnosticsProjection, String> {
    sync_runtime_registry_from_gateway(gateway.as_ref(), runtime_registry.as_ref()).await;
    let captured_at_ms = unix_timestamp_ms();
    let request = request.normalized();
    let session_id = request.session_id;
    let workflow_id = request.workflow_id;
    let workflow_name = request.workflow_name;
    let scheduler_snapshot_result = if let Some(session_id) = session_id.as_deref() {
        Some(
            workflow_service
                .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                    session_id: session_id.to_string(),
                })
                .await,
        )
    } else {
        None
    };
    let capabilities_result = if let Some(workflow_id) = workflow_id.as_ref() {
        let runtime = build_runtime(
            app,
            gateway,
            runtime_registry,
            extensions,
            workflow_service,
            None,
        )
        .await?;
        Some(
            runtime
                .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                    workflow_id: workflow_id.clone(),
                })
                .await,
        )
    } else {
        None
    };
    let stored_runtime_snapshots =
        stored_runtime_snapshots(diagnostics_store, workflow_id.as_deref());
    let stored_runtime_model_targets =
        stored_runtime_model_targets(diagnostics_store, workflow_id.as_deref());
    let gateway_snapshot = gateway.runtime_lifecycle_snapshot().await;
    let gateway_mode_info = HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let live_embedding_runtime_snapshot = gateway.embedding_runtime_lifecycle_snapshot().await;
    let runtime_projection = build_runtime_event_projection_with_registry_reconciliation(
        Some(runtime_registry.as_ref()),
        stored_runtime_snapshots
            .as_ref()
            .and_then(|(active_runtime, _)| active_runtime.as_ref()),
        stored_runtime_snapshots
            .as_ref()
            .and_then(|(_, embedding_runtime)| embedding_runtime.as_ref()),
        stored_runtime_model_targets
            .as_ref()
            .and_then(|(active_model_target, _)| active_model_target.as_deref()),
        stored_runtime_model_targets
            .as_ref()
            .and_then(|(_, embedding_model_target)| embedding_model_target.as_deref()),
        stored_runtime_trace_metrics(
            diagnostics_store,
            session_id.as_deref(),
            workflow_id.as_deref(),
        ),
        None,
        &gateway_snapshot,
        live_embedding_runtime_snapshot.as_ref(),
        &gateway_mode_info,
        None,
    );

    Ok(workflow_diagnostics_snapshot_projection(
        diagnostics_store,
        session_id,
        workflow_id,
        workflow_name,
        scheduler_snapshot_result,
        capabilities_result,
        runtime_projection.trace_runtime_metrics,
        runtime_projection.active_model_target,
        runtime_projection.embedding_model_target,
        Some(runtime_projection.active_runtime_snapshot),
        runtime_projection.embedding_runtime_snapshot,
        captured_at_ms,
    ))
}

pub async fn workflow_get_trace_snapshot(
    request: WorkflowTraceSnapshotRequest,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowTraceSnapshotResponse, String> {
    workflow_trace_snapshot_response(diagnostics_store.inner(), request)
}

pub async fn workflow_clear_diagnostics_history(
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowDiagnosticsProjection, String> {
    Ok(workflow_clear_diagnostics_history_response(
        diagnostics_store.inner(),
    ))
}
