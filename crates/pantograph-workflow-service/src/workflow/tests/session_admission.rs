use super::*;

#[tokio::test]
async fn workflow_execution_session_run_waits_for_runtime_capacity_before_admission() {
    let host = BlockingRunHost::new();
    let service = WorkflowService::with_capacity_limits(2, 1);

    let first = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-first".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create first session");
    let second = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-second".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create second session");

    let first_service = service.clone();
    let first_host = host.clone();
    let first_session_id = first.session_id.clone();
    let first_run = tokio::spawn(async move {
        first_service
            .run_workflow_execution_session(
                &first_host,
                WorkflowExecutionSessionRunRequest {
                    session_id: first_session_id,
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .await
    });

    host.wait_for_first_run_started().await;

    let second_service = service.clone();
    let second_host = host.clone();
    let second_session_id = second.session_id.clone();
    let mut second_run = tokio::spawn(async move {
        second_service
            .run_workflow_execution_session(
                &second_host,
                WorkflowExecutionSessionRunRequest {
                    session_id: second_session_id,
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: second.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot while waiting");
    let diagnostics = snapshot
        .diagnostics
        .as_ref()
        .expect("scheduler diagnostics while waiting");

    assert_eq!(
        snapshot.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Pending
    );
    assert_eq!(
        snapshot.items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity)
    );
    assert_eq!(
        diagnostics.next_admission_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity)
    );
    assert_eq!(diagnostics.next_admission_wait_ms, None);
    assert_eq!(diagnostics.next_admission_not_before_ms, None);
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut second_run)
            .await
            .is_err(),
        "second run should remain queued until capacity becomes available"
    );

    host.release_first_run();

    let first_response = first_run
        .await
        .expect("first run join")
        .expect("first run response");
    let second_response = second_run
        .await
        .expect("second run join")
        .expect("second run response");

    assert_eq!(first_response.outputs.len(), 1);
    assert_eq!(second_response.outputs.len(), 1);
}

#[tokio::test]
async fn workflow_execution_session_run_waits_for_runtime_admission_before_dequeue() {
    let admission_open = Arc::new(AtomicBool::new(false));
    let host = AdmissionGatedHost::new(admission_open.clone());
    let service = WorkflowService::with_capacity_limits(1, 1);

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-gated".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create gated session");

    let run_service = service.clone();
    let run_host = host.clone();
    let session_id = created.session_id.clone();
    let mut run = tokio::spawn(async move {
        run_service
            .run_workflow_execution_session(
                &run_host,
                WorkflowExecutionSessionRunRequest {
                    session_id,
                    workflow_semantic_version: "0.1.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .await
    });

    tokio::time::sleep(Duration::from_millis(30)).await;

    let before_snapshot_ms = unix_timestamp_ms();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot while admission is blocked");
    let after_snapshot_ms = unix_timestamp_ms();
    let diagnostics = snapshot
        .diagnostics
        .as_ref()
        .expect("scheduler diagnostics while admission is blocked");

    assert_eq!(
        snapshot.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Pending
    );
    assert_eq!(
        snapshot.items[0].scheduler_decision_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission)
    );
    assert_eq!(
        diagnostics.next_admission_reason,
        Some(WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission)
    );
    assert_eq!(diagnostics.next_admission_wait_ms, Some(10));
    let next_admission_not_before_ms = diagnostics
        .next_admission_not_before_ms
        .expect("runtime-admission wait timestamp");
    assert!(next_admission_not_before_ms >= before_snapshot_ms.saturating_add(10));
    assert!(next_admission_not_before_ms <= after_snapshot_ms.saturating_add(10));
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut run)
            .await
            .is_err(),
        "run should remain queued until runtime admission opens"
    );

    admission_open.store(true, Ordering::SeqCst);

    let response = run
        .await
        .expect("run join")
        .expect("run response after admission opens");
    assert_eq!(response.outputs.len(), 1);
}
