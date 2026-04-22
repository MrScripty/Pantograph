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
async fn workflow_session_create_returns_scheduler_busy_at_capacity() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(1);

    let _first = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create first");

    let err = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
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
async fn workflow_session_capacity_is_released_after_close() {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_max_sessions(1);
    let first = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .expect("create session");

    let err = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
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
        .close_workflow_session(
            &host,
            WorkflowSessionCloseRequest {
                session_id: first.session_id,
            },
        )
        .await
        .expect("close session");
    assert!(closed.ok);

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
        .expect("create session after close");

    let status = service
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: created.session_id,
        })
        .await
        .expect("get status");
    assert_eq!(status.session.session_kind, WorkflowSessionKind::Workflow);
    assert!(!status.session.keep_alive);
}

#[tokio::test]
async fn workflow_session_create_surfaces_runtime_capacity_details_when_no_unload_candidate_available()
 {
    let host = MockWorkflowHost::new(8, 1024);
    let service = WorkflowService::with_capacity_limits(2, 1);
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
        .expect("create loaded keep-alive session");

    {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let queue_id = store
            .enqueue_run(
                &loaded.session_id,
                &WorkflowSessionRunRequest {
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
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
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

#[tokio::test]
async fn workflow_session_capacity_rebalance_uses_host_selected_candidate() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let service = WorkflowService::with_capacity_limits(3, 2);

    let first = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("first".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create first keep-alive session");
    let second = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-2".to_string(),
                usage_profile: Some("second".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create second keep-alive session");
    let third = service
        .create_workflow_session(
            &SelectingRuntimeHost::new(String::new(), unloads.clone()),
            WorkflowSessionCreateRequest {
                workflow_id: "wf-3".to_string(),
                usage_profile: Some("third".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create third session");
    let third_session_id = third.session_id.clone();

    let selecting_host = SelectingRuntimeHost::new(second.session_id.clone(), unloads.clone());

    service
        .run_workflow_session(
            &selecting_host,
            WorkflowSessionRunRequest {
                session_id: third_session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run third session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first(),
        Some(&(
            second.session_id.clone(),
            WorkflowSessionUnloadReason::CapacityRebalance,
        ))
    );
    assert!(
        unloads
            .iter()
            .any(|(session_id, _)| session_id == &third_session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|(session_id, _)| session_id == &first.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_affine_idle_runtime_by_default() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::new(unloads.clone());
    let service = WorkflowService::with_capacity_limits(3, 2);

    let affine = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create affine keep-alive session");
    let non_affine = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other".to_string(),
                usage_profile: Some("batch".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create non-affine keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(non_affine.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &affine.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_shared_model_idle_runtime() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::with_runtime_affinity(
        unloads.clone(),
        HashMap::from([
            ("wf-target".to_string(), vec!["llama_cpp".to_string()]),
            ("wf-shared-model".to_string(), vec!["llama_cpp".to_string()]),
            ("wf-other-model".to_string(), vec!["pytorch".to_string()]),
        ]),
        HashMap::from([
            ("wf-target".to_string(), vec!["model-a".to_string()]),
            ("wf-shared-model".to_string(), vec!["model-a".to_string()]),
            ("wf-other-model".to_string(), vec!["model-b".to_string()]),
        ]),
    );
    let service = WorkflowService::with_capacity_limits(3, 2);

    let shared_model = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared-model".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create shared-model keep-alive session");
    let other_model = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other-model".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create other-model keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-target".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(other_model.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &shared_model.session_id)
    );
}

#[tokio::test]
async fn workflow_session_capacity_rebalance_preserves_shared_backend_idle_runtime() {
    let unloads = Arc::new(Mutex::new(Vec::new()));
    let host = AffinityRuntimeHost::with_runtime_affinity(
        unloads.clone(),
        HashMap::from([
            ("wf-target".to_string(), vec!["llama_cpp".to_string()]),
            (
                "wf-shared-backend".to_string(),
                vec!["llama_cpp".to_string()],
            ),
            ("wf-other-backend".to_string(), vec!["pytorch".to_string()]),
        ]),
        HashMap::from([
            ("wf-target".to_string(), vec!["model-a".to_string()]),
            ("wf-shared-backend".to_string(), vec!["model-z".to_string()]),
            ("wf-other-backend".to_string(), vec!["model-a".to_string()]),
        ]),
    );
    let service = WorkflowService::with_capacity_limits(3, 2);

    let shared_backend = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-shared-backend".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create shared-backend keep-alive session");
    let other_backend = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-other-backend".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect("create other-backend keep-alive session");
    let target = service
        .create_workflow_session(
            &host,
            WorkflowSessionCreateRequest {
                workflow_id: "wf-target".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
            },
        )
        .await
        .expect("create target session");

    service
        .run_workflow_session(
            &host,
            WorkflowSessionRunRequest {
                session_id: target.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                run_id: None,
                priority: None,
            },
        )
        .await
        .expect("run target session");

    let unloads = unloads.lock().expect("unloads lock poisoned");
    assert_eq!(
        unloads.first().map(String::as_str),
        Some(other_backend.session_id.as_str())
    );
    assert!(
        unloads
            .iter()
            .any(|session_id| session_id == &target.session_id)
    );
    assert!(
        !unloads
            .iter()
            .any(|session_id| session_id == &shared_backend.session_id)
    );
}
