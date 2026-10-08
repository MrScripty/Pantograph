use super::*;
use crate::{
    RuntimeModelResidentEstimate, RuntimeObservation, RuntimeProducerAllocationState,
    RuntimeProducerObservation, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim, RuntimeRetentionDecision, RuntimeRetentionHint,
};
use std::sync::Arc;

fn request(owner: &str) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(owner.into()),
        usage_profile: None,
        model_id: Some("task-model".into()),
        pin_runtime: false,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(10),
        ])),
        retention_hint: RuntimeRetentionHint::Ephemeral,
    }
}
fn fixture() -> (
    Arc<RuntimeRegistry>,
    RuntimeReservationLease,
    RuntimeReservationLease,
) {
    let registry = Arc::new(RuntimeRegistry::new());
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "candle".into(),
            model_id: "model-target".into(),
            requirements: RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(100),
            ]),
        }])
        .unwrap();
    let task = registry.acquire_reservation(request("task")).unwrap();
    let successor = registry.acquire_reservation(request("successor")).unwrap();
    registry
        .observe_runtime_producer(RuntimeProducerObservation {
            source_id: "actual-owner".into(),
            sequence: 1,
            allocation_state: RuntimeProducerAllocationState::Resident,
            observation: RuntimeObservation {
                runtime_id: "candle".into(),
                display_name: "Candle".into(),
                backend_keys: vec!["candle".into()],
                model_id: Some("model-target".into()),
                runtime_instance_id: Some("instance".into()),
                status: RuntimeRegistryStatus::Ready,
                last_error: None,
            },
        })
        .unwrap();
    (registry, task, successor)
}
fn owner() -> RuntimeRetainedOwnerIdentity<'static> {
    RuntimeRetainedOwnerIdentity {
        runtime_id: "candle",
        source_id: "actual-owner",
        runtime_instance_id: "instance",
        model_target: "model-target",
    }
}
fn state(
    registry: &RuntimeRegistry,
) -> (
    Vec<crate::RuntimeRegistryRuntimeSnapshot>,
    Vec<RuntimeReservationLease>,
) {
    let snapshot = registry.snapshot();
    (snapshot.runtimes, snapshot.reservations)
}

#[test]
fn retained_release_preserves_successor_and_resident_envelope_and_last_lease_refuses() {
    let (registry, task, successor) = fixture();
    let before = state(&registry);
    let result = registry
        .release_retained_reservation_for_owner(&task, owner())
        .unwrap();
    assert_eq!(result.decision, RuntimeRetentionDecision::Retain);
    let after = state(&registry);
    assert_eq!(after.1, vec![successor.clone()]);
    assert_eq!(
        before.0[0].model_resource_residency,
        after.0[0].model_resource_residency
    );
    assert_eq!(
        before.0[0].runtime_instance_id,
        after.0[0].runtime_instance_id
    );
    assert_eq!(after.0[0].active_reservation_claims.len(), 1);
    assert!(registry
        .release_retained_reservation_for_owner(&successor, owner())
        .is_err());
    assert_eq!(after, state(&registry));
}

#[test]
fn source_instance_model_lease_and_accounting_mismatch_refuse_without_mutation() {
    let (registry, task, _) = fixture();
    let before = state(&registry);
    for wrong in [
        RuntimeRetainedOwnerIdentity {
            source_id: "foreign",
            ..owner()
        },
        RuntimeRetainedOwnerIdentity {
            runtime_instance_id: "replacement",
            ..owner()
        },
        RuntimeRetainedOwnerIdentity {
            model_target: "replacement",
            ..owner()
        },
    ] {
        assert!(registry
            .release_retained_reservation_for_owner(&task, wrong)
            .is_err());
        assert_eq!(before, state(&registry));
    }
    let mut changed = task.clone();
    changed.workflow_id = "foreign".into();
    assert!(registry
        .release_retained_reservation_for_owner(&changed, owner())
        .is_err());
    assert_eq!(before, state(&registry));
    registry
        .state
        .lock()
        .unwrap()
        .runtimes
        .get_mut("candle")
        .unwrap()
        .resident_resources_uncertain = true;
    let uncertain = state(&registry);
    assert!(registry
        .release_retained_reservation_for_owner(&task, owner())
        .is_err());
    assert_eq!(uncertain, state(&registry));
    let mut guard = registry.state.lock().unwrap();
    let runtime = guard.runtimes.get_mut("candle").unwrap();
    runtime.resident_resources_uncertain = false;
    runtime
        .model_resource_residency
        .as_mut()
        .unwrap()
        .requirements = Some(RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::vram_bytes(100),
    ]));
    drop(guard);
    let missing_ram = state(&registry);
    assert!(registry
        .release_retained_reservation_for_owner(&task, owner())
        .is_err());
    assert_eq!(missing_ram, state(&registry));
}

#[test]
fn pending_same_lease_refuses_but_pending_successor_remains_charged() {
    let (registry, task, _) = fixture();
    let evaluation = registry.evaluate_reservation(request("task")).unwrap();
    let (_, custody) = registry
        .acquire_reservation_provisional(request("task"), evaluation.observation(), |_| {
            Ok::<_, ()>(())
        })
        .unwrap();
    let before = state(&registry);
    assert!(matches!(
        registry.release_retained_reservation_for_owner(&task, owner()),
        Err(RuntimeRetainedCleanupError::Refused {
            reason: RuntimeRetainedCleanupRefusal::PendingCustody,
            ..
        })
    ));
    assert_eq!(before, state(&registry));
    drop(custody);
    let evaluation = registry.evaluate_reservation(request("next")).unwrap();
    let (_, successor_custody) = registry
        .acquire_reservation_provisional(request("next"), evaluation.observation(), |_| {
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(
        registry
            .release_retained_reservation_for_owner(&task, owner())
            .unwrap()
            .decision,
        RuntimeRetentionDecision::Retain
    );
    assert_eq!(state(&registry).1.len(), 2);
    drop(successor_custody);
    assert_eq!(state(&registry).1.len(), 1);
}

#[test]
fn many_leases_choose_one_retaining_successor_without_general_priority_scan() {
    let (registry, task, _) = fixture();
    for index in 0..512 {
        let mut next = request(&format!("successor-{index}"));
        if index == 511 {
            next.retention_hint = RuntimeRetentionHint::KeepAlive;
        }
        registry.acquire_reservation(next).unwrap();
    }
    let before = state(&registry);
    let retained = registry
        .release_retained_reservation_for_owner(&task, owner())
        .unwrap();
    // The first successor is ephemeral. General retention would scan all
    // successors to prefer the final KeepAlive reason; this path need not do so.
    assert_eq!(retained.reason, RuntimeRetentionReason::ActiveReservations);
    let after = state(&registry);
    assert_eq!(after.1.len(), 513);
    assert_eq!(after.1, before.1[1..]);
    assert_eq!(
        after.0[0].model_resource_residency,
        before.0[0].model_resource_residency
    );
}

#[test]
fn task_vram_requires_resident_vram_accounting() {
    let (registry, _, _) = fixture();
    let mut charged = request("gpu-charge");
    charged.requirements = Some(RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::ram_bytes(10),
        RuntimeReservationResourceClaim::vram_bytes(10),
    ]));
    let task = registry.acquire_reservation(charged).unwrap();
    let before = state(&registry);
    assert!(matches!(
        registry.release_retained_reservation_for_owner(&task, owner()),
        Err(RuntimeRetainedCleanupError::Refused {
            reason: RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
            ..
        })
    ));
    assert_eq!(before, state(&registry));
}

#[test]
fn execution_custody_gates_all_same_lease_mutations_and_success_consumes_charge() {
    let (registry, task, successor) = fixture();
    let before = state(&registry);
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    assert!(registry.acquire_execution_custody(&task).is_err());
    assert!(registry.release_reservation(task.reservation_id).is_err());
    assert!(registry
        .release_reservation_if_present(task.reservation_id)
        .is_err());
    assert!(registry
        .update_reservation_retention_hint(task.reservation_id, RuntimeRetentionHint::KeepAlive)
        .is_err());
    assert!(registry.acquire_reservation(request("task")).is_err());
    let evaluation = registry.evaluate_reservation(request("task")).unwrap();
    assert!(registry
        .acquire_reservation_provisional(
            request("task"),
            evaluation.observation(),
            |_| Ok::<_, ()>(())
        )
        .is_err());
    assert!(evaluation.commit().is_err());
    assert!(registry
        .release_retained_reservation_for_owner(&task, owner())
        .is_err());
    assert!(custody.validate_retained_owner(owner()).is_ok());
    assert_eq!(before, state(&registry));
    custody.release_retained(owner()).unwrap();
    assert!(registry.reservation_lease(task.reservation_id).is_none());
    assert_eq!(
        registry.reservation_lease(successor.reservation_id),
        Some(successor)
    );
}
#[test]
fn prepared_drop_unpins_but_execution_and_receipt_abandonment_stay_fenced() {
    let (registry, task, _) = fixture();
    let before = state(&registry);
    drop(registry.acquire_execution_custody(&task).unwrap());
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    drop(custody);
    assert_eq!(before, state(&registry));
    assert!(registry.release_reservation(task.reservation_id).is_err());
    assert!(registry.acquire_reservation(request("task")).is_err());
    assert!(registry.acquire_execution_custody(&task).is_err());
}
#[test]
fn custodied_cleanup_refuses_missing_successor_and_keeps_charge_fenced() {
    let (registry, task, successor) = fixture();
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    registry
        .release_reservation(successor.reservation_id)
        .unwrap();
    let before = state(&registry);
    assert!(custody.release_retained(owner()).is_err());
    assert_eq!(before, state(&registry));
    assert!(registry.release_reservation(task.reservation_id).is_err());
}

fn eviction_fixture() -> (
    Arc<RuntimeRegistry>,
    RuntimeReservationLease,
    crate::RuntimeLastLeaseEviction,
) {
    let (registry, task, successor) = fixture();
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    registry
        .release_reservation(successor.reservation_id)
        .unwrap();
    let ticket = custody.begin_last_lease_eviction(owner()).unwrap();
    (registry, task, ticket)
}
fn frame(
    sequence: u64,
    allocation_state: RuntimeProducerAllocationState,
) -> RuntimeProducerObservation {
    RuntimeProducerObservation {
        source_id: "actual-owner".into(),
        sequence,
        allocation_state,
        observation: RuntimeObservation {
            runtime_id: "candle".into(),
            display_name: "Candle".into(),
            backend_keys: vec!["candle".into()],
            model_id: (allocation_state == RuntimeProducerAllocationState::Resident)
                .then(|| "model-target".into()),
            runtime_instance_id: Some("instance".into()),
            status: RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    }
}
#[test]
fn last_lease_ticket_fences_mutations_until_ack_and_preserves_sequence_tombstone() {
    let (registry, task, successor) = fixture();
    let evaluation = registry.evaluate_reservation(request("late")).unwrap();
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    registry
        .release_reservation(successor.reservation_id)
        .unwrap();
    let ticket = custody.begin_last_lease_eviction(owner()).unwrap();
    let before = state(&registry);
    assert_eq!(before.0[0].status, RuntimeRegistryStatus::Stopping);
    assert!(before.0[0].model_resource_residency.is_some());
    assert_eq!(before.1, vec![task.clone()]);
    assert!(registry.release_reservation(task.reservation_id).is_err());
    assert!(registry
        .update_reservation_retention_hint(task.reservation_id, RuntimeRetentionHint::KeepAlive)
        .is_err());
    assert!(registry.acquire_execution_custody(&task).is_err());
    assert!(registry.acquire_reservation(request("new")).is_err());
    assert!(registry.can_acquire_reservation(&request("new")).is_err());
    assert!(registry.evaluate_reservation(request("new")).is_err());
    assert!(registry
        .acquire_reservation_provisional(
            request("late"),
            evaluation.observation(),
            |_| Ok::<_, ()>(())
        )
        .is_err());
    assert!(evaluation.commit().is_err());
    assert!(registry
        .release_retained_reservation_for_owner(&task, owner())
        .is_err());
    assert!(registry.reclaim_runtime("candle", false).is_err());
    assert!(registry
        .transition_runtime(
            "candle",
            crate::RuntimeTransition::Failed {
                message: "late".into()
            }
        )
        .is_err());
    for allocation in [
        RuntimeProducerAllocationState::Resident,
        RuntimeProducerAllocationState::Released,
        RuntimeProducerAllocationState::Unknown,
    ] {
        assert!(registry
            .observe_runtime_producer(frame(2, allocation))
            .is_err());
    }
    let mut health = frame(2, RuntimeProducerAllocationState::Resident).observation;
    health.status = RuntimeRegistryStatus::Failed;
    registry.observe_runtime(health.clone());
    registry.observe_runtimes(vec![health]);
    registry.observe_runtimes(vec![]);
    assert_eq!(before, state(&registry));
    assert_eq!(
        ticket.acknowledge_stopped(3).unwrap().decision,
        RuntimeRetentionDecision::Evict
    );
    let stopped = state(&registry);
    assert!(stopped.1.is_empty());
    assert_eq!(stopped.0[0].status, RuntimeRegistryStatus::Stopped);
    assert!(stopped.0[0].model_resource_residency.is_none());
    assert!(stopped.0[0].runtime_instance_id.is_none());
    for sequence in [1, 2, 3] {
        assert!(registry
            .observe_runtime_producer(frame(sequence, RuntimeProducerAllocationState::Resident))
            .is_err());
    }
    assert_eq!(stopped, state(&registry));
    // A later trusted producer frame can represent a genuine new load.
    registry
        .observe_runtime_producer(frame(4, RuntimeProducerAllocationState::Resident))
        .unwrap();
    assert!(registry.acquire_reservation(request("new")).is_ok());
}
#[test]
fn invalid_ack_or_ticket_loss_never_clears_charges_or_admission_fence() {
    for invalid_sequence in [0, 1] {
        let (registry, task, ticket) = eviction_fixture();
        let before = state(&registry);
        assert!(ticket.acknowledge_stopped(invalid_sequence).is_err());
        assert_eq!(before, state(&registry));
        assert!(registry.release_reservation(task.reservation_id).is_err());
        assert!(registry.acquire_reservation(request("late")).is_err());
    }
    let (registry, task, ticket) = eviction_fixture();
    let before = state(&registry);
    drop(ticket);
    assert_eq!(before, state(&registry));
    assert!(registry.release_reservation(task.reservation_id).is_err());
    assert!(registry
        .observe_runtime_producer(frame(2, RuntimeProducerAllocationState::Released))
        .is_err());
    let (registry, _, ticket) = eviction_fixture();
    registry
        .state
        .lock()
        .unwrap()
        .runtimes
        .get_mut("candle")
        .unwrap()
        .model_resource_residency
        .as_mut()
        .unwrap()
        .requirements = Some(RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::ram_bytes(101),
    ]));
    let changed = state(&registry);
    assert!(ticket.acknowledge_stopped(3).is_err());
    assert_eq!(changed, state(&registry));
}
#[test]
fn eviction_refuses_successor_retention_pins_unknown_models_and_prepared_custody() {
    let (registry, task, successor) = fixture();
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    let before = state(&registry);
    assert!(custody.begin_last_lease_eviction(owner()).is_err());
    assert_eq!(before, state(&registry));
    assert_eq!(
        registry.reservation_lease(successor.reservation_id),
        Some(successor)
    );
    for mode in [
        "keepalive",
        "lease-pin",
        "model-pin",
        "extra-model",
        "prepared",
    ] {
        let (registry, task, successor) = fixture();
        registry
            .release_reservation(successor.reservation_id)
            .unwrap();
        let task = if mode == "keepalive" {
            registry
                .update_reservation_retention_hint(
                    task.reservation_id,
                    RuntimeRetentionHint::KeepAlive,
                )
                .unwrap()
        } else if mode == "lease-pin" {
            let mut req = request("task");
            req.pin_runtime = true;
            registry.acquire_reservation(req).unwrap()
        } else {
            task
        };
        if mode == "model-pin" {
            registry
                .state
                .lock()
                .unwrap()
                .runtimes
                .get_mut("candle")
                .unwrap()
                .models
                .get_mut("model-target")
                .unwrap()
                .pinned = true;
        }
        if mode == "extra-model" {
            let mut state = registry.state.lock().unwrap();
            let r = state.runtimes.get_mut("candle").unwrap();
            let mut m = r.models.get("model-target").unwrap().clone();
            m.model_id = "extra".into();
            r.models.insert("extra".into(), m);
        }
        let mut custody = registry.acquire_execution_custody(&task).unwrap();
        if mode != "prepared" {
            custody.begin_execution();
        }
        let before = state(&registry);
        assert!(
            custody.begin_last_lease_eviction(owner()).is_err(),
            "{mode}"
        );
        assert_eq!(before, state(&registry));
    }
}

#[test]
fn pending_stop_keeps_shared_ram_charged_until_exact_ack() {
    let (registry, task, successor) = fixture();
    registry.register_runtime(crate::RuntimeRegistration::new("llama_cpp", "Peer"));
    registry
        .configure_resource_domain(crate::RuntimeResourceDomain {
            domain_id: "synthetic-shared-ram".into(),
            total_bytes: 1000,
            safety_margin_bytes: 0,
            bindings: ["candle", "llama_cpp"]
                .into_iter()
                .map(|runtime_id| crate::RuntimeResourceDomainBinding {
                    runtime_id: runtime_id.into(),
                    resource_kind: crate::RuntimeAdmissionResourceKind::RamBytes,
                })
                .collect(),
        })
        .unwrap();
    registry
        .observe_runtime_producer(RuntimeProducerObservation {
            source_id: "peer-owner".into(),
            sequence: 1,
            allocation_state: RuntimeProducerAllocationState::Released,
            observation: RuntimeObservation {
                runtime_id: "llama_cpp".into(),
                display_name: "Peer".into(),
                backend_keys: vec!["llama_cpp".into()],
                model_id: None,
                runtime_instance_id: None,
                status: RuntimeRegistryStatus::Stopped,
                last_error: None,
            },
        })
        .unwrap();
    let peer = || {
        let mut r = request("peer");
        r.runtime_id = "llama_cpp".into();
        r
    };
    let mut custody = registry.acquire_execution_custody(&task).unwrap();
    custody.begin_execution();
    registry
        .release_reservation(successor.reservation_id)
        .unwrap();
    let before = registry
        .evaluate_reservation(peer())
        .unwrap()
        .observation()
        .resource_domains[0]
        .clone();
    assert_eq!(before.reserved_bytes, 110);
    assert_eq!(before.resident_bytes, 100);
    assert_eq!(before.available_bytes, 890);
    let ticket = custody.begin_last_lease_eviction(owner()).unwrap();
    assert_eq!(
        registry
            .evaluate_reservation(peer())
            .unwrap()
            .observation()
            .resource_domains[0],
        before
    );
    ticket.acknowledge_stopped(3).unwrap();
    let after = registry
        .evaluate_reservation(peer())
        .unwrap()
        .observation()
        .resource_domains[0]
        .clone();
    assert_eq!(after.reserved_bytes, 0);
    assert_eq!(after.resident_bytes, 0);
    assert_eq!(after.available_bytes, 1000);
}
