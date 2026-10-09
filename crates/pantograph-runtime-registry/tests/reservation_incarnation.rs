use std::sync::Arc;

use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeRegistration, RuntimeRegistry,
    RuntimeReservationRequest, RuntimeReservationRequirements, RuntimeReservationResourceClaim,
    RuntimeRetentionHint, RuntimeTransition,
};

fn registry() -> Arc<RuntimeRegistry> {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register_runtime(
        RuntimeRegistration::new("candle", "Candle").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(100)),
            ]),
        ),
    );
    registry
        .transition_runtime(
            "candle",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("instance.1".into()),
            },
        )
        .unwrap();
    registry
}

fn request(bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some("task.owner".into()),
        usage_profile: None,
        model_id: Some("model".into()),
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::Ephemeral,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(bytes),
        ])),
    }
}

fn assert_old_incarnation_refused(provisional: bool) {
    let registry = registry();
    let old = registry.acquire_reservation(request(10)).unwrap();
    let new = if provisional {
        let replacement = request(1);
        let observed = registry
            .evaluate_reservation(replacement.clone())
            .unwrap()
            .observation()
            .clone();
        let (new, custody) = registry
            .acquire_reservation_provisional(replacement, &observed, |lease| {
                Ok::<_, ()>(lease.clone())
            })
            .unwrap();
        custody.transfer().unwrap();
        new
    } else {
        registry.acquire_reservation(request(1)).unwrap()
    };
    let stale_custody = registry.acquire_execution_custody(&old);
    eprintln!(
        "provisional={provisional} old_id={} new_id={} leases_equal={} stale_custody_accepted={}",
        old.reservation_id,
        new.reservation_id,
        old == new,
        stale_custody.is_ok()
    );
    assert!(
        stale_custody.is_err(),
        "stale RAM10 admission executed after same-owner RAM1 replacement"
    );
    assert_ne!(old, new);
    assert!(registry.acquire_execution_custody(&new).is_ok());
}

#[test]
fn direct_replacement_refuses_original_admitted_incarnation() {
    assert_old_incarnation_refused(false);
}

#[test]
fn transferred_provisional_replacement_refuses_original_admitted_incarnation() {
    assert_old_incarnation_refused(true);
}

#[test]
fn identical_and_round_trip_replacements_cannot_revive_old_admission_or_cleanup() {
    let registry = registry();
    let old = registry.acquire_reservation(request(10)).unwrap();
    let same = registry.acquire_reservation(request(10)).unwrap();
    let small = registry.acquire_reservation(request(1)).unwrap();
    let restored = registry.acquire_reservation(request(10)).unwrap();
    assert_eq!(restored.created_at_ms, old.created_at_ms);
    for stale in [old, same, small] {
        assert!(registry.acquire_execution_custody(&stale).is_err());
        assert!(registry
            .release_reservation_if_present(stale.reservation_id)
            .unwrap()
            .is_none());
    }
    assert_eq!(
        registry.reservation_lease(restored.reservation_id),
        Some(restored.clone())
    );
    assert!(registry.acquire_execution_custody(&restored).is_ok());
    assert_eq!(
        registry.snapshot().runtimes[0].active_reservation_ids,
        vec![restored.reservation_id]
    );
}

#[test]
fn pending_predecessor_refuses_acknowledgement_and_rollback_restores_original() {
    let registry = registry();
    let old = registry.acquire_reservation(request(10)).unwrap();
    let before = registry.snapshot();
    let replacement = request(1);
    let observed = registry
        .evaluate_reservation(replacement.clone())
        .unwrap()
        .observation()
        .clone();
    let (new, custody) = registry
        .acquire_reservation_provisional(replacement, &observed, |lease| Ok::<_, ()>(lease.clone()))
        .unwrap();
    assert_ne!(new.reservation_id, old.reservation_id);
    assert!(registry
        .release_reservation_if_present(old.reservation_id)
        .is_err());
    assert!(registry
        .update_reservation_retention_hint_if_present(
            old.reservation_id,
            RuntimeRetentionHint::KeepAlive
        )
        .is_err());
    assert!(registry.acquire_execution_custody(&old).is_err());
    assert!(registry.acquire_execution_custody(&new).is_err());
    assert!(registry.acquire_reservation(request(2)).is_err());
    drop(custody);
    assert_eq!(registry.snapshot().reservations, before.reservations);
    assert_eq!(
        registry.snapshot().runtimes[0].active_reservation_ids,
        before.runtimes[0].active_reservation_ids
    );
    assert!(registry.reservation_lease(new.reservation_id).is_none());
    assert!(registry.acquire_execution_custody(&old).is_ok());
    registry.release_reservation(old.reservation_id).unwrap();
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn pending_alias_ends_on_transfer_or_explicit_current_lineage_release() {
    for release in [false, true] {
        let registry = registry();
        let old = registry.acquire_reservation(request(10)).unwrap();
        let replacement = request(1);
        let observed = registry
            .evaluate_reservation(replacement.clone())
            .unwrap()
            .observation()
            .clone();
        let (new, custody) = registry
            .acquire_reservation_provisional(replacement, &observed, |lease| {
                Ok::<_, ()>(lease.clone())
            })
            .unwrap();
        if release {
            registry.release_reservation(new.reservation_id).unwrap();
            drop(custody);
            assert!(registry.snapshot().reservations.is_empty());
        } else {
            custody.transfer().unwrap();
            assert_eq!(registry.reservation_lease(new.reservation_id), Some(new));
        }
        assert!(registry
            .release_reservation_if_present(old.reservation_id)
            .unwrap()
            .is_none());
    }
}
