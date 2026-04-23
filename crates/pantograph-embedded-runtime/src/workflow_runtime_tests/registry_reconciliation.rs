use super::*;

#[tokio::test]
async fn build_runtime_event_projection_with_registry_sync_reconciles_live_and_stored_runtimes() {
    let registry = RuntimeRegistry::new();
    let gateway_mode_info = HostRuntimeModeSnapshot {
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
    };

    let projection = build_runtime_event_projection_with_registry_sync(
        &MockRuntimeRegistryController {
            mode_info: gateway_mode_info.clone(),
            active_runtime_snapshot: gateway_mode_info
                .active_runtime
                .clone()
                .expect("gateway runtime snapshot"),
            embedding_runtime_snapshot: None,
        },
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
        None,
        None,
        gateway_mode_info
            .active_runtime
            .as_ref()
            .expect("gateway runtime snapshot"),
        None,
        &gateway_mode_info,
        None,
    )
    .await;

    assert_eq!(
        projection.active_runtime_snapshot.runtime_id.as_deref(),
        Some("PyTorch")
    );
    let snapshot = registry.snapshot();
    assert!(snapshot
        .runtimes
        .iter()
        .any(|runtime| runtime.runtime_id == "llama_cpp"));
    let stored_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("stored python runtime should be replayed");
    assert_eq!(
        stored_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:restored")
    );
}

#[test]
fn build_runtime_event_projection_with_registry_override_reconciles_execution_runtime() {
    let registry = RuntimeRegistry::new();
    let projection = build_runtime_event_projection_with_registry_override(
        Some(&registry),
        None,
        None,
        None,
        None,
        None,
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        &inference::RuntimeLifecycleSnapshot::default(),
        None,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        },
        Some("/models/sidecar.safetensors"),
    );

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("execution runtime should be reconciled into the registry");

    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/sidecar.safetensors")
    );
    assert_eq!(runtime.models.len(), 1);
    assert_eq!(runtime.models[0].model_id, "/models/sidecar.safetensors");
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:default")
    );
}

#[test]
fn build_runtime_event_projection_with_registry_reconciliation_replays_stored_runtime_into_registry(
) {
    let registry = RuntimeRegistry::new();
    crate::runtime_registry::reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
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

    let projection = build_runtime_event_projection_with_registry_reconciliation(
        Some(&registry),
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
        None,
        None,
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-live".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        },
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
        None,
    );

    assert_eq!(
        projection.active_runtime_snapshot.runtime_id.as_deref(),
        Some("PyTorch")
    );
    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/restored.safetensors")
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
    assert_eq!(restored_runtime.models.len(), 1);
    assert_eq!(
        restored_runtime.models[0].model_id,
        "/models/restored.safetensors"
    );
}

#[test]
fn reconcile_runtime_registry_stored_projection_overrides_replays_non_live_runtime_snapshot() {
    let registry = RuntimeRegistry::new();
    crate::runtime_registry::reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
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

    let gateway_mode_info = HostRuntimeModeSnapshot {
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
    };

    reconcile_runtime_registry_stored_projection_overrides(
        Some(&registry),
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
        &gateway_mode_info,
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
    assert_eq!(restored_runtime.models.len(), 1);
    assert_eq!(
        restored_runtime.models[0].model_id,
        "/models/restored.safetensors"
    );
}

#[test]
fn reconcile_runtime_registry_stored_projection_overrides_skips_live_host_runtime_ids() {
    let registry = RuntimeRegistry::new();
    crate::runtime_registry::reconcile_runtime_registry_mode_info(
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

    let gateway_mode_info = HostRuntimeModeSnapshot {
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
    };

    reconcile_runtime_registry_stored_projection_overrides(
        Some(&registry),
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
        &gateway_mode_info,
    );

    let snapshot = registry.snapshot();
    let live_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("live gateway runtime should remain present");
    assert_eq!(
        live_runtime.runtime_instance_id.as_deref(),
        Some("llama-main-live")
    );
    assert_eq!(live_runtime.models.len(), 1);
    assert_eq!(live_runtime.models[0].model_id, "/models/main.gguf");
}
