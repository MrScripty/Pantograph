use super::*;
use crate::*;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc, Barrier,
};
#[derive(Debug)]
struct Ceiling(AtomicU64);
impl RuntimeHostRamCapacitySource for Ceiling {
    fn capacity_ceiling_bytes(&self) -> Option<u64> {
        match self.0.load(Ordering::Acquire) {
            u64::MAX => None,
            n => Some(n),
        }
    }
}
fn owner() -> RuntimeRetainedOwnerIdentity<'static> {
    RuntimeRetainedOwnerIdentity {
        runtime_id: "candle",
        source_id: "producer",
        runtime_instance_id: "instance",
        model_target: "model",
    }
}
fn request(id: &str, keep: bool) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(id.into()),
        usage_profile: None,
        model_id: Some("model".into()),
        pin_runtime: false,
        retention_hint: if keep {
            RuntimeRetentionHint::KeepAlive
        } else {
            RuntimeRetentionHint::Ephemeral
        },
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(10),
        ])),
    }
}
fn fixture() -> (
    Arc<RuntimeRegistry>,
    RuntimeReservationLease,
    RuntimeReservationLease,
    Arc<Ceiling>,
) {
    let r = Arc::new(RuntimeRegistry::new());
    r.configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
        runtime_id: "candle".into(),
        model_id: "model".into(),
        requirements: RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(100),
        ]),
    }])
    .unwrap();
    r.configure_resource_domain(RuntimeResourceDomain {
        domain_id: "host".into(),
        total_bytes: 10000,
        safety_margin_bytes: 0,
        bindings: vec![RuntimeResourceDomainBinding {
            runtime_id: "candle".into(),
            resource_kind: RuntimeAdmissionResourceKind::RamBytes,
        }],
    })
    .unwrap();
    let c = Arc::new(Ceiling(AtomicU64::new(10000)));
    r.bind_host_ram_capacity_source(c.clone());
    let task = r.acquire_reservation(request("task", false)).unwrap();
    let session = r.acquire_reservation(request("session", true)).unwrap();
    r.observe_runtime_producer(RuntimeProducerObservation {
        source_id: "producer".into(),
        sequence: 1,
        allocation_state: RuntimeProducerAllocationState::Resident,
        observation: RuntimeObservation {
            runtime_id: "candle".into(),
            display_name: "Candle".into(),
            backend_keys: vec!["candle".into()],
            model_id: Some("model".into()),
            runtime_instance_id: Some("instance".into()),
            status: RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    })
    .unwrap();
    (r, task, session, c)
}
fn envelope(
    r: &Arc<RuntimeRegistry>,
    task: &RuntimeReservationLease,
    session: &RuntimeReservationLease,
) -> RuntimeRetainedExecutionEnvelope {
    r.acquire_execution_custody(task)
        .unwrap()
        .protect_retained_envelope(session, owner())
        .unwrap()
}
#[test]
fn declared_envelope_pins_exact_named_session_and_settles_only_unstarted_task() {
    let (r, t, s, _) = fixture();
    let e = envelope(&r, &t, &s);
    let before = r.snapshot();
    assert!(r.release_reservation(t.reservation_id).is_err());
    assert!(r.release_reservation(s.reservation_id).is_err());
    assert!(r
        .update_reservation_retention_hint(s.reservation_id, RuntimeRetentionHint::Ephemeral)
        .is_err());
    assert!(r.acquire_reservation(request("session", true)).is_err());
    let settled = e.settle_unstarted(owner()).unwrap();
    assert_eq!(settled.task(), &t);
    assert_eq!(settled.successor(), &s);
    assert!(r.reservation_lease(t.reservation_id).is_none());
    assert_eq!(r.reservation_lease(s.reservation_id), Some(s.clone()));
    assert_eq!(
        before.runtimes[0].model_resource_residency,
        r.snapshot().runtimes[0].model_resource_residency
    );
    drop(r.acquire_execution_custody(&s).unwrap());
}
#[test]
fn declared_envelope_capacity_loss_refuses_before_start_callback_and_recovers_without_reset() {
    let (r, t, s, c) = fixture();
    let mut e = envelope(&r, &t, &s);
    let calls = AtomicUsize::new(0);
    for ceiling in [119, u64::MAX] {
        c.0.store(ceiling, Ordering::Release);
        assert!(e
            .authorize_start(owner(), || {
                calls.fetch_add(1, Ordering::Relaxed);
                true
            })
            .is_err());
        assert!(!e.started());
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
    c.0.store(120, Ordering::Release);
    assert!(e
        .authorize_start(owner(), || {
            calls.fetch_add(1, Ordering::Relaxed);
            true
        })
        .unwrap());
    assert!(!e
        .authorize_start(owner(), || {
            calls.fetch_add(1, Ordering::Relaxed);
            true
        })
        .unwrap());
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(e.settle_unstarted(owner()).is_err()); // Consumed uncertain guard stays fenced.
    assert!(r.acquire_execution_custody(&t).is_err());
    assert!(r.acquire_execution_custody(&s).is_err());
}
#[test]
fn declared_envelope_competing_preparations_have_one_owner() {
    for _ in 0..32 {
        let (r, t, s, _) = fixture();
        let other = r.acquire_reservation(request("other", false)).unwrap();
        let start = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for task in [t.clone(), other.clone()] {
            let r = r.clone();
            let s = s.clone();
            let b = start.clone();
            threads.push(std::thread::spawn(move || {
                let custody = r.acquire_execution_custody(&task).unwrap();
                b.wait();
                custody.protect_retained_envelope(&s, owner()).ok()
            }));
        }
        start.wait();
        let results: Vec<_> = threads.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|x| x.is_some()).count(), 1);
        drop(results);
        assert_eq!(r.reservation_lease(t.reservation_id), Some(t));
        assert_eq!(r.reservation_lease(other.reservation_id), Some(other));
        assert_eq!(r.reservation_lease(s.reservation_id), Some(s));
    }
}
#[test]
fn declared_envelope_changed_local_budget_refuses_callback_but_unstarted_settlement_is_real() {
    let (r, t, s, _) = fixture();
    let mut e = envelope(&r, &t, &s);
    r.register_runtime(
        RuntimeRegistration::new("candle", "Candle").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(119)),
            ]),
        ),
    );
    let called = AtomicUsize::new(0);
    assert!(e
        .authorize_start(owner(), || {
            called.fetch_add(1, Ordering::Relaxed);
            true
        })
        .is_err());
    assert_eq!(called.load(Ordering::Relaxed), 0);
    let proof = e.settle_unstarted(owner()).unwrap();
    assert_eq!(proof.task(), &t);
    assert!(r.reservation_lease(t.reservation_id).is_none());
    assert_eq!(r.reservation_lease(s.reservation_id), Some(s));
}
#[test]
fn declared_envelope_owner_release_or_replacement_preserves_resident_charge_as_uncertain() {
    for replacement in [false, true] {
        let (r, t, s, _) = fixture();
        let mut e = envelope(&r, &t, &s);
        assert!(e.authorize_start(owner(), || true).unwrap());
        let resident = r.snapshot().runtimes[0].model_resource_residency.clone();
        r.observe_runtime_producer(RuntimeProducerObservation {
            source_id: "producer".into(),
            sequence: 2,
            allocation_state: if replacement {
                RuntimeProducerAllocationState::Resident
            } else {
                RuntimeProducerAllocationState::Released
            },
            observation: RuntimeObservation {
                runtime_id: "candle".into(),
                display_name: "Candle".into(),
                backend_keys: vec!["candle".into()],
                model_id: replacement.then(|| "replacement".into()),
                runtime_instance_id: replacement.then(|| "replacement-instance".into()),
                status: if replacement {
                    RuntimeRegistryStatus::Ready
                } else {
                    RuntimeRegistryStatus::Stopped
                },
                last_error: None,
            },
        })
        .unwrap();
        assert_eq!(r.snapshot().runtimes[0].model_resource_residency, resident);
        assert!(r.snapshot().runtimes[0].resident_resources_uncertain);
        assert!(r.acquire_reservation(request("new", false)).is_err());
        assert!(e.release_drained(owner()).is_err());
        assert!(r.release_reservation(t.reservation_id).is_err());
        assert!(r.release_reservation(s.reservation_id).is_err());
    }
}
#[test]
fn declared_envelope_bounds_and_unknown_successor_kind_refuse_without_erasing_claims() {
    let (r, t, s, _) = fixture();
    let mut big = request("huge", false);
    big.reservation_owner_id = Some("x".repeat(129));
    let big = r.acquire_reservation(big).unwrap();
    assert!(r
        .acquire_execution_custody(&t)
        .unwrap()
        .protect_retained_envelope(&s, owner())
        .is_err());
    r.release_reservation(big.reservation_id).unwrap();
    let mut unknown = request("vram", false);
    unknown
        .requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::vram_bytes(1));
    let unknown = r.acquire_reservation(unknown).unwrap();
    assert!(r
        .acquire_execution_custody(&t)
        .unwrap()
        .protect_retained_envelope(&s, owner())
        .is_err());
    r.release_reservation(unknown.reservation_id).unwrap();
    for i in 0..63 {
        let mut q = request(&format!("extra{i}"), false);
        q.requirements = Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(0),
        ]));
        r.acquire_reservation(q).unwrap();
    }
    assert!(r
        .acquire_execution_custody(&t)
        .unwrap()
        .protect_retained_envelope(&s, owner())
        .is_err());
    assert_eq!(r.reservation_lease(t.reservation_id), Some(t));
    assert_eq!(r.reservation_lease(s.reservation_id), Some(s));
}

#[test]
fn declared_envelope_exact_retaining_vram_requires_known_resident_in_local_only_ledger() {
    let r = Arc::new(RuntimeRegistry::new());
    r.register_runtime(
        RuntimeRegistration::new("candle", "Candle").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(10000)),
                RuntimeAdmissionResourceBudget::vram_bytes(Some(10000)),
            ]),
        ),
    );
    r.configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
        runtime_id: "candle".into(),
        model_id: "model".into(),
        requirements: RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(100),
        ]),
    }])
    .unwrap();
    let t = r.acquire_reservation(request("task", false)).unwrap();
    let mut retaining = request("session", true);
    retaining
        .requirements
        .as_mut()
        .unwrap()
        .claims
        .push(RuntimeReservationResourceClaim::vram_bytes(1));
    let s = r.acquire_reservation(retaining).unwrap();
    r.observe_runtime_producer(RuntimeProducerObservation {
        source_id: "producer".into(),
        sequence: 1,
        allocation_state: RuntimeProducerAllocationState::Resident,
        observation: RuntimeObservation {
            runtime_id: "candle".into(),
            display_name: "Candle".into(),
            backend_keys: vec!["candle".into()],
            model_id: Some("model".into()),
            runtime_instance_id: Some("instance".into()),
            status: RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    })
    .unwrap();
    let before = r.snapshot();
    let failure = r
        .acquire_execution_custody(&t)
        .unwrap()
        .protect_retained_envelope(&s, owner());
    assert!(matches!(
        failure,
        Err(RuntimeRetainedCleanupError::Refused {
            reason: RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
            ..
        })
    ));
    assert_eq!(r.snapshot().reservations, before.reservations);
    assert_eq!(
        r.snapshot().runtimes[0].model_resource_residency,
        before.runtimes[0].model_resource_residency
    );
    drop(r.acquire_execution_custody(&t).unwrap());
    drop(r.acquire_execution_custody(&s).unwrap());
}
#[test]
fn declared_envelope_post_start_capacity_loss_preserves_both_fences() {
    for limit in [119, u64::MAX] {
        let (r, t, s, c) = fixture();
        let mut e = envelope(&r, &t, &s);
        assert!(e.authorize_start(owner(), || true).unwrap());
        c.0.store(limit, Ordering::Release);
        assert!(e.validate_retained_owner(owner()).is_err());
        assert!(e.release_drained(owner()).is_err());
        assert_eq!(r.reservation_lease(t.reservation_id), Some(t.clone()));
        assert_eq!(r.reservation_lease(s.reservation_id), Some(s.clone()));
        assert!(r.acquire_execution_custody(&t).is_err());
        assert!(r.acquire_execution_custody(&s).is_err());
    }
}
#[test]
fn declared_envelope_oversize_diagnostic_and_owner_are_bounded() {
    let err = refused(
        &"x".repeat(129),
        RuntimeRetainedCleanupRefusal::OwnerMismatch,
    );
    assert!(
        matches!(err, RuntimeRetainedCleanupError::Refused { runtime_id, .. } if runtime_id == "retained-envelope")
    );
    let (r, t, s, _) = fixture();
    let huge = "x".repeat(4097);
    let mut bad = owner();
    bad.model_target = &huge;
    assert!(r
        .acquire_execution_custody(&t)
        .unwrap()
        .protect_retained_envelope(&s, bad)
        .is_err());
    let e = envelope(&r, &t, &s);
    let mut bad = owner();
    bad.source_id = &huge;
    assert!(e.validate_retained_owner(bad).is_err());
    drop(e);
    assert_eq!(r.reservation_lease(t.reservation_id), Some(t.clone()));
    assert_eq!(r.reservation_lease(s.reservation_id), Some(s.clone()));
    // Legacy declaration accepts this ID; the bounded opt-in path must refuse
    // before a shared-domain error/observation can copy oversized metadata.
    r.configure_resource_domain(RuntimeResourceDomain {
        domain_id: "d".repeat(129),
        total_bytes: 10000,
        safety_margin_bytes: 0,
        bindings: vec![RuntimeResourceDomainBinding {
            runtime_id: "candle".into(),
            resource_kind: RuntimeAdmissionResourceKind::VramBytes,
        }],
    })
    .unwrap();
    assert!(r
        .acquire_execution_custody(&t)
        .unwrap()
        .protect_retained_envelope(&s, owner())
        .is_err());
    assert_eq!(r.reservation_lease(t.reservation_id), Some(t));
    assert_eq!(r.reservation_lease(s.reservation_id), Some(s));
}
