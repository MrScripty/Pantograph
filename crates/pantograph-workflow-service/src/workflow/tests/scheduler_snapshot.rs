use super::*;

#[tokio::test]
async fn workflow_get_scheduler_snapshot_returns_workflow_session_summary() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow session");

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot");

    assert_eq!(snapshot.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(snapshot.session_id, created.session_id);
    assert_eq!(snapshot.session.session_kind, WorkflowSessionKind::Workflow);
    assert_eq!(snapshot.session.workflow_id, "wf-1");
    assert_eq!(
        snapshot.session.usage_profile.as_deref(),
        Some("interactive")
    );
    assert_eq!(snapshot.trace_execution_id, None);
    assert!(snapshot.items.is_empty());
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_returns_edit_session_lifecycle() {
    let service = WorkflowService::new();
    let created = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: WorkflowGraph::new(),
        })
        .await
        .expect("create edit session");

    let idle_snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("idle edit snapshot");
    assert_eq!(idle_snapshot.workflow_id, None);
    assert_eq!(
        idle_snapshot.session.session_kind,
        WorkflowSessionKind::Edit
    );
    assert_eq!(
        idle_snapshot.session.state,
        WorkflowSessionState::IdleLoaded
    );
    assert_eq!(idle_snapshot.session.queued_runs, 0);
    assert_eq!(idle_snapshot.session.run_count, 0);
    assert_eq!(idle_snapshot.trace_execution_id, None);
    assert!(idle_snapshot.items.is_empty());

    service
        .workflow_graph_mark_edit_session_running(&created.session_id)
        .await
        .expect("mark running");

    let running_snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("running edit snapshot");
    assert_eq!(
        running_snapshot.session.session_kind,
        WorkflowSessionKind::Edit
    );
    assert_eq!(
        running_snapshot.session.state,
        WorkflowSessionState::Running
    );
    assert_eq!(running_snapshot.session.queued_runs, 1);
    assert_eq!(running_snapshot.items.len(), 1);
    assert_eq!(
        running_snapshot.items[0].status,
        WorkflowSessionQueueItemStatus::Running
    );
    let started_at_ms = running_snapshot.items[0]
        .enqueued_at_ms
        .expect("edit session running item should expose start time");
    assert_eq!(
        running_snapshot.items[0].dequeued_at_ms,
        Some(started_at_ms)
    );
    assert_eq!(
        running_snapshot.items[0].run_id.as_deref(),
        Some(created.session_id.as_str())
    );
    assert_eq!(
        running_snapshot.trace_execution_id.as_deref(),
        Some(created.session_id.as_str())
    );

    service
        .workflow_graph_mark_edit_session_finished(&created.session_id)
        .await
        .expect("finish running edit session");

    let completed_snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id,
        })
        .await
        .expect("completed edit snapshot");
    assert_eq!(
        completed_snapshot.session.state,
        WorkflowSessionState::IdleLoaded
    );
    assert_eq!(completed_snapshot.session.queued_runs, 0);
    assert_eq!(completed_snapshot.session.run_count, 1);
    assert_eq!(completed_snapshot.trace_execution_id, None);
    assert!(completed_snapshot.items.is_empty());
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_exposes_single_visible_queue_run_as_trace_execution() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("queued-run-1".to_string()),
                    priority: None,
                },
            )
            .expect("enqueue run");
    }

    let session_id = created.session_id.clone();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
        .await
        .expect("scheduler snapshot");

    assert_eq!(snapshot.trace_execution_id.as_deref(), Some("queued-run-1"));
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowSessionQueueItemStatus::Pending
    );
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_exposes_next_admission_diagnostics() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(2, 1);
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow session");

    let queue_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .enqueue_run(
                &created.session_id,
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("queued-run-1".to_string()),
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
        diagnostics.next_admission_queue_id.as_deref(),
        Some(queue_id.as_str())
    );
    assert_eq!(diagnostics.next_admission_bypassed_queue_id, None);
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
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-loaded".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create loaded session");
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
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
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("queued-run-1".to_string()),
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
        recorded_requests[0].next_admission_queue_id.as_deref(),
        Some(queue_id.as_str())
    );
    assert_eq!(recorded_requests[0].reclaim_candidates.len(), 1);
    assert_eq!(
        recorded_requests[0].reclaim_candidates[0].session_id,
        loaded.session_id
    );
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_reports_bypassed_queue_head_for_warm_reuse() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow session");

    let (cold_head_queue_id, warm_queue_id) = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .update_runtime_affinity_basis(
                &created.session_id,
                vec!["llama_cpp".to_string()],
                vec!["model-a".to_string()],
            )
            .expect("update runtime affinity basis");
        store
            .mark_runtime_loaded(&created.session_id, true)
            .expect("mark runtime loaded");
        let cold_head_queue_id = store
            .enqueue_run(
                &created.session_id,
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: Some(WorkflowTechnicalFitOverride {
                        model_id: Some("model-b".to_string()),
                        backend_key: Some("pytorch".to_string()),
                    }),
                    timeout_ms: None,
                    run_id: Some("cold-head".to_string()),
                    priority: Some(1),
                },
            )
            .expect("enqueue cold head");
        let warm_queue_id = store
            .enqueue_run(
                &created.session_id,
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("warm-follow".to_string()),
                    priority: Some(1),
                },
            )
            .expect("enqueue warm follow");
        (cold_head_queue_id, warm_queue_id)
    };

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id,
        })
        .await
        .expect("scheduler snapshot");
    let diagnostics = snapshot.diagnostics.expect("scheduler diagnostics");

    assert_eq!(
        diagnostics.next_admission_queue_id.as_deref(),
        Some(warm_queue_id.as_str())
    );
    assert_eq!(
        diagnostics.next_admission_bypassed_queue_id.as_deref(),
        Some(cold_head_queue_id.as_str())
    );
    assert_eq!(
        diagnostics.next_admission_reason,
        Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
    );
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_marks_rebalance_required_when_idle_runtime_can_be_reclaimed()
 {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(3, 1);
    let _loaded = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-loaded".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create loaded session");
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
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
                &WorkflowSessionRunRequest {
                    session_id: created.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("queued-run-1".to_string()),
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
        WorkflowSessionState::IdleUnloaded,
        "the queued session should still be unloaded before admission"
    );
}

#[tokio::test]
async fn workflow_get_scheduler_snapshot_omits_trace_execution_for_ambiguous_pending_queue() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        for run_id in ["queued-run-1", "queued-run-2"] {
            store
                .enqueue_run(
                    &created.session_id,
                    &WorkflowSessionRunRequest {
                        session_id: created.session_id.clone(),
                        inputs: Vec::new(),
                        output_targets: None,
                        override_selection: None,
                        timeout_ms: None,
                        run_id: Some(run_id.to_string()),
                        priority: None,
                    },
                )
                .expect("enqueue run");
        }
    }

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id,
        })
        .await
        .expect("scheduler snapshot");

    assert_eq!(snapshot.trace_execution_id, None);
    assert_eq!(snapshot.items.len(), 2);
    assert!(
        snapshot
            .items
            .iter()
            .all(|item| item.status == WorkflowSessionQueueItemStatus::Pending)
    );
}
