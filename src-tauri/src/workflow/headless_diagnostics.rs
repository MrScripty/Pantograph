//! Headless diagnostics helpers for Tauri workflow transport.
//!
//! This module keeps diagnostics projection and trace/scheduler snapshot
//! adaptation separate from the main headless command transport file so
//! command wiring stays focused on request orchestration.

use super::commands::{SharedWorkflowDiagnosticsStore, SharedWorkflowService};
use super::diagnostics::{WorkflowDiagnosticsProjection, WorkflowDiagnosticsStore};
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowSchedulerSnapshotRequest,
    WorkflowSchedulerSnapshotResponse, WorkflowServiceError, WorkflowTraceRuntimeMetrics,
    WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
};

fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

pub(crate) fn record_headless_scheduler_snapshot(
    diagnostics_store: &WorkflowDiagnosticsStore,
    requested_session_id: &str,
    requested_workflow_id: Option<String>,
    requested_workflow_name: Option<String>,
    snapshot_result: Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError>,
    captured_at_ms: u64,
) -> String {
    diagnostics_store.set_execution_metadata(
        requested_session_id,
        requested_workflow_id.clone(),
        requested_workflow_name.clone(),
    );

    match snapshot_result {
        Ok(snapshot) => {
            let observed_execution_id = snapshot
                .trace_execution_id
                .clone()
                .unwrap_or_else(|| requested_session_id.to_string());
            if observed_execution_id != requested_session_id {
                diagnostics_store.set_execution_metadata(
                    &observed_execution_id,
                    snapshot
                        .workflow_id
                        .clone()
                        .or_else(|| requested_workflow_id.clone()),
                    requested_workflow_name,
                );
            }
            diagnostics_store.record_scheduler_snapshot(
                snapshot.workflow_id,
                observed_execution_id.clone(),
                snapshot.session_id,
                captured_at_ms,
                Some(snapshot.session),
                snapshot.items,
                None,
            );
            observed_execution_id
        }
        Err(error) => {
            diagnostics_store.record_scheduler_snapshot(
                requested_workflow_id,
                requested_session_id.to_string(),
                requested_session_id.to_string(),
                captured_at_ms,
                None,
                Vec::new(),
                Some(error.to_envelope_json()),
            );
            requested_session_id.to_string()
        }
    }
}

pub(crate) fn record_headless_runtime_snapshot(
    diagnostics_store: &WorkflowDiagnosticsStore,
    workflow_id: String,
    trace_execution_id: Option<&str>,
    capabilities_result: Result<WorkflowCapabilitiesResponse, WorkflowServiceError>,
    trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    active_model_target: Option<String>,
    embedding_model_target: Option<String>,
    active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    captured_at_ms: u64,
) {
    match (trace_execution_id, capabilities_result) {
        (Some(trace_execution_id), Ok(capabilities)) => {
            diagnostics_store.record_runtime_snapshot(
                workflow_id,
                trace_execution_id.to_string(),
                captured_at_ms,
                Some(capabilities),
                trace_runtime_metrics,
                active_model_target.clone(),
                embedding_model_target.clone(),
                active_runtime_snapshot.clone(),
                embedding_runtime_snapshot.clone(),
                None,
            );
        }
        (Some(trace_execution_id), Err(error)) => {
            diagnostics_store.record_runtime_snapshot(
                workflow_id,
                trace_execution_id.to_string(),
                captured_at_ms,
                None,
                trace_runtime_metrics,
                active_model_target.clone(),
                embedding_model_target.clone(),
                active_runtime_snapshot.clone(),
                embedding_runtime_snapshot.clone(),
                Some(error.to_envelope_json()),
            );
        }
        (None, Ok(capabilities)) => {
            diagnostics_store.update_runtime_snapshot(
                Some(workflow_id),
                Some(capabilities),
                None,
                active_model_target,
                embedding_model_target,
                active_runtime_snapshot,
                embedding_runtime_snapshot,
                captured_at_ms,
            );
        }
        (None, Err(error)) => {
            diagnostics_store.update_runtime_snapshot(
                Some(workflow_id),
                None,
                Some(error.to_envelope_json()),
                active_model_target,
                embedding_model_target,
                active_runtime_snapshot,
                embedding_runtime_snapshot,
                captured_at_ms,
            );
        }
    }
}

pub(crate) async fn workflow_scheduler_snapshot_response(
    workflow_service: &SharedWorkflowService,
    request: WorkflowSchedulerSnapshotRequest,
) -> Result<WorkflowSchedulerSnapshotResponse, String> {
    workflow_service
        .workflow_get_scheduler_snapshot(request)
        .await
        .map_err(workflow_error_json)
}

pub(crate) fn workflow_trace_snapshot_response(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    request: WorkflowTraceSnapshotRequest,
) -> Result<WorkflowTraceSnapshotResponse, String> {
    diagnostics_store
        .trace_snapshot(request)
        .map_err(workflow_error_json)
}

pub(crate) fn workflow_clear_diagnostics_history_response(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
) -> WorkflowDiagnosticsProjection {
    diagnostics_store.clear_history()
}

pub(crate) fn normalize_optional_request_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn stored_runtime_trace_metrics(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    session_id: Option<&str>,
    workflow_id: Option<&str>,
) -> Option<WorkflowTraceRuntimeMetrics> {
    diagnostics_store
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: session_id.map(ToOwned::to_owned),
            workflow_id: workflow_id.map(ToOwned::to_owned),
            include_completed: Some(true),
        })
        .ok()?
        .traces
        .into_iter()
        .next()
        .map(|trace| trace.runtime)
}

pub(crate) fn stored_runtime_snapshots(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    workflow_id: Option<&str>,
) -> Option<(
    Option<inference::RuntimeLifecycleSnapshot>,
    Option<inference::RuntimeLifecycleSnapshot>,
)> {
    let projection = diagnostics_store.snapshot();
    if let Some(workflow_id) = workflow_id {
        if projection.runtime.workflow_id.as_deref() != Some(workflow_id) {
            return None;
        }
    }

    Some((
        projection
            .runtime
            .active_runtime
            .as_ref()
            .map(inference::RuntimeLifecycleSnapshot::from),
        projection
            .runtime
            .embedding_runtime
            .as_ref()
            .map(inference::RuntimeLifecycleSnapshot::from),
    ))
}

pub(crate) fn stored_runtime_model_targets(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    workflow_id: Option<&str>,
) -> Option<(Option<String>, Option<String>)> {
    let workflow_id = workflow_id?;
    let snapshot = diagnostics_store.snapshot();
    if snapshot.runtime.workflow_id.as_deref() != Some(workflow_id) {
        return None;
    }

    Some((
        snapshot.runtime.active_model_target,
        snapshot.runtime.embedding_model_target,
    ))
}

pub(crate) fn workflow_diagnostics_snapshot_projection(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    session_id: Option<String>,
    workflow_id: Option<String>,
    workflow_name: Option<String>,
    scheduler_snapshot_result: Option<
        Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError>,
    >,
    capabilities_result: Option<Result<WorkflowCapabilitiesResponse, WorkflowServiceError>>,
    runtime_trace_metrics: WorkflowTraceRuntimeMetrics,
    active_model_target: Option<String>,
    embedding_model_target: Option<String>,
    active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    captured_at_ms: u64,
) -> WorkflowDiagnosticsProjection {
    let mut trace_execution_id = session_id.clone();

    if let Some(session_id) = session_id.as_deref() {
        trace_execution_id = Some(record_headless_scheduler_snapshot(
            diagnostics_store.as_ref(),
            session_id,
            workflow_id.clone(),
            workflow_name.clone(),
            scheduler_snapshot_result.unwrap_or_else(|| {
                Err(WorkflowServiceError::InvalidRequest(
                    "scheduler snapshot unavailable".to_string(),
                ))
            }),
            captured_at_ms,
        ));
    } else {
        diagnostics_store.update_scheduler_snapshot(
            None,
            None,
            None,
            Vec::new(),
            None,
            captured_at_ms,
        );
    }

    if let Some(workflow_id) = workflow_id {
        record_headless_runtime_snapshot(
            diagnostics_store.as_ref(),
            workflow_id,
            trace_execution_id.as_deref(),
            capabilities_result.unwrap_or_else(|| {
                Err(WorkflowServiceError::InvalidRequest(
                    "workflow capabilities unavailable".to_string(),
                ))
            }),
            runtime_trace_metrics,
            active_model_target,
            embedding_model_target,
            active_runtime_snapshot,
            embedding_runtime_snapshot,
            captured_at_ms,
        );
    } else {
        diagnostics_store.update_runtime_snapshot(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            captured_at_ms,
        );
    }

    diagnostics_store.snapshot()
}
