use std::sync::{Arc, Barrier};

use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeRegistration, RuntimeRegistry,
    RuntimeRegistryError, RuntimeReservationPublicationError, RuntimeReservationRequest,
    RuntimeReservationRequirements, RuntimeReservationResourceClaim, RuntimeRetentionHint,
    RuntimeTransition,
};

fn registry() -> Arc<RuntimeRegistry> {
    let registry = Arc::new(RuntimeRegistry::new());
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
                runtime_instance_id: Some("instance.1".into()),
            },
        )
        .unwrap();
    registry
}

fn request(owner: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: "pytorch".into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(owner.into()),
        usage_profile: None,
        model_id: Some("model.a".into()),
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::Ephemeral,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(bytes),
        ])),
    }
}

#[test]
fn rejected_publication_does_not_allocate_or_change_a_previous_lease() {
    for replacement in [false, true] {
        let registry = registry();
        if replacement {
            registry.acquire_reservation(request("owner", 30)).unwrap();
        }
        let before = registry.snapshot().reservations;
        let request = request("owner", 60);
        let expected = registry
            .evaluate_reservation(request.clone())
            .unwrap()
            .observation()
            .clone();
        let result = registry.acquire_reservation_provisional(request, &expected, |_| {
            Err::<(), _>("invalid candidate fact")
        });
        assert!(matches!(
            result,
            Err(RuntimeReservationPublicationError::Validation(
                "invalid candidate fact"
            ))
        ));
        assert_eq!(registry.snapshot().reservations, before);
    }
}

#[test]
fn dropping_unbound_custody_releases_a_new_lease_and_active_runtime_claim() {
    let registry = registry();
    let request = request("owner", 70);
    let expected = registry
        .evaluate_reservation(request.clone())
        .unwrap()
        .observation()
        .clone();
    let (_, custody) = registry
        .acquire_reservation_provisional(request, &expected, |_| Ok::<_, &str>(()))
        .unwrap();
    assert_eq!(registry.snapshot().reservations.len(), 1);
    drop(custody);
    let snapshot = registry.snapshot();
    assert!(snapshot.reservations.is_empty());
    assert!(snapshot.runtimes[0].active_reservation_ids.is_empty());
}

#[test]
fn cancelling_a_polled_preparation_future_rolls_back_its_owned_lease() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    for replacement in [false, true] {
        let registry = registry();
        if replacement {
            registry.acquire_reservation(request("owner", 80)).unwrap();
        }
        let before = registry.snapshot().reservations;
        let request = request("owner", 20);
        let expected = registry
            .evaluate_reservation(request.clone())
            .unwrap()
            .observation()
            .clone();
        let (_, custody) = registry
            .acquire_reservation_provisional(request, &expected, |_| Ok::<_, &str>(()))
            .unwrap();
        let mut preparation = Box::pin(async move {
            std::future::pending::<()>().await;
            custody.transfer().unwrap();
        });
        assert_eq!(
            preparation
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        );
        drop(preparation);
        let after = registry.snapshot();
        assert_eq!(after.reservations, before);
        assert_eq!(
            after.runtimes[0].active_reservation_ids.len(),
            usize::from(replacement)
        );
    }
}

#[test]
fn shrinking_replacement_keeps_rollback_capacity_and_all_previous_metadata() {
    let registry = registry();
    let mut previous = request("owner", 80);
    previous.pin_runtime = true;
    previous.retention_hint = RuntimeRetentionHint::KeepAlive;
    previous.usage_profile = Some("previous-profile".into());
    registry.acquire_reservation(previous).unwrap();
    let before = registry.snapshot().reservations;
    let request = request("owner", 20);
    let expected = registry
        .evaluate_reservation(request.clone())
        .unwrap()
        .observation()
        .clone();
    let (_, custody) = registry
        .acquire_reservation_provisional(request, &expected, |_| Ok::<_, &str>(()))
        .unwrap();
    assert!(matches!(
        registry.acquire_reservation(self::request("competitor", 30)),
        Err(RuntimeRegistryError::AdmissionRejected { .. })
    ));
    assert!(matches!(
        registry.acquire_reservation(self::request("owner", 10)),
        Err(RuntimeRegistryError::ReservationCustodyPending(_))
    ));
    assert!(matches!(
        registry.update_reservation_retention_hint(
            before[0].reservation_id,
            RuntimeRetentionHint::Ephemeral
        ),
        Err(RuntimeRegistryError::ReservationCustodyPending(_))
    ));
    drop(custody);
    assert_eq!(registry.snapshot().reservations, before);
}

#[test]
fn transferring_custody_preserves_the_new_lease_and_frees_predecessor_excess() {
    let registry = registry();
    registry.acquire_reservation(request("owner", 80)).unwrap();
    let request = request("owner", 20);
    let expected = registry
        .evaluate_reservation(request.clone())
        .unwrap()
        .observation()
        .clone();
    let (lease_id, custody) = registry
        .acquire_reservation_provisional(request, &expected, |lease| {
            Ok::<_, &str>(lease.reservation_id)
        })
        .unwrap();
    custody.transfer().unwrap();
    registry
        .acquire_reservation(self::request("competitor", 80))
        .unwrap();
    registry.release_reservation(lease_id).unwrap();
    assert_eq!(registry.snapshot().reservations.len(), 1);
}

#[test]
fn stale_instance_and_current_capacity_fail_before_publication_callback() {
    for changed_instance in [false, true] {
        let registry = registry();
        let request = request("owner", 60);
        let expected = registry
            .evaluate_reservation(request.clone())
            .unwrap()
            .observation()
            .clone();
        if changed_instance {
            registry
                .transition_runtime(
                    "pytorch",
                    RuntimeTransition::Ready {
                        runtime_instance_id: Some("instance.2".into()),
                    },
                )
                .unwrap();
        } else {
            registry
                .acquire_reservation(self::request("competitor", 80))
                .unwrap();
        }
        let result = registry.acquire_reservation_provisional(request, &expected, |_| {
            panic!("rejected admission must not publish")
        });
        assert!(matches!(
            result,
            Err::<((), _), RuntimeReservationPublicationError<&str>>(
                RuntimeReservationPublicationError::Registry(_)
            )
        ));
        assert_eq!(
            registry.snapshot().reservations.len(),
            usize::from(!changed_instance)
        );
    }
}

#[test]
fn concurrent_provisional_claims_cannot_spend_the_same_capacity() {
    let registry = registry();
    let a = request("a", 80);
    let b = request("b", 80);
    let expected = registry
        .evaluate_reservation(a.clone())
        .unwrap()
        .observation()
        .clone();
    let barrier = Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            registry.acquire_reservation_provisional(a, &expected, |_| Ok::<_, &str>(()))
        });
        let b = scope.spawn(|| {
            barrier.wait();
            registry.acquire_reservation_provisional(b, &expected, |_| Ok::<_, &str>(()))
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(registry.snapshot().reservations.len(), 1);
    drop(results);
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn explicit_release_before_transfer_ends_the_lease_without_reviving_its_predecessor() {
    let registry = registry();
    registry.acquire_reservation(request("owner", 80)).unwrap();
    let request = request("owner", 20);
    let expected = registry
        .evaluate_reservation(request.clone())
        .unwrap()
        .observation()
        .clone();
    let (lease_id, custody) = registry
        .acquire_reservation_provisional(request, &expected, |lease| {
            Ok::<_, &str>(lease.reservation_id)
        })
        .unwrap();
    registry.release_reservation(lease_id).unwrap();
    assert!(custody.transfer().is_err());
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn released_custody_cannot_remove_a_later_same_owner_lease() {
    let registry = registry();
    let requested = request("owner", 70);
    let expected = registry
        .evaluate_reservation(requested.clone())
        .unwrap()
        .observation()
        .clone();
    let (old_id, custody) = registry
        .acquire_reservation_provisional(requested, &expected, |lease| {
            Ok::<_, &str>(lease.reservation_id)
        })
        .unwrap();
    registry.release_reservation(old_id).unwrap();
    let next = registry.acquire_reservation(request("owner", 20)).unwrap();
    assert_ne!(old_id, next.reservation_id);
    drop(custody);
    assert_eq!(registry.snapshot().reservations.len(), 1);
    assert_eq!(
        registry.snapshot().reservations[0].reservation_id,
        next.reservation_id
    );
}

#[test]
fn reduced_budget_cannot_publish_a_replacement_that_still_holds_old_rollback_capacity() {
    let registry = registry();
    registry.acquire_reservation(request("owner", 80)).unwrap();
    let before = registry.snapshot().reservations;
    registry.register_runtime(
        RuntimeRegistration::new("pytorch", "PyTorch").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(50)),
            ]),
        ),
    );
    let request = request("owner", 20);
    let expected = registry
        .evaluate_reservation(request.clone())
        .unwrap()
        .observation()
        .clone();
    let result =
        registry.acquire_reservation_provisional(request, &expected, |_| -> Result<(), &str> {
            panic!("held capacity must be checked before publication")
        });
    assert!(matches!(
        result,
        Err(RuntimeReservationPublicationError::Registry(
            RuntimeRegistryError::AdmissionRejected { .. }
        ))
    ));
    assert_eq!(registry.snapshot().reservations, before);
}

#[test]
fn a_global_owner_conflict_after_evaluation_cannot_publish() {
    let registry = registry();
    let requested = request("owner", 40);
    let expected = registry
        .evaluate_reservation(requested.clone())
        .unwrap()
        .observation()
        .clone();
    registry.register_runtime(RuntimeRegistration::new("llamacpp", "llama.cpp"));
    let mut other = request("owner", 20);
    other.runtime_id = "llamacpp".into();
    registry.acquire_reservation(other).unwrap();
    let before = registry.snapshot().reservations;
    let result =
        registry.acquire_reservation_provisional(requested, &expected, |_| -> Result<(), &str> {
            panic!("owner conflict must precede publication")
        });
    assert!(matches!(
        result,
        Err(RuntimeReservationPublicationError::Registry(
            RuntimeRegistryError::ReservationOwnerConflict { .. }
        ))
    ));
    assert_eq!(registry.snapshot().reservations, before);
}

#[test]
fn unwind_while_preparation_owns_custody_rolls_back() {
    let registry = registry();
    let result = std::panic::catch_unwind(|| {
        let request = request("owner", 70);
        let expected = registry
            .evaluate_reservation(request.clone())
            .unwrap()
            .observation()
            .clone();
        let (_, _custody) = registry
            .acquire_reservation_provisional(request, &expected, |_| Ok::<_, &str>(()))
            .unwrap();
        panic!("controlled preparation panic");
    });
    assert!(result.is_err());
    assert!(registry.snapshot().reservations.is_empty());
}
