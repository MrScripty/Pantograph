use super::*;

#[tokio::test]
async fn execute_edit_session_graph_reconciles_registry_after_restore() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path.clone()),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let initial_runtime_instance_id = host_runtime_mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("initial runtime instance id");

    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let graph = edit_session_embedding_graph(&model_id);
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution should restore runtime even when node demand fails");
    assert!(outcome.error.is_some());

    let restored_mode_info = gateway.mode_info().await;
    let restored_runtime_instance_id = restored_mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("restored runtime instance id");
    assert_ne!(
        restored_runtime_instance_id, initial_runtime_instance_id,
        "restore path should produce a fresh runtime instance for this regression check"
    );

    let snapshot = runtime_registry.snapshot();
    let registry_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should remain registered after restore");
    assert_eq!(
        registry_runtime.runtime_instance_id.as_deref(),
        Some(restored_runtime_instance_id.as_str())
    );
    assert_eq!(registry_runtime.status, RuntimeRegistryStatus::Ready);
}

#[tokio::test]
async fn execute_edit_session_graph_restore_keeps_scheduler_runtime_registry_diagnostics_ready() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let graph = edit_session_embedding_graph(&model_id);
    let edit_session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &edit_session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution should restore runtime even when node demand fails");
    assert!(outcome.error.is_some());

    let restored_runtime_instance_id = gateway
        .mode_info()
        .await
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("restored runtime instance id");
    let restored_runtime = runtime_registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("restored runtime should remain registered");
    assert_eq!(restored_runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        restored_runtime.runtime_instance_id.as_deref(),
        Some(restored_runtime_instance_id.as_str())
    );

    let loaded = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create loaded session");

    let diagnostics_provider = EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
        gateway.clone(),
        runtime_registry.clone(),
    );
    let diagnostics = diagnostics_provider
        .scheduler_runtime_registry_diagnostics(&WorkflowSchedulerRuntimeDiagnosticsRequest {
            session_id: "queued-session".to_string(),
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
            runtime_loaded: false,
            next_admission_queue_id: Some("queue-after-restore".to_string()),
            reclaim_candidates: vec![WorkflowSessionRuntimeUnloadCandidate {
                session_id: loaded.session_id.clone(),
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                required_backends: Vec::new(),
                required_models: Vec::new(),
                keep_alive: true,
                access_tick: 1,
                run_count: 0,
            }],
        })
        .await
        .expect("scheduler diagnostics provider should succeed")
        .expect("runtime registry diagnostics should be present");

    assert_eq!(
        diagnostics,
        WorkflowSchedulerRuntimeRegistryDiagnostics {
            target_runtime_id: Some("llama_cpp".to_string()),
            reclaim_candidate_session_id: Some(loaded.session_id),
            reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
            next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,),
            next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady),
        }
    );
}

#[tokio::test]
async fn execute_edit_session_graph_reconciles_registry_after_embedding_prepare() {
    let temp = TempDir::new().expect("temp dir");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path.clone()),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let initial_runtime_instance_id = host_runtime_mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone())
        .expect("initial runtime instance id");

    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let started_snapshot = Arc::new(Mutex::new(None::<RuntimeRegistrySnapshot>));
    let started_snapshot_sink = started_snapshot.clone();
    let runtime_registry_for_sink = runtime_registry.clone();
    let event_sink = Arc::new(node_engine::CallbackEventSink::new(move |event| {
        if matches!(event, node_engine::WorkflowEvent::WorkflowStarted { .. }) {
            let mut guard = started_snapshot_sink
                .lock()
                .expect("started snapshot lock poisoned");
            *guard = Some(runtime_registry_for_sink.snapshot());
        }
    }));

    let graph = edit_session_embedding_graph(&model_id);
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            event_sink,
        )
        .await
        .expect("edit-session execution should still finish");
    assert!(outcome.error.is_some());

    let started_snapshot = started_snapshot
        .lock()
        .expect("started snapshot lock poisoned")
        .clone()
        .expect("workflow started snapshot");
    let started_runtime = started_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime snapshot at workflow start");
    assert_eq!(started_runtime.status, RuntimeRegistryStatus::Ready);
    assert_ne!(
        started_runtime.runtime_instance_id.as_deref(),
        Some(initial_runtime_instance_id.as_str()),
        "registry should be refreshed to the prepared embedding runtime before execution starts"
    );
}

#[tokio::test]
async fn execute_edit_session_graph_reconciles_registry_after_failed_restore() {
    let temp = TempDir::new().expect("temp dir");
    let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

    let pumas_api = Arc::new(
        pumas_library::PumasApi::builder(temp.path())
            .build()
            .await
            .expect("build pumas api"),
    );
    pumas_api
        .rebuild_model_index()
        .await
        .expect("rebuild model index");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let inference_model_path = temp.path().join("main.gguf");
    std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
    let mmproj_path = temp.path().join("main.mmproj");
    std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockRestoreFailureBackend {
            ready: false,
            inference_model_path: inference_model_path.clone(),
            embedding_model_path: embedding_model_path.clone(),
            embedding_started: false,
        }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&inference::BackendConfig {
            model_path: Some(inference_model_path.clone()),
            mmproj_path: Some(mmproj_path),
            ..inference::BackendConfig::default()
        })
        .await
        .expect("gateway should start in inference mode");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    extensions
        .write()
        .await
        .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway.clone(),
        extensions,
        Arc::new(WorkflowService::new()),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let graph = edit_session_embedding_graph(&model_id);
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some(embedding_model_path),
                ..inference::EmbeddingStartRequest::default()
            },
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution should still complete when restore fails");
    assert!(outcome.error.is_some());

    let mode_info = gateway.mode_info().await;
    let expected_observation = runtime_registry::active_runtime_observation(
        &HostRuntimeModeSnapshot::from_mode_info(&mode_info),
        true,
    )
    .expect("active runtime observation after failed restore");

    let snapshot = runtime_registry.snapshot();
    let registry_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == expected_observation.runtime_id)
        .expect("active runtime should remain observable after failed restore");
    assert_eq!(registry_runtime.status, expected_observation.status);
    assert_eq!(
        registry_runtime.runtime_instance_id,
        expected_observation.runtime_instance_id
    );
}

#[tokio::test]
async fn execute_edit_session_graph_reports_all_python_runtime_ids_in_trace_metrics() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::from_components(
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
        Arc::new(MockImagePythonRuntime {
            requests: Mutex::new(Vec::new()),
        }),
    );

    let graph = multi_python_edit_session_graph();
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest::default(),
            Arc::new(node_engine::NullEventSink),
        )
        .await
        .expect("edit-session execution");

    assert_eq!(
        outcome.trace_runtime_metrics.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        outcome.trace_runtime_metrics.observed_runtime_ids,
        vec!["onnx-runtime".to_string(), "diffusers".to_string()]
    );
    assert_eq!(
        outcome.trace_runtime_metrics.model_target.as_deref(),
        Some("/tmp/mock-onnx-model")
    );
    assert_eq!(
        outcome.runtime_snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        outcome.runtime_model_target.as_deref(),
        Some("/tmp/mock-onnx-model")
    );
    assert!(!outcome.waiting_for_input);
}

#[tokio::test]
async fn execute_edit_session_graph_waiting_for_input_does_not_emit_workflow_failed() {
    let temp = TempDir::new().expect("temp dir");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime = EmbeddedRuntime::from_components(
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
        Arc::new(ProcessPythonRuntimeAdapter),
    );

    let graph = WorkflowGraph {
        nodes: vec![GraphNode {
            id: "approval".to_string(),
            node_type: "human-input".to_string(),
            data: serde_json::json!({ "prompt": "Approve deployment?" }),
            position: Position::default(),
        }],
        edges: Vec::new(),
        derived_graph: None,
    };
    let session = runtime
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
        })
        .await
        .expect("create edit session");
    let event_sink = Arc::new(node_engine::VecEventSink::new());

    let outcome = runtime
        .execute_edit_session_graph(
            &session.session_id,
            &graph,
            inference::EmbeddingStartRequest::default(),
            event_sink.clone(),
        )
        .await
        .expect("edit-session execution should pause instead of failing");

    assert!(outcome.waiting_for_input);
    assert!(outcome.error.is_none());

    let events = event_sink.events();
    assert!(events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WaitingForInput {
            task_id,
            prompt: Some(prompt),
            ..
        } if task_id == "approval" && prompt == "Approve deployment?"
    )));
    assert!(!events
        .iter()
        .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowFailed { .. })));
    assert!(!events.iter().any(|event| matches!(
        event,
        node_engine::WorkflowEvent::WorkflowCompleted { .. }
            | node_engine::WorkflowEvent::WorkflowCancelled { .. }
    )));
}
