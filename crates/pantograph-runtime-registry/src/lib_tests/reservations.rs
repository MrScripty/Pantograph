use super::*;

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
