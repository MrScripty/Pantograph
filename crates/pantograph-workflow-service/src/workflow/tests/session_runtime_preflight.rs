use super::*;

#[tokio::test]
async fn workflow_execution_session_runtime_preflight_is_cached_until_graph_changes() {
    let workflow_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let runtime_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let graph_fingerprint = Arc::new(Mutex::new("graph-a".to_string()));
    let technical_fit_requests = Arc::new(Mutex::new(Vec::new()));
    let host = CountingPreflightHost {
        workflow_capabilities_calls: workflow_capabilities_calls.clone(),
        runtime_capabilities_calls: runtime_capabilities_calls.clone(),
        graph_fingerprint: graph_fingerprint.clone(),
        technical_fit_requests,
    };
    let service = WorkflowService::with_max_sessions(1);

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
        .expect("create session");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("first run");
    assert_eq!(workflow_capabilities_calls.load(Ordering::SeqCst), 1);

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("second run");
    assert_eq!(
        workflow_capabilities_calls.load(Ordering::SeqCst),
        1,
        "unchanged graph should reuse cached preflight"
    );
    assert_eq!(runtime_capabilities_calls.load(Ordering::SeqCst), 3);

    *graph_fingerprint
        .lock()
        .expect("graph fingerprint lock poisoned") = "graph-b".to_string();

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("third run after graph change");
    assert_eq!(
        workflow_capabilities_calls.load(Ordering::SeqCst),
        2,
        "graph change should invalidate cached preflight"
    );
}

#[tokio::test]
async fn workflow_execution_session_runtime_preflight_cache_invalidates_on_override_selection_change(
) {
    let workflow_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let runtime_capabilities_calls = Arc::new(AtomicUsize::new(0));
    let technical_fit_requests = Arc::new(Mutex::new(Vec::new()));
    let host = CountingPreflightHost {
        workflow_capabilities_calls: workflow_capabilities_calls.clone(),
        runtime_capabilities_calls: runtime_capabilities_calls.clone(),
        graph_fingerprint: Arc::new(Mutex::new("graph-a".to_string())),
        technical_fit_requests: technical_fit_requests.clone(),
    };
    let service = WorkflowService::with_max_sessions(1);

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
        .expect("create session");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: Some(WorkflowTechnicalFitOverride {
                    model_id: None,
                    backend_key: Some("llama.cpp".to_string()),
                }),
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("first run");

    service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id,
                inputs: Vec::new(),
                output_targets: None,
                override_selection: Some(WorkflowTechnicalFitOverride {
                    model_id: Some("model-a".to_string()),
                    backend_key: Some("llama.cpp".to_string()),
                }),
                timeout_ms: None,
                priority: None,
            },
        )
        .await
        .expect("second run");

    let requests = technical_fit_requests
        .lock()
        .expect("technical-fit requests lock poisoned");
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].override_selection,
        Some(WorkflowTechnicalFitOverride {
            model_id: None,
            backend_key: Some("llama_cpp".to_string()),
        })
    );
    assert_eq!(
        requests[1].override_selection,
        Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama_cpp".to_string()),
        })
    );
    assert_eq!(
        workflow_capabilities_calls.load(Ordering::SeqCst),
        2,
        "override changes should invalidate cached preflight"
    );
    assert_eq!(runtime_capabilities_calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn keep_alive_session_create_blocks_when_runtime_preflight_fails() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        8,
        1024,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::MissingRuntimeState,
                None,
            )],
        },
    );
    let service = WorkflowService::with_max_sessions(1);

    let err = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "wf-1".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            },
        )
        .await
        .expect_err("keep-alive session create should fail when runtime preflight blocks");

    assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
    assert_eq!(
        service
            .session_store
            .lock()
            .expect("session store lock poisoned")
            .active
            .len(),
        0,
        "failed keep-alive create should roll back session creation"
    );
}

#[tokio::test]
async fn keep_alive_enable_blocks_when_runtime_preflight_fails() {
    let host = MockWorkflowHost::with_technical_fit_decision(
        8,
        1024,
        WorkflowTechnicalFitDecision {
            selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: None,
            selected_runtime_id: None,
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: None,
            reasons: vec![WorkflowTechnicalFitReason::new(
                WorkflowTechnicalFitReasonCode::MissingRuntimeState,
                None,
            )],
        },
    );
    let service = WorkflowService::with_max_sessions(1);
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
        .expect("create unloaded session");

    let err = service
        .workflow_set_execution_session_keep_alive(
            &host,
            WorkflowExecutionSessionKeepAliveRequest {
                session_id: created.session_id.clone(),
                keep_alive: true,
            },
        )
        .await
        .expect_err("keep-alive enable should fail when runtime preflight blocks");

    assert!(matches!(err, WorkflowServiceError::RuntimeNotReady(_)));
    let summary = service
        .session_store
        .lock()
        .expect("session store lock poisoned")
        .session_summary(&created.session_id)
        .expect("session summary after failed keep-alive enable");
    assert_eq!(summary.state, WorkflowExecutionSessionState::IdleUnloaded);
    assert!(!summary.keep_alive);
}
