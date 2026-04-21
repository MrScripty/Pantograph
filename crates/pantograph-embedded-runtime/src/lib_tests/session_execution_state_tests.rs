use super::*;

#[tokio::test]
async fn keep_alive_session_reuses_backend_executor_and_carries_forward_inputs() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

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

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");
    let session_id = created.session_id.clone();

    let first_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
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
            run_id: Some("run-alpha".to_string()),
        })
        .await
        .expect("run keep-alive session first time");
    assert_eq!(first_run.outputs[0].value, serde_json::json!("alpha"));

    let first_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should exist");
    let first_snapshots = {
        let executor = first_executor.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&session_id)
            .await
    };
    assert_eq!(first_snapshots.len(), 2);
    assert!(
        first_snapshots
            .iter()
            .any(|snapshot| snapshot.identity.node_id == "text-input-1")
    );
    assert!(
        first_snapshots
            .iter()
            .any(|snapshot| snapshot.identity.node_id == "text-output-1")
    );

    let second_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-carry-forward".to_string()),
        })
        .await
        .expect("run keep-alive session with carried-forward inputs");
    assert_eq!(second_run.outputs[0].value, serde_json::json!("alpha"));

    let second_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should still exist");
    assert!(Arc::ptr_eq(&first_executor, &second_executor));

    let third_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
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
            run_id: Some("run-beta".to_string()),
        })
        .await
        .expect("run keep-alive session after updating one input");
    assert_eq!(third_run.outputs[0].value, serde_json::json!("beta"));

    let third_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should still exist");
    assert!(Arc::ptr_eq(&first_executor, &third_executor));

    runtime
        .close_workflow_session(WorkflowSessionCloseRequest {
            session_id: session_id.clone(),
        })
        .await
        .expect("close keep-alive session");
    assert!(
        runtime
            .session_executions
            .handle(&session_id)
            .expect("session execution lookup should succeed")
            .is_none()
    );
}

#[tokio::test]
async fn keep_alive_session_reconciles_graph_change_and_replays_carried_inputs() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

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

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");
    let session_id = created.session_id.clone();

    let first_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
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
            run_id: Some("run-before-edit".to_string()),
        })
        .await
        .expect("run before workflow edit");
    assert_eq!(first_run.outputs[0].value, serde_json::json!("alpha"));

    let first_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should exist");

    rewrite_test_workflow_input_description(
        temp.path(),
        "runtime-text",
        "Prompt updated after session creation",
    );

    let second_run = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id: session_id.clone(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-after-edit".to_string()),
        })
        .await
        .expect("run after workflow edit");
    assert_eq!(second_run.outputs[0].value, serde_json::json!("alpha"));

    let second_executor = runtime
        .session_executions
        .handle(&session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive session executor should still exist");
    assert!(Arc::ptr_eq(&first_executor, &second_executor));

    let snapshots = {
        let executor = second_executor.lock().await;
        executor
            .workflow_session_node_memory_snapshots(&session_id)
            .await
    };
    assert_eq!(snapshots.len(), 2);
    assert!(
        snapshots
            .iter()
            .all(|snapshot| snapshot.status == node_engine::NodeMemoryStatus::Ready)
    );
}
