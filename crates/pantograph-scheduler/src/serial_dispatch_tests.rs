use super::*;
use crate::{SchedulerSerialAdmission, SchedulerSerialAdmissionRefusal};
use std::sync::Mutex;

struct Lease(Mutex<SchedulerSerialOwnerSnapshot>);
impl SchedulerSerialRuntimeOwnerLease for Lease {
    fn snapshot(&self) -> SchedulerSerialOwnerSnapshot {
        *self.0.lock().unwrap()
    }
}

fn snapshot() -> SchedulerSerialOwnerSnapshot {
    SchedulerSerialOwnerSnapshot {
        loaded_instance: [1; 16],
        loaded_profile: [2; 32],
        effective_settings: [3; 32],
        generation: 2,
        cpu_threads: 1,
    }
}
fn identity() -> SchedulerSerialAttemptIdentity<'static> {
    SchedulerSerialAttemptIdentity {
        workflow_id: "workflow.1",
        workflow_run_id: "run.1",
        node_id: "node.1",
        task_id: "task.1",
        attempt_id: "attempt.1",
        execution_request_id: "request.1",
        candidate_id: "candidate.1",
        reservation_lease_id: "lease.1",
    }
}
fn bound(owner: &SchedulerSerialAdmission) -> SchedulerSerialBoundDispatch {
    owner
        .try_prepare()
        .unwrap()
        .begin_dispatch()
        .bind_attempt(identity(), snapshot())
        .unwrap()
}
fn drained(owner: &SchedulerSerialAdmission) -> SchedulerSerialDrainedDispatch {
    let lease = Lease(Mutex::new(snapshot()));
    bound(owner)
        .acquire_owner(&lease)
        .unwrap()
        .record_drained_response(identity(), SchedulerSerialDrainState::Completed)
        .unwrap()
}
fn cleanup_event() -> SchedulerSerialCleanupEvent<'static> {
    SchedulerSerialCleanupEvent {
        identity: identity(),
        lifecycle_event_id: "cleanup.1",
        releases_reservation: true,
    }
}
fn assert_poisoned(owner: &SchedulerSerialAdmission) {
    assert!(matches!(
        owner.try_prepare(),
        Err(SchedulerSerialAdmissionRefusal::Poisoned)
    ));
}

#[test]
fn matching_owner_terminal_drain_and_exact_cleanup_release_before_continuation() {
    let owner = SchedulerSerialAdmission::new();
    let ticket = drained(&owner).expect_cleanup(cleanup_event()).unwrap();
    assert!(matches!(
        owner.try_prepare(),
        Err(SchedulerSerialAdmissionRefusal::Busy)
    ));
    ticket
        .acknowledge_cleanup("cleanup.1", "lease.1", SchedulerSerialCleanupState::Applied)
        .unwrap();
    assert!(owner.try_prepare().is_ok());
}

#[test]
fn each_foreign_attempt_response_field_refuses_and_poisons() {
    let id = identity();
    let variations = [
        SchedulerSerialAttemptIdentity {
            workflow_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            workflow_run_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            node_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            task_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            attempt_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            execution_request_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            candidate_id: "foreign",
            ..id
        },
        SchedulerSerialAttemptIdentity {
            reservation_lease_id: "foreign",
            ..id
        },
    ];
    for foreign in variations {
        let owner = SchedulerSerialAdmission::new();
        let lease = Lease(Mutex::new(snapshot()));
        let execution = bound(&owner).acquire_owner(&lease).unwrap();
        assert!(matches!(
            execution.record_drained_response(foreign, SchedulerSerialDrainState::Completed),
            Err(SchedulerSerialDispatchRefusal::ForeignResponse)
        ));
        assert_poisoned(&owner);
    }
}

#[test]
fn every_loaded_owner_dimension_is_rechecked_at_actual_acquisition() {
    let expected = snapshot();
    let changed = [
        SchedulerSerialOwnerSnapshot {
            loaded_instance: [9; 16],
            ..expected
        },
        SchedulerSerialOwnerSnapshot {
            loaded_profile: [9; 32],
            ..expected
        },
        SchedulerSerialOwnerSnapshot {
            effective_settings: [9; 32],
            ..expected
        },
        SchedulerSerialOwnerSnapshot {
            generation: 4,
            ..expected
        },
        SchedulerSerialOwnerSnapshot {
            cpu_threads: 2,
            ..expected
        },
    ];
    for snapshot in changed {
        let owner = SchedulerSerialAdmission::new();
        let lease = Lease(Mutex::new(snapshot));
        assert!(matches!(
            bound(&owner).acquire_owner(&lease),
            Err(SchedulerSerialDispatchRefusal::OwnerChanged)
        ));
        assert_poisoned(&owner);
    }
}

#[test]
fn settings_changes_during_execution_cannot_mint_a_drain_proof() {
    let owner = SchedulerSerialAdmission::new();
    let lease = Lease(Mutex::new(snapshot()));
    let execution = bound(&owner).acquire_owner(&lease).unwrap();
    *lease.0.lock().unwrap() = SchedulerSerialOwnerSnapshot {
        effective_settings: [8; 32],
        ..snapshot()
    };
    assert!(matches!(
        execution.record_drained_response(identity(), SchedulerSerialDrainState::Completed),
        Err(SchedulerSerialDispatchRefusal::OwnerChanged)
    ));
    assert_poisoned(&owner);
}

#[test]
fn acceptance_or_abandoned_drain_keeps_admission_poisoned() {
    for accepted in [true, false] {
        let owner = SchedulerSerialAdmission::new();
        let lease = Lease(Mutex::new(snapshot()));
        let execution = bound(&owner).acquire_owner(&lease).unwrap();
        if accepted {
            assert!(matches!(
                execution.record_drained_response(identity(), SchedulerSerialDrainState::Accepted),
                Err(SchedulerSerialDispatchRefusal::NotDrained)
            ));
        } else {
            drop(execution);
        }
        assert_poisoned(&owner);
    }
}

#[test]
fn dispatch_started_foreign_attempt_or_wrong_lease_cannot_be_expected_cleanup() {
    for event in [
        SchedulerSerialCleanupEvent {
            releases_reservation: false,
            ..cleanup_event()
        },
        SchedulerSerialCleanupEvent {
            identity: SchedulerSerialAttemptIdentity {
                attempt_id: "old-attempt",
                ..identity()
            },
            ..cleanup_event()
        },
        SchedulerSerialCleanupEvent {
            identity: SchedulerSerialAttemptIdentity {
                reservation_lease_id: "old-lease",
                ..identity()
            },
            ..cleanup_event()
        },
    ] {
        let owner = SchedulerSerialAdmission::new();
        assert!(matches!(
            drained(&owner).expect_cleanup(event),
            Err(SchedulerSerialDispatchRefusal::ForeignCleanup)
        ));
        assert_poisoned(&owner);
    }
}

#[test]
fn wrong_echo_failed_cleanup_or_abandoned_ticket_never_reopen() {
    for (event, lease, state) in [
        (
            "old-cleanup",
            "lease.1",
            SchedulerSerialCleanupState::Applied,
        ),
        (
            "cleanup.1",
            "old-lease",
            SchedulerSerialCleanupState::AlreadyApplied,
        ),
        ("cleanup.1", "lease.1", SchedulerSerialCleanupState::Failed),
    ] {
        let owner = SchedulerSerialAdmission::new();
        let ticket = drained(&owner).expect_cleanup(cleanup_event()).unwrap();
        assert!(ticket.acknowledge_cleanup(event, lease, state).is_err());
        assert_poisoned(&owner);
    }
    let owner = SchedulerSerialAdmission::new();
    drop(drained(&owner).expect_cleanup(cleanup_event()).unwrap());
    assert_poisoned(&owner);
}

#[test]
fn same_immutable_cleanup_replay_is_accepted_but_old_attempt_is_not() {
    let owner = SchedulerSerialAdmission::new();
    drained(&owner)
        .expect_cleanup(cleanup_event())
        .unwrap()
        .acknowledge_cleanup(
            "cleanup.1",
            "lease.1",
            SchedulerSerialCleanupState::AlreadyApplied,
        )
        .unwrap();
    assert!(owner.try_prepare().is_ok());
}

#[test]
fn oversized_identity_and_unpublished_owner_refuse_before_cloning() {
    let owner = SchedulerSerialAdmission::new();
    let oversized = "a".repeat(129);
    assert!(matches!(
        owner.try_prepare().unwrap().begin_dispatch().bind_attempt(
            SchedulerSerialAttemptIdentity {
                attempt_id: &oversized,
                ..identity()
            },
            snapshot()
        ),
        Err(SchedulerSerialDispatchRefusal::InvalidIdentity)
    ));
    assert_poisoned(&owner);
    for bad in [
        SchedulerSerialOwnerSnapshot {
            generation: 0,
            ..snapshot()
        },
        SchedulerSerialOwnerSnapshot {
            generation: 3,
            ..snapshot()
        },
        SchedulerSerialOwnerSnapshot {
            loaded_instance: [0; 16],
            ..snapshot()
        },
        SchedulerSerialOwnerSnapshot {
            cpu_threads: 0,
            ..snapshot()
        },
        SchedulerSerialOwnerSnapshot {
            cpu_threads: 5,
            ..snapshot()
        },
    ] {
        let owner = SchedulerSerialAdmission::new();
        assert!(matches!(
            owner
                .try_prepare()
                .unwrap()
                .begin_dispatch()
                .bind_attempt(identity(), bad),
            Err(SchedulerSerialDispatchRefusal::InvalidOwner)
        ));
        assert_poisoned(&owner);
    }
}

#[test]
fn ranked_attempt_cannot_downgrade_to_unranked_drain() {
    let owner = SchedulerSerialAdmission::new();
    assert!(matches!(
        bound(&owner)
            .record_unranked_drained_response(identity(), SchedulerSerialDrainState::Completed),
        Err(SchedulerSerialDispatchRefusal::NotDrained)
    ));
    assert_poisoned(&owner);
}
#[test]
fn unranked_owner_requires_actual_terminal_identity_and_matched_cleanup() {
    let owner = SchedulerSerialAdmission::new();
    let bound = owner
        .try_prepare()
        .unwrap()
        .begin_dispatch()
        .bind_unranked_attempt(identity())
        .unwrap();
    assert_eq!(bound.expected_owner(), None);
    assert_eq!(bound.identity(), identity());
    let drained = bound
        .record_unranked_drained_response(identity(), SchedulerSerialDrainState::Completed)
        .unwrap();
    drained
        .expect_cleanup(cleanup_event())
        .unwrap()
        .acknowledge_cleanup("cleanup.1", "lease.1", SchedulerSerialCleanupState::Applied)
        .unwrap();
    assert!(owner.try_prepare().is_ok());
    let bound = owner
        .try_prepare()
        .unwrap()
        .begin_dispatch()
        .bind_unranked_attempt(identity())
        .unwrap();
    assert!(matches!(
        bound.record_unranked_drained_response(identity(), SchedulerSerialDrainState::Accepted),
        Err(SchedulerSerialDispatchRefusal::NotDrained)
    ));
    assert_poisoned(&owner);
}
