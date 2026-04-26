use super::*;

#[test]
fn headless_scheduler_snapshot_helper_uses_workflow_run_identity() {
    let diagnostics_store = WorkflowDiagnosticsStore::default();

    let execution_id = record_headless_scheduler_snapshot(
        &diagnostics_store,
        "session-1",
        Some("wf-1".to_string()),
        Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                workflow_run_id: "run-1".to_string(),

                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(110),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
        }),
        120,
    );

    assert_eq!(execution_id.as_deref(), Some("run-1"));
    let trace = diagnostics_store
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("run-1".to_string()),
            session_id: None,
            workflow_id: None,

            include_completed: None,
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("scheduler trace");
    assert_eq!(trace.workflow_run_id, "run-1");
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.queue.enqueued_at_ms, Some(100));
    assert_eq!(trace.queue.dequeued_at_ms, Some(110));
}

#[test]
fn headless_scheduler_snapshot_helper_keeps_error_overlay_without_invented_run_identity() {
    let diagnostics_store = WorkflowDiagnosticsStore::default();

    let execution_id = record_headless_scheduler_snapshot(
        &diagnostics_store,
        "session-1",
        Some("wf-1".to_string()),
        Err(WorkflowServiceError::InvalidRequest(
            "session missing".to_string(),
        )),
        120,
    );

    assert_eq!(execution_id, None);
    let projection = diagnostics_store.snapshot();
    assert_eq!(projection.scheduler.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(
        projection.scheduler.session_id.as_deref(),
        Some("session-1")
    );
    assert_eq!(projection.scheduler.workflow_run_id, None);
    assert_eq!(
        projection.scheduler.last_error.as_deref(),
        Some("{\"code\":\"invalid_request\",\"message\":\"session missing\"}")
    );
    assert!(projection.run_order.is_empty());
}

#[test]
fn headless_runtime_snapshot_helper_records_trace_for_identified_execution() {
    let diagnostics_store = WorkflowDiagnosticsStore::default();

    record_headless_runtime_snapshot(
        &diagnostics_store,
        HeadlessRuntimeSnapshotRecordInput {
            workflow_id: "wf-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            capabilities_result: Ok(capability_response()),
            trace_runtime_metrics: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:13b".to_string()),
                warmup_started_at_ms: Some(100),
                warmup_completed_at_ms: Some(110),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            active_model_target: Some("llava:13b".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime_snapshot: None,
            embedding_runtime_snapshot: None,
            managed_runtimes: Vec::new(),
            captured_at_ms: 120,
        },
    );

    let trace = diagnostics_store
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("run-1".to_string()),
            session_id: None,
            workflow_id: None,

            include_completed: None,
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("runtime trace");
    assert_eq!(trace.workflow_run_id, "run-1");
    assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(
        trace.runtime.runtime_instance_id.as_deref(),
        Some("runtime-1")
    );
    assert_eq!(trace.runtime.model_target.as_deref(), Some("llava:13b"));
    assert_eq!(
        trace.runtime.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
    let projection = diagnostics_store.snapshot();
    assert_eq!(
        projection.runtime.active_model_target.as_deref(),
        Some("llava:13b")
    );
    assert_eq!(
        projection.runtime.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
}

#[test]
fn headless_runtime_snapshot_helper_keeps_trace_store_empty_without_execution_identity() {
    let diagnostics_store = WorkflowDiagnosticsStore::default();

    record_headless_runtime_snapshot(
        &diagnostics_store,
        HeadlessRuntimeSnapshotRecordInput {
            workflow_id: "wf-1".to_string(),
            workflow_run_id: None,
            capabilities_result: Ok(capability_response()),
            trace_runtime_metrics: WorkflowTraceRuntimeMetrics::default(),
            active_model_target: Some("llava:7b".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime_snapshot: None,
            embedding_runtime_snapshot: None,
            managed_runtimes: Vec::new(),
            captured_at_ms: 120,
        },
    );

    let projection = diagnostics_store.snapshot();
    assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(
        projection.runtime.active_model_target.as_deref(),
        Some("llava:7b")
    );
    assert_eq!(
        projection.runtime.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    let trace_snapshot = diagnostics_store
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            workflow_run_id: None,
            session_id: None,
            workflow_id: Some("wf-1".to_string()),

            include_completed: None,
        })
        .expect("trace snapshot");
    assert!(trace_snapshot.traces.is_empty());
}

#[test]
fn headless_scheduler_and_runtime_helpers_join_on_workflow_run_identity() {
    let diagnostics_store = WorkflowDiagnosticsStore::default();

    let execution_id = record_headless_scheduler_snapshot(
        &diagnostics_store,
        "session-1",
        Some("wf-1".to_string()),
        Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                workflow_run_id: "run-1".to_string(),

                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(110),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
        }),
        120,
    );
    assert_eq!(execution_id.as_deref(), Some("run-1"));

    record_headless_runtime_snapshot(
        &diagnostics_store,
        HeadlessRuntimeSnapshotRecordInput {
            workflow_id: "wf-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            capabilities_result: Ok(capability_response()),
            trace_runtime_metrics: WorkflowTraceRuntimeMetrics {
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
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime_snapshot: None,
            embedding_runtime_snapshot: None,
            managed_runtimes: Vec::new(),
            captured_at_ms: 130,
        },
    );

    let trace = diagnostics_store
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("run-1".to_string()),
            session_id: None,
            workflow_id: None,

            include_completed: None,
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("joined trace");
    assert_eq!(trace.workflow_run_id, "run-1");
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.queue.enqueued_at_ms, Some(100));
    assert_eq!(trace.queue.dequeued_at_ms, Some(110));
    assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(
        trace.runtime.runtime_instance_id.as_deref(),
        Some("runtime-1")
    );
    assert_eq!(trace.runtime.model_target.as_deref(), Some("llava:34b"));
    assert_eq!(
        trace.runtime.lifecycle_decision_reason.as_deref(),
        Some("runtime_reused")
    );
}
