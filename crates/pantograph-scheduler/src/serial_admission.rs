//! Local serial admission for an explicitly opted-in dispatcher.
//!
//! This is an ownership contract, not a scheduler/runtime adapter. Construct it
//! once with the execution owner and share clones with every dispatch route.
//! Busy callers must defer before taking any event, assignment or resource claim.
//! The trusted adapter is responsible for promoting before its first claim and
//! acknowledging only actual host drain plus matched reservation cleanup.
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const IDLE: u8 = 0;
const PREPARING: u8 = 1;
const DISPATCHING: u8 = 2;
const POISONED: u8 = 3;

/// Immutable, shared serial mode. There is deliberately no reset operation:
/// recovery cannot infer runtime release from a terminal assignment record.
#[derive(Debug, Clone)]
pub struct SchedulerSerialAdmission {
    state: Arc<AtomicU8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerSerialAdmissionRefusal {
    Busy,
    Poisoned,
}

/// Owns exclusion before the first event/assignment/reservation mutation.
/// Dropping preparation releases exclusion because no execution was admitted.
#[derive(Debug)]
#[must_use]
pub struct SchedulerSerialPreparation {
    state: Arc<AtomicU8>,
}

/// Owns exclusion from the first claim through host drain and lifecycle cleanup.
/// Abandonment permanently poisons every clone, including after a panic.
#[derive(Debug)]
#[must_use]
pub struct SchedulerSerialDispatch {
    state: Arc<AtomicU8>,
    cleanup_acknowledged: bool,
}

impl Default for SchedulerSerialAdmission {
    fn default() -> Self {
        Self::new()
    }
}

impl SchedulerSerialAdmission {
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(IDLE)),
        }
    }

    /// Fixed work, no waiting, locks, callback, or runtime access. Preparation
    /// consumes the same shared slot as execution; there is no admission race
    /// between snapshot creation and the first ownership mutation.
    pub fn try_prepare(
        &self,
    ) -> Result<SchedulerSerialPreparation, SchedulerSerialAdmissionRefusal> {
        match self
            .state
            .compare_exchange(IDLE, PREPARING, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(SchedulerSerialPreparation {
                state: self.state.clone(),
            }),
            Err(POISONED) => Err(SchedulerSerialAdmissionRefusal::Poisoned),
            Err(_) => Err(SchedulerSerialAdmissionRefusal::Busy),
        }
    }

    pub fn is_poisoned(&self) -> bool {
        self.state.load(Ordering::Acquire) == POISONED
    }
}

impl SchedulerSerialPreparation {
    /// Call BEFORE the first event/assignment/reservation claim, not merely
    /// before the host call. Any subsequent unwind/drop requires refusal until
    /// the execution owner is reconstructed with independently verified cleanup.
    pub fn begin_dispatch(self) -> SchedulerSerialDispatch {
        self.state.store(DISPATCHING, Ordering::Release);
        SchedulerSerialDispatch {
            state: self.state.clone(),
            cleanup_acknowledged: false,
        }
    }
}

impl Drop for SchedulerSerialPreparation {
    fn drop(&mut self) {
        // Promotion has already moved the slot to DISPATCHING. Never release a
        // promoted owner's exclusion when the preparation value is consumed.
        let _ = self
            .state
            .compare_exchange(PREPARING, IDLE, Ordering::AcqRel, Ordering::Acquire);
    }
}

impl SchedulerSerialDispatch {
    /// Trusted dispatch boundary only: the host has actually drained, and its
    /// reservation lifecycle acknowledgement matches this owned attempt.
    /// Assignment terminal state or an arbitrary response alone is insufficient.
    /// Release BEFORE driving continuations; the next attempt reacquires and
    /// recomputes Ready/evidence snapshots.
    pub(super) fn release_after_bound_cleanup(mut self) {
        self.cleanup_acknowledged = true;
        self.state.store(IDLE, Ordering::Release);
    }
}

impl Drop for SchedulerSerialDispatch {
    fn drop(&mut self) {
        if !self.cleanup_acknowledged {
            self.state.store(POISONED, Ordering::Release);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Barrier;

    #[test]
    fn preparation_refusal_takes_no_dispatch_ownership() {
        let owner = SchedulerSerialAdmission::new();
        let preparation = owner.try_prepare().unwrap();
        assert!(matches!(
            owner.clone().try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Busy)
        ));
        drop(preparation);
        assert!(owner.try_prepare().is_ok());
        assert!(!owner.is_poisoned());
    }

    #[test]
    fn promotion_keeps_exclusion_until_acknowledged_cleanup() {
        let owner = SchedulerSerialAdmission::new();
        let dispatch = owner.try_prepare().unwrap().begin_dispatch();
        assert!(matches!(
            owner.try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Busy)
        ));
        dispatch.release_after_bound_cleanup();
        let continuation = owner.try_prepare().unwrap().begin_dispatch();
        continuation.release_after_bound_cleanup();
        assert!(!owner.is_poisoned());
    }

    #[test]
    fn abandoned_dispatch_poisons_all_existing_and_future_clones() {
        let owner = SchedulerSerialAdmission::new();
        let existing_clone = owner.clone();
        drop(owner.try_prepare().unwrap().begin_dispatch());
        for handle in [owner.clone(), existing_clone, owner] {
            assert!(handle.is_poisoned());
            assert!(matches!(
                handle.try_prepare(),
                Err(SchedulerSerialAdmissionRefusal::Poisoned)
            ));
        }
    }

    #[test]
    fn worker_panic_poisons_instead_of_reopening() {
        let owner = SchedulerSerialAdmission::new();
        let worker_owner = owner.clone();
        let result = std::panic::catch_unwind(move || {
            let _dispatch = worker_owner.try_prepare().unwrap().begin_dispatch();
            panic!("synthetic worker failure before cleanup");
        });
        assert!(result.is_err());
        assert!(matches!(
            owner.try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Poisoned)
        ));
    }

    #[test]
    fn concurrent_preparation_has_exactly_one_owner() {
        let owner = SchedulerSerialAdmission::new();
        let start = Arc::new(Barrier::new(8));
        let finish = Arc::new(Barrier::new(8));
        let admitted = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let owner = owner.clone();
                let start = start.clone();
                let finish = finish.clone();
                let admitted = admitted.clone();
                scope.spawn(move || {
                    start.wait();
                    let preparation = owner.try_prepare();
                    if preparation.is_ok() {
                        admitted.fetch_add(1, Ordering::Relaxed);
                    }
                    finish.wait();
                    drop(preparation);
                });
            }
        });
        assert_eq!(admitted.load(Ordering::Relaxed), 1);
        assert!(owner.try_prepare().is_ok());
    }

    #[test]
    fn a_new_owner_does_not_recover_or_mutate_an_abandoned_owner() {
        let abandoned = SchedulerSerialAdmission::new();
        drop(abandoned.try_prepare().unwrap().begin_dispatch());
        let replacement = SchedulerSerialAdmission::new();
        assert!(replacement.try_prepare().is_ok());
        assert!(abandoned.is_poisoned());
    }
}
