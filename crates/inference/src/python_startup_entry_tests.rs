//! Independent Rust-only admission/exposure tests. No GIL, import, model or
//! native cleanup is exercised; the exposure record is the production worker's.
use super::*;
use crate::{CapabilityAvailabilityId, RuntimeVariantId};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Barrier};
use std::time::Duration;

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

fn poison(broker: &PythonStartupBroker) {
    assert!(catch_unwind(AssertUnwindSafe(|| {
        let _guard = broker.0.lock().unwrap();
        panic!("controlled broker poison");
    }))
    .is_err());
}

#[test]
fn entry_refused_reserved_pinned_and_poisoned_states_do_not_enter_effect() {
    for phase in 0..3 {
        let broker = Arc::new(PythonStartupBroker::default());
        let reservation = match phase {
            0 => Some(
                broker
                    .reserve_declared(&declaration(), Arc::new(()))
                    .unwrap(),
            ),
            1 => {
                broker
                    .reserve_declared(&declaration(), Arc::new(()))
                    .unwrap()
                    .pin_for_process()
                    .unwrap();
                None
            }
            _ => {
                poison(&broker);
                None
            }
        };
        let record = PythonLegacyExposure::default();
        let effects = AtomicUsize::new(0);
        let result = record.prepare(&broker).map(|_admission| {
            effects.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(
            result.err(),
            Some(if phase == 2 {
                PythonStartupReservationRefusal::StatePoisoned
            } else {
                PythonStartupReservationRefusal::RegistrationBusy
            })
        );
        assert_eq!(effects.load(Ordering::SeqCst), 0);
        assert!(record.cleanup_admission().is_none());
        drop(reservation);
    }
}

#[test]
fn entry_admission_is_sticky_and_never_opens_managed_start() {
    let broker = Arc::new(PythonStartupBroker::default());
    let token = broker.admit_legacy().unwrap();
    assert!(Arc::ptr_eq(&token._broker, &broker));
    drop(token.clone());
    drop(token);
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
    assert_eq!(
        broker.reserve_declared(&declaration(), Arc::new(())).err(),
        Some(PythonStartupReservationRefusal::LegacyClaimed)
    );
    for initialized in [false, true] {
        assert_eq!(
            broker.closed_start_refusal(&declaration(), || initialized),
            if initialized {
                ManagedPythonBindingRefusal::LegacyWorkerAlreadyInitialized
            } else {
                ManagedPythonBindingRefusal::RegisteredOwnerCustodyUnavailable
            }
        );
    }
}

#[test]
fn entry_prepared_but_never_exposed_has_no_cleanup_admission() {
    let broker = Arc::new(PythonStartupBroker::default());
    let record = PythonLegacyExposure::default();
    drop(record.prepare(&broker).unwrap());
    assert!(record.cleanup_admission().is_none());
    drop(record);
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
}

#[test]
fn entry_exposure_without_admission_cannot_create_cleanup_capability() {
    let record = PythonLegacyExposure::default();
    assert!(catch_unwind(AssertUnwindSafe(|| record.mark_exposed())).is_err());
    assert!(record.cleanup_admission().is_none());
}

#[test]
fn entry_partial_initialization_failure_keeps_original_cleanup_without_lock() {
    let broker = Arc::new(PythonStartupBroker::default());
    let record = PythonLegacyExposure::default();
    let token = record.prepare(&broker).unwrap();
    let initialization: Result<(), &'static str> = {
        record.mark_exposed();
        Err("failure after potential partial import")
    };
    assert!(initialization.is_err());
    let cleanup = record.cleanup_admission().unwrap();
    assert!(Arc::ptr_eq(&cleanup._broker, &token._broker));
    let _free_guard = broker
        .0
        .try_lock()
        .expect("cleanup record holds no broker guard");
}

#[test]
fn entry_cleanup_after_poison_uses_original_token_without_readmission() {
    let broker = Arc::new(PythonStartupBroker::default());
    let record = PythonLegacyExposure::default();
    drop(record.prepare(&broker).unwrap());
    record.mark_exposed();
    poison(&broker);
    assert_eq!(
        broker.admit_legacy().err(),
        Some(PythonStartupReservationRefusal::StatePoisoned)
    );
    let original = record.cleanup_admission().unwrap();
    assert!(Arc::ptr_eq(&original._broker, &broker));
    // Poisoned is distinct from WouldBlock: the original cleanup has no mutex
    // held and requires no successful state query. No Python callback is run.
    assert!(matches!(
        original._broker.0.try_lock(),
        Err(std::sync::TryLockError::Poisoned(_))
    ));
    let retry = record.prepare(&broker).unwrap();
    assert!(Arc::ptr_eq(&retry._broker, &original._broker));
}

#[test]
fn entry_caller_loss_retains_actual_broker_until_controlled_job_drains() {
    let broker = Arc::new(PythonStartupBroker::default());
    let weak_broker = Arc::downgrade(&broker);
    let record = Arc::new(PythonLegacyExposure::default());
    drop(record.prepare(&broker).unwrap());
    let job_record = record.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (drain_tx, drain_rx) = mpsc::channel();
    let job = std::thread::spawn(move || {
        job_record.mark_exposed();
        entered_tx.send(()).unwrap();
        drain_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(job_record.cleanup_admission().is_some());
        drop(job_record);
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(record);
    drop(broker);
    assert!(weak_broker.upgrade().is_some());
    drain_tx.send(()).unwrap();
    job.join().unwrap();
    assert!(weak_broker.upgrade().is_none());
}

#[test]
fn entry_concurrent_preparation_keeps_one_original_same_broker_record() {
    let broker = Arc::new(PythonStartupBroker::default());
    let record = Arc::new(PythonLegacyExposure::default());
    let barrier = Arc::new(Barrier::new(3));
    let jobs: Vec<_> = (0..2)
        .map(|_| {
            let broker = broker.clone();
            let record = record.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let admission = record.prepare(&broker).unwrap();
                assert!(Arc::ptr_eq(&admission._broker, &broker));
                record.mark_exposed();
            })
        })
        .collect();
    barrier.wait();
    for job in jobs {
        job.join().unwrap();
    }
    assert!(Arc::ptr_eq(
        &record.cleanup_admission().unwrap()._broker,
        &broker
    ));
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
}

#[test]
fn entry_competing_managed_and_legacy_admissions_have_one_winner() {
    let broker = Arc::new(PythonStartupBroker::default());
    let barrier = Arc::new(Barrier::new(3));
    let (release_tx, release_rx) = mpsc::channel();
    let managed_broker = broker.clone();
    let managed_barrier = barrier.clone();
    let managed = std::thread::spawn(move || {
        managed_barrier.wait();
        let reservation = managed_broker.reserve_declared(&declaration(), Arc::new(()));
        let won = reservation.is_ok();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(reservation);
        won
    });
    let legacy_broker = broker.clone();
    let legacy_barrier = barrier.clone();
    let legacy = std::thread::spawn(move || {
        legacy_barrier.wait();
        legacy_broker.admit_legacy().is_ok()
    });
    barrier.wait();
    let legacy_won = legacy.join().unwrap();
    release_tx.send(()).unwrap();
    let managed_won = managed.join().unwrap();
    assert_ne!(legacy_won, managed_won);
    assert_eq!(
        broker.phase().unwrap(),
        if legacy_won {
            PythonStartupPhase::LegacyClaimed
        } else {
            PythonStartupPhase::Unclaimed
        }
    );
}

#[test]
fn entry_panicked_initializer_does_not_erase_exposure_or_admission() {
    let broker = Arc::new(PythonStartupBroker::default());
    let record = PythonLegacyExposure::default();
    drop(record.prepare(&broker).unwrap());
    assert!(catch_unwind(AssertUnwindSafe(|| {
        record.mark_exposed();
        panic!("controlled initializer fails after exposure");
    }))
    .is_err());
    assert!(record.cleanup_admission().is_some());
    assert_eq!(broker.phase().unwrap(), PythonStartupPhase::LegacyClaimed);
    assert!(broker.0.try_lock().is_ok());
}

#[test]
fn entry_inherited_original_admission_survives_poison_without_implying_exposure() {
    // Exercise the exact const constructor used by the process-owned record.
    let process_record = PythonLegacyExposure::new();
    let broker = Arc::new(PythonStartupBroker::default());
    let original = process_record.prepare(&broker).unwrap();
    poison(&broker);
    assert_eq!(
        broker.admit_legacy().err(),
        Some(PythonStartupReservationRefusal::StatePoisoned)
    );
    let recovered = process_record.prepare(&broker).unwrap();
    assert!(Arc::ptr_eq(&recovered._broker, &original._broker));
    let private_record = PythonLegacyExposure::default();
    let inherited = private_record.retain_admission(&recovered).unwrap();
    assert!(Arc::ptr_eq(&inherited._broker, &broker));
    assert!(process_record.cleanup_admission().is_none());
    assert!(private_record.cleanup_admission().is_none());
    // Preparing custody before caller cancellation does not cause Python entry;
    // only an actually entered effect marks potential partial initialization.
    private_record.mark_exposed();
    assert!(Arc::ptr_eq(
        &private_record.cleanup_admission().unwrap()._broker,
        &original._broker
    ));
    assert!(process_record.cleanup_admission().is_none());
}

#[test]
fn entry_wrong_broker_prepare_or_inheritance_never_replaces_original_record() {
    let first = Arc::new(PythonStartupBroker::default());
    let other = Arc::new(PythonStartupBroker::default());
    let record = PythonLegacyExposure::default();
    let original = record.prepare(&first).unwrap();
    assert_eq!(
        record.prepare(&other).err(),
        Some(PythonStartupReservationRefusal::RegistrationBusy)
    );
    assert_eq!(other.phase().unwrap(), PythonStartupPhase::Unclaimed);
    let foreign = other.admit_legacy().unwrap();
    assert_eq!(
        record.retain_admission(&foreign).err(),
        Some(PythonStartupReservationRefusal::RegistrationBusy)
    );
    assert!(record.cleanup_admission().is_none());
    record.mark_exposed();
    assert!(Arc::ptr_eq(
        &record.cleanup_admission().unwrap()._broker,
        &original._broker
    ));
    assert!(!Arc::ptr_eq(
        &record.cleanup_admission().unwrap()._broker,
        &other
    ));
}
