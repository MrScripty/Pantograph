use super::*;

#[tokio::test]
async fn runtime_registry_snapshot_syncs_controller_mode_info() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-7".to_string()),
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
                runtime_instance_id: Some("llama-embed-7".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: Some(16),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        }),
        stopped_producers: Mutex::new(Vec::new()),
        stop_all_calls: Mutex::new(0),
        restore_calls: Mutex::new(Vec::new()),
        restore_should_fail: Mutex::new(false),
    };
    let registry = RuntimeRegistry::new();

    let snapshot = runtime_registry_snapshot(&controller, &registry).await;

    assert!(
        snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama_cpp")
    );
    assert!(
        snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama.cpp.embedding")
    );
}

#[tokio::test]
async fn stop_all_runtime_producers_and_reconcile_runtime_registry_syncs_after_stop_all() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
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
            embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-embed-9".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: Some(16),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        }),
        stopped_producers: Mutex::new(Vec::new()),
        stop_all_calls: Mutex::new(0),
        restore_calls: Mutex::new(Vec::new()),
        restore_should_fail: Mutex::new(false),
    };
    let registry = RuntimeRegistry::new();
    reconcile_runtime_registry_mode_info(&registry, &controller.mode_info_snapshot().await);

    stop_all_runtime_producers_and_reconcile_runtime_registry(&controller, &registry).await;

    assert_eq!(
        *controller
            .stop_all_calls
            .lock()
            .expect("stop-all calls lock poisoned"),
        1
    );
    let snapshot = registry.snapshot();
    assert!(
        snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.status == RuntimeRegistryStatus::Stopped)
    );
}

#[tokio::test]
async fn restore_runtime_and_reconcile_runtime_registry_syncs_after_restore() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
            embedding_runtime: None,
        }),
        stopped_producers: Mutex::new(Vec::new()),
        stop_all_calls: Mutex::new(0),
        restore_calls: Mutex::new(Vec::new()),
        restore_should_fail: Mutex::new(false),
    };
    let registry = RuntimeRegistry::new();

    restore_runtime_and_reconcile_runtime_registry(
        &controller,
        &registry,
        Some(inference::BackendConfig::default()),
    )
    .await
    .expect("restore should succeed");

    assert_eq!(
        controller
            .restore_calls
            .lock()
            .expect("restore calls lock poisoned")
            .len(),
        1
    );
    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("restored runtime snapshot");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-main-restored")
    );
}

#[tokio::test]
async fn restore_runtime_and_reconcile_runtime_registry_applies_matching_unhealthy_assessment() {
    let controller = HealthAwareLifecycleController {
        mode_info: HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-restored".to_string()),
                warmup_started_at_ms: Some(30),
                warmup_completed_at_ms: Some(40),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
        health_assessments: RuntimeHealthAssessmentSnapshot {
            active: Some(crate::runtime_health::RuntimeHealthAssessmentRecord {
                runtime_id: "llama.cpp".to_string(),
                runtime_instance_id: Some("llama-main-restored".to_string()),
                assessment: RuntimeHealthAssessment {
                    healthy: false,
                    state: RuntimeHealthState::Unhealthy {
                        reason: "port bind failed".to_string(),
                    },
                    response_time_ms: None,
                    error: Some("port bind failed".to_string()),
                    consecutive_failures: 1,
                },
            }),
            embedding: None,
        },
        restore_calls: Mutex::new(Vec::new()),
    };
    let registry = RuntimeRegistry::new();

    restore_runtime_and_reconcile_runtime_registry(
        &controller,
        &registry,
        Some(inference::BackendConfig::default()),
    )
    .await
    .expect("restore should succeed");

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("restored runtime snapshot");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Unhealthy);
    assert_eq!(runtime.last_error.as_deref(), Some("port bind failed"));
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-main-restored")
    );
}

#[tokio::test]
async fn restore_runtime_and_reconcile_runtime_registry_replaces_old_unhealthy_instance() {
    let controller = HealthAwareLifecycleController {
        mode_info: HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-restored".to_string()),
                warmup_started_at_ms: Some(30),
                warmup_completed_at_ms: Some(40),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
        health_assessments: RuntimeHealthAssessmentSnapshot::default(),
        restore_calls: Mutex::new(Vec::new()),
    };
    let registry = RuntimeRegistry::new();
    registry.observe_runtime(RuntimeObservation {
        runtime_id: "llama_cpp".to_string(),
        display_name: "llama.cpp".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("/models/old.gguf".to_string()),
        status: RuntimeRegistryStatus::Unhealthy,
        runtime_instance_id: Some("llama-main-old".to_string()),
        last_error: Some("old crash".to_string()),
    });

    restore_runtime_and_reconcile_runtime_registry(
        &controller,
        &registry,
        Some(inference::BackendConfig::default()),
    )
    .await
    .expect("restore should succeed");

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("restored runtime snapshot");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(runtime.last_error, None);
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-main-restored")
    );
    assert_eq!(runtime.models[0].model_id, "/models/qwen.gguf");
}

#[tokio::test]
async fn run_runtime_transition_and_reconcile_runtime_registry_syncs_after_success() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
            embedding_runtime: None,
        }),
        stopped_producers: Mutex::new(Vec::new()),
        stop_all_calls: Mutex::new(0),
        restore_calls: Mutex::new(Vec::new()),
        restore_should_fail: Mutex::new(false),
    };
    let registry = RuntimeRegistry::new();

    let result = run_runtime_transition_and_reconcile_runtime_registry(
        &controller,
        &registry,
        |controller| {
            let mut mode_info = controller
                .mode_info
                .lock()
                .expect("mode info lock poisoned");
            mode_info.active_runtime = Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-transition".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            });
            std::future::ready(Ok::<_, &'static str>("transition-complete"))
        },
    )
    .await
    .expect("transition should succeed");

    assert_eq!(result, "transition-complete");
    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime snapshot");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-main-transition")
    );
}

#[tokio::test]
async fn run_runtime_transition_and_reconcile_runtime_registry_syncs_after_failure() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-before-failure".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        }),
        stopped_producers: Mutex::new(Vec::new()),
        stop_all_calls: Mutex::new(0),
        restore_calls: Mutex::new(Vec::new()),
        restore_should_fail: Mutex::new(false),
    };
    let registry = RuntimeRegistry::new();
    reconcile_runtime_registry_mode_info(&registry, &controller.mode_info_snapshot().await);

    let error = run_runtime_transition_and_reconcile_runtime_registry(
        &controller,
        &registry,
        |controller| {
            let mut mode_info = controller
                .mode_info
                .lock()
                .expect("mode info lock poisoned");
            mode_info.active_runtime = Some(inference::RuntimeLifecycleSnapshot::default());
            std::future::ready(Err::<(), _>("transition failed"))
        },
    )
    .await
    .expect_err("transition should fail");

    assert_eq!(error, "transition failed");
    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime snapshot");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Stopped);
}
