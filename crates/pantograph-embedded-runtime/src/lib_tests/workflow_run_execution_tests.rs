use super::*;

async fn run_workflow_through_scheduler(
    runtime: &EmbeddedRuntime,
    workflow_id: &str,
    inputs: Vec<WorkflowPortBinding>,
    output_targets: Option<Vec<WorkflowOutputTarget>>,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    run_workflow_through_scheduler_with_override(runtime, workflow_id, inputs, output_targets, None)
        .await
}

async fn run_workflow_through_scheduler_with_override(
    runtime: &EmbeddedRuntime,
    workflow_id: &str,
    inputs: Vec<WorkflowPortBinding>,
    output_targets: Option<Vec<WorkflowOutputTarget>>,
    override_selection: Option<WorkflowTechnicalFitOverride>,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: workflow_id.to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await?;

    runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id,
            workflow_semantic_version: "0.1.0".to_string(),
            inputs,
            output_targets,
            override_selection,
            timeout_ms: None,
            priority: None,
        })
        .await
}

#[tokio::test]
async fn test_runtime_run_and_session_execution() {
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
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let run_response = run_workflow_through_scheduler(
        &runtime,
        "runtime-text",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("hello"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
    )
    .await
    .expect("workflow run through scheduler");
    assert_eq!(run_response.outputs.len(), 1);
    assert_eq!(run_response.outputs[0].value, serde_json::json!("hello"));

    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");

    let session_response = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("world"),
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
        .expect("run session");
    assert_eq!(session_response.outputs.len(), 1);
    assert_eq!(
        session_response.outputs[0].value,
        serde_json::json!("world")
    );

    runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: created.session_id,
        })
        .await
        .expect("close session");
}

#[tokio::test]
async fn scheduler_run_retains_node_io_status_and_terminal_output_projection() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let workflow_service = workflow_service_with_artifact_store_and_ledger(&temp);
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service.clone(),
        None,
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let response = run_workflow_through_scheduler(
        &runtime,
        "runtime-text",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("retained vertical text"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
    )
    .await
    .expect("workflow run through scheduler");
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(
        response.outputs[0].value,
        serde_json::json!("retained vertical text")
    );

    let detail = workflow_service
        .workflow_run_detail_query(WorkflowRunDetailQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
            projection_batch_size: Some(50),
        })
        .expect("run detail query");
    let run = detail.run.expect("run detail");
    assert_eq!(
        run.status,
        pantograph_workflow_service::RunListProjectionStatus::Completed
    );
    let output_status = detail
        .node_statuses
        .iter()
        .find(|status| status.node_id == "text-output-1")
        .expect("text output node status");
    assert_eq!(
        output_status.status,
        pantograph_diagnostics_ledger::NodeExecutionProjectionStatus::Completed
    );

    let artifacts = workflow_service
        .workflow_io_artifact_query(WorkflowIoArtifactQueryRequest {
            workflow_run_id: Some(response.workflow_run_id.clone()),
            node_id: None,
            producer_node_id: None,
            consumer_node_id: None,
            artifact_role: None,
            media_type: None,
            retention_state: None,
            retention_policy_id: None,
            runtime_id: None,
            selected_backend_key: None,
            model_id: None,
            after_event_seq: None,
            limit: Some(50),
            projection_batch_size: Some(50),
        })
        .expect("io artifact query")
        .artifacts;

    let text_output_input = artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_role == "node_input"
                && artifact.consumer_node_id.as_deref() == Some("text-output-1")
                && artifact.consumer_port_id.as_deref() == Some("text")
        })
        .expect("retained text output node input");
    assert_eq!(
        text_output_input.retention_state,
        pantograph_workflow_service::IoArtifactRetentionState::Retained
    );
    assert_eq!(
        workflow_service
            .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                artifact_id: text_output_input.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained text output input")
            .body,
        b"retained vertical text"
    );

    let text_output_output = artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_role == "node_output"
                && artifact.producer_node_id.as_deref() == Some("text-output-1")
                && artifact.producer_port_id.as_deref() == Some("text")
        })
        .unwrap_or_else(|| {
            panic!(
                "retained text output node output; artifacts: {}",
                serde_json::to_string_pretty(&artifacts).expect("serialize artifacts")
            )
        });
    assert_eq!(
        workflow_service
            .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                artifact_id: text_output_output.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained text output output")
            .body,
        b"retained vertical text"
    );

    let workflow_output = artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_role == "workflow_output"
                && artifact.producer_node_id.as_deref() == Some("text-output-1")
                && artifact.producer_port_id.as_deref() == Some("text")
        })
        .expect("retained terminal workflow output");
    assert_eq!(
        workflow_service
            .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                artifact_id: workflow_output.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("read retained workflow output")
            .body,
        b"retained vertical text"
    );
}

#[tokio::test]
async fn scheduler_session_live_events_use_backend_workflow_run_id() {
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
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));
    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");
    let session_id = created.session_id.clone();
    let event_sink = Arc::new(node_engine::VecEventSink::new());

    let response = runtime
        .run_workflow_execution_session_with_event_sink(
            WorkflowExecutionSessionRunRequest {
                session_id: session_id.clone(),
                workflow_semantic_version: "0.1.0".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                priority: None,
            },
            event_sink.clone(),
        )
        .await
        .expect("run session");

    let events = event_sink.events();
    assert!(events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::TaskCompleted { task_id, execution_id, .. }
            if task_id == "text-output-1" && execution_id == &response.workflow_run_id
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::TaskCompleted { execution_id, .. }
            if execution_id == &session_id
    )));
}

#[tokio::test]
async fn embedded_workflow_host_run_workflow_returns_cancelled_for_precancelled_run_handle() {
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
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let run_handle = pantograph_workflow_service::WorkflowRunHandle::new();
    run_handle.cancel();

    let error = runtime
        .host()
        .run_workflow(
            "runtime-text",
            &[WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("hello"),
            }],
            Some(&[WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            WorkflowRunOptions {
                timeout_ms: None,
                workflow_execution_session_id: None,
                workflow_run_id: Some("pre-cancelled-run".to_string()),
            },
            run_handle,
        )
        .await
        .expect_err("pre-cancelled host run should return cancelled");

    match error {
        WorkflowServiceError::Cancelled(message) => {
            assert!(
                message.contains("cancelled before execution started"),
                "unexpected cancelled message: {message}"
            );
        }
        other => panic!("expected cancelled error, got {other:?}"),
    }
}

#[tokio::test]
async fn workflow_run_execution_session_returns_invalid_request_for_human_input_workflow() {
    let temp = TempDir::new().expect("temp dir");
    write_human_input_workflow(temp.path(), "interactive-human-input");

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
    )
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let created = runtime
        .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
            workflow_id: "interactive-human-input".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
        })
        .await
        .expect("create interactive session");

    let error = runtime
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id,
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "human-input-1".to_string(),
                port_id: "value".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        })
        .await
        .expect_err(
            "interactive workflow execution session run should fail for non-streaming callers",
        );

    match error {
        WorkflowServiceError::InvalidRequest(message) => {
            assert!(
                message.contains("interactive") || message.contains("input"),
                "unexpected invalid-request message: {message}"
            );
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_runtime_routes_onnx_audio_workflow_through_python_adapter() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_onnx_audio_workflow(temp.path(), "runtime-onnx-audio");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let python_runtime = Arc::new(MockMediaPythonRuntime {
        requests: Mutex::new(Vec::new()),
    });
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service_with_artifact_store(&temp),
        None,
        python_runtime.clone(),
    )
    .with_additional_runtime_capabilities(vec![onnx_python_sidecar_capability()])
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let response = run_workflow_through_scheduler_with_override(
        &runtime,
        "runtime-onnx-audio",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("a tiny painted robot"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "audio-output-1".to_string(),
            port_id: "audio".to_string(),
        }]),
        Some(WorkflowTechnicalFitOverride {
            model_id: None,
            backend_key: Some("onnx-runtime".to_string()),
        }),
    )
    .await
    .expect("workflow run through scheduler");

    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].node_id, "audio-output-1");
    assert_eq!(response.outputs[0].port_id, "audio");
    assert_eq!(
        response.outputs[0].value["artifact_role"],
        serde_json::json!("workflow_output")
    );
    assert_eq!(
        response.outputs[0].value["payload_kind"],
        serde_json::json!("audio")
    );
    assert_eq!(
        response.outputs[0].value["attribution"]["workflow_id"],
        serde_json::json!("runtime-onnx-audio")
    );

    let requests = python_runtime.requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].node_type, "onnx-inference");
    assert_eq!(
        requests[0].inputs.get("prompt"),
        Some(&serde_json::json!("a tiny painted robot"))
    );
}

#[tokio::test]
async fn workflow_run_execution_session_uses_graph_node_type_for_gui_style_input_ids() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_onnx_audio_workflow_with_prompt_node(
        temp.path(),
        "runtime-onnx-audio",
        "prompt-input",
    );

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let python_runtime = Arc::new(MockMediaPythonRuntime {
        requests: Mutex::new(Vec::new()),
    });
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service_with_artifact_store(&temp),
        None,
        python_runtime.clone(),
    )
    .with_additional_runtime_capabilities(vec![onnx_python_sidecar_capability()])
    .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

    let response = run_workflow_through_scheduler_with_override(
        &runtime,
        "runtime-onnx-audio",
        vec![WorkflowPortBinding {
            node_id: "prompt-input".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("a GUI style prompt node"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "audio-output-1".to_string(),
            port_id: "audio".to_string(),
        }]),
        Some(WorkflowTechnicalFitOverride {
            model_id: None,
            backend_key: Some("onnx-runtime".to_string()),
        }),
    )
    .await
    .expect("workflow run through scheduler");

    assert_eq!(response.outputs.len(), 1);
    let requests = python_runtime.requests.lock().expect("requests lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].inputs.get("prompt"),
        Some(&serde_json::json!("a GUI style prompt node"))
    );
}

#[tokio::test]
async fn test_runtime_run_reconciles_python_sidecar_runtime_into_registry() {
    let temp = TempDir::new().expect("temp dir");
    write_mock_onnx_audio_workflow(temp.path(), "runtime-onnx-audio");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::from_components(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        Arc::new(inference::InferenceGateway::new()),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        workflow_service_with_artifact_store(&temp),
        None,
        Arc::new(MockMediaPythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    )
    .with_additional_runtime_capabilities(vec![onnx_python_sidecar_capability()])
    .with_runtime_registry(runtime_registry.clone());

    run_workflow_through_scheduler_with_override(
        &runtime,
        "runtime-onnx-audio",
        vec![WorkflowPortBinding {
            node_id: "text-input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("a tiny painted robot"),
        }],
        Some(vec![WorkflowOutputTarget {
            node_id: "audio-output-1".to_string(),
            port_id: "audio".to_string(),
        }]),
        Some(WorkflowTechnicalFitOverride {
            model_id: None,
            backend_key: Some("onnx-runtime".to_string()),
        }),
    )
    .await
    .expect("workflow run through scheduler");

    let snapshot = runtime_registry.snapshot();
    let onnx = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "onnx-runtime")
        .expect("python runtime should be observed");
    assert_eq!(onnx.display_name, "ONNX Runtime (Python sidecar)");
    assert_eq!(onnx.status, RuntimeRegistryStatus::Stopped);
    assert!(onnx.runtime_instance_id.is_none());
    assert!(onnx.models.is_empty());
}
