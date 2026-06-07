use super::*;

#[tokio::test]
async fn keep_alive_session_releases_residency_across_capacity_rebalance() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let first = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create first keep-alive session");

    let first_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: first.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("run first keep-alive session");
    assert_eq!(first_output.outputs[0].value, serde_json::json!("alpha"));
    assert_no_session_executor(&runtime, &first.session_id);
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &first.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("release keep-alive reservation for capacity rebalance");
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    let missing_input_error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: first.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect_err("capacity resume must not replay previous inputs");
    assert_missing_required_input_diagnostic(&missing_input_error);
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: first.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("resume keep-alive session with fresh input");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("beta"));
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: first.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
    assert_no_session_executor(&runtime, &first.session_id);
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");
}

#[tokio::test]
async fn scheduler_driven_rebalance_releases_keep_alive_until_fresh_resume() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let keep_alive = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    let first_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: keep_alive.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("run keep-alive session");
    assert_eq!(first_output.outputs[0].value, serde_json::json!("alpha"));
    assert_single_runtime_reservation(
        &runtime_registry,
        "runtime-text",
        &keep_alive.session_id,
        RuntimeRetentionHint::KeepAlive,
    );

    let one_shot = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("batch".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create one-shot session");

    let second_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: one_shot.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("run one-shot session under capacity pressure");
    assert_eq!(second_output.outputs[0].value, serde_json::json!("beta"));
    assert_single_runtime_reservation(
        &runtime_registry,
        "runtime-text",
        &keep_alive.session_id,
        RuntimeRetentionHint::KeepAlive,
    );

    let missing_input_error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: keep_alive.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect_err("scheduler resume must require fresh input");
    assert_missing_required_input_diagnostic(&missing_input_error);
    assert_single_runtime_reservation(
        &runtime_registry,
        "runtime-text",
        &keep_alive.session_id,
        RuntimeRetentionHint::KeepAlive,
    );

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: keep_alive.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("gamma"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("resume keep-alive session after scheduler rebalance");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("gamma"));
    assert_single_runtime_reservation(
        &runtime_registry,
        "runtime-text",
        &keep_alive.session_id,
        RuntimeRetentionHint::KeepAlive,
    );

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: keep_alive.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: one_shot.session_id.clone(),
        })
        .await
        .expect("close one-shot session");
}

#[tokio::test]
async fn repeated_capacity_unload_is_idempotent_and_keep_alive_disable_clears_residency() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let session = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("alpha"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect("run keep-alive session");
    assert_single_runtime_reservation(
        &runtime_registry,
        "runtime-text",
        &session.session_id,
        RuntimeRetentionHint::KeepAlive,
    );

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("first capacity unload");
    assert_no_runtime_reservations(&runtime_registry);

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("second capacity unload should be idempotent");
    assert_no_runtime_reservations(&runtime_registry);

    runtime
        .workflow_set_execution_session_keep_alive(WorkflowExecutionSessionKeepAliveRequest {
            session_id: session.session_id.clone(),
            keep_alive: false,
        })
        .await
        .expect("disable keep-alive after capacity unload");
    assert_no_runtime_reservations(&runtime_registry);
    assert_no_session_executor(&runtime, &session.session_id);
}

fn assert_single_runtime_reservation(
    runtime_registry: &RuntimeRegistry,
    workflow_id: &str,
    session_id: &str,
    retention_hint: RuntimeRetentionHint,
) -> u64 {
    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    let reservation = &snapshot.reservations[0];
    assert_eq!(reservation.workflow_id, workflow_id);
    assert_eq!(
        reservation.reservation_owner_id.as_deref(),
        Some(session_id)
    );
    assert_eq!(reservation.retention_hint, retention_hint);
    assert!(
        snapshot.runtimes.iter().any(|runtime| runtime
            .active_reservation_ids
            .contains(&reservation.reservation_id)),
        "runtime snapshot should expose the active keep-alive reservation"
    );
    reservation.reservation_id
}

fn assert_no_runtime_reservations(runtime_registry: &RuntimeRegistry) {
    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert!(snapshot
        .runtimes
        .iter()
        .all(|runtime| runtime.active_reservation_ids.is_empty()));
}

fn assert_absent_or_single_keep_alive_reservation(
    runtime_registry: &RuntimeRegistry,
    workflow_id: &str,
) {
    let snapshot = runtime_registry.snapshot();
    if snapshot.reservations.is_empty() {
        assert!(snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty()));
        return;
    }

    assert_eq!(snapshot.reservations.len(), 1);
    let reservation = &snapshot.reservations[0];
    assert_eq!(reservation.workflow_id, workflow_id);
    assert_eq!(reservation.retention_hint, RuntimeRetentionHint::KeepAlive);
    assert!(
        snapshot.runtimes.iter().any(|runtime| runtime
            .active_reservation_ids
            .contains(&reservation.reservation_id)),
        "runtime snapshot should expose the active keep-alive reservation"
    );
}

fn assert_no_session_executor(runtime: &EmbeddedRuntime, session_id: &str) {
    assert!(runtime
        .session_executions
        .handle(session_id)
        .expect("session execution lookup should succeed")
        .is_none());
}

fn assert_missing_required_input_diagnostic(error: &WorkflowServiceError) {
    let message = match error {
        WorkflowServiceError::InvalidRequest(message) => message,
        WorkflowServiceError::Internal(message) => message,
        other => panic!("expected typed missing-input diagnostic, got {other:?}"),
    };
    assert!(
        message.contains("text-input-1") || message.contains("input"),
        "missing-input diagnostic should identify the omitted input, got: {message}"
    );
}
