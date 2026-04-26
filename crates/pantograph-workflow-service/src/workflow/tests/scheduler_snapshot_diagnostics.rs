use super::*;

#[tokio::test]
async fn workflow_get_scheduler_snapshot_exposes_next_admission_diagnostics() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(2, 1);
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    let queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .expect("enqueue run")
    };

    let session_id = created.session_id.clone();
    let before_snapshot_ms = unix_timestamp_ms();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
        .await
        .expect("scheduler snapshot");
    let after_snapshot_ms = unix_timestamp_ms();
    let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

    assert_eq!(diagnostics.loaded_session_count, 0);
    assert_eq!(diagnostics.max_loaded_sessions, 1);
    assert_eq!(diagnostics.reclaimable_loaded_session_count, 0);
    assert_eq!(
        diagnostics.runtime_capacity_pressure,
        WorkflowSchedulerRuntimeCapacityPressure::Available
    );
    assert!(!diagnostics.active_run_blocks_admission);
    assert_eq!(
        diagnostics.next_admission_workflow_run_id.as_deref(),
        Some(queue_id.as_str())
    );
    assert_eq!(diagnostics.next_admission_bypassed_workflow_run_id, None);
    assert_eq!(diagnostics.next_admission_after_runs, Some(0));
    assert_eq!(diagnostics.next_admission_wait_ms, Some(0));
    let next_admission_not_before_ms = diagnostics
        .next_admission_not_before_ms
        .expect("immediate admission not-before timestamp");
    assert!(next_admission_not_before_ms >= before_snapshot_ms);
    assert!(next_admission_not_before_ms <= after_snapshot_ms);
    assert_eq!(
        diagnostics.next_admission_reason,
        Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
    );
    assert_eq!(diagnostics.runtime_registry, None);
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_merges_runtime_registry_diagnostics_from_provider() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(3, 1);
    let requests = Arc::new(Mutex::new(Vec::new()));
    service
        .set_scheduler_diagnostics_provider(Some(Arc::new(MockSchedulerDiagnosticsProvider {
            diagnostics: WorkflowSchedulerRuntimeRegistryDiagnostics {
                target_runtime_id: Some("llama_cpp".to_string()),
                reclaim_candidate_session_id: Some("session-loaded".to_string()),
                reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
                next_warmup_decision: Some(
                    WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,
                ),
                next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady),
            },
            requests: requests.clone(),
        })))
        .expect("scheduler diagnostics provider should be installed");

    let loaded = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-loaded".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create loaded session");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-queued".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create queued session");

    let queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .expect("enqueue run")
    };

    let session_id = created.session_id.clone();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
        .await
        .expect("scheduler snapshot");
    let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

    assert_eq!(
        diagnostics.runtime_registry,
        Some(WorkflowSchedulerRuntimeRegistryDiagnostics {
            target_runtime_id: Some("llama_cpp".to_string()),
            reclaim_candidate_session_id: Some("session-loaded".to_string()),
            reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
            next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,),
            next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady,),
        })
    );
    let recorded_requests = requests
        .lock()
        .expect("scheduler diagnostics requests lock poisoned");
    assert_eq!(recorded_requests.len(), 1);
    assert_eq!(recorded_requests[0].session_id, created.session_id);
    assert_eq!(recorded_requests[0].workflow_id, "wf-queued");
    assert_eq!(
        recorded_requests[0].usage_profile.as_deref(),
        Some("interactive")
    );
    assert!(!recorded_requests[0].keep_alive);
    assert!(!recorded_requests[0].runtime_loaded);
    assert_eq!(
        recorded_requests[0]
            .next_admission_workflow_run_id
            .as_deref(),
        Some(queue_id.as_str())
    );
    assert_eq!(recorded_requests[0].reclaim_candidates.len(), 1);
    assert_eq!(
        recorded_requests[0].reclaim_candidates[0].session_id,
        loaded.session_id
    );
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_marks_rebalance_required_when_idle_runtime_can_be_reclaimed(
) {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(3, 1);
    let _loaded = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-loaded".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create loaded session");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-queued".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create queued session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: Some(1),
                },
            )
            .expect("enqueue run");
    }

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id,
        })
        .await
        .expect("scheduler snapshot");
    let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

    assert_eq!(diagnostics.loaded_session_count, 1);
    assert_eq!(diagnostics.max_loaded_sessions, 1);
    assert_eq!(diagnostics.reclaimable_loaded_session_count, 1);
    assert_eq!(
        diagnostics.runtime_capacity_pressure,
        WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired
    );
    assert_eq!(
        snapshot.session.state,
        WorkflowExecutionSessionState::IdleUnloaded,
        "the queued session should still be unloaded before admission"
    );
}
