//! Independent controlled-owner tests. No Python, native import or real Pumas lease.
use super::*;
use crate::{CapabilityAvailabilityId, RuntimeVariantId};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Barrier, Weak,
};

fn declaration() -> ManagedPythonRuntimeStartRequest {
    ManagedPythonRuntimeStartRequest {
        contract_version: 1,
        environment_id: CapabilityAvailabilityId::parse("pumas.torch.cpu.reviewed").unwrap(),
        runtime_revision: "reviewed-local-revision".into(),
        runtime_manifest_sha256: "0123456789abcdef".repeat(4),
        runtime_variant_id: RuntimeVariantId::parse("pytorch.cpu").unwrap(),
        provider_wheel_sha256: "678b155145bb06c271ad6d8eb2df95a8a8173155323fafd4b102e50349940b95"
            .into(),
        provider_extension_sha256:
            "5a7eacfad202bcacf122716f5a45e9796d098c8e65f86a56536efddceb573423".into(),
        provider_build_binding_sha256:
            "b2a4fc2a43b6ae5631b2f5ec45deb5394cd2059e456f97213a87d1bc5d1f703a".into(),
    }
}

struct DropProbe {
    broker: Weak<PythonStartupBroker>,
    drops: Arc<AtomicUsize>,
    outside_lock: Arc<AtomicBool>,
}
impl Drop for DropProbe {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        if let Some(broker) = self.broker.upgrade() {
            // Never block a failing test. A free guard proves destruction is
            // outside the mutex; then exercise actual public re-entry.
            let unlocked = broker.0.try_lock().is_ok();
            self.outside_lock.store(unlocked, Ordering::SeqCst);
            if unlocked {
                assert!(broker.phase().is_ok());
            }
        }
    }
}
fn probe(broker: &Arc<PythonStartupBroker>) -> (Arc<DropProbe>, Arc<AtomicUsize>, Arc<AtomicBool>) {
    let drops = Arc::new(AtomicUsize::new(0));
    let unlocked = Arc::new(AtomicBool::new(false));
    (
        Arc::new(DropProbe {
            broker: Arc::downgrade(broker),
            drops: drops.clone(),
            outside_lock: unlocked.clone(),
        }),
        drops,
        unlocked,
    )
}

#[test]
fn startup_invalid_declaration_does_not_reserve_or_query_and_drops_outside_lock() {
    let broker = Arc::new(PythonStartupBroker::default());
    let mut invalid = declaration();
    invalid.provider_wheel_sha256 = "0".repeat(64);
    let (held, drops, unlocked) = probe(&broker);
    assert_eq!(
        broker.reserve_declared(&invalid, held).err(),
        Some(PythonStartupReservationRefusal::InvalidDeclaration)
    );
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
    assert_eq!(broker.0.lock().unwrap().next_generation, 0);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(unlocked.load(Ordering::SeqCst));
    assert_eq!(
        broker.closed_start_refusal(&invalid, || panic!("invalid metadata queried")),
        ManagedPythonBindingRefusal::InvalidStartRequest
    );
}

#[test]
fn startup_reservation_keeps_exact_selection_and_rolls_back_before_exposure() {
    let broker = Arc::new(PythonStartupBroker::default());
    let expected = declaration();
    let (held, drops, unlocked) = probe(&broker);
    let reservation = broker.reserve_declared(&expected, held).unwrap();
    assert_eq!(reservation.declared_selection().unwrap(), expected);
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::ManagedReserved);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        broker.claim_legacy(),
        Err(PythonStartupReservationRefusal::RegistrationBusy)
    );
    assert_eq!(
        broker.reserve_declared(&expected, Arc::new(())).err(),
        Some(PythonStartupReservationRefusal::RegistrationBusy)
    );
    drop(reservation);
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(unlocked.load(Ordering::SeqCst));
    broker.claim_legacy().unwrap();
}

#[test]
fn startup_legacy_claim_is_idempotent_and_rejects_declared_adoption() {
    let broker = Arc::new(PythonStartupBroker::default());
    broker.claim_legacy().unwrap();
    broker.claim_legacy().unwrap();
    let (held, drops, unlocked) = probe(&broker);
    assert_eq!(
        broker.reserve_declared(&declaration(), held).err(),
        Some(PythonStartupReservationRefusal::LegacyClaimed)
    );
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(unlocked.load(Ordering::SeqCst));
}

#[test]
fn startup_process_pin_survives_token_consumption_and_blocks_every_replacement() {
    let broker = Arc::new(PythonStartupBroker::default());
    let held = Arc::new(42_u64);
    let weak = Arc::downgrade(&held);
    let reservation = broker.reserve_declared(&declaration(), held).unwrap();
    reservation.pin_for_process().unwrap();
    assert!(weak.upgrade().is_some());
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::ProcessPinned);
    assert_eq!(
        broker.claim_legacy(),
        Err(PythonStartupReservationRefusal::RegistrationBusy)
    );
    assert_eq!(
        broker.reserve_declared(&declaration(), Arc::new(())).err(),
        Some(PythonStartupReservationRefusal::RegistrationBusy)
    );
    // Fresh controlled-owner destruction is not actual process retirement.
    drop(broker);
    assert!(weak.upgrade().is_none());
}

#[test]
fn startup_all_four_phases_and_both_worker_flags_remain_closed_without_lock_callback() {
    for phase in [
        PythonStartupPhase::Unclaimed,
        PythonStartupPhase::LegacyClaimed,
        PythonStartupPhase::ManagedReserved,
        PythonStartupPhase::ProcessPinned,
    ] {
        let broker = Arc::new(PythonStartupBroker::default());
        let reservation = match phase {
            PythonStartupPhase::Unclaimed => None,
            PythonStartupPhase::LegacyClaimed => {
                broker.claim_legacy().unwrap();
                None
            }
            PythonStartupPhase::ManagedReserved => Some(
                broker
                    .reserve_declared(&declaration(), Arc::new(()))
                    .unwrap(),
            ),
            PythonStartupPhase::ProcessPinned => {
                broker
                    .reserve_declared(&declaration(), Arc::new(()))
                    .unwrap()
                    .pin_for_process()
                    .unwrap();
                None
            }
        };
        for flag in [false, true] {
            let calls = AtomicUsize::new(0);
            assert_eq!(
                broker.closed_start_refusal(&declaration(), || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(broker.phase().unwrap(), phase);
                    flag
                }),
                if flag {
                    ManagedPythonBindingRefusal::LegacyWorkerAlreadyInitialized
                } else {
                    ManagedPythonBindingRefusal::RegisteredOwnerCustodyUnavailable
                }
            );
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert_eq!(broker.phase().unwrap(), phase);
        }
        drop(reservation);
    }
}

#[test]
fn startup_checked_generation_exhaustion_keeps_state_and_releases_refused_hold() {
    let broker = Arc::new(PythonStartupBroker::default());
    broker.0.lock().unwrap().next_generation = u64::MAX - 1;
    let last = broker
        .reserve_declared(&declaration(), Arc::new(()))
        .unwrap();
    assert_eq!(last.generation, u64::MAX);
    drop(last);
    let (held, drops, unlocked) = probe(&broker);
    assert_eq!(
        broker.reserve_declared(&declaration(), held).err(),
        Some(PythonStartupReservationRefusal::GenerationExhausted)
    );
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
    assert_eq!(broker.0.lock().unwrap().next_generation, u64::MAX);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(unlocked.load(Ordering::SeqCst));
}

#[test]
fn startup_stale_generation_cannot_pin_or_rollback_current_reservation() {
    let broker = Arc::new(PythonStartupBroker::default());
    let old = broker
        .reserve_declared(&declaration(), Arc::new(()))
        .unwrap();
    let old_generation = old.generation;
    drop(old);
    let current = broker
        .reserve_declared(&declaration(), Arc::new(()))
        .unwrap();
    assert_ne!(old_generation, current.generation);
    // Controlled private stale token; production fields have no constructor.
    let stale = PythonStartupReservation {
        broker: broker.clone(),
        generation: old_generation,
    };
    assert_eq!(
        stale.declared_selection().err(),
        Some(PythonStartupReservationRefusal::RegistrationBusy)
    );
    assert_eq!(
        stale.pin_for_process(),
        Err(PythonStartupReservationRefusal::RegistrationBusy)
    );
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::ManagedReserved);
    assert_eq!(current.declared_selection().unwrap(), declaration());
    drop(current);
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::Unclaimed);
}

#[test]
fn startup_poison_preserves_unknown_hold_and_never_observes_worker() {
    let broker = Arc::new(PythonStartupBroker::default());
    let held = Arc::new(7_u64);
    let weak = Arc::downgrade(&held);
    let reservation = broker.reserve_declared(&declaration(), held).unwrap();
    let other = broker.clone();
    assert!(std::thread::spawn(move || {
        let _guard = other.0.lock().unwrap();
        panic!("controlled poison");
    })
    .join()
    .is_err());
    drop(reservation);
    assert!(weak.upgrade().is_some());
    assert_eq!(
        broker.phase(),
        Err(PythonStartupReservationRefusal::StatePoisoned)
    );
    assert_eq!(
        broker.claim_legacy(),
        Err(PythonStartupReservationRefusal::StatePoisoned)
    );
    assert_eq!(
        broker.reserve_declared(&declaration(), Arc::new(())).err(),
        Some(PythonStartupReservationRefusal::StatePoisoned)
    );
    assert_eq!(
        broker.closed_start_refusal(&declaration(), || panic!("poison queried worker")),
        ManagedPythonBindingRefusal::RegisteredOwnerCustodyUnavailable
    );
    drop(broker);
    assert!(weak.upgrade().is_none());
}

#[test]
fn startup_two_brokers_keep_independent_controlled_owner_custody() {
    let a = Arc::new(PythonStartupBroker::default());
    let b = Arc::new(PythonStartupBroker::default());
    let reservation = a.reserve_declared(&declaration(), Arc::new(())).unwrap();
    b.claim_legacy().unwrap();
    reservation.pin_for_process().unwrap();
    assert_eq!(a.phase().unwrap(), PythonStartupPhase::ProcessPinned);
    assert_eq!(b.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
}

#[test]
fn startup_competing_legacy_and_reservation_choose_one_owner_category() {
    let broker = Arc::new(PythonStartupBroker::default());
    let barrier = Arc::new(Barrier::new(3));
    let a = broker.clone();
    let start_a = barrier.clone();
    let reserved = std::thread::spawn(move || {
        start_a.wait();
        a.reserve_declared(&declaration(), Arc::new(()))
    });
    let b = broker.clone();
    let start_b = barrier.clone();
    let legacy = std::thread::spawn(move || {
        start_b.wait();
        b.claim_legacy()
    });
    barrier.wait();
    let held = reserved.join().unwrap();
    let legacy = legacy.join().unwrap();
    match (held, legacy) {
        (Ok(reservation), Err(PythonStartupReservationRefusal::RegistrationBusy)) => {
            assert_eq!(broker.phase().unwrap(), PythonStartupPhase::ManagedReserved);
            drop(reservation);
        }
        (Err(PythonStartupReservationRefusal::LegacyClaimed), Ok(())) => {
            assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed)
        }
        _ => panic!("competing categories both admitted or neither explains exclusion"),
    }
}
