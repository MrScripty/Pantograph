use super::*;

#[tokio::test]
async fn sync_runtime_registry_uses_controller_health_assessment_snapshot() {
    let controller = HealthAwareHostRuntimeController {
        mode_info: HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-10".to_string()),
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
                runtime_instance_id: Some("llama-embed-10".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: Some(16),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        },
        health_assessments: RuntimeHealthAssessmentSnapshot {
            active: None,
            embedding: Some(crate::runtime_health::RuntimeHealthAssessmentRecord {
                runtime_id: "llama.cpp.embedding".to_string(),
                runtime_instance_id: Some("llama-embed-10".to_string()),
                assessment: RuntimeHealthAssessment {
                    healthy: false,
                    state: RuntimeHealthState::Unhealthy {
                        reason: "Connection refused".to_string(),
                    },
                    response_time_ms: None,
                    error: Some("Connection refused".to_string()),
                    consecutive_failures: 3,
                },
            }),
        },
    };
    let registry = RuntimeRegistry::new();

    sync_runtime_registry(&controller, &registry).await;

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Unhealthy);
    assert_eq!(runtime.last_error.as_deref(), Some("Connection refused"));
}

#[test]
fn reconcile_mode_info_marks_active_runtime_unhealthy_from_health_assessment() {
    let registry = RuntimeRegistry::new();

    let snapshots = reconcile_runtime_registry_mode_info_with_active_health_assessment(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-unhealthy".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
        Some(&RuntimeHealthAssessment {
            healthy: false,
            state: RuntimeHealthState::Unhealthy {
                reason: "Request timeout".to_string(),
            },
            response_time_ms: None,
            error: Some("Request timeout".to_string()),
            consecutive_failures: 3,
        }),
    );

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].runtime_id, "llama_cpp");
    assert_eq!(snapshots[0].status, RuntimeRegistryStatus::Unhealthy);
    assert_eq!(snapshots[0].last_error.as_deref(), Some("Request timeout"));
    assert_eq!(
        snapshots[0].runtime_instance_id.as_deref(),
        Some("llama-main-unhealthy")
    );
}

#[test]
fn reconcile_mode_info_keeps_ready_runtime_when_health_is_only_degraded() {
    let registry = RuntimeRegistry::new();

    let snapshots = reconcile_runtime_registry_mode_info_with_active_health_assessment(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-ready".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
        Some(&RuntimeHealthAssessment {
            healthy: true,
            state: RuntimeHealthState::Degraded {
                reason: "HTTP 503".to_string(),
            },
            response_time_ms: Some(42),
            error: Some("HTTP 503".to_string()),
            consecutive_failures: 1,
        }),
    );

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].runtime_id, "llama_cpp");
    assert_eq!(snapshots[0].status, RuntimeRegistryStatus::Ready);
    assert_eq!(snapshots[0].last_error, None);
}

#[test]
fn reconcile_mode_info_marks_embedding_runtime_unhealthy_from_health_assessment() {
    let registry = RuntimeRegistry::new();

    let snapshots = reconcile_runtime_registry_mode_info_with_health_assessments(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-ready".to_string()),
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
                runtime_instance_id: Some("llama-embed-unhealthy".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: Some(16),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        },
        None,
        Some(&RuntimeHealthAssessment {
            healthy: false,
            state: RuntimeHealthState::Unhealthy {
                reason: "Connection refused".to_string(),
            },
            response_time_ms: None,
            error: Some("Connection refused".to_string()),
            consecutive_failures: 3,
        }),
    );

    let embedding = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(embedding.status, RuntimeRegistryStatus::Unhealthy);
    assert_eq!(embedding.last_error.as_deref(), Some("Connection refused"));
    assert_eq!(
        embedding.runtime_instance_id.as_deref(),
        Some("llama-embed-unhealthy")
    );
}

#[tokio::test]
async fn consume_active_runtime_warmup_disposition_marks_runtime_warming_after_mode_sync() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-warm".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: false,
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
    register_active_runtime(&registry, &controller.mode_info_snapshot().await);

    consume_active_runtime_warmup_disposition(
        &controller,
        &registry,
        "llama.cpp",
        Duration::from_millis(1),
        Duration::from_millis(50),
    )
    .await
    .expect("warmup should be marked as started");

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime should remain registered");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Warming);
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-main-warm")
    );
}

#[tokio::test]
async fn consume_active_runtime_warmup_disposition_waits_for_ready_transition() {
    let controller = Arc::new(MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-wait".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_starting".to_string()),
                active: false,
                last_error: None,
            }),
            embedding_runtime: None,
        }),
        stopped_producers: Mutex::new(Vec::new()),
        stop_all_calls: Mutex::new(0),
        restore_calls: Mutex::new(Vec::new()),
        restore_should_fail: Mutex::new(false),
    });
    let registry = RuntimeRegistry::new();
    register_active_runtime(&registry, &controller.mode_info_snapshot().await);
    registry
        .transition_runtime(
            "llama.cpp",
            pantograph_runtime_registry::RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("llama-main-wait".to_string()),
            },
        )
        .expect("runtime should start in warming state");

    let ready_controller = controller.clone();
    let ready_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        let mut mode_info = ready_controller
            .mode_info
            .lock()
            .expect("mode info lock poisoned");
        mode_info.active_runtime = Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-wait".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
    });

    consume_active_runtime_warmup_disposition(
        controller.as_ref(),
        &registry,
        "llama.cpp",
        Duration::from_millis(1),
        Duration::from_millis(100),
    )
    .await
    .expect("warmup wait should observe ready transition");
    ready_task.await.expect("ready task should complete");

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime should remain registered");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-main-wait")
    );
}
