use super::*;

#[tokio::test]
async fn keep_alive_session_requires_inputs_per_run_without_executor_carry_forward() {
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
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");
    let session_id = created.session_id.clone();

    let first_run = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        .expect("run keep-alive session first time");
    assert_eq!(first_run.outputs[0].value, serde_json::json!("alpha"));

    assert!(runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .is_none());
    assert_keep_alive_runtime_residency(&runtime_registry, "runtime-text");

    let missing_input_error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        .expect_err("omitted required input must not carry forward");
    assert_missing_required_input_diagnostic(&missing_input_error);

    let third_run = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        .expect("run keep-alive session after updating one input");
    assert_eq!(third_run.outputs[0].value, serde_json::json!("beta"));
    assert_keep_alive_runtime_residency(&runtime_registry, "runtime-text");

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("close keep-alive session");
    assert!(runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .is_none());
}

#[tokio::test]
async fn keep_alive_session_graph_change_requires_fresh_inputs() {
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
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone());

    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");
    let session_id = created.session_id.clone();

    let first_run = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        .expect("run before workflow edit");
    assert_eq!(first_run.outputs[0].value, serde_json::json!("alpha"));
    assert_keep_alive_runtime_residency(&runtime_registry, "runtime-text");

    rewrite_test_workflow_input_description(
        temp.path(),
        "runtime-text",
        "Prompt updated after session creation",
    );

    let missing_input_error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        .expect_err("graph change must not replay carried inputs");
    assert_missing_required_input_diagnostic(&missing_input_error);

    let second_run = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
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
        .expect("run after workflow edit with fresh input");
    assert_eq!(second_run.outputs[0].value, serde_json::json!("beta"));
    assert_keep_alive_runtime_residency(&runtime_registry, "runtime-text");
}

fn assert_keep_alive_runtime_residency(runtime_registry: &RuntimeRegistry, workflow_id: &str) {
    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.reservations[0].workflow_id, workflow_id);
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
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
