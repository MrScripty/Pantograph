use super::*;

#[test]
fn reconcile_mode_info_registers_active_and_embedding_runtimes() {
    let registry = RuntimeRegistry::new();

    let snapshots = reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-1".to_string()),
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
                runtime_instance_id: Some("llama-embed-1".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        },
    );

    assert_eq!(snapshots.len(), 2);
    let active_runtime = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama_cpp")
        .expect("active runtime snapshot");
    assert_eq!(active_runtime.status, RuntimeRegistryStatus::Ready);
    assert_eq!(active_runtime.models[0].model_id, "/models/qwen.gguf");

    let embedding_runtime = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(embedding_runtime.status, RuntimeRegistryStatus::Warming);
    assert_eq!(embedding_runtime.models[0].model_id, "/models/embed.gguf");
}

#[test]
fn reconcile_mode_info_stops_unobserved_runtimes_without_reservations() {
    let registry = RuntimeRegistry::new();

    reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-1".to_string()),
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
    );

    let snapshots = reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("ollama".to_string()),
            backend_key: Some("ollama".to_string()),
            active_model_target: Some("llava:13b".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("ollama".to_string()),
                runtime_instance_id: Some("ollama-1".to_string()),
                warmup_started_at_ms: Some(30),
                warmup_completed_at_ms: Some(35),
                warmup_duration_ms: Some(5),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    assert_eq!(snapshots.len(), 2);
    let llama = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "llama_cpp")
        .expect("llama snapshot");
    assert_eq!(llama.status, RuntimeRegistryStatus::Stopped);
    assert!(llama.models.is_empty());

    let ollama = snapshots
        .iter()
        .find(|snapshot| snapshot.runtime_id == "ollama")
        .expect("ollama snapshot");
    assert_eq!(ollama.status, RuntimeRegistryStatus::Ready);
    assert_eq!(ollama.models[0].model_id, "llava:13b");
}

#[test]
fn reconcile_snapshot_override_adds_python_runtime_without_stopping_gateway_runtime() {
    let registry = RuntimeRegistry::new();

    reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-1".to_string()),
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
    );

    let pytorch = reconcile_runtime_registry_snapshot_override(
        &registry,
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        },
        Some("/models/demo"),
    )
    .expect("python snapshot should be reconciled");

    assert_eq!(pytorch.runtime_id, "pytorch");
    assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
    assert_eq!(pytorch.status, RuntimeRegistryStatus::Ready);
    assert_eq!(pytorch.models[0].model_id, "/models/demo");

    let snapshot = registry.snapshot();
    let llama = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("gateway runtime should remain in registry");
    assert_eq!(llama.status, RuntimeRegistryStatus::Ready);

    let pytorch = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("python runtime should be present in registry");
    assert!(pytorch.backend_keys.contains(&"pytorch".to_string()));
}

#[test]
fn reconcile_snapshot_override_preserves_matching_unhealthy_runtime() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtime(RuntimeObservation {
        runtime_id: "pytorch".to_string(),
        display_name: "PyTorch (Python sidecar)".to_string(),
        backend_keys: vec!["pytorch".to_string()],
        model_id: Some("/models/failed.safetensors".to_string()),
        status: RuntimeRegistryStatus::Unhealthy,
        runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
        last_error: Some("probe timeout".to_string()),
    });

    let pytorch = reconcile_runtime_registry_snapshot_override(
        &registry,
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        },
        Some("/models/retry.safetensors"),
    )
    .expect("python snapshot should be reconciled");

    assert_eq!(pytorch.status, RuntimeRegistryStatus::Unhealthy);
    assert_eq!(pytorch.last_error.as_deref(), Some("probe timeout"));
    assert_eq!(pytorch.models[0].model_id, "/models/retry.safetensors");
}

#[test]
fn reconcile_snapshot_override_marks_runtime_unhealthy_from_assessment() {
    let registry = RuntimeRegistry::new();

    let pytorch = reconcile_runtime_registry_snapshot_override_with_health_assessment(
        &registry,
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: false,
            last_error: Some("python sidecar crashed".to_string()),
        },
        Some("/models/retry.safetensors"),
        Some(&RuntimeHealthAssessment {
            healthy: false,
            state: RuntimeHealthState::Unhealthy {
                reason: "python sidecar crashed".to_string(),
            },
            response_time_ms: None,
            error: Some("python sidecar crashed".to_string()),
            consecutive_failures: 1,
        }),
    )
    .expect("python snapshot should be reconciled");

    assert_eq!(pytorch.status, RuntimeRegistryStatus::Unhealthy);
    assert_eq!(
        pytorch.last_error.as_deref(),
        Some("python sidecar crashed")
    );
    assert_eq!(pytorch.models[0].model_id, "/models/retry.safetensors");
}

#[test]
fn reconcile_snapshot_override_keeps_failed_status_when_health_is_only_degraded() {
    let registry = RuntimeRegistry::new();

    let pytorch = reconcile_runtime_registry_snapshot_override_with_health_assessment(
        &registry,
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: false,
            last_error: Some("python sidecar crashed".to_string()),
        },
        Some("/models/retry.safetensors"),
        Some(&RuntimeHealthAssessment {
            healthy: true,
            state: RuntimeHealthState::Degraded {
                reason: "python sidecar crashed".to_string(),
            },
            response_time_ms: None,
            error: Some("python sidecar crashed".to_string()),
            consecutive_failures: 1,
        }),
    )
    .expect("python snapshot should be reconciled");

    assert_eq!(pytorch.status, RuntimeRegistryStatus::Failed);
    assert_eq!(
        pytorch.last_error.as_deref(),
        Some("python sidecar crashed")
    );
    assert!(pytorch.models.is_empty());
}

#[test]
fn reconcile_stored_projection_overrides_replays_non_live_runtime_snapshot() {
    let registry = RuntimeRegistry::new();

    reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    reconcile_runtime_registry_stored_projection_overrides(
        &registry,
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:restored".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(12),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        None,
        Some("/models/restored.safetensors"),
        None,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    let snapshot = registry.snapshot();
    let gateway_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("gateway runtime should remain present");
    assert_eq!(
        gateway_runtime.runtime_instance_id.as_deref(),
        Some("llama-main-live")
    );

    let restored_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("stored sidecar runtime should be replayed");
    assert_eq!(
        restored_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:restored")
    );
    assert_eq!(
        restored_runtime.models[0].model_id,
        "/models/restored.safetensors"
    );
}

#[test]
fn reconcile_stored_projection_overrides_skips_live_host_runtime_ids() {
    let registry = RuntimeRegistry::new();

    reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    reconcile_runtime_registry_stored_projection_overrides(
        &registry,
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-stale".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(12),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        None,
        Some("/models/stale.gguf"),
        None,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    let snapshot = registry.snapshot();
    let gateway_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("gateway runtime should remain present");
    assert_eq!(
        gateway_runtime.runtime_instance_id.as_deref(),
        Some("llama-main-live")
    );
    assert_eq!(gateway_runtime.models[0].model_id, "/models/main.gguf");
}

#[test]
fn register_active_runtime_registers_descriptor_backend_keys() {
    let registry = RuntimeRegistry::new();
    let descriptor = register_active_runtime(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-registered".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    assert_eq!(descriptor.runtime_id, "llama.cpp");
    assert_eq!(descriptor.display_name, "llama.cpp");
    assert_eq!(descriptor.backend_keys, vec!["llama_cpp".to_string()]);

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| {
            runtime.runtime_id
                == pantograph_runtime_identity::canonical_runtime_id(&descriptor.runtime_id)
        })
        .expect("active runtime should be registered");
    assert_eq!(runtime.display_name, "llama.cpp");
    assert_eq!(runtime.backend_keys, vec!["llama_cpp".to_string()]);
}

#[test]
fn active_runtime_reservation_request_registers_runtime_and_preserves_model_target() {
    let registry = RuntimeRegistry::new();
    let request = active_runtime_reservation_request(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-request".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
        "wf-1",
        Some("session-1"),
        Some("interactive"),
        None,
        pantograph_runtime_registry::RuntimeRetentionHint::KeepAlive,
    );

    assert_eq!(request.runtime_id, "llama.cpp");
    assert_eq!(request.workflow_id, "wf-1");
    assert_eq!(request.reservation_owner_id.as_deref(), Some("session-1"));
    assert_eq!(request.usage_profile.as_deref(), Some("interactive"));
    assert_eq!(request.model_id.as_deref(), Some("/models/main.gguf"));
    assert_eq!(
        request.retention_hint,
        pantograph_runtime_registry::RuntimeRetentionHint::KeepAlive
    );

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("active runtime should be registered");
    assert_eq!(runtime.display_name, "llama.cpp");
    assert_eq!(runtime.backend_keys, vec!["llama_cpp".to_string()]);
}

#[test]
fn sync_runtime_reservation_retention_hint_updates_existing_reservation() {
    let registry = RuntimeRegistry::new();
    let lease = registry
        .acquire_reservation(active_runtime_reservation_request(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/main.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-hint".to_string()),
                    warmup_started_at_ms: Some(1),
                    warmup_completed_at_ms: Some(2),
                    warmup_duration_ms: Some(1),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
            "wf-1",
            Some("session-1"),
            Some("interactive"),
            None,
            RuntimeRetentionHint::Ephemeral,
        ))
        .expect("reservation should be created");

    sync_runtime_reservation_retention_hint(
        &registry,
        lease.reservation_id,
        RuntimeRetentionHint::KeepAlive,
    )
    .expect("retention hint should update");

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
}

#[test]
fn scheduler_runtime_registry_diagnostics_reports_start_runtime_without_loaded_candidate() {
    let registry = RuntimeRegistry::new();
    let candidate_request = active_runtime_reservation_request(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
            embedding_runtime: None,
        },
        "wf-loaded",
        Some("session-loaded"),
        Some("interactive"),
        None,
        RuntimeRetentionHint::Ephemeral,
    );
    registry
        .acquire_reservation(candidate_request)
        .expect("loaded session reservation should be created");

    let diagnostics = scheduler_runtime_registry_diagnostics(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
            embedding_runtime: None,
        },
        &WorkflowSchedulerRuntimeDiagnosticsRequest {
            session_id: "session-queued".to_string(),
            workflow_id: "wf-queued".to_string(),
            usage_profile: Some("interactive".to_string()),
            keep_alive: false,
            runtime_loaded: false,
            next_admission_queue_id: Some("queue-1".to_string()),
            reclaim_candidates: vec![WorkflowExecutionSessionRuntimeUnloadCandidate {
                session_id: "session-loaded".to_string(),
                workflow_id: "wf-loaded".to_string(),
                keep_alive: false,
                usage_profile: Some("interactive".to_string()),
                required_backends: Vec::new(),
                required_models: Vec::new(),
                access_tick: 1,
                run_count: 1,
            }],
        },
    )
    .expect("scheduler diagnostics should succeed");

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
