use super::*;

#[test]
fn admission_budget_rejects_reservations_that_exceed_remaining_vram() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                vram_budget_mib(Some(8192)).with_safety_margin_bytes(vram_mib(1024))
            ]),
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
            requirements: Some(reservation_requirements(vec![vram_claim_mib(6144)])),
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
            requirements: Some(reservation_requirements(vec![vram_claim_mib(2048)])),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("second reservation should exceed remaining vram");

    assert_eq!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "llama_cpp".to_string(),
            failure: RuntimeAdmissionFailure::InsufficientVram {
                requested_bytes: vram_mib(2048),
                available_bytes: vram_mib(1024),
                reserved_bytes: vram_mib(6144),
                total_bytes: vram_mib(8192),
                safety_margin_bytes: vram_mib(1024),
            },
        }
    );
}

#[test]
fn can_acquire_reservation_reports_admission_failure_without_creating_reservation() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![vram_budget_mib(Some(4096))]),
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

    let err = registry
        .can_acquire_reservation(&RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-blocked".to_string(),
            reservation_owner_id: Some("session-blocked".to_string()),
            usage_profile: Some("interactive".to_string()),
            model_id: Some("model-blocked".to_string()),
            pin_runtime: false,
            requirements: Some(reservation_requirements(vec![vram_claim_mib(8192)])),
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
            RuntimeAdmissionBudget::from_resources(vec![
                ram_budget_mib(Some(4096)).with_safety_margin_bytes(ram_mib(512))
            ]),
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
            requirements: Some(reservation_requirements(vec![ram_claim_mib(3584)])),
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
            requirements: Some(reservation_requirements(vec![ram_claim_mib(1)])),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("no ram should remain after first reservation");

    assert_eq!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "pytorch".to_string(),
            failure: RuntimeAdmissionFailure::InsufficientRam {
                requested_bytes: ram_mib(1),
                available_bytes: 0,
                reserved_bytes: ram_mib(3584),
                total_bytes: ram_mib(4096),
                safety_margin_bytes: ram_mib(512),
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
            requirements: Some(reservation_requirements(vec![ram_claim_mib(1024)])),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("released capacity should admit a new reservation");
}

#[test]
fn runtime_snapshot_exposes_reduced_admission_budget_and_active_claims() {
    let registry = RuntimeRegistry::new();
    let admission_budget = RuntimeAdmissionBudget::from_resources(vec![
        ram_budget_mib(Some(2048)).with_safety_margin_bytes(ram_mib(128)),
        vram_budget_mib(Some(4096)).with_safety_margin_bytes(vram_mib(256)),
    ]);
    registry.register_runtime(
        RuntimeRegistration::new("pytorch", "PyTorch")
            .with_admission_budget(admission_budget.clone()),
    );
    registry
        .transition_runtime(
            "pytorch",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-snapshot".to_string()),
            },
        )
        .expect("ready transition");

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-snapshot".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-snapshot".to_string()),
            pin_runtime: false,
            requirements: Some(reservation_requirements(vec![
                ram_claim_mib(1024),
                vram_claim_mib(1536),
            ])),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation should fit budget");

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes.len(), 1);
    let runtime = &snapshot.runtimes[0];
    assert_eq!(runtime.admission_budget, Some(admission_budget));
    assert_eq!(
        runtime.active_reservation_claims,
        vec![RuntimeActiveReservationClaim {
            reservation_id: lease.reservation_id,
            claims: vec![ram_claim_mib(1024), vram_claim_mib(1536)],
        }]
    );
}

#[test]
fn reserved_resource_accounting_overflow_returns_typed_error() {
    let mut reservations = BTreeMap::new();
    reservations.insert(
        1,
        RuntimeReservationRecord {
            reservation_id: 1,
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-overflow-1".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            retention_hint: RuntimeRetentionHint::Ephemeral,
            created_at_ms: 0,
            claim: RuntimeReservationClaim {
                ram_bytes: Some(u64::MAX),
                vram_bytes: None,
            },
        },
    );
    reservations.insert(
        2,
        RuntimeReservationRecord {
            reservation_id: 2,
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-overflow-2".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            retention_hint: RuntimeRetentionHint::Ephemeral,
            created_at_ms: 0,
            claim: RuntimeReservationClaim {
                ram_bytes: Some(1),
                vram_bytes: None,
            },
        },
    );

    let err =
        total_reserved_resource_bytes("pytorch", "ram_bytes", &reservations, None, |reservation| {
            reservation.claim.ram_bytes
        })
        .expect_err("reserved resource accounting should reject overflow");

    assert_eq!(
        err,
        RuntimeRegistryError::ResourceAccountingOverflow {
            runtime_id: "pytorch".to_string(),
            resource_kind: "ram_bytes",
        }
    );
}

#[test]
fn available_budget_underflow_returns_typed_error() {
    let budget = RuntimeAdmissionResourceBudget::ram_bytes(Some(ram_mib(1024)))
        .with_safety_margin_bytes(ram_mib(1536));
    let err = available_budget_bytes("pytorch", "ram_bytes", Some(&budget), 0)
        .expect_err("safety margin above total budget should fail");

    assert_eq!(
        err,
        RuntimeRegistryError::ResourceBudgetUnderflow {
            runtime_id: "pytorch".to_string(),
            resource_kind: "ram_bytes",
            total_bytes: ram_mib(1024),
            safety_margin_bytes: ram_mib(1536),
            reserved_bytes: 0,
        }
    );
}
