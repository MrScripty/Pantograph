use super::*;

#[tokio::test]
async fn failed_restore_keeps_checkpoint_until_resume_succeeds() {
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
    .expect("checkpoint keep-alive session");

    let checkpointed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);

    rewrite_test_workflow_output_node_to_human_input(temp.path(), "runtime-text");

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
            inputs: Vec::new(),
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
        WorkflowServiceError::InvalidRequest(message) => {
            assert!(
                message.contains("text-output-1"),
                "unexpected invalid-request message: {message}"
            );
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }

    let failed_restore_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(failed_restore_summary.checkpoint_available);
    assert_eq!(
        failed_restore_summary.checkpointed_at_ms,
        checkpointed_summary.checkpointed_at_ms
    );
    assert_eq!(
        failed_restore_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );

    write_test_workflow(temp.path(), "runtime-text");

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
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
        .expect("resume should succeed after restoring a runnable graph");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::Warm
    );

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn runtime_not_ready_resume_keeps_checkpoint_until_runtime_returns() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

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

    let one_shot = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("batch".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create one-shot session");

    let one_shot_output = runtime
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
        .expect("run one-shot session to force keep-alive rebalance");
    assert_eq!(one_shot_output.outputs[0].value, serde_json::json!("beta"));

    let checkpointed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(checkpointed_summary.checkpoint_available);

    std::fs::remove_dir_all(app_data_dir.join("runtimes").join("llama-cpp"))
        .expect("remove fake runtime before resume");

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
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
        .expect_err("resume should fail when the selected runtime is no longer ready");
    match error {
        WorkflowServiceError::RuntimeNotReady(message) => {
            assert!(
                message.contains("llama.cpp"),
                "unexpected runtime-not-ready message: {message}"
            );
        }
        other => panic!("expected runtime-not-ready error, got {other:?}"),
    }

    let failed_resume_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(failed_resume_summary.checkpoint_available);
    assert_eq!(
        failed_resume_summary.checkpointed_at_ms,
        checkpointed_summary.checkpointed_at_ms
    );
    assert_eq!(
        failed_resume_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );

    install_fake_default_runtime(&app_data_dir);

    let resumed_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session.session_id.clone(),
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
        .expect("resume should succeed after the runtime becomes ready again");
    assert_eq!(resumed_output.outputs[0].value, serde_json::json!("alpha"));

    let resumed_summary = {
        let executor = executor.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session.session_id)
            .await
    };
    assert!(!resumed_summary.checkpoint_available);
    assert_eq!(
        resumed_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::Warm
    );

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("close resumed keep-alive session");
}

#[tokio::test]
async fn scheduler_reclaim_keeps_checkpointed_sessions_isolated_across_resumes() {
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

    let executor_a = runtime
        .session_executions
        .handle(&session_a.session_id)
        .expect("first session execution lookup should succeed")
        .expect("first keep-alive executor should exist");
    {
        let executor = executor_a.lock().await;
        executor
            .record_workflow_execution_session_node_memory(synthetic_kv_node_memory_snapshot(
                &session_a.session_id,
                "kv-memory-a",
                "cache-session-a",
            ))
            .await;
    }

    let second_output = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_b.session_id.clone(),
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

    let executor_b = runtime
        .session_executions
        .handle(&session_b.session_id)
        .expect("second session execution lookup should succeed")
        .expect("second keep-alive executor should exist");
    {
        let executor = executor_b.lock().await;
        executor
            .record_workflow_execution_session_node_memory(synthetic_kv_node_memory_snapshot(
                &session_b.session_id,
                "kv-memory-b",
                "cache-session-b",
            ))
            .await;
    }
    assert!(
        !Arc::ptr_eq(&executor_a, &executor_b),
        "distinct workflow execution sessions must not share the same executor"
    );

    let first_checkpoint_summary = {
        let executor = executor_a.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session_a.session_id)
            .await
    };
    assert!(first_checkpoint_summary.checkpoint_available);
    assert_eq!(
        first_checkpoint_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );

    let resumed_a = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_a.session_id.clone(),
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
        .expect("resume first session after scheduler reclaim");
    assert_eq!(resumed_a.outputs[0].value, serde_json::json!("alpha"));

    let resumed_a_summary = {
        let executor = executor_a.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session_a.session_id)
            .await
    };
    assert!(!resumed_a_summary.checkpoint_available);
    assert_eq!(
        resumed_a_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::Warm
    );
    let resumed_a_snapshots = {
        let executor = executor_a.lock().await;
        executor
            .workflow_execution_session_node_memory_snapshots(&session_a.session_id)
            .await
    };
    assert!(
        resumed_a_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory-a"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-a")
        }),
        "session A should retain only its own KV node-memory reference after resume"
    );
    assert!(
        resumed_a_snapshots.iter().all(|snapshot| {
            snapshot
                .indirect_state_reference
                .as_ref()
                .map(|reference| reference.reference_id.as_str())
                != Some("cache-session-b")
        }),
        "session A should not observe session B KV references"
    );

    let second_checkpoint_summary = {
        let executor = executor_b.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session_b.session_id)
            .await
    };
    assert!(second_checkpoint_summary.checkpoint_available);
    assert_eq!(
        second_checkpoint_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );

    let resumed_b = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: session_b.session_id.clone(),
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
        .expect("resume second session after reclaiming the first");
    assert_eq!(resumed_b.outputs[0].value, serde_json::json!("beta"));

    let resumed_b_summary = {
        let executor = executor_b.lock().await;
        executor
            .workflow_execution_session_checkpoint_summary(&session_b.session_id)
            .await
    };
    assert!(!resumed_b_summary.checkpoint_available);
    assert_eq!(
        resumed_b_summary.residency,
        node_engine::WorkflowExecutionSessionResidencyState::Warm
    );
    let resumed_b_snapshots = {
        let executor = executor_b.lock().await;
        executor
            .workflow_execution_session_node_memory_snapshots(&session_b.session_id)
            .await
    };
    assert!(
        resumed_b_snapshots.iter().any(|snapshot| {
            snapshot.identity.node_id == "kv-memory-b"
                && snapshot
                    .indirect_state_reference
                    .as_ref()
                    .map(|reference| reference.reference_id.as_str())
                    == Some("cache-session-b")
        }),
        "session B should retain only its own KV node-memory reference after resume"
    );
    assert!(
        resumed_b_snapshots.iter().all(|snapshot| {
            snapshot
                .indirect_state_reference
                .as_ref()
                .map(|reference| reference.reference_id.as_str())
                != Some("cache-session-a")
        }),
        "session B should not observe session A KV references"
    );

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
