use super::*;

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
