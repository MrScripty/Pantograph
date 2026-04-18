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

pub(crate) fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
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
                diagnostics_store.record_scheduler_snapshot(
                    snapshot.workflow_id,
                    observed_execution_id.clone(),
                    snapshot.session_id,
                    captured_at_ms,
                    Some(snapshot.session),
                    snapshot.items,
                    snapshot.diagnostics,
                    None,
                );
                Some(observed_execution_id)
            } else {
                diagnostics_store.update_scheduler_snapshot(
                    snapshot.workflow_id,
                    Some(snapshot.session_id),
                    Some(snapshot.session),
                    snapshot.items,
                    snapshot.diagnostics,
                    None,
                    captured_at_ms,
                );
                None
            }
        }
        Err(error) => {
            diagnostics_store.update_scheduler_snapshot(
                requested_workflow_id,
                Some(requested_session_id.to_string()),
                None,
                Vec::new(),
                None,
                Some(error.to_envelope_json()),
                captured_at_ms,
            );
            None
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
            workflow_name: None,
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
    let mut trace_execution_id = None;

    if let Some(session_id) = session_id.as_deref() {
        trace_execution_id = record_headless_scheduler_snapshot(
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
        );
    } else {
        diagnostics_store.update_scheduler_snapshot(
            None,
            None,
            None,
            Vec::new(),
            None,
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pantograph_workflow_service::{
        graph::WorkflowSessionKind, WorkflowCapabilitiesResponse, WorkflowRuntimeRequirements,
        WorkflowSchedulerSnapshotResponse, WorkflowSessionQueueItem,
        WorkflowSessionQueueItemStatus, WorkflowSessionState, WorkflowSessionSummary,
        WorkflowTraceRuntimeMetrics,
    };

    use super::{stored_runtime_trace_metrics, workflow_diagnostics_snapshot_projection};
    use crate::workflow::diagnostics::WorkflowDiagnosticsStore;

    fn running_session_summary() -> WorkflowSessionSummary {
        WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 2,
        }
    }

    fn capability_response() -> WorkflowCapabilitiesResponse {
        WorkflowCapabilitiesResponse {
            max_input_bindings: 4,
            max_output_targets: 2,
            max_value_bytes: 2_048,
            runtime_requirements: WorkflowRuntimeRequirements::default(),
            models: Vec::new(),
            runtime_capabilities: Vec::new(),
        }
    }

    #[test]
    fn workflow_diagnostics_snapshot_projection_preserves_ambiguous_scheduler_identity() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

        let projection = workflow_diagnostics_snapshot_projection(
            &diagnostics_store,
            Some("session-1".to_string()),
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: None,
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: None,
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Pending,
                }],
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:34b".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
            },
            Some("llava:34b".to_string()),
            None,
            None,
            None,
            120,
        );

        assert_eq!(
            projection.scheduler.session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(projection.scheduler.trace_execution_id, None);
        assert!(projection.run_order.is_empty());
        assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
        assert!(
            stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1"))
                .is_none()
        );
    }
}
