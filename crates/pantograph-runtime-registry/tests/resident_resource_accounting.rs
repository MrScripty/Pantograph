use std::sync::{Arc, Barrier};

use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeAdmissionResourceKind as Kind,
    RuntimeObservation, RuntimeRegistration, RuntimeRegistry, RuntimeRegistryError,
    RuntimeRegistryStatus as Status, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim as Claim, RuntimeResourceDomain, RuntimeResourceDomainBinding,
    RuntimeRetentionHint, RuntimeTransition,
};

fn claims(ram: u64, vram: Option<u64>) -> RuntimeReservationRequirements {
    let mut result = vec![Claim::ram_bytes(ram)];
    if let Some(vram) = vram {
        result.push(Claim::vram_bytes(vram));
    }
    RuntimeReservationRequirements::from_claims(result)
}

fn domain(capacity: u64, unified: bool) -> RuntimeResourceDomain {
    let mut bindings = vec![
        RuntimeResourceDomainBinding {
            runtime_id: "pytorch".into(),
            resource_kind: Kind::RamBytes,
        },
        RuntimeResourceDomainBinding {
            runtime_id: "candle".into(),
            resource_kind: Kind::RamBytes,
        },
    ];
    if unified {
        bindings.push(RuntimeResourceDomainBinding {
            runtime_id: "pytorch".into(),
            resource_kind: Kind::VramBytes,
        });
    }
    RuntimeResourceDomain {
        domain_id: "pool".into(),
        total_bytes: capacity,
        safety_margin_bytes: 0,
        bindings,
    }
}

fn registry(capacity: u64, unified: bool) -> Arc<RuntimeRegistry> {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register_runtime(RuntimeRegistration::new("pytorch", "PyTorch"));
    registry.register_runtime(RuntimeRegistration::new("candle", "Candle"));
    registry
        .configure_resource_domain(domain(capacity, unified))
        .unwrap();
    registry
}

fn observation(model: Option<&str>, instance: Option<&str>, status: Status) -> RuntimeObservation {
    RuntimeObservation {
        runtime_id: "Torch".into(),
        display_name: "PyTorch".into(),
        backend_keys: vec!["pytorch".into()],
        model_id: model.map(str::to_owned),
        runtime_instance_id: instance.map(str::to_owned),
        status,
        last_error: None,
    }
}

fn resident(registry: &RuntimeRegistry, bytes: u64) {
    registry.observe_runtime(observation(
        Some("model-a"),
        Some("instance-a"),
        Status::Ready,
    ));
    registry
        .declare_model_residency_resources("Torch", "model-a", "instance-a", claims(bytes, None))
        .unwrap();
}

fn task(runtime: &str, owner: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: runtime.into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(owner.into()),
        model_id: Some("model-a".into()),
        usage_profile: None,
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::Ephemeral,
        requirements: Some(claims(bytes, None)),
    }
}

#[test]
fn uncertain_failed_owner_is_reclaimable_but_keeps_shared_admission_blocked() {
    use pantograph_runtime_registry::{
        RuntimeProducerAllocationState, RuntimeProducerObservation, RuntimeReclaimAction,
        RuntimeRetentionDecision,
    };

    let registry = registry(100, false);
    let lease = registry
        .acquire_reservation(task("pytorch", "failed-load", 50))
        .unwrap();
    registry
        .observe_runtime_producer(RuntimeProducerObservation {
            source_id: "owner".into(),
            sequence: 1,
            allocation_state: RuntimeProducerAllocationState::Unknown,
            observation: observation(None, None, Status::Failed),
        })
        .unwrap();
    assert!(registry.eviction_candidates().is_empty());
    assert_eq!(
        registry.retention_disposition("pytorch").unwrap().decision,
        RuntimeRetentionDecision::Retain
    );
    assert_eq!(registry.eviction_reservation_candidates().len(), 1);
    registry.release_reservation(lease.reservation_id).unwrap();
    assert_eq!(registry.eviction_candidates().len(), 1);
    assert_eq!(
        registry.reclaim_runtime("pytorch", false).unwrap().action,
        RuntimeReclaimAction::StopProducer
    );
    assert!(matches!(
        registry.acquire_reservation(task("candle", "before-ack", 1)),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
    ));
    registry
        .observe_runtime_producer(RuntimeProducerObservation {
            source_id: "owner".into(),
            sequence: 2,
            allocation_state: RuntimeProducerAllocationState::Released,
            observation: observation(None, None, Status::Failed),
        })
        .unwrap();
    assert!(registry.eviction_candidates().is_empty());
    assert_eq!(
        registry.reclaim_runtime("pytorch", false).unwrap().action,
        RuntimeReclaimAction::None
    );
    registry
        .acquire_reservation(task("candle", "after-ack", 100))
        .unwrap();
}

#[test]
fn task_completion_keeps_weights_charged_until_confirmed_stop() {
    let registry = registry(100, false);
    resident(&registry, 60);
    let lease = registry
        .acquire_reservation(task("pytorch", "execution", 30))
        .unwrap();
    let before = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    let pool = &before.observation().resource_domains[0];
    assert_eq!(
        (
            pool.resident_bytes,
            pool.reserved_bytes,
            pool.available_bytes
        ),
        (60, 90, 10)
    );
    registry.release_reservation(lease.reservation_id).unwrap();
    let idle = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    let pool = &idle.observation().resource_domains[0];
    assert_eq!(
        (
            pool.resident_bytes,
            pool.reserved_bytes,
            pool.available_bytes
        ),
        (60, 60, 40)
    );
    assert!(registry
        .acquire_reservation(task("candle", "too-big", 41))
        .is_err());
    registry.observe_runtime(observation(None, Some("instance-a"), Status::Stopped));
    registry
        .acquire_reservation(task("candle", "after-stop", 100))
        .unwrap();
}

#[test]
fn loaded_unknown_member_blocks_other_runtime_until_explicit_estimate_exists() {
    let registry = registry(100, false);
    registry.observe_runtime(observation(
        Some("model-a"),
        Some("instance-a"),
        Status::Ready,
    ));
    assert!(matches!(
        registry.evaluate_reservation(task("candle", "other", 1)),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable {
            resource_kind: "ram_bytes",
            ..
        })
    ));
    registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-a", claims(60, None))
        .unwrap();
    registry
        .acquire_reservation(task("candle", "other", 40))
        .unwrap();
}

#[test]
fn repeated_health_and_omitted_observations_do_not_free_declared_memory() {
    let registry = registry(100, false);
    resident(&registry, 60);
    for status in [
        Status::Busy,
        Status::Ready,
        Status::Unhealthy,
        Status::Failed,
        Status::Stopping,
    ] {
        registry.observe_runtime(observation(Some("model-a"), None, status));
        assert!(registry
            .acquire_reservation(task("candle", "probe", 41))
            .is_err());
        let probe = registry
            .evaluate_reservation(task("candle", "probe", 0))
            .unwrap();
        assert_eq!(probe.observation().resource_domains[0].resident_bytes, 60);
    }
    registry.observe_runtime(observation(None, None, Status::Failed));
    registry.observe_runtimes(Vec::new());
    let probe = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    assert_eq!(probe.observation().resource_domains[0].available_bytes, 40);
    registry.observe_runtime(observation(None, None, Status::Stopped));
    registry
        .acquire_reservation(task("candle", "after-stop", 100))
        .unwrap();
}

#[test]
fn model_or_instance_change_invalidates_estimates_and_rejects_stale_publication() {
    let registry = registry(100, false);
    resident(&registry, 60);
    for (model, instance) in [("model-b", "instance-a"), ("model-b", "instance-b")] {
        registry.observe_runtime(observation(Some(model), Some(instance), Status::Ready));
        assert!(matches!(
            registry.acquire_reservation(task("candle", "probe", 1)),
            Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
        ));
        assert!(matches!(
            registry.declare_model_residency_resources(
                "pytorch",
                "model-a",
                "instance-a",
                claims(10, None)
            ),
            Err(RuntimeRegistryError::ModelResidencyObservationChanged(_))
        ));
        registry
            .declare_model_residency_resources("pytorch", model, instance, claims(50, None))
            .unwrap();
        let probe = registry
            .evaluate_reservation(task("candle", "probe", 0))
            .unwrap();
        assert_eq!(probe.observation().resource_domains[0].resident_bytes, 50);
    }
}

#[test]
fn resident_growth_and_budget_shrink_reject_without_losing_previous_declaration() {
    let registry = registry(100, false);
    resident(&registry, 60);
    registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-a", claims(1, None))
        .unwrap();
    registry
        .acquire_reservation(task("candle", "other", 40))
        .unwrap();
    assert!(registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-a", claims(61, None))
        .is_err());
    assert!(registry
        .configure_resource_domain(domain(99, false))
        .is_err());
    let probe = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    let pool = &probe.observation().resource_domains[0];
    assert_eq!(
        (pool.resident_bytes, pool.reserved_bytes, pool.total_bytes),
        (60, 100, 100)
    );
}

#[test]
fn unified_memory_sums_declared_resident_kinds_and_requires_each_kind() {
    let registry = registry(100, true);
    resident(&registry, 20);
    assert!(matches!(
        registry.evaluate_reservation(task("candle", "probe", 1)),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable {
            resource_kind: "vram_bytes",
            ..
        })
    ));
    registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-a", claims(20, Some(50)))
        .unwrap();
    registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-a", claims(10, None))
        .unwrap();
    let probe = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    assert_eq!(probe.observation().resource_domains[0].resident_bytes, 70);
    registry
        .acquire_reservation(task("candle", "fits", 30))
        .unwrap();
    assert!(registry
        .acquire_reservation(task("candle", "extra", 1))
        .is_err());
}

#[test]
fn runtime_local_budgets_include_resident_memory_and_keep_task_peak_envelopes() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("pytorch", "PyTorch").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(100)),
            ]),
        ),
    );
    resident(&registry, 60);
    // The same model ID does not prove that a 50-byte peak envelope is only a
    // 10-byte transient allocation. Charge all 50 until an owner supplies a split.
    assert!(registry
        .acquire_reservation(task("pytorch", "peak", 50))
        .is_err());
    let probe = registry
        .evaluate_reservation(task("pytorch", "probe", 0))
        .unwrap();
    assert_eq!(
        (
            probe.observation().resources[0].resident_bytes,
            probe.observation().resources[0].reserved_bytes
        ),
        (60, 60)
    );
    assert!(registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-a", claims(101, None))
        .is_err());
    registry
        .acquire_reservation(task("pytorch", "fits", 40))
        .unwrap();
}

#[test]
fn concurrent_transient_requests_compete_against_one_retained_charge() {
    let registry = registry(100, false);
    resident(&registry, 60);
    let a = registry
        .evaluate_reservation(task("pytorch", "a", 30))
        .unwrap();
    let b = registry
        .evaluate_reservation(task("candle", "b", 30))
        .unwrap();
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
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(registry.snapshot().reservations.len(), 1);
}

#[test]
fn custody_rollback_and_transfer_preserve_resident_plus_predecessor_capacity() {
    let registry = registry(100, false);
    resident(&registry, 40);
    let original = registry
        .acquire_reservation(task("pytorch", "owner", 60))
        .unwrap();
    for transfer in [false, true] {
        let replacement = task("pytorch", "owner", 10);
        let expected = registry
            .evaluate_reservation(replacement.clone())
            .unwrap()
            .observation()
            .clone();
        let (_, custody) = registry
            .acquire_reservation_provisional(replacement, &expected, |_| Ok::<(), ()>(()))
            .unwrap();
        assert!(registry
            .acquire_reservation(task("candle", "other", 1))
            .is_err());
        if transfer {
            custody.transfer().unwrap();
        } else {
            drop(custody);
            assert_eq!(
                registry.snapshot().reservations[0].reservation_id,
                original.reservation_id
            );
        }
    }
    registry
        .acquire_reservation(task("candle", "other", 50))
        .unwrap();
}

#[test]
fn resident_overflow_invalid_claims_and_legacy_snapshots_never_fabricate_capacity() {
    let registry = registry(u64::MAX, true);
    registry.observe_runtime(observation(
        Some("model-a"),
        Some("instance-a"),
        Status::Ready,
    ));
    assert!(matches!(
        registry.declare_model_residency_resources(
            "pytorch",
            "model-a",
            "instance-a",
            RuntimeReservationRequirements::default()
        ),
        Err(RuntimeRegistryError::InvalidModelResidencyResources { .. })
    ));
    assert!(matches!(
        registry.declare_model_residency_resources(
            "pytorch",
            "model-a",
            "instance-a",
            claims(u64::MAX, Some(1))
        ),
        Err(RuntimeRegistryError::ResourceDomainAccountingOverflow { .. })
    ));
    assert!(registry
        .snapshot()
        .runtimes
        .iter()
        .find(|r| r.runtime_id == "pytorch")
        .unwrap()
        .model_resource_residency
        .as_ref()
        .unwrap()
        .requirements
        .is_none());
    let mut legacy = serde_json::to_value(registry.snapshot()).unwrap();
    for runtime in legacy["runtimes"].as_array_mut().unwrap() {
        runtime
            .as_object_mut()
            .unwrap()
            .remove("model_resource_residency");
    }
    let decoded: pantograph_runtime_registry::RuntimeRegistrySnapshot =
        serde_json::from_value(legacy).unwrap();
    assert!(decoded
        .runtimes
        .iter()
        .all(|runtime| runtime.model_resource_residency.is_none()));
    registry.observe_runtime(observation(None, None, Status::Stopped));
    registry
        .transition_runtime(
            "pytorch",
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: Some("instance-new".into()),
            },
        )
        .unwrap();
    assert!(matches!(
        registry.declare_model_residency_resources(
            "pytorch",
            "model-a",
            "instance-a",
            claims(1, None)
        ),
        Err(RuntimeRegistryError::ModelResidencyObservationChanged(_))
    ));
}

#[test]
fn evaluated_task_commit_rechecks_new_resident_allocation() {
    let registry = registry(100, false);
    let task = registry
        .evaluate_reservation(task("candle", "stale", 80))
        .unwrap();
    resident(&registry, 60);
    assert!(matches!(
        task.commit(),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            available_bytes: 40,
            ..
        })
    ));
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn stopping_and_reclaim_requests_hold_memory_until_producer_confirms_inactivity() {
    let registry = registry(100, false);
    resident(&registry, 60);
    registry.reclaim_runtime("pytorch", true).unwrap();
    let probe = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    assert_eq!(probe.observation().resource_domains[0].resident_bytes, 60);
    registry.reclaim_runtime("pytorch", false).unwrap();
    registry
        .acquire_reservation(task("candle", "after-stop", 100))
        .unwrap();

    let registry = self::registry(100, false);
    resident(&registry, 60);
    registry
        .transition_runtime("pytorch", RuntimeTransition::StopRequested)
        .unwrap();
    assert!(registry
        .acquire_reservation(task("candle", "during-stop", 41))
        .is_err());
    registry
        .transition_runtime("pytorch", RuntimeTransition::Stopped)
        .unwrap();
    registry
        .acquire_reservation(task("candle", "after-stop", 100))
        .unwrap();
}

#[test]
fn equal_model_content_in_distinct_producers_is_not_an_allocation_alias() {
    let registry = registry(100, false);
    resident(&registry, 60);
    let mut candle = observation(Some("model-a"), Some("instance-candle"), Status::Ready);
    candle.runtime_id = "candle".into();
    registry.observe_runtime(candle);
    assert!(registry
        .declare_model_residency_resources("candle", "model-a", "instance-candle", claims(60, None))
        .is_err());
    registry
        .declare_model_residency_resources("candle", "model-a", "instance-candle", claims(40, None))
        .unwrap();
    let probe = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    assert_eq!(probe.observation().resource_domains[0].resident_bytes, 100);
}

#[test]
fn partial_model_metadata_cannot_reuse_an_old_instances_estimate() {
    let registry = registry(100, false);
    resident(&registry, 60);
    registry.observe_runtime(observation(None, Some("instance-new"), Status::Ready));
    assert!(matches!(
        registry.acquire_reservation(task("candle", "probe", 1)),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
    ));
    assert!(matches!(
        registry.declare_model_residency_resources(
            "pytorch",
            "model-a",
            "instance-a",
            claims(60, None)
        ),
        Err(RuntimeRegistryError::ModelResidencyObservationChanged(_))
    ));
    assert!(matches!(
        registry.declare_model_residency_resources(
            "pytorch",
            "model-a",
            "instance-new",
            claims(60, None)
        ),
        Err(RuntimeRegistryError::ModelResidencyObservationChanged(_))
    ));
}

#[test]
fn explicit_runtime_transitions_invalidate_estimates_for_a_new_instance() {
    let registry = registry(100, false);
    resident(&registry, 60);
    registry
        .transition_runtime(
            "pytorch",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("instance-new".into()),
            },
        )
        .unwrap();
    assert!(matches!(
        registry.acquire_reservation(task("candle", "probe", 1)),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
    ));
    assert!(matches!(
        registry.declare_model_residency_resources(
            "pytorch",
            "model-a",
            "instance-new",
            claims(60, None)
        ),
        Err(RuntimeRegistryError::ModelResidencyObservationChanged(_))
    ));
    registry.observe_runtime(observation(
        Some("model-a"),
        Some("instance-new"),
        Status::Ready,
    ));
    registry
        .declare_model_residency_resources("pytorch", "model-a", "instance-new", claims(60, None))
        .unwrap();
    registry
        .acquire_reservation(task("candle", "after-observation", 40))
        .unwrap();
}
