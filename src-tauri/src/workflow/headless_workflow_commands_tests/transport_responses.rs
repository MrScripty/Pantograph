use super::*;

#[test]
fn diagnostics_snapshot_request_still_allows_optional_scheduler_context() {
    let request = WorkflowDiagnosticsSnapshotRequest {
        session_id: Some("session-1".to_string()),
        workflow_id: Some("wf-1".to_string()),
        workflow_name: Some("Workflow 1".to_string()),
    };

    let value = serde_json::to_value(request).expect("serialize diagnostics request");
    assert_eq!(
        value,
        serde_json::json!({
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "workflow_name": "Workflow 1"
        })
    );
}

#[tokio::test]
async fn workflow_scheduler_snapshot_response_reads_backend_owned_service_snapshot() {
    let workflow_service = Arc::new(WorkflowService::new());
    let created = workflow_service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: WorkflowGraph::new(),
            workflow_id: None,
        })
        .await
        .expect("create edit session");

    let snapshot = workflow_scheduler_snapshot_response(
        &workflow_service,
        WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        },
    )
    .await
    .expect("scheduler snapshot");

    assert_eq!(snapshot.session_id, created.session_id);
    assert_eq!(snapshot.workflow_id, None);
    assert_eq!(
        snapshot.session.session_kind,
        WorkflowExecutionSessionKind::Edit
    );
    assert_eq!(
        snapshot.session.state,
        WorkflowExecutionSessionState::IdleLoaded
    );
    assert!(snapshot.items.is_empty());
}

#[test]
fn workflow_trace_snapshot_response_reads_backend_owned_trace_snapshot() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let execution_id = record_headless_scheduler_snapshot(
        diagnostics_store.as_ref(),
        "session-1",
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            trace_execution_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("run-1".to_string()),
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

    let snapshot = workflow_trace_snapshot_response(
        &diagnostics_store,
        WorkflowTraceSnapshotRequest {
            execution_id: Some("run-1".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: None,
        },
    )
    .expect("trace snapshot");

    assert_eq!(snapshot.traces.len(), 1);
    let trace = &snapshot.traces[0];
    assert_eq!(trace.execution_id, "run-1");
    assert_eq!(trace.session_id.as_deref(), Some("session-1"));
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.workflow_name.as_deref(), Some("Workflow 1"));
    assert_eq!(trace.queue.enqueued_at_ms, Some(100));
    assert_eq!(trace.queue.dequeued_at_ms, Some(110));
}

#[test]
fn workflow_trace_snapshot_response_filters_by_backend_session_id() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let execution_id = record_headless_scheduler_snapshot(
        diagnostics_store.as_ref(),
        "session-1",
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            trace_execution_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("run-1".to_string()),
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

    let snapshot = workflow_trace_snapshot_response(
        &diagnostics_store,
        WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: None,
            workflow_name: None,
            include_completed: None,
        },
    )
    .expect("session-filtered trace snapshot");

    assert_eq!(snapshot.traces.len(), 1);
    let trace = &snapshot.traces[0];
    assert_eq!(trace.execution_id, "run-1");
    assert_eq!(trace.session_id.as_deref(), Some("session-1"));
}

#[test]
fn workflow_trace_snapshot_response_returns_backend_validation_error() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

    let error = workflow_trace_snapshot_response(
        &diagnostics_store,
        WorkflowTraceSnapshotRequest {
            execution_id: Some("   ".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: None,
        },
    )
    .expect_err("blank execution id should be rejected");

    assert!(error.contains("\"code\":\"invalid_request\""));
    assert!(
        error.contains("workflow trace snapshot request field 'execution_id' must not be blank")
    );
}

#[test]
fn workflow_transport_error_json_preserves_backend_error_envelopes() {
    let cases = [
        (
            WorkflowServiceError::InvalidRequest(
                "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
                    .to_string(),
            ),
            WorkflowErrorCode::InvalidRequest,
            "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'",
            None,
        ),
        (
            WorkflowServiceError::RuntimeNotReady("runtime unavailable".to_string()),
            WorkflowErrorCode::RuntimeNotReady,
            "runtime unavailable",
            None,
        ),
        (
            WorkflowServiceError::CapabilityViolation("runtime admission rejected".to_string()),
            WorkflowErrorCode::CapabilityViolation,
            "runtime admission rejected",
            None,
        ),
        (
            WorkflowServiceError::Cancelled("workflow run cancelled".to_string()),
            WorkflowErrorCode::Cancelled,
            "workflow run cancelled",
            None,
        ),
        (
            WorkflowServiceError::scheduler_runtime_capacity_exhausted(1, 1, 0),
            WorkflowErrorCode::SchedulerBusy,
            "runtime capacity exhausted; no idle session runtime available for unload",
            Some(WorkflowErrorDetails::Scheduler(
                WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(1, 1, 0),
            )),
        ),
    ];

    for (error, expected_code, expected_message, expected_details) in cases {
        let envelope: WorkflowErrorEnvelope =
            serde_json::from_str(&super::workflow_error_json(error)).expect("parse error envelope");

        assert_eq!(envelope.code, expected_code);
        assert_eq!(envelope.message, expected_message);
        assert_eq!(envelope.details, expected_details);
    }
}
