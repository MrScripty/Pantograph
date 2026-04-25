use super::*;

#[tokio::test]
async fn test_runtime_unload_candidate_selection_uses_registry_eviction_order() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    runtime_registry.observe_runtimes(vec![pantograph_runtime_registry::RuntimeObservation {
        runtime_id: "shared-runtime".to_string(),
        display_name: "shared-runtime".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("shared-runtime-1".to_string()),
        last_error: None,
    }]);
    runtime_registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-a".to_string(),
            reservation_owner_id: Some("session-a".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("keep-alive reservation");
    runtime_registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-b".to_string(),
            reservation_owner_id: Some("session-b".to_string()),
            usage_profile: Some("batch".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("ephemeral reservation");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry);

    let selected = runtime
        .host()
        .select_runtime_unload_candidate(
            &WorkflowExecutionSessionRuntimeSelectionTarget {
                session_id: "session-target".to_string(),
                workflow_id: "wf-a".to_string(),
                usage_profile: Some("interactive".to_string()),
                required_backends: Vec::new(),
                required_models: Vec::new(),
            },
            &[
                WorkflowExecutionSessionRuntimeUnloadCandidate {
                    session_id: "session-a".to_string(),
                    workflow_id: "wf-a".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    required_backends: Vec::new(),
                    required_models: Vec::new(),
                    keep_alive: true,
                    access_tick: 1,
                    run_count: 0,
                },
                WorkflowExecutionSessionRuntimeUnloadCandidate {
                    session_id: "session-b".to_string(),
                    workflow_id: "wf-b".to_string(),
                    usage_profile: Some("batch".to_string()),
                    required_backends: Vec::new(),
                    required_models: Vec::new(),
                    keep_alive: false,
                    access_tick: 99,
                    run_count: 5,
                },
            ],
        )
        .await
        .expect("select unload candidate")
        .expect("candidate should exist");

    assert_eq!(selected.session_id, "session-b");
}

#[tokio::test]
async fn workflow_preflight_reports_candle_runtime_as_available() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::with_backend(
            Box::new(inference::CandleBackend::new()),
            "Candle",
        )),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let response = runtime
        .workflow_preflight(WorkflowPreflightRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
        })
        .await
        .expect("workflow preflight");

    assert!(response.blocking_runtime_issues.is_empty());
    assert!(response.can_run);

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert_eq!(
        capabilities.runtime_requirements.required_backends,
        vec!["candle".to_string()]
    );
    let candle = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "candle")
        .expect("candle capability");
    assert_eq!(candle.source_kind, WorkflowRuntimeSourceKind::Host);
    assert!(candle.selected);
}

#[tokio::test]
async fn workflow_preflight_blocks_selected_runtime_failed_after_restart() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    rewrite_test_workflow_required_backend(temp.path(), "runtime-text", "llama_cpp");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);
    persist_failed_selected_runtime_version(&app_data_dir, "b8248", "validation failed");

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert_eq!(
        capabilities.runtime_requirements.required_backends,
        vec!["llama_cpp".to_string()]
    );
    let runtime_capability = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "llama_cpp")
        .expect("llama.cpp capability");
    assert_eq!(
        runtime_capability.readiness_state,
        Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Failed)
    );
    assert!(!runtime_capability.configured);
    assert_eq!(
        runtime_capability.unavailable_reason.as_deref(),
        Some("validation failed")
    );

    let preflight = runtime
        .workflow_preflight(WorkflowPreflightRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
        })
        .await
        .expect("workflow preflight");
    assert!(!preflight.can_run);
    assert!(preflight
        .blocking_runtime_issues
        .iter()
        .any(|issue| issue.message.contains("validation failed")));

    let session = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create workflow execution session");
    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id,
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: None,
        })
        .await
        .expect_err("workflow run should fail when selected runtime failed validation");
    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
}

#[tokio::test]
async fn workflow_preflight_blocks_interrupted_runtime_job_after_restart() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    rewrite_test_workflow_required_backend(temp.path(), "runtime-text", "llama_cpp");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);
    persist_interrupted_runtime_job(&app_data_dir);

    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert_eq!(
        capabilities.runtime_requirements.required_backends,
        vec!["llama_cpp".to_string()]
    );
    let runtime_capability = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "llama_cpp")
        .expect("llama.cpp capability");
    assert_eq!(
        runtime_capability.readiness_state,
        Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Failed)
    );
    assert!(!runtime_capability.configured);
    assert!(runtime_capability
        .unavailable_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("reconciled during startup")));

    let preflight = runtime
        .workflow_preflight(WorkflowPreflightRequest {
            workflow_id: "runtime-text".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
        })
        .await
        .expect("workflow preflight");
    assert!(!preflight.can_run);
    assert!(preflight
        .blocking_runtime_issues
        .iter()
        .any(|issue| issue.message.contains("reconciled during startup")));

    let session = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create workflow execution session");
    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id,
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: None,
        })
        .await
        .expect_err("workflow run should fail when restart reconciles an interrupted runtime job");
    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
}
