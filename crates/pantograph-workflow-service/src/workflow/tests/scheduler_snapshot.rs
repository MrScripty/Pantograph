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
    assert!(snapshot
        .items
        .iter()
        .all(|item| item.status == WorkflowSessionQueueItemStatus::Pending));
}
