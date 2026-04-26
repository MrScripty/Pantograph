use std::sync::Arc;

use pantograph_workflow_service::{
    graph::{WorkflowExecutionSessionKind, WorkflowGraphSessionStateView},
    WorkflowCapabilitiesResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionQueueItemStatus, WorkflowExecutionSessionState,
    WorkflowExecutionSessionSummary, WorkflowRuntimeRequirements,
    WorkflowSchedulerSnapshotResponse, WorkflowTraceRuntimeMetrics,
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
                workflow_run_id: None,
                session_id: $session_id,
                workflow_id: $workflow_id,

                scheduler_snapshot_result: $scheduler_snapshot_result,
                capabilities_result: $capabilities_result,
                current_session_state: $current_session_state,
                workflow_graph: None,
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

fn running_session_summary() -> WorkflowExecutionSessionSummary {
    WorkflowExecutionSessionSummary {
        session_id: "session-1".to_string(),
        workflow_id: "wf-1".to_string(),
        session_kind: WorkflowExecutionSessionKind::Workflow,
        usage_profile: Some("interactive".to_string()),
        keep_alive: true,
        state: WorkflowExecutionSessionState::Running,
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
            workflow_run_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),

            scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                workflow_run_id: None,
                session: running_session_summary(),
                items: vec![WorkflowExecutionSessionQueueItem {
                    workflow_run_id: "queue-1".to_string(),

                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: None,
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowExecutionSessionQueueItemStatus::Pending,
                }],
                diagnostics: None,
            })),
            capabilities_result: Some(Ok(capability_response())),
            current_session_state: None,
            workflow_graph: None,
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
    assert_eq!(projection.scheduler.workflow_run_id, None);
    assert_eq!(
        projection.context.requested_session_id.as_deref(),
        Some("session-1")
    );
    assert_eq!(
        projection.context.requested_workflow_id.as_deref(),
        Some("wf-1")
    );
    assert_eq!(projection.context.relevant_workflow_run_id, None);
    assert!(projection.context.relevant);
    assert!(projection.run_order.is_empty());
    assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
    assert!(
        stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1")).is_none()
    );
}

#[test]
fn workflow_diagnostics_snapshot_projection_does_not_record_trace_from_read_snapshot() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

    let projection = workflow_projection!(
        &diagnostics_store,
        WorkflowDiagnosticsSnapshotProjectionInput {
            workflow_run_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),

            scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                workflow_run_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowExecutionSessionQueueItem {
                    workflow_run_id: "run-1".to_string(),

                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    queue_position: Some(0),
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowExecutionSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            })),
            capabilities_result: Some(Ok(capability_response())),
            current_session_state: None,
            workflow_graph: None,
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
        projection.scheduler.workflow_run_id.as_deref(),
        Some("run-1")
    );
    assert_eq!(
        projection.context.relevant_workflow_run_id.as_deref(),
        Some("run-1")
    );
    assert!(projection.run_order.is_empty());

    let trace_snapshot = diagnostics_store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("run-1".to_string()),
            session_id: None,
            workflow_id: None,
            include_completed: Some(true),
        })
        .expect("trace snapshot");
    assert!(trace_snapshot.traces.is_empty());
}

#[test]
fn workflow_diagnostics_snapshot_projection_preserves_current_session_state() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let current_session_state = WorkflowGraphSessionStateView::new(
        node_engine::WorkflowExecutionSessionResidencyState::Warm,
        Vec::new(),
        None,
        None,
    );

    let projection = workflow_projection!(
        &diagnostics_store,
        WorkflowDiagnosticsSnapshotProjectionInput {
            workflow_run_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),

            scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                workflow_run_id: None,
                session: running_session_summary(),
                items: Vec::new(),
                diagnostics: None,
            })),
            capabilities_result: Some(Ok(capability_response())),
            current_session_state: Some(current_session_state.clone()),
            workflow_graph: None,
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
                workflow_run_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: Some("wf-1".to_string()),

                scheduler_snapshot_result: Some(Ok(WorkflowSchedulerSnapshotResponse {
                    workflow_id: Some("wf-1".to_string()),
                    session_id: "session-1".to_string(),
                    workflow_run_id: Some(execution_id.to_string()),
                    session: running_session_summary(),
                    items: vec![WorkflowExecutionSessionQueueItem {
                        workflow_run_id: format!("queue-{execution_id}"),

                        enqueued_at_ms: Some(captured_at_ms.saturating_sub(10)),
                        dequeued_at_ms: Some(captured_at_ms),
                        priority: 5,
                        queue_position: Some(0),
                        scheduler_admission_outcome: None,
                        scheduler_decision_reason: None,
                        status: WorkflowExecutionSessionQueueItemStatus::Running,
                    }],
                    diagnostics: None,
                })),
                capabilities_result: Some(Ok(capability_response())),
                current_session_state: None,
                workflow_graph: None,
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
        stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1")).is_none()
    );
}
