use super::*;

#[tokio::test]
async fn failed_resume_does_not_recreate_residency_until_fresh_runnable_input_succeeds() {
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
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("release keep-alive reservation");
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    rewrite_test_workflow_output_node_to_human_input(temp.path(), "runtime-text");

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("beta"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "value".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect_err("resume should fail when the output node now requires interactive input");
    match error {
        WorkflowServiceError::InvalidRequest(message)
        | WorkflowServiceError::CapabilityViolation(message) => {
            assert!(
                message.contains("text-output-1")
                    || message.contains("scheduler task session runner"),
                "unexpected fail-closed scheduler message: {message}"
            );
        }
        other => panic!("expected typed scheduler rejection, got {other:?}"),
    }
    assert_no_runtime_reservations(&runtime_registry);

    write_test_workflow(temp.path(), "runtime-text");

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
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
        .expect("resume should succeed after restoring a runnable graph");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("gamma"));
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn failed_pre_execution_resume_does_not_create_residency_until_fresh_input_succeeds() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir: app_data_dir.clone(),
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
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("release keep-alive reservation");
    assert_no_runtime_reservations(&runtime_registry);

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
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
        .expect_err("resume without fresh input must fail before residency is recreated");
    assert_missing_required_input_diagnostic(&error);
    assert_no_runtime_reservations(&runtime_registry);

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
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
        .expect("resume should succeed after the runtime becomes ready again");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("gamma"));
    assert_absent_or_single_keep_alive_reservation(&runtime_registry, "runtime-text");

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn scheduler_reclaim_keeps_residency_owner_isolated_across_fresh_resumes() {
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

    let session_a = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create first keep-alive session");
    let session_b = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create second keep-alive session");

    let first_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_a.session_id.clone(),
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
    assert_single_keep_alive_reservation_for_workflow(&runtime_registry, "runtime-text");

    let second_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_b.session_id.clone(),
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
        .expect("run second keep-alive session under reclaim pressure");
    assert_eq!(second_output.outputs[0].value, serde_json::json!("beta"));
    assert_single_keep_alive_reservation_for_workflow(&runtime_registry, "runtime-text");

    let resumed_a = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_a.session_id.clone(),
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
        .expect("resume first session after scheduler reclaim");
    assert_eq!(resumed_a.outputs[0].value, serde_json::json!("gamma"));
    assert_single_keep_alive_reservation_for_workflow(&runtime_registry, "runtime-text");

    let resumed_b = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_b.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("delta"),
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
        .expect("resume second session after reclaiming the first");
    assert_eq!(resumed_b.outputs[0].value, serde_json::json!("delta"));
    assert_single_keep_alive_reservation_for_workflow(&runtime_registry, "runtime-text");

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session_a.session_id.clone(),
        })
        .await
        .expect("close first resumed keep-alive session");
    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session_b.session_id.clone(),
        })
        .await
        .expect("close second resumed keep-alive session");
}

fn assert_single_keep_alive_reservation_for_workflow(
    runtime_registry: &RuntimeRegistry,
    workflow_id: &str,
) -> u64 {
    let snapshot = runtime_registry.snapshot();
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
