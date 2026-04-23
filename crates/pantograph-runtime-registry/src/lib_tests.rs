use super::*;

#[test]
fn register_runtime_canonicalizes_runtime_and_backend_keys() {
    let registry = RuntimeRegistry::new();

    let snapshot = registry.register_runtime(
        RuntimeRegistration::new("PyTorch", "PyTorch sidecar").with_backend_keys(vec![
            "torch".to_string(),
            "PyTorch".to_string(),
            "torch".to_string(),
        ]),
    );

    assert_eq!(snapshot.runtime_id, "pytorch");
    assert_eq!(snapshot.backend_keys, vec!["pytorch".to_string()]);
    assert_eq!(snapshot.status, RuntimeRegistryStatus::Stopped);
}

#[test]
fn transition_runtime_rejects_invalid_state_changes() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));

    let err = registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Busy {
                runtime_instance_id: Some("runtime-1".to_string()),
            },
        )
        .expect_err("busy from stopped should be rejected");

    assert_eq!(
        err,
        RuntimeRegistryError::InvalidTransition {
            runtime_id: "llama_cpp".to_string(),
            from: RuntimeRegistryStatus::Stopped,
            to: RuntimeRegistryStatus::Busy,
        }
    );
}

#[test]
fn reservation_lifecycle_updates_snapshot_state() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("onnxruntime", "ONNX Runtime")
            .with_backend_keys(vec!["onnxruntime".to_string()]),
    );
    registry
        .transition_runtime(
            "onnx-runtime",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("python-runtime:onnx-runtime".to_string()),
            },
        )
        .expect("ready transition");

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "onnxruntime".to_string(),
            workflow_id: "wf-1".to_string(),
            reservation_owner_id: None,
            usage_profile: Some("audio".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: true,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("acquire reservation");

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes.len(), 1);
    assert_eq!(snapshot.runtimes[0].runtime_id, "onnx-runtime");
    assert_eq!(
        snapshot.runtimes[0].active_reservation_ids,
        vec![lease.reservation_id]
    );
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.reservations[0].runtime_id, "onnx-runtime");
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
    assert_eq!(
        registry
            .retention_disposition("onnxruntime")
            .expect("disposition"),
        RuntimeRetentionDisposition::retain(
            "onnx-runtime",
            RuntimeRetentionReason::KeepAliveReservation,
        )
    );

    let disposition = registry
        .release_reservation_with_disposition(lease.reservation_id)
        .expect("release reservation");
    assert_eq!(
        disposition,
        RuntimeRetentionDisposition::evict("onnx-runtime")
    );

    let released = registry.snapshot();
    assert!(released.runtimes[0].active_reservation_ids.is_empty());
    assert!(released.reservations.is_empty());
}

#[test]
fn register_runtime_without_new_budget_preserves_existing_budget() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
            RuntimeAdmissionBudget::new(Some(4096), Some(8192))
                .with_safety_margin_ram_mb(256)
                .with_safety_margin_vram_mb(1024),
        ),
    );

    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp")
            .with_backend_keys(vec!["llama_cpp".to_string()]),
    );
    registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-budget".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: Some(7200),
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: Some(4096),
                estimated_min_ram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("preserved vram budget should still reject oversized request");

    assert_eq!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "llama_cpp".to_string(),
            failure: RuntimeAdmissionFailure::InsufficientVram {
                requested_mb: 7200,
                available_mb: 7168,
                reserved_mb: 0,
                total_mb: 8192,
                safety_margin_mb: 1024,
            },
        }
    );
}

#[test]
fn reservations_are_rejected_while_runtime_is_stopping() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-llama.cpp".to_string()),
            },
        )
        .expect("ready transition");
    registry
        .transition_runtime("llama.cpp", RuntimeTransition::StopRequested)
        .expect("stop transition");

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-stop".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("stopping runtime should reject reservations");

    assert_eq!(
        err,
        RuntimeRegistryError::ReservationRejected("llama_cpp".to_string())
    );
}

#[test]
fn observe_runtimes_registers_active_and_embedding_runtimes() {
    let registry = RuntimeRegistry::new();

    let snapshots = registry.observe_runtimes(vec![
        RuntimeObservation {
            runtime_id: "llama.cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/qwen.gguf".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("llama-main-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "llama.cpp.embedding".to_string(),
            display_name: "Dedicated embedding runtime".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/embed.gguf".to_string()),
            status: RuntimeRegistryStatus::Warming,
            runtime_instance_id: Some("llama-embed-1".to_string()),
            last_error: None,
        },
    ]);

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
fn observe_runtimes_stops_unobserved_runtimes_without_reservations() {
    let registry = RuntimeRegistry::new();

    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "llama.cpp".to_string(),
        display_name: "llama.cpp".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("/models/qwen.gguf".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("llama-main-1".to_string()),
        last_error: None,
    }]);

    let snapshots = registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "ollama".to_string(),
        display_name: "ollama".to_string(),
        backend_keys: vec!["ollama".to_string()],
        model_id: Some("llava:13b".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("ollama-1".to_string()),
        last_error: None,
    }]);

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
fn observe_runtime_updates_single_runtime_without_stopping_others() {
    let registry = RuntimeRegistry::new();

    registry.observe_runtimes(vec![
        RuntimeObservation {
            runtime_id: "llama.cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("/models/qwen.gguf".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("llama-main-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "onnx-runtime".to_string(),
            display_name: "ONNX Runtime (Python sidecar)".to_string(),
            backend_keys: vec!["onnx-runtime".to_string()],
            model_id: Some("/models/voice.onnx".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
            last_error: None,
        },
    ]);

    let updated = registry.observe_runtime(RuntimeObservation {
        runtime_id: "onnx-runtime".to_string(),
        display_name: "ONNX Runtime (Python sidecar)".to_string(),
        backend_keys: vec!["onnx-runtime".to_string()],
        model_id: Some("/models/voice-v2.onnx".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
        last_error: None,
    });

    assert_eq!(updated.runtime_id, "onnx-runtime");
    assert_eq!(updated.status, RuntimeRegistryStatus::Ready);
    assert_eq!(updated.models[0].model_id, "/models/voice-v2.onnx");

    let snapshot = registry.snapshot();
    let llama = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("llama runtime should remain observed");
    assert_eq!(llama.status, RuntimeRegistryStatus::Ready);
    assert_eq!(llama.models[0].model_id, "/models/qwen.gguf");
}

#[test]
fn eviction_candidates_exclude_reserved_and_pinned_runtimes() {
    let registry = RuntimeRegistry::new();

    registry.observe_runtimes(vec![
        RuntimeObservation {
            runtime_id: "ready-old".to_string(),
            display_name: "ready-old".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("model-a".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("ready-old-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "reserved-ready".to_string(),
            display_name: "reserved-ready".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("model-b".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("reserved-ready-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "pinned-ready".to_string(),
            display_name: "pinned-ready".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("model-c".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("pinned-ready-1".to_string()),
            last_error: None,
        },
    ]);

    let reserved = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "reserved-ready".to_string(),
            workflow_id: "wf-reserved".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-b".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reserve runtime");

    {
        let mut guard = registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let pinned_runtime = guard
            .runtimes
            .get_mut("pinned-ready")
            .expect("pinned runtime should exist");
        let pinned_model = pinned_runtime
            .models
            .get_mut("model-c")
            .expect("pinned model should exist");
        pinned_model.pinned = true;
    }

    let candidates = registry.eviction_candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].runtime_id, "ready-old");

    registry
        .release_reservation(reserved.reservation_id)
        .expect("release reservation");
}

#[test]
fn release_disposition_prefers_keep_alive_hint_when_other_reservations_remain() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "shared-runtime".to_string(),
        display_name: "shared-runtime".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("shared-runtime-1".to_string()),
        last_error: None,
    }]);

    let keep_alive = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-keep-alive".to_string(),
            reservation_owner_id: None,
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("keep-alive reservation");
    let ephemeral = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-ephemeral".to_string(),
            reservation_owner_id: None,
            usage_profile: Some("batch".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("ephemeral reservation");

    let disposition = registry
        .release_reservation_with_disposition(ephemeral.reservation_id)
        .expect("release ephemeral reservation");

    assert_eq!(
        disposition,
        RuntimeRetentionDisposition::retain(
            "shared-runtime",
            RuntimeRetentionReason::KeepAliveReservation,
        )
    );

    registry
        .release_reservation(keep_alive.reservation_id)
        .expect("release keep-alive reservation");
}

#[test]
fn update_reservation_retention_hint_mutates_existing_lease() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "shared-runtime".to_string(),
        display_name: "shared-runtime".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("shared-runtime-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "shared-runtime".to_string(),
            workflow_id: "wf-update".to_string(),
            reservation_owner_id: Some("session-update".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation should be created");

    let updated = registry
        .update_reservation_retention_hint(lease.reservation_id, RuntimeRetentionHint::KeepAlive)
        .expect("retention hint should update");
    assert_eq!(updated.retention_hint, RuntimeRetentionHint::KeepAlive);

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
}

#[test]
fn retention_disposition_returns_status_reason_for_non_evictable_runtime() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("busy", "busy"));
    registry
        .transition_runtime(
            "busy",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("busy-runtime".to_string()),
            },
        )
        .expect("ready transition");
    registry
        .transition_runtime(
            "busy",
            RuntimeTransition::Busy {
                runtime_instance_id: Some("busy-runtime".to_string()),
            },
        )
        .expect("busy transition");

    assert_eq!(
        registry.retention_disposition("busy").expect("disposition"),
        RuntimeRetentionDisposition::retain(
            "busy",
            RuntimeRetentionReason::Status(RuntimeRegistryStatus::Busy),
        )
    );
}

#[test]
fn warmup_disposition_starts_stopped_runtime() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));

    assert_eq!(
        registry
            .warmup_disposition("llama.cpp")
            .expect("warmup disposition"),
        RuntimeWarmupDisposition::start(
            "llama_cpp",
            RuntimeWarmupReason::NoLoadedInstance,
            RuntimeRegistryStatus::Stopped,
        )
    );
}

#[test]
fn warmup_disposition_reuses_ready_runtime_instance() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("ready", "Ready"));
    registry
        .transition_runtime(
            "ready",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("ready-1".to_string()),
            },
        )
        .expect("runtime should become ready");

    assert_eq!(
        registry
            .warmup_disposition("ready")
            .expect("warmup disposition"),
        RuntimeWarmupDisposition::reuse(
            "ready",
            RuntimeWarmupReason::LoadedInstanceReady,
            RuntimeRegistryStatus::Ready,
            Some("ready-1".to_string()),
        )
    );
}

#[test]
fn warmup_disposition_waits_for_warming_runtime() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("warming", "Warming"));
    registry
        .transition_runtime(
            "warming",
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("warming-1".to_string()),
            },
        )
        .expect("warmup should start");

    assert_eq!(
        registry
            .warmup_disposition("warming")
            .expect("warmup disposition"),
        RuntimeWarmupDisposition::wait(
            "warming",
            RuntimeWarmupReason::WarmupInProgress,
            RuntimeRegistryStatus::Warming,
            Some("warming-1".to_string()),
        )
    );
}

#[test]
fn warmup_disposition_marks_failed_runtime_for_recovery() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("failed", "Failed"));
    registry
        .transition_runtime(
            "failed",
            RuntimeTransition::Failed {
                message: "boom".to_string(),
            },
        )
        .expect("failure should be recorded");

    assert_eq!(
        registry
            .warmup_disposition("failed")
            .expect("warmup disposition"),
        RuntimeWarmupDisposition::start(
            "failed",
            RuntimeWarmupReason::RecoveryRequired,
            RuntimeRegistryStatus::Failed,
        )
    );
}

#[test]
fn release_reservation_if_present_is_idempotent() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "idempotent-runtime".to_string(),
        display_name: "idempotent-runtime".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("idempotent-runtime-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "idempotent-runtime".to_string(),
            workflow_id: "wf-idempotent".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");

    assert_eq!(
        registry
            .release_reservation_if_present(lease.reservation_id)
            .expect("first release"),
        Some(RuntimeRetentionDisposition::evict("idempotent-runtime"))
    );
    assert_eq!(
        registry
            .release_reservation_if_present(lease.reservation_id)
            .expect("second release"),
        None
    );
}

#[test]
fn reclaim_runtime_marks_inactive_evictable_runtime_stopped() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "inactive-reclaim".to_string(),
        display_name: "inactive-reclaim".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("inactive-reclaim-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "inactive-reclaim".to_string(),
            workflow_id: "wf-inactive".to_string(),
            reservation_owner_id: Some("session-inactive".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    registry
        .release_reservation_if_present(lease.reservation_id)
        .expect("release");

    assert_eq!(
        registry
            .reclaim_runtime("inactive-reclaim", false)
            .expect("reclaim disposition"),
        RuntimeReclaimDisposition::no_action(
            "inactive-reclaim",
            RuntimeRetentionReason::Evictable,
            RuntimeRegistryStatus::Stopped,
        )
    );

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
    assert!(snapshot.runtimes[0].runtime_instance_id.is_none());
    assert!(snapshot.runtimes[0].models.is_empty());
}

#[test]
fn reclaim_runtime_requests_stop_for_active_evictable_runtime() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "active-reclaim".to_string(),
        display_name: "active-reclaim".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("active-reclaim-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "active-reclaim".to_string(),
            workflow_id: "wf-active".to_string(),
            reservation_owner_id: Some("session-active".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    registry
        .release_reservation_if_present(lease.reservation_id)
        .expect("release");

    assert_eq!(
        registry
            .reclaim_runtime("active-reclaim", true)
            .expect("reclaim disposition"),
        RuntimeReclaimDisposition::stop_producer("active-reclaim", RuntimeRegistryStatus::Stopping,)
    );

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopping);
    assert_eq!(
        snapshot.runtimes[0].runtime_instance_id.as_deref(),
        Some("active-reclaim-1")
    );
}

#[test]
fn reclaim_runtime_keeps_keep_alive_runtime_retained() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "retained-reclaim".to_string(),
        display_name: "retained-reclaim".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("retained-reclaim-1".to_string()),
        last_error: None,
    }]);

    let _lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "retained-reclaim".to_string(),
            workflow_id: "wf-retained".to_string(),
            reservation_owner_id: Some("session-retained".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("reservation");

    assert_eq!(
        registry
            .reclaim_runtime("retained-reclaim", true)
            .expect("reclaim disposition"),
        RuntimeReclaimDisposition::no_action(
            "retained-reclaim",
            RuntimeRetentionReason::KeepAliveReservation,
            RuntimeRegistryStatus::Ready,
        )
    );

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
}

#[test]
fn acquire_reservation_reuses_existing_owner_binding() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![
        RuntimeObservation {
            runtime_id: "owner-runtime-a".to_string(),
            display_name: "owner-runtime-a".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("model-a".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("owner-runtime-a-1".to_string()),
            last_error: None,
        },
        RuntimeObservation {
            runtime_id: "owner-runtime-b".to_string(),
            display_name: "owner-runtime-b".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("model-b".to_string()),
            status: RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("owner-runtime-b-1".to_string()),
            last_error: None,
        },
    ]);

    let first = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "owner-runtime-a".to_string(),
            workflow_id: "wf-owner".to_string(),
            reservation_owner_id: Some("session-owner".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("first owner reservation");

    let reused = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "owner-runtime-a".to_string(),
            workflow_id: "wf-owner".to_string(),
            reservation_owner_id: Some("session-owner".to_string()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("second owner reservation should reuse existing lease");

    assert_eq!(first.reservation_id, reused.reservation_id);
    assert_eq!(
        reused.reservation_owner_id.as_deref(),
        Some("session-owner")
    );
    assert_eq!(reused.retention_hint, RuntimeRetentionHint::Ephemeral);
    assert_eq!(reused.usage_profile, None);
    assert_eq!(reused.model_id, None);

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::Ephemeral
    );
    assert_eq!(snapshot.reservations[0].usage_profile, None);
    assert_eq!(snapshot.reservations[0].model_id, None);

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "owner-runtime-b".to_string(),
            workflow_id: "wf-owner".to_string(),
            reservation_owner_id: Some("session-owner".to_string()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("owner should not bind to another runtime");

    assert_eq!(
        err,
        RuntimeRegistryError::ReservationOwnerConflict {
            owner_id: "session-owner".to_string(),
            existing_runtime_id: "owner-runtime-a".to_string(),
            requested_runtime_id: "owner-runtime-b".to_string(),
        }
    );
}

#[test]
fn owner_reservation_reuse_recomputes_admission_against_other_claims_only() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("budget-runtime", "budget-runtime")
            .with_admission_budget(RuntimeAdmissionBudget::new(Some(1024), Some(1024))),
    );
    registry
        .transition_runtime(
            "budget-runtime",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("budget-runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    let first = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "budget-runtime".to_string(),
            workflow_id: "wf-first".to_string(),
            reservation_owner_id: Some("session-first".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_ram_mb: Some(700),
                estimated_peak_vram_mb: None,
                estimated_min_ram_mb: None,
                estimated_min_vram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("first reservation");

    registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "budget-runtime".to_string(),
            workflow_id: "wf-second".to_string(),
            reservation_owner_id: Some("session-second".to_string()),
            usage_profile: None,
            model_id: Some("model-b".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_ram_mb: Some(200),
                estimated_peak_vram_mb: None,
                estimated_min_ram_mb: None,
                estimated_min_vram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("second reservation");

    let reused = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "budget-runtime".to_string(),
            workflow_id: "wf-first".to_string(),
            reservation_owner_id: Some("session-first".to_string()),
            usage_profile: None,
            model_id: Some("model-a-v2".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_ram_mb: Some(824),
                estimated_peak_vram_mb: None,
                estimated_min_ram_mb: None,
                estimated_min_vram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("owner reuse should exclude existing claim when rechecking admission");

    assert_eq!(reused.reservation_id, first.reservation_id);
    assert_eq!(reused.model_id.as_deref(), Some("model-a-v2"));
    assert_eq!(reused.retention_hint, RuntimeRetentionHint::KeepAlive);

    let snapshot = registry.snapshot();
    let updated = snapshot
        .reservations
        .iter()
        .find(|reservation| reservation.reservation_id == first.reservation_id)
        .expect("updated reservation");
    assert_eq!(updated.model_id.as_deref(), Some("model-a-v2"));
    assert_eq!(updated.retention_hint, RuntimeRetentionHint::KeepAlive);
}

#[test]
fn eviction_candidates_use_deterministic_status_and_age_ordering() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("ready-old", "ready-old"));
    registry.register_runtime(RuntimeRegistration::new("ready-new", "ready-new"));
    registry.register_runtime(RuntimeRegistration::new("warming", "warming"));
    registry.register_runtime(RuntimeRegistration::new("unhealthy", "unhealthy"));
    registry.register_runtime(RuntimeRegistration::new("busy", "busy"));

    {
        let mut guard = registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let ready_old = guard
            .runtimes
            .get_mut("ready-old")
            .expect("ready-old runtime");
        ready_old.status = RuntimeRegistryStatus::Ready;
        ready_old.last_transition_at_ms = 10;

        let ready_new = guard
            .runtimes
            .get_mut("ready-new")
            .expect("ready-new runtime");
        ready_new.status = RuntimeRegistryStatus::Ready;
        ready_new.last_transition_at_ms = 20;

        let warming = guard.runtimes.get_mut("warming").expect("warming runtime");
        warming.status = RuntimeRegistryStatus::Warming;
        warming.last_transition_at_ms = 5;

        let unhealthy = guard
            .runtimes
            .get_mut("unhealthy")
            .expect("unhealthy runtime");
        unhealthy.status = RuntimeRegistryStatus::Unhealthy;
        unhealthy.last_transition_at_ms = 30;

        let busy = guard.runtimes.get_mut("busy").expect("busy runtime");
        busy.status = RuntimeRegistryStatus::Busy;
        busy.last_transition_at_ms = 1;
    }

    let candidates = registry.eviction_candidates();
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.runtime_id.as_str())
            .collect::<Vec<_>>(),
        vec!["unhealthy", "ready-old", "ready-new", "warming"]
    );
}

#[test]
fn eviction_reservation_candidates_follow_runtime_and_retention_ordering() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("ready-old", "ready-old"));
    registry.register_runtime(RuntimeRegistration::new("ready-new", "ready-new"));
    registry.register_runtime(RuntimeRegistration::new("busy", "busy"));

    {
        let mut guard = registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let ready_old = guard
            .runtimes
            .get_mut("ready-old")
            .expect("ready-old runtime");
        ready_old.status = RuntimeRegistryStatus::Ready;
        ready_old.last_transition_at_ms = 10;

        let ready_new = guard
            .runtimes
            .get_mut("ready-new")
            .expect("ready-new runtime");
        ready_new.status = RuntimeRegistryStatus::Ready;
        ready_new.last_transition_at_ms = 20;

        let busy = guard.runtimes.get_mut("busy").expect("busy runtime");
        busy.status = RuntimeRegistryStatus::Busy;
        busy.last_transition_at_ms = 1;
    }

    let old_keep_alive = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "ready-old".to_string(),
            workflow_id: "wf-a".to_string(),
            reservation_owner_id: Some("session-a".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("old keep-alive reservation");
    let old_ephemeral = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "ready-old".to_string(),
            workflow_id: "wf-b".to_string(),
            reservation_owner_id: Some("session-b".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("old ephemeral reservation");
    let new_ephemeral = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "ready-new".to_string(),
            workflow_id: "wf-c".to_string(),
            reservation_owner_id: Some("session-c".to_string()),
            usage_profile: None,
            model_id: Some("model-b".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("new ephemeral reservation");
    let _busy_ephemeral = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "busy".to_string(),
            workflow_id: "wf-d".to_string(),
            reservation_owner_id: Some("session-d".to_string()),
            usage_profile: None,
            model_id: Some("model-c".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("busy reservation");

    let candidates = registry.eviction_reservation_candidates();
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.reservation_owner_id.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("session-b"), Some("session-a"), Some("session-c")]
    );
    assert_eq!(candidates[0].reservation_id, old_ephemeral.reservation_id);
    assert_eq!(candidates[1].reservation_id, old_keep_alive.reservation_id);
    assert_eq!(candidates[2].reservation_id, new_ephemeral.reservation_id);
}

#[test]
fn eviction_reservation_candidate_for_owners_returns_first_matching_owner() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("ready-old", "ready-old"));
    registry.register_runtime(RuntimeRegistration::new("ready-new", "ready-new"));

    {
        let mut guard = registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let ready_old = guard
            .runtimes
            .get_mut("ready-old")
            .expect("ready-old runtime");
        ready_old.status = RuntimeRegistryStatus::Ready;
        ready_old.last_transition_at_ms = 10;

        let ready_new = guard
            .runtimes
            .get_mut("ready-new")
            .expect("ready-new runtime");
        ready_new.status = RuntimeRegistryStatus::Ready;
        ready_new.last_transition_at_ms = 20;
    }

    let old_ephemeral = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "ready-old".to_string(),
            workflow_id: "wf-a".to_string(),
            reservation_owner_id: Some("session-a".to_string()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("old ephemeral reservation");
    let _new_ephemeral = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "ready-new".to_string(),
            workflow_id: "wf-b".to_string(),
            reservation_owner_id: Some("session-b".to_string()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("new ephemeral reservation");

    let selected = registry
        .eviction_reservation_candidate_for_owners(&["session-b", "session-a"])
        .expect("matching candidate should exist");
    assert_eq!(selected.reservation_id, old_ephemeral.reservation_id);

    assert!(
        registry
            .eviction_reservation_candidate_for_owners(&["missing-session"])
            .is_none()
    );
}

#[test]
fn admission_budget_rejects_reservations_that_exceed_remaining_vram() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
            RuntimeAdmissionBudget::new(None, Some(8192)).with_safety_margin_vram_mb(1024),
        ),
    );
    registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-1".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: Some(6144),
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: Some(4096),
                estimated_min_ram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("first reservation should fit available vram");

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-2".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-b".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: Some(2048),
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: Some(1024),
                estimated_min_ram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("second reservation should exceed remaining vram");

    assert_eq!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "llama_cpp".to_string(),
            failure: RuntimeAdmissionFailure::InsufficientVram {
                requested_mb: 2048,
                available_mb: 1024,
                reserved_mb: 6144,
                total_mb: 8192,
                safety_margin_mb: 1024,
            },
        }
    );
}

#[test]
fn can_acquire_reservation_reports_admission_failure_without_creating_reservation() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp")
            .with_admission_budget(RuntimeAdmissionBudget::new(None, Some(4096))),
    );
    registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    let err = registry
        .can_acquire_reservation(&RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-blocked".to_string(),
            reservation_owner_id: Some("session-blocked".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-blocked".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: Some(8192),
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: Some(4096),
                estimated_min_ram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("dry-run admission check should reject oversized request");

    assert!(matches!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id,
            failure: RuntimeAdmissionFailure::InsufficientVram { .. },
        } if runtime_id == "llama_cpp"
    ));

    let snapshot = registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert_eq!(snapshot.runtimes.len(), 1);
    assert!(snapshot.runtimes[0].active_reservation_ids.is_empty());
}

#[test]
fn admission_budget_uses_peak_ram_claim_and_release_restores_capacity() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("pytorch", "PyTorch").with_admission_budget(
            RuntimeAdmissionBudget::new(Some(4096), None).with_safety_margin_ram_mb(512),
        ),
    );
    registry
        .transition_runtime(
            "pytorch",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-ram".to_string()),
            },
        )
        .expect("ready transition");

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-ram-1".to_string(),
            reservation_owner_id: None,
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-ram-a".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: Some(3584),
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: Some(1024),
            }),
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("peak ram claim should fit exactly");

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-ram-2".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-ram-b".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: Some(1),
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: Some(1),
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("no ram should remain after first reservation");

    assert_eq!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "pytorch".to_string(),
            failure: RuntimeAdmissionFailure::InsufficientRam {
                requested_mb: 1,
                available_mb: 0,
                reserved_mb: 3584,
                total_mb: 4096,
                safety_margin_mb: 512,
            },
        }
    );

    registry
        .release_reservation(lease.reservation_id)
        .expect("release reservation");

    registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-ram-3".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-ram-c".to_string()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: Some(1024),
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: Some(512),
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("released capacity should admit a new reservation");
}
