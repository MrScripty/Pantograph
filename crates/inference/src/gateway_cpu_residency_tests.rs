use super::*;
use crate::CandleCpuCalibrationConfig;
use pantograph_runtime_registry::{
    RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeModelResidentEstimate,
    RuntimeRegistration, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim,
};

struct Fixture {
    _directory: tempfile::TempDir,
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    request: InferenceExecutionRequest,
    target: crate::PumasArtifactLoadTarget,
    decision: crate::BackendExecutionDecision,
    guard: RuntimeReservationLease,
}

fn requirements(ram: u64) -> RuntimeReservationRequirements {
    RuntimeReservationRequirements::from_claims(vec![
        RuntimeReservationResourceClaim::ram_bytes(ram),
        RuntimeReservationResourceClaim::vram_bytes(0),
    ])
}

fn fixture() -> Fixture {
    let (directory, mut request, target, decision) =
        crate::selected_embedding_execution::fixture(8);
    let InferenceExecutionInput::Embedding { texts } = &mut request.input else {
        panic!("fixture embedding request");
    };
    texts.truncate(1);
    let gateway = Arc::new(
        InferenceGateway::new_calibrated_candle_cpu(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
        )
        .unwrap(),
    );
    let registry = Arc::new(RuntimeRegistry::new());
    // Operator envelopes for this fixture; not allocator/RSS measurements.
    registry.register_runtime(
        RuntimeRegistration::new("candle", "Candle").with_admission_budget(
            RuntimeAdmissionBudget::from_resources(vec![
                RuntimeAdmissionResourceBudget::ram_bytes(Some(64 * 1024 * 1024)),
                RuntimeAdmissionResourceBudget::vram_bytes(Some(0)),
            ]),
        ),
    );
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "candle".into(),
            model_id: target.local_load_path.clone(),
            requirements: requirements(4 * 1024 * 1024),
        }])
        .unwrap();
    let guard = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "candle".into(),
            workflow_id: "native-residency".into(),
            reservation_owner_id: Some("mode-guard".into()),
            usage_profile: None,
            model_id: Some(target.model_ref.model_id.clone()),
            pin_runtime: false,
            requirements: Some(requirements(8 * 1024 * 1024)),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .unwrap();
    Fixture {
        _directory: directory,
        gateway,
        registry,
        request,
        target,
        decision,
        guard,
    }
}

impl Fixture {
    fn cold_input(&self) -> CandleCpuColdResidencyRequest<'_> {
        CandleCpuColdResidencyRequest {
            request: self.request.clone(),
            target: self.target.clone(),
            decision: self.decision.clone(),
            guard: &self.guard,
        }
    }
    async fn load(&self) -> CandleCpuIdleResidency {
        self.gateway
            .load_cpu_idle_residency(
                self.registry.clone(),
                self.cold_input(),
                InferenceExecutionCancellationHandle::running(),
            )
            .await
            .unwrap()
    }
    fn task(&self, owner: &str) -> RuntimeReservationLease {
        self.registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "candle".into(),
                workflow_id: "native-residency".into(),
                reservation_owner_id: Some(owner.into()),
                usage_profile: None,
                model_id: Some(self.target.model_ref.model_id.clone()),
                pin_runtime: false,
                requirements: Some(requirements(1024 * 1024)),
                retention_hint: RuntimeRetentionHint::Ephemeral,
            })
            .unwrap()
    }
    fn warm_input<'a>(
        &self,
        lease: &'a RuntimeReservationLease,
    ) -> CandleCpuResidencyWarmAttemptRequest<'a> {
        CandleCpuResidencyWarmAttemptRequest {
            request: self.request.clone(),
            target: self.target.clone(),
            decision: self.decision.clone(),
            identity: CandleCpuWarmAttemptIdentity {
                workflow_id: "native-residency",
                workflow_run_id: "run",
                node_id: "node",
                task_id: "embedding",
                attempt_id: "attempt",
                execution_request_id: "selected-embedding-test",
                candidate_id: "candidate",
                reservation_lease_id: lease.reservation_id,
            },
            lease,
        }
    }
}

#[tokio::test]
async fn actual_load_only_capability_survives_two_warm_task_acks_then_stops_original_guard() {
    let f = fixture();
    assert!(!f.gateway.is_ready().await);
    let mut idle = f.load().await;
    let first_owner = idle.owner_facts();
    assert!(idle.matches_target(&f.target));
    assert!(idle.load_duration() > Duration::ZERO);
    assert!(
        f.gateway.backend.try_read().is_ok(),
        "no writer retained between turns"
    );
    let resident = f.registry.snapshot().runtimes[0]
        .model_resource_residency
        .clone();
    let idle_claims = f.registry.snapshot().runtimes[0]
        .active_reservation_claims
        .clone();
    let mut outputs = Vec::new();
    for (index, owner) in ["first-task", "second-task"].into_iter().enumerate() {
        assert!(f
            .registry
            .release_reservation(f.guard.reservation_id)
            .is_err());
        let task = f.task(owner);
        let (output, receipt) = f
            .gateway
            .execute_resident_cpu_warm_attempt(
                idle,
                f.warm_input(&task),
                InferenceExecutionCancellationHandle::running(),
            )
            .await
            .unwrap();
        assert_eq!(
            receipt.previous_owner_facts().generation,
            first_owner.generation + 2 * index as u64
        );
        assert_eq!(
            receipt.current_owner_facts().generation,
            first_owner.generation + 2 * (index + 1) as u64
        );
        let (disposition, returned) = receipt.release_retained_with_residency().await.unwrap();
        assert_eq!(
            disposition.decision,
            pantograph_runtime_registry::RuntimeRetentionDecision::Retain
        );
        assert_eq!(
            returned.owner_facts().loaded_instance,
            first_owner.loaded_instance
        );
        let snapshot = f.registry.snapshot();
        assert_eq!(snapshot.reservations, [f.guard.clone()]);
        assert_eq!(snapshot.runtimes[0].active_reservation_claims, idle_claims);
        assert_eq!(snapshot.runtimes[0].model_resource_residency, resident);
        outputs.push(output);
        idle = returned;
    }
    assert_eq!(outputs[0], outputs[1]);
    let outcome = idle
        .stop(InferenceExecutionCancellationHandle::running())
        .await
        .unwrap();
    assert_eq!(
        outcome.disposition.decision,
        pantograph_runtime_registry::RuntimeRetentionDecision::Evict
    );
    assert!(outcome.stop_duration > Duration::ZERO);
    assert!(outcome.acknowledgement_duration > Duration::ZERO);
    assert!(!f.gateway.is_ready().await);
    let stopped = f.registry.snapshot();
    assert!(stopped.reservations.is_empty());
    assert!(stopped.runtimes[0].model_resource_residency.is_none());
    assert!(stopped.runtimes[0].runtime_instance_id.is_none());
}

#[tokio::test]
async fn resident_receipt_cannot_use_ordinary_cleanup_and_lost_capability_stays_fenced() {
    let f = fixture();
    let idle = f.load().await;
    let task = f.task("task");
    let (_, receipt) = f
        .gateway
        .execute_resident_cpu_warm_attempt(
            idle,
            f.warm_input(&task),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let before = f.registry.snapshot().reservations;
    assert!(receipt.release_retained().await.is_err());
    assert_eq!(before, f.registry.snapshot().reservations);
    for lease in [&f.guard, &task] {
        assert!(f
            .registry
            .release_reservation(lease.reservation_id)
            .is_err());
        assert!(f.registry.acquire_execution_custody(lease).is_err());
    }
    // Test teardown stops physical work; it supplies no registry recovery/ACK.
    f.gateway.stop().await.unwrap();
    assert_eq!(before, f.registry.snapshot().reservations);
}

#[tokio::test]
async fn cold_bounds_and_foreign_guard_refuse_before_native_owner_publication() {
    let f = fixture();
    let mut input = f.cold_input();
    let InferenceExecutionInput::Embedding { texts } = &mut input.request.input else {
        unreachable!()
    };
    texts[0] = "x".repeat(4097);
    assert!(f
        .gateway
        .load_cpu_idle_residency(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut changed = f.guard.clone();
    changed.created_at_ms = changed.created_at_ms.saturating_add(1);
    let mut input = f.cold_input();
    input.guard = &changed;
    assert!(f
        .gateway
        .load_cpu_idle_residency(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert!(!f.gateway.is_ready().await);
    assert!(f.registry.snapshot().runtimes[0]
        .model_resource_residency
        .is_none());
    assert_eq!(
        f.registry.reservation_lease(f.guard.reservation_id),
        Some(f.guard.clone())
    );
    // Both refusals precede load-started custody; the valid original can load.
    f.load()
        .await
        .stop(InferenceExecutionCancellationHandle::running())
        .await
        .unwrap();
}

struct ControlledCancellation(Arc<AtomicBool>);
impl crate::InferenceExecutionCancellationSignal for ControlledCancellation {
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if self.0.load(Ordering::Acquire) {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "controlled post-load native cancellation".into(),
            ))
        } else {
            crate::InferenceExecutionCancellationSnapshot::running()
        }
    }
}

async fn controlled_post_load_loss(abandon: bool) {
    let f = fixture();
    let reached = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let hook_reached = reached.clone();
    let hook_gate = gate.clone();
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(move |phase| {
        if phase == "residency_after_load" {
            hook_reached.notify_one();
            let mut open = hook_gate.0.lock().unwrap();
            while !*open {
                open = hook_gate.1.wait(open).unwrap();
            }
        }
    }));
    let cancelled = Arc::new(AtomicBool::new(false));
    let host = InferenceExecutionCancellationHandle::with_signal(Arc::new(ControlledCancellation(
        cancelled.clone(),
    )));
    let gateway = f.gateway.clone();
    let registry = f.registry.clone();
    let request = f.request.clone();
    let target = f.target.clone();
    let decision = f.decision.clone();
    let guard = f.guard.clone();
    let mut caller = tokio::spawn(async move {
        gateway
            .load_cpu_idle_residency(
                registry,
                CandleCpuColdResidencyRequest {
                    request,
                    target,
                    decision,
                    guard: &guard,
                },
                host,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(10), reached.notified())
        .await
        .unwrap();
    // This barrier follows a real completed load and worker drain. The signal
    // is native-only; no public Running task or interrupted compute is claimed.
    assert!(f.gateway.backend.try_read().is_err());
    assert!(f
        .registry
        .release_reservation(f.guard.reservation_id)
        .is_err());
    assert!(f.registry.acquire_execution_custody(&f.guard).is_err());
    if abandon {
        caller.abort();
        assert!(matches!((&mut caller).await, Err(error) if error.is_cancelled()));
    } else {
        cancelled.store(true, Ordering::Release);
    }
    assert!(f.gateway.backend.try_write().is_err());
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    if !abandon {
        assert!(matches!(caller.await, Ok(Err(_))));
    }
    let backend = tokio::time::timeout(Duration::from_secs(10), f.gateway.backend.write())
        .await
        .unwrap();
    assert!(backend.is_ready());
    assert!(backend.resident_cpu_calibration_instance().is_some());
    drop(backend);
    let snapshot = f.registry.snapshot();
    assert_eq!(snapshot.reservations, [f.guard.clone()]);
    assert_eq!(
        snapshot.runtimes[0].active_reservation_claims[0].claims,
        requirements(8 * 1024 * 1024).claims
    );
    assert!(snapshot.runtimes[0].model_resource_residency.is_none());
    assert!(f
        .registry
        .release_reservation(f.guard.reservation_id)
        .is_err());
    assert!(f.registry.acquire_execution_custody(&f.guard).is_err());
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = None;
    // Physical test teardown supplies no registry stop authority or recovery.
    f.gateway.stop().await.unwrap();
    assert_eq!(snapshot.reservations, f.registry.snapshot().reservations);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_completed_load_native_cancellation_keeps_original_peak_fenced() {
    controlled_post_load_loss(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_load_collector_cannot_drop_supervised_writer_or_guard_custody() {
    controlled_post_load_loss(true).await;
}

#[tokio::test]
async fn actual_idle_guard_stop_refuses_retainer_and_private_owner_change_without_cleanup() {
    for stale_owner in [false, true] {
        let f = fixture();
        let idle = f.load().await;
        if stale_owner {
            // Controlled private-owner invalidation, not an actual load/stop.
            f.gateway
                .candle_cpu_calibration
                .as_ref()
                .unwrap()
                .invalidate();
        } else {
            let mut retaining_request = RuntimeReservationRequest {
                runtime_id: "candle".into(),
                workflow_id: "user-session".into(),
                reservation_owner_id: Some("user-retainer".into()),
                usage_profile: None,
                model_id: Some(f.target.model_ref.model_id.clone()),
                pin_runtime: false,
                requirements: Some(requirements(1024)),
                retention_hint: RuntimeRetentionHint::KeepAlive,
            };
            let retainer = f
                .registry
                .acquire_reservation(retaining_request.clone())
                .unwrap();
            retaining_request.pin_runtime = true;
            let pinned = f.registry.acquire_reservation(retaining_request).unwrap();
            assert_ne!(retainer.reservation_id, pinned.reservation_id);
        }
        let before = f.registry.snapshot();
        assert!(idle
            .stop(InferenceExecutionCancellationHandle::running())
            .await
            .is_err());
        let after = f.registry.snapshot();
        assert_eq!(before.runtimes, after.runtimes);
        assert_eq!(before.reservations, after.reservations);
        assert!(f.gateway.is_ready().await);
        assert!(f
            .registry
            .release_reservation(f.guard.reservation_id)
            .is_err());
        f.gateway.stop().await.unwrap();
    }
}

#[tokio::test]
async fn foreign_gateway_cannot_adopt_actual_idle_capability_or_release_its_guard() {
    let f = fixture();
    let idle = f.load().await;
    let task = f.task("task");
    let foreign = Arc::new(
        InferenceGateway::new_calibrated_candle_cpu(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
        )
        .unwrap(),
    );
    let before = f.registry.snapshot();
    assert!(foreign
        .execute_resident_cpu_warm_attempt(
            idle,
            f.warm_input(&task),
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let after = f.registry.snapshot();
    assert_eq!(before.runtimes, after.runtimes);
    assert_eq!(before.reservations, after.reservations);
    assert!(f
        .registry
        .release_reservation(f.guard.reservation_id)
        .is_err());
    assert!(!foreign.is_ready().await);
    f.gateway.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controlled_idle_stop_supervision_and_ack_faults_preserve_original_guard_and_resident_charge(
) {
    for before_stop in [true, false] {
        let f = fixture();
        let idle = f.load().await;
        let before = f.registry.snapshot();
        if before_stop {
            // Controlled supervisor fault; this is not a Candle stop failure.
            *f.gateway
                .candle_cpu_calibration
                .as_ref()
                .unwrap()
                .attempt_hook
                .lock()
                .unwrap() = Some(Arc::new(|phase| {
                if phase == "eviction_before_stop" {
                    panic!("controlled idle stop supervision fault");
                }
            }));
        } else {
            // Controlled producer sequence exhaustion after a genuine load.
            // Physical stop succeeds, but it cannot obtain a valid stop ACK.
            f.gateway
                .resident_observation_sequence
                .store(u64::MAX, Ordering::Release);
        }
        assert!(idle
            .stop(InferenceExecutionCancellationHandle::running())
            .await
            .is_err());
        assert_eq!(f.gateway.is_ready().await, before_stop);
        let fenced = f.registry.snapshot();
        assert_eq!(fenced.reservations, before.reservations);
        assert_eq!(
            fenced.runtimes[0].active_reservation_claims,
            before.runtimes[0].active_reservation_claims
        );
        assert_eq!(
            fenced.runtimes[0].model_resource_residency,
            before.runtimes[0].model_resource_residency
        );
        assert_eq!(fenced.runtimes[0].status, RuntimeRegistryStatus::Stopping);
        assert!(f
            .registry
            .release_reservation(f.guard.reservation_id)
            .is_err());
        assert!(f.registry.acquire_execution_custody(&f.guard).is_err());
        *f.gateway
            .candle_cpu_calibration
            .as_ref()
            .unwrap()
            .attempt_hook
            .lock()
            .unwrap() = None;
        // A separate test teardown proves no authority to clear registry fences.
        f.gateway.stop().await.unwrap();
        assert_eq!(f.registry.snapshot().reservations, before.reservations);
    }
}
