//! Headless diagnostics helpers for Tauri workflow transport.
//!
//! This module keeps diagnostics projection and trace/scheduler snapshot
//! adaptation separate from the main headless command transport file so
//! command wiring stays focused on request orchestration.

use super::commands::{SharedWorkflowDiagnosticsStore, SharedWorkflowService};
use super::diagnostics::{
    WorkflowDiagnosticsProjection, WorkflowDiagnosticsProjectionContext, WorkflowDiagnosticsStore,
    WorkflowRuntimeSnapshotRecord, WorkflowRuntimeSnapshotUpdate, WorkflowSchedulerSnapshotRecord,
    WorkflowSchedulerSnapshotUpdate,
};
use pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView;
use pantograph_workflow_service::{
    graph::WorkflowGraphSessionStateView, WorkflowCapabilitiesResponse, WorkflowGraph,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse, WorkflowServiceError,
    WorkflowTraceRuntimeMetrics, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
};

pub(crate) fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

pub(crate) struct HeadlessRuntimeSnapshotInput {
    pub workflow_id: String,
    pub trace_execution_id: Option<String>,
    pub capabilities_result: Result<WorkflowCapabilitiesResponse, WorkflowServiceError>,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
    pub captured_at_ms: u64,
}

pub(crate) struct WorkflowDiagnosticsSnapshotProjectionInput {
    pub session_id: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    pub scheduler_snapshot_result:
        Option<Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError>>,
    pub capabilities_result: Option<Result<WorkflowCapabilitiesResponse, WorkflowServiceError>>,
    pub current_session_state: Option<WorkflowGraphSessionStateView>,
    pub workflow_graph: Option<WorkflowGraph>,
    pub runtime_trace_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
    pub captured_at_ms: u64,
}

pub(crate) fn record_headless_scheduler_snapshot(
    diagnostics_store: &WorkflowDiagnosticsStore,
    requested_session_id: &str,
    requested_workflow_id: Option<String>,
    requested_workflow_name: Option<String>,
    snapshot_result: Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError>,
    captured_at_ms: u64,
) -> Option<String> {
    diagnostics_store.set_execution_metadata(
        requested_session_id,
        requested_workflow_id.clone(),
        requested_workflow_name.clone(),
    );

    match snapshot_result {
        Ok(snapshot) => {
            if let Some(observed_execution_id) = snapshot.trace_execution_id.clone() {
                diagnostics_store.set_execution_metadata(
                    &observed_execution_id,
                    snapshot
                        .workflow_id
                        .clone()
                        .or_else(|| requested_workflow_id.clone()),
                    requested_workflow_name,
                );
                diagnostics_store.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
                    workflow_id: snapshot.workflow_id,
                    execution_id: observed_execution_id.clone(),
                    session_id: snapshot.session_id,
                    captured_at_ms,
                    session: Some(snapshot.session),
                    items: snapshot.items,
                    diagnostics: snapshot.diagnostics,
                    error: None,
                });
                Some(observed_execution_id)
            } else {
                diagnostics_store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
                    workflow_id: snapshot.workflow_id,
                    session_id: Some(snapshot.session_id),
                    session: Some(snapshot.session),
                    items: snapshot.items,
                    diagnostics: snapshot.diagnostics,
                    last_error: None,
                    captured_at_ms,
                });
                None
            }
        }
        Err(error) => {
            diagnostics_store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
                workflow_id: requested_workflow_id,
                session_id: Some(requested_session_id.to_string()),
                session: None,
                items: Vec::new(),
                diagnostics: None,
                last_error: Some(error.to_envelope_json()),
                captured_at_ms,
            });
            None
        }
    }
}

pub(crate) fn record_headless_runtime_snapshot(
    diagnostics_store: &WorkflowDiagnosticsStore,
    input: HeadlessRuntimeSnapshotInput,
) {
    match (
        input.trace_execution_id.as_deref(),
        input.capabilities_result,
    ) {
        (Some(trace_execution_id), Ok(capabilities)) => {
            diagnostics_store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
                workflow_id: input.workflow_id,
                execution_id: trace_execution_id.to_string(),
                captured_at_ms: input.captured_at_ms,
                capabilities: Some(capabilities),
                trace_runtime_metrics: input.trace_runtime_metrics,
                active_model_target: input.active_model_target.clone(),
                embedding_model_target: input.embedding_model_target.clone(),
                active_runtime_snapshot: input.active_runtime_snapshot.clone(),
                embedding_runtime_snapshot: input.embedding_runtime_snapshot.clone(),
                managed_runtimes: input.managed_runtimes.clone(),
                error: None,
            });
        }
        (Some(trace_execution_id), Err(error)) => {
            diagnostics_store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
                workflow_id: input.workflow_id,
                execution_id: trace_execution_id.to_string(),
                captured_at_ms: input.captured_at_ms,
                capabilities: None,
                trace_runtime_metrics: input.trace_runtime_metrics,
                active_model_target: input.active_model_target.clone(),
                embedding_model_target: input.embedding_model_target.clone(),
                active_runtime_snapshot: input.active_runtime_snapshot.clone(),
                embedding_runtime_snapshot: input.embedding_runtime_snapshot.clone(),
                managed_runtimes: input.managed_runtimes.clone(),
                error: Some(error.to_envelope_json()),
            });
        }
        (None, Ok(capabilities)) => {
            diagnostics_store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
                workflow_id: Some(input.workflow_id),
                capabilities: Some(capabilities),
                last_error: None,
                active_model_target: input.active_model_target,
                embedding_model_target: input.embedding_model_target,
                active_runtime_snapshot: input.active_runtime_snapshot,
                embedding_runtime_snapshot: input.embedding_runtime_snapshot,
                managed_runtimes: input.managed_runtimes,
                captured_at_ms: input.captured_at_ms,
            });
        }
        (None, Err(error)) => {
            diagnostics_store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
                workflow_id: Some(input.workflow_id),
                capabilities: None,
                last_error: Some(error.to_envelope_json()),
                active_model_target: input.active_model_target,
                embedding_model_target: input.embedding_model_target,
                active_runtime_snapshot: input.active_runtime_snapshot,
                embedding_runtime_snapshot: input.embedding_runtime_snapshot,
                managed_runtimes: input.managed_runtimes,
                captured_at_ms: input.captured_at_ms,
            });
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

pub(crate) fn stored_runtime_trace_metrics(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    session_id: Option<&str>,
    workflow_id: Option<&str>,
) -> Option<WorkflowTraceRuntimeMetrics> {
    diagnostics_store
        .select_trace_runtime_metrics(&WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: session_id.map(ToOwned::to_owned),
            workflow_id: workflow_id.map(ToOwned::to_owned),
            workflow_name: None,
            include_completed: Some(true),
        })
        .ok()?
        .runtime
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
    input: WorkflowDiagnosticsSnapshotProjectionInput,
) -> WorkflowDiagnosticsProjection {
    let mut trace_execution_id = None;

    if let Some(session_id) = input.session_id.as_deref() {
        trace_execution_id = record_headless_scheduler_snapshot(
            diagnostics_store.as_ref(),
            session_id,
            input.workflow_id.clone(),
            input.workflow_name.clone(),
            input.scheduler_snapshot_result.unwrap_or_else(|| {
                Err(WorkflowServiceError::InvalidRequest(
                    "scheduler snapshot unavailable".to_string(),
                ))
            }),
            input.captured_at_ms,
        );
    } else {
        diagnostics_store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
            workflow_id: None,
            session_id: None,
            session: None,
            items: Vec::new(),
            diagnostics: None,
            last_error: None,
            captured_at_ms: input.captured_at_ms,
        });
    }

    if let Some(workflow_id) = input.workflow_id.clone() {
        record_headless_runtime_snapshot(
            diagnostics_store.as_ref(),
            HeadlessRuntimeSnapshotInput {
                workflow_id,
                trace_execution_id: trace_execution_id.clone(),
                capabilities_result: input.capabilities_result.unwrap_or_else(|| {
                    Err(WorkflowServiceError::InvalidRequest(
                        "workflow capabilities unavailable".to_string(),
                    ))
                }),
                trace_runtime_metrics: input.runtime_trace_metrics,
                active_model_target: input.active_model_target,
                embedding_model_target: input.embedding_model_target,
                active_runtime_snapshot: input.active_runtime_snapshot,
                embedding_runtime_snapshot: input.embedding_runtime_snapshot,
                managed_runtimes: input.managed_runtimes,
                captured_at_ms: input.captured_at_ms,
            },
        );
    } else {
        diagnostics_store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
            workflow_id: None,
            capabilities: None,
            last_error: None,
            active_model_target: None,
            embedding_model_target: None,
            active_runtime_snapshot: None,
            embedding_runtime_snapshot: None,
            managed_runtimes: Vec::new(),
            captured_at_ms: input.captured_at_ms,
        });
    }

    let mut projection = diagnostics_store.snapshot();
    projection.workflow_timing_history = input.workflow_graph.as_ref().and_then(|graph| {
        input.workflow_id.as_ref().map(|workflow_id| {
            diagnostics_store.workflow_timing_history(
                workflow_id.clone(),
                input.workflow_name.clone(),
                graph,
            )
        })
    });
    projection.current_session_state = input.current_session_state;
    projection.with_context(WorkflowDiagnosticsProjectionContext {
        requested_session_id: input.session_id,
        requested_workflow_id: input.workflow_id,
        requested_workflow_name: input.workflow_name,
        source_execution_id: None,
        relevant_execution_id: trace_execution_id,
        relevant: true,
    })
}

#[cfg(test)]
#[path = "headless_diagnostics_tests.rs"]
mod tests;
