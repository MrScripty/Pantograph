use pantograph_runtime_registry::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Barrier,
};

#[derive(Debug)]
struct Owner(AtomicU64);
impl RuntimeHostRamCapacitySource for Owner {
    fn capacity_ceiling_bytes(&self) -> Option<u64> {
        match self.0.load(Ordering::SeqCst) {
            u64::MAX => None,
            bytes => Some(bytes),
        }
    }
}
fn fixture(ceiling: u64) -> (Arc<RuntimeRegistry>, Arc<Owner>) {
    let registry = Arc::new(RuntimeRegistry::new());
    for runtime in ["pytorch", "candle"] {
        registry.register_runtime(RuntimeRegistration::new(runtime, runtime));
    }
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "host.ram".into(),
            total_bytes: 100,
            safety_margin_bytes: 0,
            bindings: ["pytorch", "candle"]
                .into_iter()
                .map(|runtime| RuntimeResourceDomainBinding {
                    runtime_id: runtime.into(),
                    resource_kind: RuntimeAdmissionResourceKind::RamBytes,
                })
                .collect(),
        })
        .unwrap();
    let owner = Arc::new(Owner(AtomicU64::new(ceiling)));
    registry.bind_host_ram_capacity_source(owner.clone());
    (registry, owner)
}
fn request(runtime: &str, owner: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: runtime.into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: Some(owner.into()),
        usage_profile: None,
        model_id: None,
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::Ephemeral,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(bytes),
        ])),
    }
}

#[test]
fn owner_ram_ceiling_constrains_contended_cross_runtime_commit() {
    let (registry, _) = fixture(60);
    let a = registry
        .evaluate_reservation(request("pytorch", "a", 40))
        .unwrap();
    let b = registry
        .evaluate_reservation(request("candle", "b", 40))
        .unwrap();
    assert_eq!(a.observation().resource_domains[0].total_bytes, 60);
    assert!(registry.snapshot().reservations.is_empty());
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
    assert!(results.iter().any(|result| matches!(
        result,
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            available_bytes: 20,
            ..
        })
    )));
    assert_eq!(registry.snapshot().reservations.len(), 1);
}

#[test]
fn owner_ram_ceiling_shrink_and_unavailability_preserve_live_claims() {
    let (registry, owner) = fixture(80);
    let held = registry
        .acquire_reservation(request("pytorch", "held", 60))
        .unwrap();
    let advisory = registry
        .evaluate_reservation(request("candle", "stale", 10))
        .unwrap();
    owner.0.store(40, Ordering::SeqCst);
    assert!(advisory.commit().is_err());
    assert!(registry
        .acquire_reservation(request("candle", "new", 1))
        .is_err());
    assert_eq!(
        registry.snapshot().reservations[0].reservation_id,
        held.reservation_id
    );
    assert_eq!(
        registry
            .snapshot()
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .unwrap()
            .active_reservation_claims[0]
            .claims[0]
            .bytes,
        60
    );
    owner.0.store(u64::MAX, Ordering::SeqCst);
    assert!(registry
        .acquire_reservation(request("candle", "unknown", 1))
        .is_err());
    assert_eq!(registry.snapshot().reservations.len(), 1);
    registry.release_reservation(held.reservation_id).unwrap();
    assert!(registry
        .acquire_reservation(request("candle", "still-unknown", 1))
        .is_err());
    owner.0.store(80, Ordering::SeqCst);
    registry
        .acquire_reservation(request("candle", "fresh", 80))
        .unwrap();
}

#[test]
fn owner_ram_ceiling_never_raises_budget_and_cannot_be_replaced() {
    let (registry, owner) = fixture(200);
    let observation = registry
        .evaluate_reservation(request("pytorch", "a", 100))
        .unwrap();
    assert_eq!(
        observation.observation().resource_domains[0].total_bytes,
        100
    );
    assert_eq!(
        observation.observation().resource_domains[0].owner_capacity_ceiling_bytes,
        Some(200)
    );
    owner.0.store(40, Ordering::SeqCst);
    registry.bind_host_ram_capacity_source(Arc::new(Owner(AtomicU64::new(200))));
    assert!(registry
        .acquire_reservation(request("pytorch", "b", 41))
        .is_err());
}

#[test]
fn owner_ram_ceiling_retains_resident_charge_and_full_task_peak() {
    let (registry, owner) = fixture(100);
    registry.observe_runtime(RuntimeObservation {
        runtime_id: "pytorch".into(),
        display_name: "PyTorch".into(),
        backend_keys: vec!["pytorch".into()],
        model_id: Some("model".into()),
        runtime_instance_id: Some("instance".into()),
        status: RuntimeRegistryStatus::Ready,
        last_error: None,
    });
    registry
        .declare_model_residency_resources(
            "pytorch",
            "model",
            "instance",
            RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(40),
            ]),
        )
        .unwrap();
    let held = registry
        .acquire_reservation(request("candle", "held", 50))
        .unwrap();
    owner.0.store(60, Ordering::SeqCst);
    assert!(registry
        .acquire_reservation(request("candle", "new", 1))
        .is_err());
    registry.release_reservation(held.reservation_id).unwrap();
    let probe = registry
        .evaluate_reservation(request("candle", "probe", 20))
        .unwrap();
    let pool = &probe.observation().resource_domains[0];
    assert_eq!(
        (
            pool.reserved_bytes,
            pool.resident_bytes,
            pool.available_bytes
        ),
        (40, 40, 20)
    );
    assert!(registry
        .acquire_reservation(request("candle", "peak", 21))
        .is_err());
}

#[test]
fn owner_ram_ceiling_leaves_vram_only_pool_to_its_declared_owner() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("pytorch", "PyTorch"));
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "gpu.0".into(),
            total_bytes: 100,
            safety_margin_bytes: 10,
            bindings: vec![RuntimeResourceDomainBinding {
                runtime_id: "pytorch".into(),
                resource_kind: RuntimeAdmissionResourceKind::VramBytes,
            }],
        })
        .unwrap();
    registry.bind_host_ram_capacity_source(Arc::new(Owner(AtomicU64::new(0))));
    let mut request = request("pytorch", "gpu", 0);
    request.requirements = Some(RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::vram_bytes(90),
    ]));
    registry.acquire_reservation(request).unwrap();
}

#[test]
fn owner_ram_ceiling_applies_to_later_unified_pool_and_preserves_margin() {
    let registry = RuntimeRegistry::new();
    registry.bind_host_ram_capacity_source(Arc::new(Owner(AtomicU64::new(60))));
    registry.register_runtime(RuntimeRegistration::new("pytorch", "PyTorch"));
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "unified".into(),
            total_bytes: 100,
            safety_margin_bytes: 10,
            bindings: [
                RuntimeAdmissionResourceKind::RamBytes,
                RuntimeAdmissionResourceKind::VramBytes,
            ]
            .into_iter()
            .map(|resource_kind| RuntimeResourceDomainBinding {
                runtime_id: "pytorch".into(),
                resource_kind,
            })
            .collect(),
        })
        .unwrap();
    let mut request = request("pytorch", "unified", 30);
    request.requirements = Some(RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::ram_bytes(30),
        RuntimeReservationResourceClaim::vram_bytes(21),
    ]));
    assert!(matches!(
        registry.acquire_reservation(request.clone()),
        Err(RuntimeRegistryError::ResourceDomainAdmissionRejected {
            requested_bytes: 51,
            available_bytes: 50,
            ..
        })
    ));
    request.requirements.as_mut().unwrap().claims[1].bytes = 20;
    registry.acquire_reservation(request).unwrap();
}

#[test]
fn owner_ram_ceiling_blocked_resident_publication_retries_without_weakening_source_fence() {
    let (registry, owner) = fixture(100);
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "pytorch".into(),
            model_id: "model".into(),
            requirements: RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(40),
            ]),
        }])
        .unwrap();
    let frame = RuntimeProducerObservation {
        source_id: "owned-runtime".into(),
        sequence: 1,
        allocation_state: RuntimeProducerAllocationState::Resident,
        observation: RuntimeObservation {
            runtime_id: "pytorch".into(),
            display_name: "PyTorch".into(),
            backend_keys: vec!["pytorch".into()],
            model_id: Some("model".into()),
            runtime_instance_id: Some("instance".into()),
            status: RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    };
    let held = registry
        .acquire_reservation(request("candle", "held", 50))
        .unwrap();
    owner.0.store(u64::MAX, Ordering::SeqCst);
    assert!(registry.observe_runtime_producer(frame.clone()).is_err());
    registry.release_reservation(held.reservation_id).unwrap();
    owner.0.store(100, Ordering::SeqCst);
    assert!(registry
        .evaluate_reservation(request("candle", "unknown-resident", 1))
        .is_err());
    let mut retry = frame.clone();
    retry.sequence = 2;
    registry.observe_runtime_producer(retry.clone()).unwrap();
    assert!(
        registry.observe_runtime_producer(frame).is_err(),
        "stale callback cannot erase resident charge"
    );
    retry.source_id = "other-owner".into();
    retry.sequence = 3;
    assert!(
        registry.observe_runtime_producer(retry).is_err(),
        "capacity recovery cannot replace the lifecycle owner"
    );
    let probe = registry
        .evaluate_reservation(request("candle", "probe", 60))
        .unwrap();
    assert_eq!(probe.observation().resource_domains[0].resident_bytes, 40);
    assert_eq!(probe.observation().resource_domains[0].available_bytes, 60);
    assert!(registry
        .acquire_reservation(request("candle", "over-peak", 61))
        .is_err());
}

#[test]
fn owner_ram_ceiling_known_zero_and_unavailable_remain_distinct_facts() {
    let (registry, owner) = fixture(0);
    let zero = registry
        .evaluate_reservation(request("pytorch", "probe", 0))
        .unwrap();
    assert!(zero.observation().resource_domains[0].host_ram_capacity_source_bound);
    assert_eq!(
        zero.observation().resource_domains[0].owner_capacity_ceiling_bytes,
        Some(0)
    );
    owner.0.store(u64::MAX, Ordering::SeqCst);
    let unknown = registry
        .evaluate_reservation(request("pytorch", "probe", 0))
        .unwrap();
    assert!(unknown.observation().resource_domains[0].host_ram_capacity_source_bound);
    assert_eq!(
        unknown.observation().resource_domains[0].owner_capacity_ceiling_bytes,
        None
    );
    assert!(registry
        .acquire_reservation(request("pytorch", "positive", 1))
        .is_err());
    assert!(registry.snapshot().reservations.is_empty());
}

#[test]
fn owner_ram_shrink_does_not_block_unrelated_vram_resident_publication() {
    let registry = Arc::new(RuntimeRegistry::new());
    for runtime in ["pytorch", "candle"] {
        registry.register_runtime(RuntimeRegistration::new(runtime, runtime));
    }
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "host.ram".into(),
            total_bytes: 100,
            safety_margin_bytes: 0,
            bindings: vec![RuntimeResourceDomainBinding {
                runtime_id: "pytorch".into(),
                resource_kind: RuntimeAdmissionResourceKind::RamBytes,
            }],
        })
        .unwrap();
    let owner = Arc::new(Owner(AtomicU64::new(100)));
    registry.bind_host_ram_capacity_source(owner.clone());
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "candle.vram".into(),
            total_bytes: 100,
            safety_margin_bytes: 0,
            bindings: vec![RuntimeResourceDomainBinding {
                runtime_id: "candle".into(),
                resource_kind: RuntimeAdmissionResourceKind::VramBytes,
            }],
        })
        .unwrap();
    let held = registry
        .acquire_reservation(request("pytorch", "held", 60))
        .unwrap();
    owner.0.store(40, Ordering::SeqCst);
    registry.observe_runtime(RuntimeObservation {
        runtime_id: "candle".into(),
        display_name: "Candle".into(),
        backend_keys: vec!["candle".into()],
        model_id: Some("candle-model".into()),
        runtime_instance_id: Some("candle-instance".into()),
        status: RuntimeRegistryStatus::Ready,
        last_error: None,
    });
    registry
        .declare_model_residency_resources(
            "candle",
            "candle-model",
            "candle-instance",
            RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::vram_bytes(10),
            ]),
        )
        .unwrap();
    let mut vram = request("candle", "vram", 0);
    vram.requirements = Some(RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::vram_bytes(20),
    ]));
    let probe = registry.evaluate_reservation(vram.clone()).unwrap();
    let pool = probe
        .observation()
        .resource_domains
        .iter()
        .find(|pool| pool.domain_id == "candle.vram")
        .unwrap();
    assert_eq!((pool.resident_bytes, pool.available_bytes), (10, 90));
    registry.acquire_reservation(vram).unwrap();
    assert!(registry
        .acquire_reservation(request("pytorch", "new-ram", 1))
        .is_err());
    assert!(registry
        .snapshot()
        .reservations
        .iter()
        .any(|lease| lease.reservation_id == held.reservation_id));
    assert_eq!(
        registry
            .snapshot()
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .unwrap()
            .active_reservation_claims[0]
            .claims[0]
            .bytes,
        60
    );
}
