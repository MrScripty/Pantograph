use std::sync::{Arc, Barrier};

use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeAdmissionResourceKind,
    RuntimeRegistration, RuntimeRegistry, RuntimeRegistryError, RuntimeReservationPublicationError,
    RuntimeReservationRequest, RuntimeReservationRequirements, RuntimeReservationResourceClaim,
    RuntimeResourceDomain, RuntimeResourceDomainBinding, RuntimeRetentionHint, RuntimeTransition,
};

fn registry() -> Arc<RuntimeRegistry> {
    let registry = Arc::new(RuntimeRegistry::new());
    for runtime in ["pytorch", "candle"] {
        registry.register_runtime(RuntimeRegistration::new(runtime, runtime));
        registry
            .transition_runtime(
                runtime,
                RuntimeTransition::Ready {
                    runtime_instance_id: Some(format!("{runtime}.1")),
                },
            )
            .unwrap();
    }
    registry
}

fn binding(
    runtime: &str,
    resource_kind: RuntimeAdmissionResourceKind,
) -> RuntimeResourceDomainBinding {
    RuntimeResourceDomainBinding {
        runtime_id: runtime.into(),
        resource_kind,
    }
}

fn domain() -> RuntimeResourceDomain {
    RuntimeResourceDomain {
        domain_id: "host.ram".into(),
        total_bytes: 100,
        safety_margin_bytes: 0,
        bindings: vec![
            binding("pytorch", RuntimeAdmissionResourceKind::RamBytes),
            binding("candle", RuntimeAdmissionResourceKind::RamBytes),
        ],
    }
}

fn request(runtime: &str, owner: &str, ram: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: runtime.into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(owner.into()),
        usage_profile: None,
        model_id: Some("model".into()),
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::Ephemeral,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(ram),
        ])),
    }
}

#[test]
fn alternative_evaluation_is_read_only_and_cross_runtime_commit_is_atomic() {
    let registry = registry();
    registry.configure_resource_domain(domain()).unwrap();
    let a = registry
        .evaluate_reservation(request("pytorch", "a", 80))
        .unwrap();
    let b = registry
        .evaluate_reservation(request("candle", "b", 80))
        .unwrap();
    for evaluation in [&a, &b] {
        let observation = &evaluation.observation().resource_domains[0];
        assert_eq!(observation.requested_bytes, 80);
        assert_eq!(observation.available_bytes, 100);
        assert_eq!(observation.reserved_bytes, 0);
    }
    assert!(registry.snapshot().reservations.is_empty());
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            a.commit()
        });
        let b = scope.spawn(|| {
            barrier.wait();
            b.commit()
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
                    requested_bytes: 80,
                    available_bytes: 20,
                    ..
                })
            ))
            .count(),
        1
    );
    assert_eq!(registry.snapshot().reservations.len(), 1);
}

#[test]
fn shared_margin_and_runtime_local_budgets_both_apply() {
    let registry = registry();
    let mut pool = domain();
    pool.safety_margin_bytes = 10;
    registry.configure_resource_domain(pool).unwrap();
    registry
        .acquire_reservation(request("pytorch", "a", 60))
        .unwrap();
    assert!(matches!(
        registry.acquire_reservation(request("candle", "b", 31)),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            available_bytes: 30,
            ..
        })
    ));
    registry.register_runtime(
        RuntimeRegistration::new("candle", "candle").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(20)),
            ]),
        ),
    );
    assert!(matches!(
        registry.acquire_reservation(request("candle", "b", 21)),
        Err(RuntimeRegistryError::AdmissionRejected { .. })
    ));
    registry
        .acquire_reservation(request("candle", "b", 20))
        .unwrap();
}

#[test]
fn unified_memory_charges_ram_and_vram_without_aliasing_equal_copies() {
    let registry = registry();
    let mut pool = domain();
    pool.bindings
        .push(binding("pytorch", RuntimeAdmissionResourceKind::VramBytes));
    registry.configure_resource_domain(pool).unwrap();
    let mut mixed = request("pytorch", "a", 40);
    mixed
        .requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::vram_bytes(40));
    let evaluation = registry.evaluate_reservation(mixed).unwrap();
    assert_eq!(
        evaluation.observation().resource_domains[0].requested_bytes,
        80
    );
    evaluation.commit().unwrap();
    assert!(matches!(
        registry.acquire_reservation(request("candle", "b", 21)),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            available_bytes: 20,
            ..
        })
    ));
}

#[test]
fn independent_domains_do_not_spend_each_others_capacity() {
    let registry = registry();
    for runtime in ["pytorch", "candle"] {
        let mut pool = domain();
        pool.domain_id = format!("{runtime}.ram");
        pool.bindings = vec![binding(runtime, RuntimeAdmissionResourceKind::RamBytes)];
        registry.configure_resource_domain(pool).unwrap();
        registry
            .acquire_reservation(request(runtime, runtime, 100))
            .unwrap();
    }
    assert_eq!(registry.snapshot().reservations.len(), 2);
}

#[test]
fn failed_replacement_keeps_original_and_release_restores_shared_capacity() {
    let registry = registry();
    registry.configure_resource_domain(domain()).unwrap();
    let original = registry
        .acquire_reservation(request("pytorch", "a", 30))
        .unwrap();
    let other = registry
        .acquire_reservation(request("candle", "b", 20))
        .unwrap();
    let replacement = registry
        .evaluate_reservation(request("pytorch", "a", 70))
        .unwrap();
    assert_eq!(
        replacement.observation().resource_domains[0].reserved_bytes,
        20
    );
    registry
        .acquire_reservation(request("candle", "c", 30))
        .unwrap();
    assert!(matches!(
        replacement.commit(),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            available_bytes: 50,
            ..
        })
    ));
    let held = registry
        .evaluate_reservation(request("candle", "probe", 0))
        .unwrap();
    assert_eq!(held.observation().resource_domains[0].reserved_bytes, 80);
    drop(held);
    registry.release_reservation(other.reservation_id).unwrap();
    registry
        .acquire_reservation(request("candle", "d", 40))
        .unwrap();
    assert!(registry
        .snapshot()
        .reservations
        .iter()
        .any(|lease| lease.reservation_id == original.reservation_id));
}

#[test]
fn provisional_unified_replacement_protects_both_rollback_and_transfer_capacity() {
    let registry = registry();
    let mut pool = domain();
    pool.bindings
        .push(binding("pytorch", RuntimeAdmissionResourceKind::VramBytes));
    registry.configure_resource_domain(pool).unwrap();
    registry
        .acquire_reservation(request("pytorch", "a", 60))
        .unwrap();
    let mut changed = request("pytorch", "a", 0);
    changed
        .requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::vram_bytes(60));
    let expected = registry
        .evaluate_reservation(changed.clone())
        .unwrap()
        .observation()
        .clone();
    let rejected =
        registry.acquire_reservation_provisional(changed, &expected, |_| Ok::<(), ()>(()));
    assert!(matches!(
        rejected,
        Err(RuntimeReservationPublicationError::Registry(
            RuntimeRegistryError::ResourceDomainAdmissionRejected {
                requested_bytes: 120,
                ..
            }
        ))
    ));
    let smaller = request("pytorch", "a", 10);
    let expected = registry
        .evaluate_reservation(smaller.clone())
        .unwrap()
        .observation()
        .clone();
    let (_, custody) = registry
        .acquire_reservation_provisional(smaller, &expected, |_| Ok::<(), ()>(()))
        .unwrap();
    assert!(registry
        .acquire_reservation(request("candle", "b", 41))
        .is_err());
    drop(custody);
    assert_eq!(
        registry
            .evaluate_reservation(request("candle", "probe", 0))
            .unwrap()
            .observation()
            .resource_domains[0]
            .reserved_bytes,
        60
    );
    let smaller = request("pytorch", "a", 10);
    let expected = registry
        .evaluate_reservation(smaller.clone())
        .unwrap()
        .observation()
        .clone();
    let (_, custody) = registry
        .acquire_reservation_provisional(smaller, &expected, |_| Ok::<(), ()>(()))
        .unwrap();
    custody.transfer().unwrap();
    registry
        .acquire_reservation(request("candle", "b", 90))
        .unwrap();
}

#[test]
fn configuration_changes_are_atomic_and_existing_evaluations_revalidate_new_budget() {
    let registry = registry();
    registry.configure_resource_domain(domain()).unwrap();
    registry
        .acquire_reservation(request("pytorch", "a", 60))
        .unwrap();
    let evaluation = registry
        .evaluate_reservation(request("candle", "b", 30))
        .unwrap();
    let mut pool = domain();
    pool.total_bytes = 50;
    assert!(registry.configure_resource_domain(pool).is_err());
    let mut pool = domain();
    pool.total_bytes = 80;
    registry.configure_resource_domain(pool).unwrap();
    assert!(matches!(
        evaluation.commit(),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            available_bytes: 20,
            ..
        })
    ));
    assert_eq!(registry.snapshot().reservations.len(), 1);
}

#[test]
fn ambiguous_bindings_and_changed_membership_are_rejected() {
    let registry = registry();
    registry.configure_resource_domain(domain()).unwrap();
    let mut conflict = domain();
    conflict.domain_id = "other".into();
    assert!(registry.configure_resource_domain(conflict).is_err());
    let mut changed = domain();
    changed.bindings.pop();
    assert!(registry.configure_resource_domain(changed).is_err());
    let mut duplicate = domain();
    duplicate.bindings.push(duplicate.bindings[0].clone());
    assert!(registry.configure_resource_domain(duplicate).is_err());
    let mut missing = domain();
    missing.bindings[0].runtime_id = "unregistered".into();
    assert!(matches!(
        registry.configure_resource_domain(missing),
        Err(RuntimeRegistryError::RuntimeNotFound(_))
    ));
}

#[test]
fn unified_claim_overflow_rejects_without_allocating_a_lease() {
    let registry = registry();
    let mut pool = domain();
    pool.total_bytes = u64::MAX;
    pool.bindings
        .push(binding("pytorch", RuntimeAdmissionResourceKind::VramBytes));
    registry.configure_resource_domain(pool).unwrap();
    let mut huge = request("pytorch", "a", u64::MAX);
    huge.requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::vram_bytes(1));
    assert!(matches!(
        registry.acquire_reservation(huge),
        Err(RuntimeRegistryError::ResourceDomainAccountingOverflow { .. })
    ));
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn unconfigured_runtime_capacity_remains_independent() {
    let registry = registry();
    registry
        .acquire_reservation(request("pytorch", "a", 80))
        .unwrap();
    registry
        .acquire_reservation(request("candle", "b", 80))
        .unwrap();
    assert!(registry
        .evaluate_reservation(request("pytorch", "probe", 0))
        .unwrap()
        .observation()
        .resource_domains
        .is_empty());
}

#[test]
fn domain_installation_accounts_for_live_leases_and_rejects_invalid_capacity() {
    let registry = registry();
    let a = registry
        .acquire_reservation(request("pytorch", "a", 80))
        .unwrap();
    let b = registry
        .acquire_reservation(request("candle", "b", 80))
        .unwrap();
    assert!(registry.configure_resource_domain(domain()).is_err());
    registry.release_reservation(b.reservation_id).unwrap();
    let mut invalid = domain();
    invalid.safety_margin_bytes = 101;
    assert!(registry.configure_resource_domain(invalid).is_err());
    let mut empty = domain();
    empty.bindings.clear();
    assert!(registry.configure_resource_domain(empty).is_err());
    let mut blank = domain();
    blank.domain_id = " ".into();
    assert!(registry.configure_resource_domain(blank).is_err());
    registry.configure_resource_domain(domain()).unwrap();
    let observation = registry
        .evaluate_reservation(request("candle", "probe", 0))
        .unwrap();
    assert_eq!(
        observation.observation().resource_domains[0].reserved_bytes,
        80
    );
    assert_eq!(
        registry.snapshot().reservations[0].reservation_id,
        a.reservation_id
    );
}
