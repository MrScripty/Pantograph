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
    graph::WorkflowGraphSessionStateView, WorkflowCapabilitiesResponse,
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
mod tests {
    use std::sync::Arc;

    use pantograph_workflow_service::{
        graph::{WorkflowGraphSessionStateView, WorkflowSessionKind},
        WorkflowCapabilitiesResponse, WorkflowRuntimeRequirements,
        WorkflowSchedulerSnapshotResponse, WorkflowSessionQueueItem,
        WorkflowSessionQueueItemStatus, WorkflowSessionState, WorkflowSessionSummary,
        WorkflowTraceRuntimeMetrics,
    };

    use super::{
        stored_runtime_trace_metrics, workflow_diagnostics_snapshot_projection,
        WorkflowDiagnosticsSnapshotProjectionInput,
    };
    use crate::workflow::diagnostics::WorkflowDiagnosticsStore;

    macro_rules! workflow_projection {
        ($store:expr, $input:expr $(,)?) => {
            workflow_diagnostics_snapshot_projection($store, $input)
        };
        (
            $store:expr,
            $session_id:expr,
            $workflow_id:expr,
            $workflow_name:expr,
            $scheduler_snapshot_result:expr,
            $capabilities_result:expr,
            $current_session_state:expr,
            $runtime_trace_metrics:expr,
            $active_model_target:expr,
            $embedding_model_target:expr,
            $active_runtime_snapshot:expr,
            $embedding_runtime_snapshot:expr,
            $managed_runtimes:expr,
            $captured_at_ms:expr $(,)?
        ) => {
            workflow_diagnostics_snapshot_projection(
                $store,
                WorkflowDiagnosticsSnapshotProjectionInput {
                    session_id: $session_id,
                    workflow_id: $workflow_id,
                    workflow_name: $workflow_name,
                    scheduler_snapshot_result: $scheduler_snapshot_result,
                    capabilities_result: $capabilities_result,
                    current_session_state: $current_session_state,
                    runtime_trace_metrics: $runtime_trace_metrics,
                    active_model_target: $active_model_target,
                    embedding_model_target: $embedding_model_target,
                    active_runtime_snapshot: $active_runtime_snapshot,
                    embedding_runtime_snapshot: $embedding_runtime_snapshot,
                    managed_runtimes: $managed_runtimes,
                    captured_at_ms: $captured_at_ms,
                },
            )
        };
    }

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

        let projection = workflow_projection!(
            &diagnostics_store,
            WorkflowDiagnosticsSnapshotProjectionInput {
                session_id: Some("session-1".to_string()),
                workflow_id: Some("wf-1".to_string()),
                workflow_name: Some("Workflow 1".to_string()),
                scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
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
                capabilities_result: Some(Ok(capability_response())),
                current_session_state: None,
                runtime_trace_metrics: WorkflowTraceRuntimeMetrics {
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
                active_model_target: Some("llava:34b".to_string()),
                embedding_model_target: None,
                active_runtime_snapshot: None,
                embedding_runtime_snapshot: None,
                managed_runtimes: Vec::new(),
                captured_at_ms: 120,
            },
        );

        assert_eq!(
            projection.scheduler.session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(projection.scheduler.trace_execution_id, None);
        assert_eq!(
            projection.context.requested_session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(
            projection.context.requested_workflow_id.as_deref(),
            Some("wf-1")
        );
        assert_eq!(
            projection.context.requested_workflow_name.as_deref(),
            Some("Workflow 1")
        );
        assert_eq!(projection.context.relevant_execution_id, None);
        assert!(projection.context.relevant);
        assert!(projection.run_order.is_empty());
        assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
        assert!(
            stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1"))
                .is_none()
        );
    }

    #[test]
    fn workflow_diagnostics_snapshot_projection_preserves_current_session_state() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let current_session_state = WorkflowGraphSessionStateView::new(
            node_engine::WorkflowSessionResidencyState::Warm,
            Vec::new(),
            None,
            None,
        );

        let projection = workflow_projection!(
            &diagnostics_store,
            WorkflowDiagnosticsSnapshotProjectionInput {
                session_id: Some("session-1".to_string()),
                workflow_id: Some("wf-1".to_string()),
                workflow_name: Some("Workflow 1".to_string()),
                scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
                    workflow_id: Some("wf-1".to_string()),
                    session_id: "session-1".to_string(),
                    trace_execution_id: None,
                    session: running_session_summary(),
                    items: Vec::new(),
                    diagnostics: None,
                })),
                capabilities_result: Some(Ok(capability_response())),
                current_session_state: Some(current_session_state.clone()),
                runtime_trace_metrics: WorkflowTraceRuntimeMetrics::default(),
                active_model_target: None,
                embedding_model_target: None,
                active_runtime_snapshot: None,
                embedding_runtime_snapshot: None,
                managed_runtimes: Vec::new(),
                captured_at_ms: 120,
            },
        );

        assert_eq!(
            projection.current_session_state,
            Some(current_session_state)
        );
    }

    #[test]
    fn stored_runtime_trace_metrics_returns_none_for_ambiguous_multi_run_scope() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

        for (execution_id, runtime_id, captured_at_ms) in [
            ("run-1", "llama_cpp", 120_u64),
            ("run-2", "llama_cpp.embedding", 220_u64),
        ] {
            workflow_projection!(
                &diagnostics_store,
                WorkflowDiagnosticsSnapshotProjectionInput {
                    session_id: Some("session-1".to_string()),
                    workflow_id: Some("wf-1".to_string()),
                    workflow_name: Some("Workflow 1".to_string()),
                    scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
                        workflow_id: Some("wf-1".to_string()),
                        session_id: "session-1".to_string(),
                        trace_execution_id: Some(execution_id.to_string()),
                        session: running_session_summary(),
                        items: vec![WorkflowSessionQueueItem {
                            queue_id: format!("queue-{execution_id}"),
                            run_id: Some(execution_id.to_string()),
                            enqueued_at_ms: Some(captured_at_ms.saturating_sub(10)),
                            dequeued_at_ms: Some(captured_at_ms),
                            priority: 5,
                            queue_position: Some(0),
                            scheduler_admission_outcome: None,
                            scheduler_decision_reason: None,
                            status: WorkflowSessionQueueItemStatus::Running,
                        }],
                        diagnostics: None,
                    })),
                    capabilities_result: Some(Ok(capability_response())),
                    current_session_state: None,
                    runtime_trace_metrics: WorkflowTraceRuntimeMetrics {
                        runtime_id: Some(runtime_id.to_string()),
                        observed_runtime_ids: vec![runtime_id.to_string()],
                        runtime_instance_id: Some(format!("{runtime_id}-instance")),
                        model_target: Some(format!("/models/{runtime_id}.gguf")),
                        warmup_started_at_ms: Some(captured_at_ms.saturating_sub(9)),
                        warmup_completed_at_ms: Some(captured_at_ms),
                        warmup_duration_ms: Some(9),
                        runtime_reused: Some(false),
                        lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    },
                    active_model_target: Some(format!("/models/{runtime_id}.gguf")),
                    embedding_model_target: None,
                    active_runtime_snapshot: None,
                    embedding_runtime_snapshot: None,
                    managed_runtimes: Vec::new(),
                    captured_at_ms,
                },
            );
        }

        assert!(
            stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1"))
                .is_none()
        );
    }
}
