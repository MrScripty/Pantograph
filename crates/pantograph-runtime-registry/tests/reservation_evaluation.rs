use std::sync::{Arc, Barrier};

use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeAdmissionResourceKind,
    RuntimeRegistration, RuntimeRegistry, RuntimeRegistryError, RuntimeReservationRequest,
    RuntimeReservationRequirements, RuntimeReservationResourceClaim, RuntimeRetentionHint,
    RuntimeTransition,
};

fn registry() -> RuntimeRegistry {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("pytorch", "PyTorch").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(100)),
            ]),
        ),
    );
    registry
        .transition_runtime(
            "pytorch",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime.001".into()),
            },
        )
        .unwrap();
    registry
}

fn request(owner: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: "pytorch".into(),
        workflow_id: "wf".into(),
        reservation_owner_id: Some(owner.into()),
        usage_profile: None,
        model_id: Some("model.001".into()),
        pin_runtime: false,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(bytes),
        ])),
        retention_hint: RuntimeRetentionHint::Ephemeral,
    }
}

#[test]
fn evaluating_and_discarding_alternatives_does_not_mutate_registry_or_lease_ids() {
    let registry = registry();
    let before = registry.snapshot();
    let a = registry.evaluate_reservation(request("a", 80)).unwrap();
    let b = registry.evaluate_reservation(request("b", 80)).unwrap();
    assert_eq!(a.observation().resources[0].available_bytes, Some(100));
    assert_eq!(a.observation().resources[0].reserved_bytes, 0);
    assert_eq!(
        a.observation().runtime_instance_id.as_deref(),
        Some("runtime.001")
    );
    assert_eq!(registry.snapshot().runtimes, before.runtimes);
    assert!(registry.snapshot().reservations.is_empty());
    drop(b);
    let lease = a.commit().unwrap();
    assert_eq!(lease.reservation_id, 1);
    assert_eq!(lease.reservation_owner_id.as_deref(), Some("a"));
    assert_eq!(registry.snapshot().reservations.len(), 1);
}

#[test]
fn concurrent_commits_revalidate_and_cannot_spend_the_same_capacity_twice() {
    let registry = Arc::new(registry());
    let a = registry.evaluate_reservation(request("a", 80)).unwrap();
    let b = registry.evaluate_reservation(request("b", 80)).unwrap();
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
            .filter(|result| matches!(result, Err(RuntimeRegistryError::AdmissionRejected { .. })))
            .count(),
        1
    );
    assert_eq!(registry.snapshot().reservations.len(), 1);
    assert_eq!(
        registry.snapshot().runtimes[0].active_reservation_claims[0].claims[0].bytes,
        80
    );
}

#[test]
fn replacement_observation_excludes_own_lease_and_failed_commit_preserves_it() {
    let registry = registry();
    let lease = registry.acquire_reservation(request("a", 30)).unwrap();
    registry.acquire_reservation(request("b", 20)).unwrap();
    let evaluation = registry.evaluate_reservation(request("a", 70)).unwrap();
    assert_eq!(
        evaluation.observation().replaces_reservation_id,
        Some(lease.reservation_id)
    );
    assert_eq!(evaluation.observation().resources[0].reserved_bytes, 20);
    assert_eq!(
        evaluation.observation().resources[0].available_bytes,
        Some(80)
    );
    registry.acquire_reservation(request("c", 30)).unwrap();
    assert!(matches!(
        evaluation.commit(),
        Err(RuntimeRegistryError::AdmissionRejected { .. })
    ));
    let claims = registry
        .snapshot()
        .runtimes
        .remove(0)
        .active_reservation_claims;
    assert_eq!(
        claims
            .iter()
            .find(|claim| claim.reservation_id == lease.reservation_id)
            .unwrap()
            .claims[0]
            .bytes,
        30
    );
}

#[test]
fn stopping_between_evaluation_and_commit_rejects_without_a_lease() {
    let registry = registry();
    let evaluation = registry.evaluate_reservation(request("a", 20)).unwrap();
    registry
        .transition_runtime("pytorch", RuntimeTransition::StopRequested)
        .unwrap();
    assert!(matches!(
        evaluation.commit(),
        Err(RuntimeRegistryError::ReservationRejected(_))
    ));
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn missing_capacity_remains_unknown_and_does_not_claim_unlimited_hardware() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("pytorch", "PyTorch"));
    let evaluation = registry.evaluate_reservation(request("a", 20)).unwrap();
    let resource = &evaluation.observation().resources[0];
    assert_eq!(resource.capacity_bytes, None);
    assert_eq!(resource.available_bytes, None);
    assert_eq!(resource.requested_bytes, 20);
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn aggregated_claims_and_safety_margin_match_authoritative_admission() {
    let registry = registry();
    registry.register_runtime(
        RuntimeRegistration::new("pytorch", "PyTorch").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(100)).with_safety_margin_bytes(10),
                RuntimeAdmissionResourceBudget::vram_bytes(Some(200)).with_safety_margin_bytes(20),
            ]),
        ),
    );
    registry.acquire_reservation(request("a", 20)).unwrap();
    let mut requested = request("b", 30);
    requested.requirements.as_mut().unwrap().claims.extend([
        RuntimeReservationResourceClaim::ram_bytes(40),
        RuntimeReservationResourceClaim::vram_bytes(50),
    ]);
    let evaluation = registry.evaluate_reservation(requested).unwrap();
    let resources = &evaluation.observation().resources;
    assert_eq!(resources[0].kind, RuntimeAdmissionResourceKind::RamBytes);
    assert_eq!(resources[0].requested_bytes, 70);
    assert_eq!(resources[0].available_bytes, Some(70));
    assert_eq!(resources[0].safety_margin_bytes, 10);
    assert_eq!(resources[1].kind, RuntimeAdmissionResourceKind::VramBytes);
    assert_eq!(resources[1].available_bytes, Some(180));
    evaluation.commit().unwrap();
}

#[test]
fn owner_conflicts_and_overflow_fail_without_mutating_observations() {
    let registry = registry();
    registry.acquire_reservation(request("a", 20)).unwrap();
    registry.register_runtime(RuntimeRegistration::new("candle", "Candle"));
    let mut conflicting = request("a", 20);
    conflicting.runtime_id = "candle".into();
    assert!(matches!(
        registry.evaluate_reservation(conflicting),
        Err(RuntimeRegistryError::ReservationOwnerConflict { .. })
    ));
    let mut overflowing = request("b", u64::MAX);
    overflowing
        .requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::ram_bytes(1));
    assert!(matches!(
        registry.evaluate_reservation(overflowing),
        Err(RuntimeRegistryError::ResourceAccountingOverflow { .. })
    ));
    assert_eq!(registry.snapshot().reservations.len(), 1);
}
