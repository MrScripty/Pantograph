use super::*;

#[test]
fn loaded_runtime_capacity_limit_clamps_to_valid_session_bounds() {
    let service = WorkflowService::with_capacity_limits(4, 4);

    service
        .set_loaded_runtime_capacity_limit(Some(2))
        .expect("set lower loaded-runtime capacity");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        2
    );

    service
        .set_loaded_runtime_capacity_limit(Some(0))
        .expect("clamp loaded-runtime capacity to minimum");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        1
    );

    service
        .set_loaded_runtime_capacity_limit(Some(99))
        .expect("clamp loaded-runtime capacity to session limit");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        4
    );

    service
        .set_loaded_runtime_capacity_limit(None)
        .expect("reset loaded-runtime capacity to session limit");
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .max_loaded_sessions,
        4
    );
}

#[tokio::test]
async fn workflow_execution_session_create_returns_scheduler_busy_at_capacity() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(1);

    let _first = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first");

    let err = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect_err("second session should fail at capacity");
    assert_eq!(
        err.to_envelope().details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::session_capacity_reached(1, 1),
        ))
    );
}

#[tokio::test]
async fn workflow_execution_session_capacity_is_released_after_close() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(1);
    let first = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let err = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect_err("scheduler should be busy at session capacity");
    assert_eq!(
        err.to_envelope().details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::session_capacity_reached(1, 1),
        ))
    );

    let closed = service
        .close_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCloseRequest {
                session_id: first.session_id,
            },
        )
        .await
        .expect("close session");
    assert!(closed.ok);

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
        .expect("create session after close");

    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("get status");
    assert_eq!(
        status.session.session_kind,
        WorkflowExecutionSessionKind::Workflow
    );
    assert!(!status.session.keep_alive);
}

#[tokio::test]
async fn workflow_execution_session_create_surfaces_runtime_capacity_details_when_no_unload_candidate_available(
) {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(2, 1);
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
        .expect("create loaded keep-alive session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let queue_id = store
            .enqueue_run(
                &loaded.session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: loaded.session_id.clone(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    run_id: Some("run-loaded".to_string()),
                    priority: None,
                },
            )
            .expect("enqueue run for loaded session");
        let dequeued = store
            .begin_queued_run(&loaded.session_id, &queue_id)
            .expect("begin queued run");
        assert!(
            dequeued.is_some(),
            "loaded session should transition into an active run"
        );
    }

    let err = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-blocked".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect_err("second keep-alive session should fail while loaded capacity is pinned");
    assert_eq!(
        err.to_envelope().details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(1, 1, 0),
        ))
    );
}
