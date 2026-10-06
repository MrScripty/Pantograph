use std::sync::Mutex;

use inference::resident_lifecycle::{ResidentAllocationState, ResidentLifecycleSnapshot};
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeModelResidentEstimate, RuntimeReclaimAction,
    RuntimeRegistration, RuntimeRegistryError, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim, RuntimeResourceDomain, RuntimeResourceDomainBinding,
};

use super::*;
use crate::runtime_registry::reclaim_runtime_and_reconcile_runtime_registry;

fn requirements(bytes: u64) -> RuntimeReservationRequirements {
    RuntimeReservationRequirements::from_claims(vec![RuntimeReservationResourceClaim::ram_bytes(
        bytes,
    )])
}

fn registry() -> Arc<RuntimeRegistry> {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.register_runtime(RuntimeRegistration::new("pytorch", "PyTorch"));
    registry.register_runtime(RuntimeRegistration::new("candle", "Candle"));
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "pool".into(),
            total_bytes: 100,
            safety_margin_bytes: 0,
            bindings: vec![
                RuntimeResourceDomainBinding {
                    runtime_id: "pytorch".into(),
                    resource_kind: RuntimeAdmissionResourceKind::RamBytes,
                },
                RuntimeResourceDomainBinding {
                    runtime_id: "candle".into(),
                    resource_kind: RuntimeAdmissionResourceKind::RamBytes,
                },
            ],
        })
        .expect("shared domain");
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "pytorch".into(),
            model_id: "model-a".into(),
            requirements: requirements(40),
        }])
        .expect("resident estimate");
    registry
}

fn task(runtime: &str, owner: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: runtime.into(),
        workflow_id: "wf-image".into(),
        reservation_owner_id: Some(owner.into()),
        usage_profile: None,
        model_id: Some("model-a".into()),
        pin_runtime: false,
        requirements: Some(requirements(bytes)),
        retention_hint: RuntimeRetentionHint::Ephemeral,
    }
}

struct ResidentController {
    registry: Arc<RuntimeRegistry>,
    allocation: Mutex<ResidentAllocationState>,
    samples: AtomicUsize,
    stop_count: AtomicUsize,
    stop_fails: AtomicBool,
    stop_acknowledged: AtomicBool,
    probe_after_release: AtomicBool,
    probed_after_release: AtomicBool,
}

impl ResidentController {
    fn new(registry: Arc<RuntimeRegistry>, allocation: ResidentAllocationState) -> Self {
        Self {
            registry,
            allocation: Mutex::new(allocation),
            samples: AtomicUsize::new(0),
            stop_count: AtomicUsize::new(0),
            stop_fails: AtomicBool::new(false),
            stop_acknowledged: AtomicBool::new(true),
            probe_after_release: AtomicBool::new(false),
            probed_after_release: AtomicBool::new(false),
        }
    }

    fn lifecycle(&self) -> inference::RuntimeLifecycleSnapshot {
        let allocation = *self.allocation.lock().expect("allocation");
        inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("pytorch".into()),
            runtime_instance_id: Some("pytorch-1".into()),
            active: allocation == ResidentAllocationState::Resident,
            warmup_started_at_ms: (allocation != ResidentAllocationState::Released).then_some(1),
            warmup_completed_at_ms: (allocation == ResidentAllocationState::Resident).then_some(2),
            last_error: (allocation == ResidentAllocationState::Unknown)
                .then(|| "load failed after allocation".into()),
            ..Default::default()
        }
    }
}

#[async_trait]
impl HostRuntimeRegistryController for ResidentController {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        HostRuntimeModeSnapshot {
            backend_name: Some("PyTorch".into()),
            backend_key: Some("pytorch".into()),
            active_runtime: Some(self.lifecycle()),
            ..Default::default()
        }
    }

    async fn resident_lifecycle_snapshot(&self) -> Option<ResidentLifecycleSnapshot> {
        let sequence = self.samples.fetch_add(1, Ordering::SeqCst) + 1;
        if self.probe_after_release.load(Ordering::SeqCst)
            && self.registry.snapshot().reservations.len() == 1
        {
            // This runs at the awaited publication boundary after custody was
            // released, before the retry can install the 40-byte estimate.
            assert!(matches!(
                self.registry
                    .acquire_reservation(task("candle", "racing", 20)),
                Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
            ));
            self.probed_after_release.store(true, Ordering::SeqCst);
        }
        let allocation_state = *self.allocation.lock().expect("allocation");
        Some(ResidentLifecycleSnapshot {
            source_id: "test-owner".into(),
            sequence: sequence as u64,
            allocation_state,
            model_target: (allocation_state == ResidentAllocationState::Resident)
                .then(|| "model-a".into()),
            lifecycle: self.lifecycle(),
        })
    }

    async fn stop_runtime_producer(
        &self,
        producer: HostRuntimeProducer,
    ) -> Result<(), inference::GatewayError> {
        assert_eq!(producer, HostRuntimeProducer::Active);
        self.stop_count.fetch_add(1, Ordering::SeqCst);
        let allocation = *self.allocation.lock().expect("allocation");
        let request = if allocation == ResidentAllocationState::Resident {
            task("candle", "before-stop-ack", 61)
        } else {
            task("candle", "before-stop-ack", 1)
        };
        assert!(self.registry.acquire_reservation(request).is_err());
        if self.stop_fails.load(Ordering::SeqCst) {
            return Err(inference::GatewayError::SwitchFailed("stop failed".into()));
        }
        if self.stop_acknowledged.load(Ordering::SeqCst) {
            *self.allocation.lock().expect("allocation") = ResidentAllocationState::Released;
        }
        Ok(())
    }
}

#[tokio::test]
async fn terminal_cold_load_retains_residency_without_an_admission_window() {
    let registry = registry();
    let first = registry
        .acquire_reservation(task("pytorch", "first", 50))
        .unwrap();
    let second = registry
        .acquire_reservation(task("pytorch", "second", 50))
        .unwrap();
    // The selected execution has cold-loaded weights, but has not published
    // resident facts. Both full-peak leases are still held at completion.
    let controller = Arc::new(ResidentController::new(
        registry.clone(),
        ResidentAllocationState::Resident,
    ));
    controller.probe_after_release.store(true, Ordering::SeqCst);
    let port = EmbeddedReservationLifecyclePort::new(registry.clone(), controller.clone());
    let application = port
        .apply_reservation_lifecycle(event(
            first.reservation_id,
            ReservationLifecycleOutcome::RuntimeHostCompleted,
            Vec::new(),
        ))
        .await
        .expect("terminal release");
    assert_eq!(
        application.state,
        ReservationLifecycleApplicationState::Applied
    );
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 0);
    let snapshot = registry.snapshot();
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(
        snapshot.reservations[0].reservation_id,
        second.reservation_id
    );
    let probe = registry
        .evaluate_reservation(task("candle", "probe", 0))
        .unwrap();
    let pool = &probe.observation().resource_domains[0];
    assert_eq!(
        (
            pool.resident_bytes,
            pool.reserved_bytes,
            pool.available_bytes
        ),
        (40, 90, 10)
    );
    assert!(controller.probed_after_release.load(Ordering::SeqCst));
    assert!(registry
        .acquire_reservation(task("candle", "competing", 20))
        .is_err());
}

#[tokio::test]
async fn ordinary_reclaim_reaches_failed_inactive_owner_without_terminal_cleanup() {
    let registry = registry();
    let controller = ResidentController::new(registry.clone(), ResidentAllocationState::Unknown);
    controller.stop_acknowledged.store(false, Ordering::SeqCst);
    crate::runtime_registry::sync_runtime_registry(&controller, &registry).await;
    let failed = registry
        .eviction_candidates()
        .into_iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("failed uncertain allocation must be ordinarily reclaimable");
    assert_eq!(
        failed.status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Failed
    );
    assert!(failed.resident_resources_uncertain);
    assert!(!controller.lifecycle().active);

    let reclaim = reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "pytorch")
        .await
        .expect("ordinary reclaim reaches inactive owner");
    assert_eq!(reclaim.action, RuntimeReclaimAction::StopProducer);
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 1);
    assert!(registry
        .acquire_reservation(task("candle", "unacknowledged", 1))
        .is_err());

    controller.stop_acknowledged.store(true, Ordering::SeqCst);
    reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "pytorch")
        .await
        .expect("acknowledged owner shutdown");
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 2);
    registry
        .acquire_reservation(task("candle", "acknowledged", 100))
        .unwrap();
}

#[tokio::test]
async fn terminal_last_lease_keeps_residency_until_owned_stop_acknowledges_release() {
    let registry = registry();
    let lease = registry
        .acquire_reservation(task("pytorch", "only", 50))
        .unwrap();
    let controller = Arc::new(ResidentController::new(
        registry.clone(),
        ResidentAllocationState::Resident,
    ));
    let port = EmbeddedReservationLifecyclePort::new(registry.clone(), controller.clone());
    port.apply_reservation_lifecycle(event(
        lease.reservation_id,
        ReservationLifecycleOutcome::RuntimeHostCompleted,
        Vec::new(),
    ))
    .await
    .expect("terminal release");
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 1);
    registry
        .acquire_reservation(task("candle", "after-ack", 100))
        .unwrap();
}

#[tokio::test]
async fn terminal_failed_load_reclaims_inactive_owner_and_requires_release_acknowledgment() {
    let registry = registry();
    let lease = registry
        .acquire_reservation(task("pytorch", "failed", 50))
        .unwrap();
    let controller = Arc::new(ResidentController::new(
        registry.clone(),
        ResidentAllocationState::Unknown,
    ));
    controller.stop_fails.store(true, Ordering::SeqCst);
    let port = EmbeddedReservationLifecyclePort::new(registry.clone(), controller.clone());
    port.apply_reservation_lifecycle(event(
        lease.reservation_id,
        ReservationLifecycleOutcome::RuntimeHostFailed,
        vec![diagnostic(
            ReservationLifecycleDiagnosticSeverity::Error,
            ReservationLifecycleDiagnosticCode::RuntimeHostFailed,
            "load failed after allocation",
        )],
    ))
    .await
    .expect_err("failed owned stop must be reported");
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 1);
    assert!(registry.snapshot().reservations.is_empty());
    assert!(registry
        .acquire_reservation(task("candle", "blocked", 1))
        .is_err());

    controller.stop_fails.store(false, Ordering::SeqCst);
    controller.stop_acknowledged.store(false, Ordering::SeqCst);
    let reclaim =
        reclaim_runtime_and_reconcile_runtime_registry(controller.as_ref(), &registry, "pytorch")
            .await
            .expect("retry reaches owner");
    assert_eq!(reclaim.action, RuntimeReclaimAction::StopProducer);
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 2);
    assert!(registry
        .acquire_reservation(task("candle", "still-blocked", 1))
        .is_err());
    assert!(registry.snapshot().runtimes.iter().any(|runtime| {
        runtime.runtime_id == "pytorch" && runtime.resident_resources_uncertain
    }));

    controller.stop_acknowledged.store(true, Ordering::SeqCst);
    reclaim_runtime_and_reconcile_runtime_registry(controller.as_ref(), &registry, "pytorch")
        .await
        .expect("acknowledged owned stop");
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 3);
    registry
        .acquire_reservation(task("candle", "after-ack", 100))
        .unwrap();
}
