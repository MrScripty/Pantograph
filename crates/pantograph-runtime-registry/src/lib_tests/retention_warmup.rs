use super::*;

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
