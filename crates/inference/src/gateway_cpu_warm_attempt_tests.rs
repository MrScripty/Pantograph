use super::*;
use crate::CandleCpuCalibrationConfig;
use pantograph_runtime_registry::{
    RuntimeModelResidentEstimate, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim, RuntimeRetentionDecision, RuntimeRetentionHint,
};
use std::time::Duration;

struct Fixture {
    _directory: tempfile::TempDir,
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    request: InferenceExecutionRequest,
    target: crate::PumasArtifactLoadTarget,
    decision: crate::BackendExecutionDecision,
    task: RuntimeReservationLease,
    successor: RuntimeReservationLease,
    owner: CandleCpuCleanupOwner,
    cold: InferenceExecutionResult,
}
fn identity(lease: &RuntimeReservationLease) -> CandleCpuWarmAttemptIdentity<'_> {
    CandleCpuWarmAttemptIdentity {
        workflow_id: &lease.workflow_id,
        workflow_run_id: "run",
        node_id: "node",
        task_id: "embedding",
        attempt_id: "attempt",
        execution_request_id: "selected-embedding-test",
        candidate_id: "candidate",
        reservation_lease_id: lease.reservation_id,
    }
}
impl Fixture {
    fn input(&self) -> CandleCpuWarmAttemptRequest<'_> {
        CandleCpuWarmAttemptRequest {
            request: self.request.clone(),
            target: self.target.clone(),
            decision: self.decision.clone(),
            identity: identity(&self.task),
            lease: &self.task,
            expected_owner: &self.owner,
        }
    }
}
async fn fixture() -> Fixture {
    let (directory, mut request, target, decision) =
        crate::selected_embedding_execution::fixture(8);
    if let crate::InferenceExecutionInput::Embedding { texts } = &mut request.input {
        texts.truncate(1);
    }
    let gateway = Arc::new(
        InferenceGateway::new_calibrated_candle_cpu(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
        )
        .unwrap(),
    );
    let cold = gateway
        .execute_selected_embedding_with_cancellation(
            request.clone(),
            target.clone(),
            decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let registry = Arc::new(RuntimeRegistry::new());
    // Synthetic operator declaration; no allocator/phase-capacity qualification.
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "candle".into(),
            model_id: target.local_load_path.clone(),
            requirements: RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(32768),
            ]),
        }])
        .unwrap();
    let lease = |owner: &str, retention_hint| {
        registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "candle".into(),
                workflow_id: "native-attempt".into(),
                reservation_owner_id: Some(owner.into()),
                usage_profile: None,
                model_id: Some(target.model_ref.model_id.clone()),
                pin_runtime: false,
                requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                    RuntimeReservationResourceClaim::ram_bytes(256),
                ])),
                retention_hint,
            })
            .unwrap()
    };
    let task = lease("task", RuntimeRetentionHint::Ephemeral);
    let successor = lease("session", RuntimeRetentionHint::KeepAlive);
    let owner = gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    Fixture {
        _directory: directory,
        gateway,
        registry,
        request,
        target,
        decision,
        task,
        successor,
        owner,
        cold,
    }
}
fn state(
    registry: &RuntimeRegistry,
) -> (
    Vec<pantograph_runtime_registry::RuntimeRegistryRuntimeSnapshot>,
    Vec<RuntimeReservationLease>,
) {
    let s = registry.snapshot();
    (s.runtimes, s.reservations)
}

#[tokio::test]
async fn real_warm_attempt_mints_exact_transition_and_drained_receipt_releases_only_task() {
    let f = fixture().await;
    let loaded = f.gateway.cpu_calibration_instance_for_test().unwrap();
    let before = state(&f.registry);
    let (result, receipt) = f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert_eq!(result, f.cold);
    assert_eq!(receipt.execution_request_id(), "selected-embedding-test");
    assert_eq!(
        receipt.binding.owner.epoch_for_test(),
        f.owner.epoch_for_test() + 2
    );
    let crate::InferenceExecutionInput::Embedding { texts } = &f.request.input else {
        panic!("fixture embedding input")
    };
    assert_eq!(
        receipt.input_fingerprint(),
        *blake3::hash(texts[0].as_bytes()).as_bytes()
    );
    assert_eq!(
        loaded,
        f.gateway.cpu_calibration_instance_for_test().unwrap()
    );
    assert_eq!(before, state(&f.registry));
    assert_eq!(
        receipt.release_retained().await.unwrap().decision,
        RuntimeRetentionDecision::Retain
    );
    let after = state(&f.registry);
    assert_eq!(after.1, vec![f.successor.clone()]);
    assert_eq!(
        before.0[0].model_resource_residency,
        after.0[0].model_resource_residency
    );
    let owner = f
        .gateway
        .publish_resident_cpu_cleanup_owner(&f.registry)
        .await
        .unwrap();
    let mut last = f.input();
    last.identity = identity(&f.successor);
    last.lease = &f.successor;
    last.expected_owner = &owner;
    let before_last = state(&f.registry);
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            last,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert_eq!(before_last, state(&f.registry));
    f.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn changed_lease_and_unbounded_request_tags_refuse_before_load() {
    let f = fixture().await;
    let before = state(&f.registry);
    let mut changed = f.task.clone();
    changed.created_at_ms = changed.created_at_ms.wrapping_add(1);
    let mut input = f.input();
    input.lease = &changed;
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let oversized = "x".repeat(129);
    let mut input = f.input();
    input.identity.attempt_id = &oversized;
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut input = f.input();
    input.request.request_id = Some("foreign".into());
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut input = f.input();
    input.identity.reservation_lease_id += 1;
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    let fresh = f
        .gateway
        .publish_resident_cpu_cleanup_owner(&f.registry)
        .await
        .unwrap();
    assert_eq!(fresh.epoch_for_test(), f.owner.epoch_for_test());
    f.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn changed_recipe_refuses_before_cold_replacement_and_keeps_claims() {
    let f = fixture().await;
    let instance = f
        .gateway
        .backend
        .read()
        .await
        .resident_cpu_calibration_instance()
        .unwrap();
    let before = state(&f.registry);
    let path = f._directory.path().join("config.json");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.push(b' ');
    std::fs::write(path, bytes).unwrap();
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let backend = f.gateway.backend.read().await;
    assert!(backend.is_ready());
    assert_eq!(backend.resident_cpu_calibration_instance(), Some(instance));
    drop(backend);
    assert_eq!(before, state(&f.registry));
    f.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn receipt_loss_and_intervening_actual_load_cannot_free_claims() {
    let f = fixture().await;
    let before = state(&f.registry);
    let (_, receipt) = f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    f.gateway
        .execute_selected_embedding_with_cancellation(
            f.request.clone(),
            f.target.clone(),
            f.decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert!(receipt.release_retained().await.is_err());
    assert_eq!(before, state(&f.registry));
    let fresh = f
        .gateway
        .publish_resident_cpu_cleanup_owner(&f.registry)
        .await
        .unwrap();
    let mut input = f.input();
    input.expected_owner = &fresh;
    let before_drop = state(&f.registry);
    let (_, receipt) = f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    drop(receipt);
    assert_eq!(before_drop, state(&f.registry));
    f.gateway.stop().await.unwrap();
}

struct CancelSignal(Arc<AtomicBool>);
impl crate::InferenceExecutionCancellationSignal for CancelSignal {
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if self.0.load(Ordering::Acquire) {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "test cancellation".into(),
            ))
        } else {
            crate::InferenceExecutionCancellationSnapshot::running()
        }
    }
}
async fn controlled_loss(phase: &'static str, abandon: bool, end_lease: bool, custodied: bool) {
    let f = fixture().await;
    let started = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let hook_gate = gate.clone();
    let hook_started = started.clone();
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(move |current| {
        if current == phase {
            hook_started.notify_one();
            let mut ready = hook_gate.0.lock().unwrap();
            while !*ready {
                ready = hook_gate.1.wait(ready).unwrap();
            }
        }
    }));
    let cancellation = Arc::new(AtomicBool::new(false));
    let handle = InferenceExecutionCancellationHandle::with_signal(Arc::new(CancelSignal(
        cancellation.clone(),
    )));
    let gateway = f.gateway.clone();
    let registry = f.registry.clone();
    let request = f.request.clone();
    let target = f.target.clone();
    let decision = f.decision.clone();
    let lease = f.task.clone();
    let owner = f.owner.clone();
    let before = state(&f.registry);
    let task = tokio::spawn(async move {
        let input = CandleCpuWarmAttemptRequest {
            request,
            target,
            decision,
            identity: identity(&lease),
            lease: &lease,
            expected_owner: &owner,
        };
        if custodied {
            gateway
                .execute_custodied_cpu_warm_attempt(registry, input, handle)
                .await
        } else {
            gateway
                .execute_retained_cpu_warm_attempt(registry, input, handle)
                .await
        }
    });
    let mut task = Some(task);
    tokio::time::timeout(Duration::from_secs(5), started.notified())
        .await
        .unwrap();
    assert!(f.gateway.backend.try_write().is_err());
    if abandon {
        task.as_ref().unwrap().abort();
    } else if !end_lease {
        cancellation.store(true, Ordering::Release);
    }
    if custodied {
        assert!(f
            .registry
            .release_reservation(f.task.reservation_id)
            .is_err());
        assert!(f
            .registry
            .update_reservation_retention_hint(
                f.task.reservation_id,
                RuntimeRetentionHint::KeepAlive
            )
            .is_err());
        assert!(f
            .registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "candle".into(),
                workflow_id: f.task.workflow_id.clone(),
                reservation_owner_id: f.task.reservation_owner_id.clone(),
                usage_profile: None,
                model_id: f.task.model_id.clone(),
                pin_runtime: false,
                requirements: None,
                retention_hint: RuntimeRetentionHint::Ephemeral
            })
            .is_err());
    } else if end_lease {
        f.registry
            .release_reservation(f.task.reservation_id)
            .unwrap();
    }
    let expected = if end_lease && !custodied {
        state(&f.registry)
    } else {
        before
    };
    if abandon {
        assert!(matches!(task.take().unwrap().await, Err(error) if error.is_cancelled()));
    }
    assert!(
        f.gateway.backend.try_write().is_err(),
        "actual supervised writer remains held until drain"
    );
    assert_eq!(expected, state(&f.registry));
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    let mut receipt = None;
    if !abandon {
        let outcome = task.take().unwrap().await;
        if custodied && end_lease {
            receipt = Some(outcome.unwrap().unwrap().1);
        } else {
            assert!(matches!(outcome, Ok(Err(_))));
        }
    }
    let backend = tokio::time::timeout(Duration::from_secs(5), f.gateway.backend.write())
        .await
        .unwrap();
    assert!(backend.is_ready());
    assert!(
        backend.resident_cpu_calibration_instance().is_some(),
        "real jobs are idle only after actual drain"
    );
    drop(backend);
    assert_eq!(expected, state(&f.registry));
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = None;
    if let Some(receipt) = receipt {
        receipt.release_retained().await.unwrap();
    } else if custodied {
        assert!(f
            .registry
            .release_reservation(f.task.reservation_id)
            .is_err());
    }
    f.gateway.stop().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_at_real_load_forward_and_drain_mints_no_receipt() {
    for phase in ["load", "forward", "drain"] {
        controlled_loss(phase, false, false, false).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_collector_keeps_actual_writer_through_real_load_forward_and_drain() {
    for phase in ["load", "forward", "drain"] {
        controlled_loss(phase, true, false, false).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn external_lease_release_during_real_work_refuses_receipt_without_readmitting_cleanup() {
    // Read-only preflight does not pin ordinary registry mutations. The caller
    // must own lease lifecycle; losing it cannot produce a valid drain receipt.
    controlled_loss("forward", false, true, false).await;
}

#[tokio::test]
async fn oversized_deep_and_wide_metadata_refuse_before_actual_load() {
    let f = fixture().await;
    let before = state(&f.registry);
    let mut input = f.input();
    input.request.extra_options = serde_json::Value::String("x".repeat(65537));
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut input = f.input();
    input.request.extra_options = serde_json::Value::Array(vec![serde_json::Value::Null; 2049]);
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut input = f.input();
    let mut deep = serde_json::Value::Null;
    for _ in 0..33 {
        deep = serde_json::Value::Array(vec![deep]);
    }
    input.request.extra_options = deep;
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut input = f.input();
    input.target.library_root_id = Some("x".repeat(65537));
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let mut input = f.input();
    input.decision.selected_model_ref.as_mut().unwrap().revision = Some("x".repeat(65537));
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    let fresh = f
        .gateway
        .publish_resident_cpu_cleanup_owner(&f.registry)
        .await
        .unwrap();
    assert_eq!(fresh.epoch_for_test(), f.owner.epoch_for_test());
    f.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn actual_foreign_owner_and_pending_lease_refuse_without_epoch_change() {
    let f = fixture().await;
    let foreign = Arc::new(
        InferenceGateway::new_calibrated_candle_cpu(
            CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
        )
        .unwrap(),
    );
    foreign
        .execute_selected_embedding_with_cancellation(
            f.request.clone(),
            f.target.clone(),
            f.decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let owner = foreign
        .publish_resident_cpu_cleanup_owner(&RuntimeRegistry::new())
        .await
        .unwrap();
    let before = state(&f.registry);
    let mut input = f.input();
    input.expected_owner = &owner;
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            input,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    let request = RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: f.task.workflow_id.clone(),
        reservation_owner_id: f.task.reservation_owner_id.clone(),
        usage_profile: None,
        model_id: f.task.model_id.clone(),
        pin_runtime: false,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(256),
        ])),
        retention_hint: RuntimeRetentionHint::Ephemeral,
    };
    let evaluation = f.registry.evaluate_reservation(request.clone()).unwrap();
    let (_, custody) = f
        .registry
        .acquire_reservation_provisional(request, evaluation.observation(), |_| Ok::<_, ()>(()))
        .unwrap();
    let pending = state(&f.registry);
    assert!(f
        .gateway
        .execute_retained_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert_eq!(pending, state(&f.registry));
    drop(custody);
    let fresh = f
        .gateway
        .publish_resident_cpu_cleanup_owner(&f.registry)
        .await
        .unwrap();
    assert_eq!(fresh.epoch_for_test(), f.owner.epoch_for_test());
    foreign.stop().await.unwrap();
    f.gateway.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custody_excludes_competing_release_replacement_and_retention_through_actual_phases() {
    for phase in ["load", "forward", "drain"] {
        controlled_loss(phase, false, true, true).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn custodied_cancellation_and_collector_loss_keep_claim_fenced_after_physical_drain() {
    for phase in ["load", "forward", "drain"] {
        controlled_loss(phase, false, false, true).await;
        controlled_loss(phase, true, false, true).await;
    }
}
#[tokio::test]
async fn advisory_epoch_tampering_cannot_forge_native_epoch_or_restore_stale_cleanup() {
    for offset in [0, 1, 2, 100] {
        let f = fixture().await;
        let old = f.owner.serial_facts();
        old.generation_handle
            .store(old.generation + offset, Ordering::Release);
        let actual = f
            .gateway
            .resident_cpu_serial_owner(&f.target.model_ref)
            .unwrap()
            .serial_facts();
        assert_eq!(actual.generation, old.generation);
        let (_, receipt) = f
            .gateway
            .execute_custodied_cpu_warm_attempt(
                f.registry.clone(),
                f.input(),
                InferenceExecutionCancellationHandle::running(),
            )
            .await
            .unwrap();
        assert_eq!(receipt.current_owner_facts().generation, old.generation + 2);
        old.generation_handle
            .store(old.generation, Ordering::Release); // replay stale advisory generation
        assert!(f
            .gateway
            .release_retained_cpu_reservation(&f.registry, &f.task, &f.owner)
            .await
            .is_err());
        receipt.release_retained().await.unwrap(); // private actual current epoch still authoritative
        assert!(f
            .registry
            .reservation_lease(f.task.reservation_id)
            .is_none());
        assert_eq!(
            f.registry.reservation_lease(f.successor.reservation_id),
            Some(f.successor)
        );
        f.gateway.stop().await.unwrap();
    }
}

async fn last_receipt(f: &Fixture) -> CandleCpuDrainedAttempt {
    let (warm, receipt) = f
        .gateway
        .execute_custodied_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert_eq!(warm, f.cold);
    f.registry
        .release_reservation(f.successor.reservation_id)
        .unwrap();
    receipt
}
fn competing_request() -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: "competing".into(),
        reservation_owner_id: Some("competing".into()),
        usage_profile: None,
        model_id: None,
        pin_runtime: false,
        requirements: None,
        retention_hint: RuntimeRetentionHint::Ephemeral,
    }
}
#[tokio::test]
async fn actual_last_lease_eviction_clears_only_after_stop_and_rejects_undelivered_frames() {
    let f = fixture().await;
    let receipt = last_receipt(&f).await;
    // Actual owner snapshot/sequence sampled under the backend reader but not
    // delivered. Stop must exceed this counter, not only delivered sequence.
    let backend = f.gateway.backend.read().await;
    let lifecycle = f.gateway.runtime_lifecycle.read().await;
    let delayed = pantograph_runtime_registry::RuntimeProducerObservation {
        source_id: f.gateway.resident_source_id.clone(),
        sequence: f
            .gateway
            .resident_observation_sequence
            .fetch_add(1, Ordering::AcqRel)
            + 1,
        allocation_state: pantograph_runtime_registry::RuntimeProducerAllocationState::Resident,
        observation: pantograph_runtime_registry::RuntimeObservation {
            runtime_id: "candle".into(),
            display_name: "Candle".into(),
            backend_keys: vec!["candle".into()],
            model_id: Some(f.target.local_load_path.clone()),
            runtime_instance_id: lifecycle.runtime_instance_id.clone(),
            status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    };
    drop(lifecycle);
    drop(backend);
    let result = receipt
        .evict_last(InferenceExecutionCancellationHandle::running())
        .await
        .unwrap();
    assert_eq!(result.decision, RuntimeRetentionDecision::Evict);
    assert!(!f.gateway.is_ready().await);
    assert!(!f.gateway.is_embedding_mode().await);
    assert!(f.gateway.current_runtime_config.read().await.is_none());
    let stopped = state(&f.registry);
    assert!(stopped.1.is_empty());
    assert!(stopped.0[0].model_resource_residency.is_none());
    assert_eq!(
        stopped.0[0].status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
    );
    assert!(f.registry.observe_runtime_producer(delayed).is_err());
    assert_eq!(stopped, state(&f.registry));
    assert!(f
        .gateway
        .resident_cpu_serial_owner(&f.target.model_ref)
        .is_none());
}
#[tokio::test]
async fn native_eviction_refuses_retaining_successor_stale_owner_and_precancelled_host() {
    let f = fixture().await;
    let (_, receipt) = f
        .gateway
        .execute_custodied_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let before = state(&f.registry);
    assert!(receipt
        .evict_last(InferenceExecutionCancellationHandle::running())
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    assert!(f.gateway.is_ready().await);
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
    f.gateway.stop().await.unwrap();
    let f = fixture().await;
    let receipt = last_receipt(&f).await;
    f.gateway
        .execute_selected_embedding_with_cancellation(
            f.request.clone(),
            f.target.clone(),
            f.decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let before = state(&f.registry);
    assert!(receipt
        .evict_last(InferenceExecutionCancellationHandle::running())
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    assert!(f.gateway.is_ready().await);
    f.gateway.stop().await.unwrap();
    let f = fixture().await;
    let receipt = last_receipt(&f).await;
    let flag = Arc::new(AtomicBool::new(true));
    let before = state(&f.registry);
    assert!(receipt
        .evict_last(InferenceExecutionCancellationHandle::with_signal(Arc::new(
            CancelSignal(flag)
        )))
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    assert!(f.gateway.is_ready().await);
    f.gateway.stop().await.unwrap();
}
async fn controlled_eviction(phase: &'static str, abandon: bool, stale_owner: bool) {
    let f = fixture().await;
    let receipt = last_receipt(&f).await;
    let started = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let hook_started = started.clone();
    let hook_gate = gate.clone();
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(move |current| {
        if current == phase {
            hook_started.notify_one();
            let (lock, wake) = &*hook_gate;
            let mut open = lock.lock().unwrap();
            while !*open {
                open = wake.wait(open).unwrap();
            }
        }
    }));
    let flag = Arc::new(AtomicBool::new(false));
    let host =
        InferenceExecutionCancellationHandle::with_signal(Arc::new(CancelSignal(flag.clone())));
    let mut task = tokio::spawn(async move { receipt.evict_last(host).await });
    tokio::time::timeout(Duration::from_secs(10), started.notified())
        .await
        .unwrap();
    let pending = state(&f.registry);
    assert_eq!(pending.1, vec![f.task.clone()]);
    assert!(pending.0[0].model_resource_residency.is_some());
    assert_eq!(
        pending.0[0].status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Stopping
    );
    assert!(f.gateway.backend.try_read().is_err());
    assert!(f.registry.acquire_reservation(competing_request()).is_err());
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
    if stale_owner {
        f.gateway
            .candle_cpu_calibration
            .as_ref()
            .unwrap()
            .invalidate();
    }
    if abandon {
        task.abort();
    } else if !stale_owner {
        flag.store(true, Ordering::Release);
    }
    if abandon {
        assert!((&mut task).await.unwrap_err().is_cancelled());
    }
    let (lock, wake) = &*gate;
    *lock.lock().unwrap() = true;
    wake.notify_all();
    if !abandon {
        let result = tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .unwrap()
            .unwrap();
        if phase == "eviction_before_stop" {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap().decision, RuntimeRetentionDecision::Evict);
        }
    }
    let backend = tokio::time::timeout(Duration::from_secs(10), f.gateway.backend.write())
        .await
        .unwrap();
    if phase == "eviction_before_stop" {
        assert!(backend.is_ready());
        assert_eq!(pending, state(&f.registry));
    } else {
        assert!(!backend.is_ready());
        let stopped = state(&f.registry);
        assert!(stopped.1.is_empty());
        assert!(stopped.0[0].model_resource_residency.is_none());
    }
    drop(backend);
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = None;
    if phase == "eviction_before_stop" {
        assert!(f.registry.acquire_reservation(competing_request()).is_err());
        f.gateway.stop().await.unwrap();
        assert_eq!(pending, state(&f.registry));
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_eviction_supervision_survives_cancel_and_collector_loss_at_both_stop_boundaries() {
    for phase in ["eviction_before_stop", "eviction_after_stop"] {
        for abandon in [false, true] {
            controlled_eviction(phase, abandon, false).await;
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn eviction_supervision_failure_leaves_accounting_and_runtime_fence() {
    let f = fixture().await;
    let receipt = last_receipt(&f).await;
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(|phase| {
        if phase == "eviction_before_stop" {
            panic!("controlled supervision fault before physical stop");
        }
    }));
    assert!(receipt
        .evict_last(InferenceExecutionCancellationHandle::running())
        .await
        .is_err());
    assert!(f.gateway.is_ready().await);
    assert_eq!(state(&f.registry).1, vec![f.task.clone()]);
    assert!(state(&f.registry).0[0].model_resource_residency.is_some());
    assert!(f.registry.acquire_reservation(competing_request()).is_err());
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = None;
    f.gateway.stop().await.unwrap();
    assert_eq!(state(&f.registry).1, vec![f.task.clone()]); // Ordinary teardown proves no ticket recovery.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn actual_eviction_rechecks_private_owner_immediately_before_stop() {
    controlled_eviction("eviction_before_stop", false, true).await;
}
#[tokio::test]
async fn actual_stopped_owner_with_failed_ack_remains_charged_and_fenced() {
    let f = fixture().await;
    let receipt = last_receipt(&f).await;
    f.gateway
        .resident_observation_sequence
        .store(u64::MAX, Ordering::Release);
    assert!(receipt
        .evict_last(InferenceExecutionCancellationHandle::running())
        .await
        .is_err());
    assert!(!f.gateway.is_ready().await);
    let pending = state(&f.registry);
    assert_eq!(pending.1, vec![f.task.clone()]);
    assert!(pending.0[0].model_resource_residency.is_some());
    assert_eq!(
        pending.0[0].status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Stopping
    );
    assert!(f.registry.acquire_reservation(competing_request()).is_err());
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
}
