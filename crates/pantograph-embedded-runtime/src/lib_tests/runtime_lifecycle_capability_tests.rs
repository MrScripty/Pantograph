use super::*;

#[tokio::test]
async fn hosted_runtime_constructor_syncs_registry_and_derives_capabilities_from_mode_info() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let mode_info = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        mode: "sidecar_inference".to_string(),
        ready: true,
        url: Some("http://127.0.0.1:11434".to_string()),
        model_path: None,
        is_embedding_mode: false,
        active_model_target: Some("/models/qwen.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-2".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-8".to_string()),
            warmup_started_at_ms: Some(11),
            warmup_completed_at_ms: Some(19),
            warmup_duration_ms: Some(8),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
    });
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
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
        Some(runtime_registry.clone()),
        Some(mode_info),
    )
    .await;

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.runtimes.len(), 2);
    let active = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime");
    assert_eq!(active.status, RuntimeRegistryStatus::Ready);
    let embedding = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime");
    assert_eq!(embedding.status, RuntimeRegistryStatus::Ready);

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");
    assert!(capabilities
        .runtime_capabilities
        .iter()
        .any(|capability| capability.runtime_id == "llama.cpp.embedding"));
}

#[tokio::test]
async fn embedded_runtime_shutdown_reconciles_registry_to_stopped() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let mode_info = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        mode: "sidecar_inference".to_string(),
        ready: true,
        url: Some("http://127.0.0.1:11434".to_string()),
        model_path: None,
        is_embedding_mode: false,
        active_model_target: Some("/models/qwen.gguf".to_string()),
        embedding_model_target: None,
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-9".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: None,
    });
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
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
        Some(runtime_registry.clone()),
        Some(mode_info),
    )
    .await;

    let ready_snapshot = runtime_registry.snapshot();
    let ready_runtime = ready_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should be registered before shutdown");
    assert_eq!(ready_runtime.status, RuntimeRegistryStatus::Ready);

    runtime.shutdown().await;

    let stopped_snapshot = runtime_registry.snapshot();
    let stopped_runtime = stopped_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should remain observable after shutdown");
    assert_eq!(stopped_runtime.status, RuntimeRegistryStatus::Stopped);
}

#[tokio::test]
async fn embedded_runtime_shutdown_marks_loaded_sessions_unloaded() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
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
        Some(runtime_registry),
        None,
    )
    .await;

    let session = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create workflow session");

    let status = runtime
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("session status before shutdown");
    assert_eq!(status.session.state, WorkflowSessionState::IdleLoaded);

    runtime.shutdown().await;

    let status = runtime
        .workflow_get_session_status(WorkflowSessionStatusRequest {
            session_id: session.session_id,
        })
        .await
        .expect("session status after shutdown");
    assert_eq!(status.session.state, WorkflowSessionState::IdleUnloaded);
}

#[tokio::test]
async fn workflow_capabilities_include_injected_runtime_capabilities() {
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
    .with_additional_runtime_capabilities(vec![WorkflowRuntimeCapability {
        runtime_id: "llama.cpp.embedding".to_string(),
        display_name: "Dedicated embedding runtime".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: false,
        source_kind: WorkflowRuntimeSourceKind::Host,
        selected: false,
        readiness_state: Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready),
        selected_version: None,
        supports_external_connection: false,
        backend_keys: vec!["llama_cpp".to_string(), "llamacpp".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }]);

    let capabilities = runtime
        .workflow_get_capabilities(WorkflowCapabilitiesRequest {
            workflow_id: "runtime-text".to_string(),
        })
        .await
        .expect("workflow capabilities");

    let embedding_runtime = capabilities
        .runtime_capabilities
        .iter()
        .find(|capability| capability.runtime_id == "llama.cpp.embedding")
        .expect("dedicated embedding capability");
    assert_eq!(
        embedding_runtime.source_kind,
        WorkflowRuntimeSourceKind::Host
    );
    assert!(!embedding_runtime.selected);
    assert!(embedding_runtime.available);
}
