use super::*;

#[tokio::test]
async fn keep_alive_session_retains_checkpoint_across_capacity_rebalance() {
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
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
    );

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

    let first_executor = runtime
        .session_executions
        .handle(&first.session_id)
        .expect("first session execution lookup should succeed")
        .expect("first keep-alive executor should exist");
    {
        let executor = first_executor.lock().await;
        executor
            .record_workflow_execution_session_node_memory(synthetic_kv_node_memory_snapshot(
                &first.session_id,
                "kv-memory",
                "cache-session-1",
            ))
            .await;
    }

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &first.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("checkpoint keep-alive session for capacity rebalance");

    let checkpointed_summary = {
        let executor = first_executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&first.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);
    assert_eq!(
        checkpointed_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );
    assert!(
        checkpointed_summary.preserved_node_count >= 2,
        "checkpoint should preserve node memory for the keep-alive session"
    );
    let checkpointed_snapshots = {
        let executor = first_executor.lock().await;
        executor
            .workflow_execution_session_node_memory_snapshots(&first.session_id)
            .await
    };
    assert!(
        checkpointed_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-1")
        }),
        "checkpoint should preserve the synthetic KV node-memory reference"
    );

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: first.session_id.clone(),
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
        .expect("resume first keep-alive session from checkpoint");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_executor = runtime
        .session_executions
        .handle(&first.session_id)
        .expect("resumed session execution lookup should succeed")
        .expect("resumed keep-alive executor should exist");
    assert!(Arc::ptr_eq(&first_executor, &resumed_executor));

    let resumed_summary = {
        let executor = resumed_executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&first.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::Warm
    );
    let resumed_snapshots = {
        let executor = resumed_executor.lock().await;
        executor
            .workflow_execution_session_node_memory_snapshots(&first.session_id)
            .await
    };
    assert!(
        resumed_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-1")
        }),
        "restored keep-alive session should retain its KV node-memory reference"
    );

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: first.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn scheduler_driven_rebalance_checkpoints_keep_alive_session() {
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
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

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

    let keep_alive_executor = runtime
        .session_executions
        .handle(&keep_alive.session_id)
        .expect("keep-alive session lookup should succeed")
        .expect("keep-alive executor should exist");

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

    let checkpointed_summary = {
        let executor = keep_alive_executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&keep_alive.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);
    assert_eq!(
        checkpointed_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );
    assert!(
        checkpointed_summary.preserved_node_count >= 2,
        "scheduler-driven rebalance should preserve node memory for keep-alive sessions"
    );

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: keep_alive.session_id.clone(),
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
        .expect("resume keep-alive session after scheduler rebalance");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_executor = runtime
        .session_executions
        .handle(&keep_alive.session_id)
        .expect("resumed keep-alive session lookup should succeed")
        .expect("resumed keep-alive executor should exist");
    assert!(Arc::ptr_eq(&keep_alive_executor, &resumed_executor));

    let resumed_summary = {
        let executor = resumed_executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&keep_alive.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::Warm
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
async fn repeated_capacity_unload_keeps_checkpoint_identity_and_keep_alive_disable_clears_it() {
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
            max_loaded_sessions: Some(1),
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    );

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

    let executor = runtime
        .session_executions
        .handle(&session.session_id)
        .expect("session execution lookup should succeed")
        .expect("keep-alive executor should exist");

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("first capacity unload");
    let first_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };

    tokio::time::sleep(std::time::Duration::from_millis(2)).await;

    WorkflowHost::unload_session_runtime(
        &runtime.host(),
        &session.session_id,
        "runtime-text",
        pantograph_workflow_service::WorkflowExecutionSessionUnloadReason::CapacityRebalance,
    )
    .await
    .expect("second capacity unload should be idempotent");
    let second_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };

    assert!(first_summary.checkpoint_available);
    assert_eq!(
        first_summary.checkpointed_at_ms,
        second_summary.checkpointed_at_ms
    );
    assert_eq!(
        second_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );

    runtime
        .workflow_set_execution_session_keep_alive(WorkflowExecutionSessionKeepAliveRequest {
            session_id: session.session_id.clone(),
            keep_alive: false,
        })
        .await
        .expect("disable keep-alive after checkpoint");

    assert!(
        runtime
            .session_executions
            .handle(&session.session_id)
            .expect("session execution lookup should succeed")
            .is_none()
    );
}
