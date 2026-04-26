use super::*;

#[tokio::test]
async fn workflow_cleanup_stale_execution_sessions_removes_idle_non_keep_alive_session() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    let response = service
        .workflow_cleanup_stale_execution_sessions(WorkflowExecutionSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert_eq!(
        response.cleaned_session_ids,
        vec![created.session_id.clone()]
    );
    let err = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect_err("cleaned session should be removed");
    assert!(matches!(err, WorkflowServiceError::SessionNotFound(_)));

    let second_response = service
        .workflow_cleanup_stale_execution_sessions(WorkflowExecutionSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("second cleanup stale sessions");
    assert!(
        second_response.cleaned_session_ids.is_empty(),
        "repeat cleanup should be idempotent once the stale session is gone"
    );
}

#[tokio::test]
async fn workflow_cleanup_stale_execution_sessions_keeps_session_with_queued_work() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

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
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    let response = service
        .workflow_cleanup_stale_execution_sessions(WorkflowExecutionSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert!(
        response.cleaned_session_ids.is_empty(),
        "queued sessions should remain scheduler-visible until the queue drains"
    );

    let session_id = created.session_id.clone();
    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest { session_id })
        .await
        .expect("scheduler snapshot");
    assert_eq!(snapshot.session.session_id, created.session_id);
    assert_eq!(snapshot.session.queued_runs, 1);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Pending
    );
}

#[tokio::test]
async fn workflow_cleanup_stale_execution_sessions_keeps_keep_alive_session() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow execution session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    let response = service
        .workflow_cleanup_stale_execution_sessions(WorkflowExecutionSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert!(response.cleaned_session_ids.is_empty());
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("keep-alive session should remain accessible");
    assert!(status.session.keep_alive);
}

#[tokio::test]
async fn workflow_get_execution_session_inspection_uses_host_owned_live_state_view() {
    let create_host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &create_host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .expect("create workflow execution session");

    let calls = Arc::new(Mutex::new(Vec::new()));
    let inspection_state = WorkflowGraphSessionStateView::new(
        node_engine::WorkflowExecutionSessionResidencyState::Warm,
        Vec::new(),
        None,
        None,
    );
    let inspection_host = InspectionHost {
        calls: calls.clone(),
        state: Some(inspection_state.clone()),
    };

    let response = service
        .workflow_get_execution_session_inspection(
            &inspection_host,
            WorkflowExecutionSessionInspectionRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .expect("inspect workflow execution session");

    assert_eq!(response.session.session_id, created.session_id);
    assert_eq!(response.session.workflow_id, "wf-1");
    assert_eq!(
        response.workflow_execution_session_state,
        Some(inspection_state)
    );
    assert_eq!(
        calls
            .lock()
            .expect("inspection host calls lock poisoned")
            .as_slice(),
        &[(created.session_id, "wf-1".to_string())]
    );
}

#[tokio::test]
async fn workflow_cleanup_stale_execution_sessions_respects_recent_status_reads() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("status read should refresh session access");

    let response = service
        .workflow_cleanup_stale_execution_sessions(WorkflowExecutionSessionStaleCleanupRequest {
            idle_timeout_ms: 1_000,
        })
        .await
        .expect("cleanup stale sessions");

    assert!(response.cleaned_session_ids.is_empty());
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("recently accessed session should remain accessible");
    assert_eq!(
        status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
}

#[tokio::test]
async fn workflow_stale_cleanup_worker_removes_stale_sessions() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_execution_session_stale_cleanup_worker(
            WorkflowExecutionSessionStaleCleanupWorkerConfig {
                interval: Duration::from_millis(10),
                idle_timeout: Duration::from_millis(20),
            },
        )
        .expect("spawn stale cleanup worker");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let removed = {
                let store = service
                    .session_store
                    .lock()
                    .expect("session store lock poisoned");
                !store.active.contains_key(&created.session_id)
            };
            if removed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("worker should remove stale workflow execution session");

    worker.shutdown().await;
}

#[tokio::test]
async fn workflow_stale_cleanup_worker_keeps_sessions_with_queued_work() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_execution_session_stale_cleanup_worker(
            WorkflowExecutionSessionStaleCleanupWorkerConfig {
                interval: Duration::from_millis(10),
                idle_timeout: Duration::from_millis(20),
            },
        )
        .expect("spawn stale cleanup worker");
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

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
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    tokio::time::sleep(Duration::from_millis(80)).await;

    let snapshot = service
        .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .expect("scheduler snapshot");
    assert_eq!(snapshot.session.session_id, created.session_id);
    assert_eq!(snapshot.session.queued_runs, 1);
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(
        snapshot.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Pending
    );

    worker.shutdown().await;
}

#[tokio::test]
async fn workflow_stale_cleanup_worker_shutdown_stops_future_cleanup() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_execution_session_stale_cleanup_worker(
            WorkflowExecutionSessionStaleCleanupWorkerConfig {
                interval: Duration::from_secs(1),
                idle_timeout: Duration::from_millis(20),
            },
        )
        .expect("spawn stale cleanup worker");
    worker.shutdown().await;
    worker.shutdown().await;

    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create workflow execution session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let state = store
            .active
            .get_mut(&created.session_id)
            .expect("session state should exist");
        state.last_accessed_at_ms = unix_timestamp_ms().saturating_sub(5_000);
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("shutdown worker should not remove stale sessions");
    assert_eq!(
        status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
}

#[test]
fn workflow_stale_cleanup_worker_requires_active_tokio_runtime() {
    let service = Arc::new(WorkflowService::new());
    let err = match service.spawn_workflow_execution_session_stale_cleanup_worker(
        WorkflowExecutionSessionStaleCleanupWorkerConfig::default(),
    ) {
        Ok(_) => panic!("spawn should fail without an active tokio runtime"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        WorkflowServiceError::Internal(ref message)
            if message.contains("requires an active Tokio runtime")
    ));
}

#[test]
fn workflow_stale_cleanup_worker_accepts_explicit_runtime_handle() {
    let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
    let service = Arc::new(WorkflowService::new());
    let worker = service
        .spawn_workflow_execution_session_stale_cleanup_worker_with_handle(
            WorkflowExecutionSessionStaleCleanupWorkerConfig::default(),
            runtime.handle().clone(),
        )
        .expect("spawn stale cleanup worker with explicit runtime handle");

    runtime.block_on(async move {
        worker.shutdown().await;
    });
}
