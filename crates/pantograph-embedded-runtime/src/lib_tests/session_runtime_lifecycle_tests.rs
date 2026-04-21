use super::*;

fn mock_runtime_capability() -> WorkflowRuntimeCapability {
    WorkflowRuntimeCapability {
        runtime_id: "mock".to_string(),
        display_name: "Mock runtime".to_string(),
        install_state: WorkflowRuntimeInstallState::Installed,
        available: true,
        configured: true,
        can_install: false,
        can_remove: false,
        source_kind: WorkflowRuntimeSourceKind::Host,
        selected: true,
        readiness_state: Some(pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready),
        selected_version: None,
        supports_external_connection: false,
        backend_keys: vec!["mock".to_string()],
        missing_files: Vec::new(),
        unavailable_reason: None,
    }
}

#[tokio::test]
async fn test_keep_alive_session_load_tracks_registry_reservation_lifecycle() {
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
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    let reserved_snapshot = runtime_registry.snapshot();
    assert_eq!(reserved_snapshot.reservations.len(), 1);
    assert_eq!(
        reserved_snapshot.reservations[0].workflow_id,
        "runtime-text"
    );
    assert_eq!(
        reserved_snapshot.reservations[0].usage_profile.as_deref(),
        Some("interactive")
    );
    assert_eq!(
        reserved_snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
    assert_eq!(
        reserved_snapshot.runtimes[0].active_reservation_ids.len(),
        1
    );
    assert_eq!(
        reserved_snapshot.runtimes[0].status,
        RuntimeRegistryStatus::Warming
    );

    runtime
        .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
            session_id: created.session_id.clone(),
            keep_alive: false,
        })
        .await
        .expect("disable keep alive");

    let released_snapshot = runtime_registry.snapshot();
    assert!(released_snapshot.reservations.is_empty());
    assert!(
        released_snapshot.runtimes[0]
            .active_reservation_ids
            .is_empty()
    );
    assert_eq!(
        released_snapshot.runtimes[0].status,
        RuntimeRegistryStatus::Stopped
    );
}

#[tokio::test]
async fn keep_alive_disable_reclaim_flips_scheduler_runtime_registry_diagnostics_to_start_runtime()
{
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "llama.cpp",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let host_runtime_mode_info =
        HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: Some(1),
        },
        gateway.clone(),
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::with_capacity_limits(4, 1)),
        None,
        Some(runtime_registry.clone()),
        Some(host_runtime_mode_info),
    )
    .await;

    let created = runtime
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
        })
        .await
        .expect("create keep-alive session");

    runtime
        .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
            session_id: created.session_id,
            keep_alive: false,
        })
        .await
        .expect("disable keep alive");

    let snapshot = runtime_registry.snapshot();
    let runtime_record = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime should remain observable after reclaim");
    assert_eq!(runtime_record.status, RuntimeRegistryStatus::Stopped);

    let diagnostics_provider = EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
        gateway.clone(),
        runtime_registry.clone(),
    );
    let diagnostics = diagnostics_provider
        .scheduler_runtime_registry_diagnostics(&WorkflowSchedulerRuntimeDiagnosticsRequest {
            session_id: "queued-after-reclaim".to_string(),
            workflow_id: "runtime-text".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
            runtime_loaded: false,
            next_admission_queue_id: Some("queue-after-reclaim".to_string()),
            reclaim_candidates: Vec::new(),
        })
        .await
        .expect("scheduler diagnostics provider should succeed")
        .expect("runtime registry diagnostics should be present");

    assert_eq!(
        diagnostics,
        WorkflowSchedulerRuntimeRegistryDiagnostics {
            target_runtime_id: Some("llama_cpp".to_string()),
            reclaim_candidate_session_id: None,
            reclaim_candidate_runtime_id: None,
            next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::StartRuntime,),
            next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::NoLoadedInstance),
        }
    );
}

#[tokio::test]
async fn test_sync_loaded_session_runtime_retention_hint_updates_running_session() {
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

    runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    runtime_registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("llama-runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    let lease = runtime_registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "runtime-text".to_string(),
            reservation_owner_id: Some("session-running".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation should be created");
    let host = runtime.host();
    host.record_session_runtime_reservation("session-running", lease.reservation_id)
        .expect("reservation id should be recorded");

    host.sync_loaded_session_runtime_retention_hint(
        "session-running",
        true,
        WorkflowSessionState::Running,
    )
    .expect("running session retention hint should update");

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
}

#[tokio::test]
async fn test_session_runtime_load_reuses_ready_gateway_runtime_in_registry() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "mock",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway,
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone())
    .with_additional_runtime_capabilities(vec![mock_runtime_capability()]);

    runtime
        .host()
        .load_session_runtime(
            "session-ready",
            "runtime-text",
            Some("interactive"),
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect("ready runtime should be reused");

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.runtimes.len(), 1);
    assert_eq!(snapshot.runtimes[0].runtime_id, "mock");
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
    assert!(snapshot.runtimes[0].runtime_instance_id.is_some());
}

#[tokio::test]
async fn test_session_runtime_load_waits_for_existing_warmup_transition() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    runtime_registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("llama-1".to_string()),
            },
        )
        .expect("runtime should enter warming");

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

    let ready_registry = runtime_registry.clone();
    let ready_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        ready_registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("llama-1".to_string()),
                },
            )
            .expect("runtime should become ready");
    });

    runtime
        .host()
        .load_session_runtime(
            "session-wait",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect("load should wait for warmup completion");
    ready_task.await.expect("ready transition task");

    let snapshot = runtime_registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        snapshot.runtimes[0].runtime_instance_id.as_deref(),
        Some("llama-1")
    );
}

#[tokio::test]
async fn test_session_runtime_load_blocks_when_runtime_preflight_reports_not_ready() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");

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

    let error = runtime
        .host()
        .load_session_runtime(
            "session-not-ready",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect_err("load should fail when required runtime is not ready");

    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
    assert!(
        error.to_string().contains("llama.cpp"),
        "expected readiness error to mention llama.cpp, got: {error}"
    );

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
}

#[tokio::test]
async fn test_session_runtime_unload_stops_active_gateway_runtime_when_evictable() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(MockReadyBackend { ready: false }),
        "mock",
    ));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    let runtime = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig {
            app_data_dir,
            project_root: temp.path().to_path_buf(),
            workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
            max_loaded_sessions: None,
        },
        gateway,
        Arc::new(RwLock::new(ExecutorExtensions::new())),
        Arc::new(WorkflowService::new()),
        None,
    )
    .with_runtime_registry(runtime_registry.clone())
    .with_additional_runtime_capabilities(vec![mock_runtime_capability()]);

    runtime
        .host()
        .load_session_runtime(
            "session-stop",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect("ready runtime should load");
    runtime
        .host()
        .unload_session_runtime(
            "session-stop",
            "runtime-text",
            pantograph_workflow_service::WorkflowSessionUnloadReason::SessionClosed,
        )
        .await
        .expect("runtime should unload");

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
    assert!(!runtime.gateway().is_ready().await);
}

#[tokio::test]
async fn test_session_runtime_load_releases_reservation_after_warmup_timeout() {
    let temp = TempDir::new().expect("temp dir");
    write_test_workflow(temp.path(), "runtime-text");

    let app_data_dir = temp.path().join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("app data dir");
    install_fake_default_runtime(&app_data_dir);

    let runtime_registry = Arc::new(RuntimeRegistry::new());
    runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    runtime_registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("llama-timeout".to_string()),
            },
        )
        .expect("runtime should enter warming");

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

    let error = runtime
        .host()
        .load_session_runtime(
            "session-timeout",
            "runtime-text",
            None,
            WorkflowSessionRetentionHint::KeepAlive,
        )
        .await
        .expect_err("warming timeout should fail");
    assert!(matches!(error, WorkflowServiceError::RuntimeTimeout(_)));

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert!(
        snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty())
    );
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
}

#[tokio::test]
async fn test_session_run_without_keep_alive_releases_runtime_reservation_after_run() {
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
        .create_workflow_session(WorkflowSessionCreateRequest {
            workflow_id: "runtime-text".to_string(),
            usage_profile: None,
            keep_alive: false,
        })
        .await
        .expect("create session");
    let session_id = created.session_id.clone();

    let run_response = runtime
        .run_workflow_session(WorkflowSessionRunRequest {
            session_id,
            inputs: vec![WorkflowPortBinding {
                node_id: "text-input-1".to_string(),
                port_id: "text".to_string(),
                value: serde_json::json!("session-world"),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "text-output-1".to_string(),
                port_id: "text".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
            run_id: Some("run-queued".to_string()),
        })
        .await
        .expect("run queued session");
    assert_eq!(run_response.outputs.len(), 1);
    assert_eq!(
        run_response.outputs[0].value,
        serde_json::json!("session-world")
    );

    let snapshot = runtime_registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert!(
        snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty())
    );
    assert!(
        runtime
            .session_executions
            .handle(&created.session_id)
            .expect("session execution lookup should succeed")
            .is_none()
    );
}
