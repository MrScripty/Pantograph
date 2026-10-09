//! Actual committed tiny CPU model; resource declarations remain synthetic.
use super::*;
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeHostRamCapacitySource, RuntimeObservation,
    RuntimeProducerAllocationState, RuntimeProducerObservation, RuntimeRegistryStatus,
    RuntimeResourceDomain, RuntimeResourceDomainBinding, RuntimeRetainedOwnerIdentity,
};

fn resident(f: &Fixture) -> pantograph_runtime_registry::RuntimeRegistryRuntimeSnapshot {
    f.registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap()
}
fn frame(f: &Fixture, sequence: u64) -> RuntimeProducerObservation {
    RuntimeProducerObservation {
        source_id: f.gateway.resident_source_id.clone(),
        sequence,
        allocation_state: RuntimeProducerAllocationState::Resident,
        observation: RuntimeObservation {
            runtime_id: "candle".into(),
            display_name: "Candle".into(),
            backend_keys: vec!["candle".into()],
            model_id: Some(f.target.local_load_path.clone()),
            runtime_instance_id: resident(f).runtime_instance_id,
            status: RuntimeRegistryStatus::Ready,
            last_error: None,
        },
    }
}
struct OpenOnDrop(Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);
impl OpenOnDrop {
    fn open(&self) {
        *self.0 .0.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.0 .1.notify_all();
    }
}
impl Drop for OpenOnDrop {
    fn drop(&mut self) {
        self.open();
    }
}
fn pause(f: &Fixture) -> (Arc<tokio::sync::Notify>, OpenOnDrop) {
    let entered = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let e = entered.clone();
    let g = gate.clone();
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(move |phase| {
        if phase == "retirement_before_drain" {
            e.notify_one();
            let mut open = g.0.lock().unwrap_or_else(|e| e.into_inner());
            while !*open {
                open = g.1.wait(open).unwrap_or_else(|e| e.into_inner());
            }
        }
    }));
    (entered, OpenOnDrop(gate))
}
fn stop(f: &Fixture) -> tokio::task::JoinHandle<Result<(), GatewayError>> {
    let g = f.gateway.clone();
    let r = f.registry.clone();
    tokio::spawn(async move { g.stop_and_publish_cpu_retirement(&r).await })
}

#[tokio::test]
async fn actual_native_stop_retains_charge_until_drain_then_ack_keeps_task_claims() {
    let f = fixture().await;
    let before = state(&f.registry);
    let (entered, gate) = pause(&f);
    let job = stop(&f);
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    assert!(f.gateway.backend.try_write().is_err());
    assert_eq!(state(&f.registry), before);
    gate.open();
    tokio::time::timeout(Duration::from_secs(5), job)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let after = resident(&f);
    assert_eq!(after.status, RuntimeRegistryStatus::Stopped);
    assert!(
        after.model_resource_residency.is_none()
            && !after.resident_resources_uncertain
            && after.models.is_empty()
    );
    assert_eq!(f.registry.snapshot().reservations, before.1);
    assert!(!f.gateway.is_ready().await);
    assert!(f
        .gateway
        .resident_cpu_serial_owner(&f.target.model_ref)
        .is_none());
    f.gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
    assert!(resident(&f).model_resource_residency.is_none());
}

#[tokio::test]
async fn cancelled_actual_stop_before_drain_has_no_receipt_and_retry_keeps_charge() {
    let f = fixture().await;
    let before = state(&f.registry);
    let (entered, gate) = pause(&f);
    let job = stop(&f);
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    job.abort();
    assert!(tokio::time::timeout(Duration::from_secs(5), job)
        .await
        .unwrap()
        .unwrap_err()
        .is_cancelled());
    gate.open();
    assert_eq!(state(&f.registry), before);
    assert!(f.gateway.is_ready().await);
    // Stop invalidated the actual calibration before cancellation; a second
    // drain cannot invent a new receipt for that uncertain previous generation.
    assert!(f
        .gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    assert!(!f.gateway.is_ready().await);
    assert_eq!(
        resident(&f).model_resource_residency,
        before.0[0].model_resource_residency
    );
    assert_eq!(f.registry.snapshot().reservations, before.1);
}

#[tokio::test]
async fn checked_stop_sequence_overflow_does_not_retire_actual_charge() {
    let f = fixture().await;
    let before = state(&f.registry);
    f.gateway
        .resident_observation_sequence
        .store(u64::MAX, Ordering::Release);
    assert!(f
        .gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    assert!(!f.gateway.is_ready().await);
    assert_eq!(state(&f.registry), before);
    assert_eq!(
        f.gateway
            .resident_observation_sequence
            .load(Ordering::Acquire),
        u64::MAX
    );
}

#[tokio::test]
async fn foreign_actual_native_owner_cannot_ack_same_bytes_registry_charge() {
    let f = fixture().await;
    let before = state(&f.registry);
    let other = InferenceGateway::new_calibrated_candle_cpu(
        CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
    )
    .unwrap();
    other
        .execute_selected_embedding_with_cancellation(
            f.request.clone(),
            f.target.clone(),
            f.decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert!(other
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    assert!(!other.is_ready().await);
    assert_eq!(state(&f.registry), before);
    f.gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
}

#[tokio::test]
async fn retired_native_receipt_binds_exact_private_owner_and_is_not_a_readiness_flag() {
    let f = fixture().await;
    let other = fixture().await;
    let proof = f
        .gateway
        .backend
        .write()
        .await
        .stop_with_cpu_retirement()
        .await
        .unwrap()
        .unwrap();
    assert!(!proof.matches(&other.owner));
    assert!(f
        .gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    assert!(resident(&f).model_resource_residency.is_some());
    other
        .gateway
        .stop_and_publish_cpu_retirement(&other.registry)
        .await
        .unwrap();
}

#[tokio::test]
async fn stale_resident_frame_cannot_resurrect_stop_and_no_release_frame_survives_reload() {
    let f = fixture().await;
    let old = frame(
        &f,
        f.gateway
            .resident_observation_sequence
            .fetch_add(1, Ordering::AcqRel)
            + 1,
    );
    f.gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
    assert!(f.registry.observe_runtime_producer(old.clone()).is_err());
    assert!(resident(&f).model_resource_residency.is_none());
    // There is deliberately no deferred Candle Released snapshot to deliver
    // during load-begun/new-resident-not-yet-published or after publication.
    assert!(f
        .gateway
        .resident_lifecycle_snapshots()
        .await
        .iter()
        .all(|s| s.lifecycle.runtime_id.as_deref() != Some("candle")));
    f.gateway
        .execute_selected_embedding_with_cancellation(
            f.request.clone(),
            f.target.clone(),
            f.decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert!(f
        .gateway
        .resident_lifecycle_snapshots()
        .await
        .iter()
        .all(|s| s.lifecycle.runtime_id.as_deref() != Some("candle")));
    f.gateway
        .publish_resident_cpu_cleanup_owner(&f.registry)
        .await
        .unwrap();
    let reloaded = resident(&f).model_resource_residency.unwrap();
    assert!(f.registry.observe_runtime_producer(old).is_err());
    assert_eq!(resident(&f).model_resource_residency, Some(reloaded));
    f.gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
}

#[derive(Debug)]
struct Ceiling;
impl RuntimeHostRamCapacitySource for Ceiling {
    fn capacity_ceiling_bytes(&self) -> Option<u64> {
        Some(131072)
    }
}
#[tokio::test]
async fn declared_envelope_unknown_conversion_is_not_an_accepted_native_release() {
    let f = fixture().await;
    f.registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "host".into(),
            total_bytes: 131072,
            safety_margin_bytes: 0,
            bindings: vec![RuntimeResourceDomainBinding {
                runtime_id: "candle".into(),
                resource_kind: RuntimeAdmissionResourceKind::RamBytes,
            }],
        })
        .unwrap();
    f.registry.bind_host_ram_capacity_source(Arc::new(Ceiling));
    let instance = resident(&f).runtime_instance_id.unwrap();
    let mut envelope = f
        .registry
        .acquire_execution_custody(&f.task)
        .unwrap()
        .protect_retained_envelope(
            &f.successor,
            RuntimeRetainedOwnerIdentity {
                runtime_id: "candle",
                source_id: &f.gateway.resident_source_id,
                runtime_instance_id: &instance,
                model_target: &f.target.local_load_path,
            },
        )
        .unwrap();
    // Protocol fence setup only: authorize the exact declared envelope so
    // abandonment remains fenced. This does not assert a native forward ran.
    assert!(envelope
        .authorize_start(
            RuntimeRetainedOwnerIdentity {
                runtime_id: "candle",
                source_id: &f.gateway.resident_source_id,
                runtime_instance_id: &instance,
                model_target: &f.target.local_load_path,
            },
            || true
        )
        .unwrap());
    let before = resident(&f).model_resource_residency;
    assert!(f
        .gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    let after = resident(&f);
    assert_eq!(after.model_resource_residency, before);
    assert!(after.resident_resources_uncertain);
    assert_eq!(after.status, RuntimeRegistryStatus::Failed);
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
    assert!(f
        .registry
        .release_reservation(f.successor.reservation_id)
        .is_err());
    drop(envelope);
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
}

#[tokio::test]
async fn registry_identity_changed_during_actual_stop_refuses_exact_ack() {
    let f = fixture().await;
    let before = resident(&f).model_resource_residency;
    let (entered, gate) = pause(&f);
    let job = stop(&f);
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    // Protocol mutation only, not a pretend native replacement: verify the
    // same-lock ACK rechecks identity after stop, rather than trusting preflight.
    let mut changed = frame(
        &f,
        f.gateway
            .resident_observation_sequence
            .fetch_add(1, Ordering::AcqRel)
            + 1,
    );
    changed.observation.runtime_instance_id = Some("different-runtime-instance".into());
    f.registry.observe_runtime_producer(changed).unwrap();
    let replacement = resident(&f).model_resource_residency;
    assert_ne!(replacement, before);
    gate.open();
    assert!(tokio::time::timeout(Duration::from_secs(5), job)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert_eq!(resident(&f).model_resource_residency, replacement);
}

#[tokio::test]
async fn charge_appearing_during_actual_stop_cannot_bypass_exact_ack() {
    let f = fixture().await;
    let mut released = frame(
        &f,
        f.gateway
            .resident_observation_sequence
            .fetch_add(1, Ordering::AcqRel)
            + 1,
    );
    released.allocation_state = RuntimeProducerAllocationState::Released;
    released.observation.model_id = None;
    released.observation.status = RuntimeRegistryStatus::Stopped;
    // Protocol-only mutation: the actual loaded native owner is unchanged.
    f.registry.observe_runtime_producer(released).unwrap();
    assert!(!f.registry.requires_resident_retirement_ack("candle"));
    let (entered, gate) = pause(&f);
    let job = stop(&f);
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    let mut appeared = frame(
        &f,
        f.gateway
            .resident_observation_sequence
            .fetch_add(1, Ordering::AcqRel)
            + 1,
    );
    appeared.observation.runtime_instance_id = Some("appeared-during-stop".into());
    f.registry.observe_runtime_producer(appeared).unwrap();
    let charge = resident(&f).model_resource_residency;
    gate.open();
    assert!(tokio::time::timeout(Duration::from_secs(5), job)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert_eq!(resident(&f).model_resource_residency, charge);
    assert!(f.registry.requires_resident_retirement_ack("candle"));
}

#[tokio::test]
async fn injected_failure_in_actual_native_stop_preserves_model_and_charge() {
    let f = fixture().await;
    let before = state(&f.registry);
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(|phase| {
        if phase == "retirement_before_drain" {
            panic!("injected actual native stop hook failure");
        }
    }));
    assert!(f
        .gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    assert_eq!(state(&f.registry), before);
    assert!(f.gateway.is_ready().await);
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = None;
    assert!(f
        .gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .is_err());
    assert_eq!(state(&f.registry), before);
    assert!(!f.gateway.is_ready().await);
}

struct CandleLabelOnly;
#[async_trait::async_trait]
impl InferenceBackend for CandleLabelOnly {
    fn name(&self) -> &'static str {
        "Candle"
    }
    fn description(&self) -> &'static str {
        "label-only refusal fixture"
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn crate::process::ProcessSpawner>,
    ) -> Result<crate::backend::BackendStartOutcome, crate::backend::BackendError> {
        Err(crate::backend::BackendError::NotReady)
    }
    async fn stop(&mut self) -> Result<(), crate::backend::BackendError> {
        Ok(())
    }
    fn is_ready(&self) -> bool {
        false
    }
    async fn health_check(&self) -> bool {
        false
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn chat_completion_stream(
        &self,
        _: String,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<ChatChunk, crate::backend::BackendError>> + Send>>,
        crate::backend::BackendError,
    > {
        Err(crate::backend::BackendError::NotReady)
    }
    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> Result<Vec<EmbeddingResult>, crate::backend::BackendError> {
        Err(crate::backend::BackendError::NotReady)
    }
    async fn rerank(
        &self,
        _: crate::types::RerankRequest,
    ) -> Result<crate::types::RerankResponse, crate::backend::BackendError> {
        Err(crate::backend::BackendError::NotReady)
    }
}
#[tokio::test]
async fn injected_candle_label_nonready_and_uncalibrated_builtin_never_mint_ack() {
    let f = fixture().await;
    let before = state(&f.registry);
    let mut label = CandleLabelOnly;
    assert!(label.stop_with_cpu_retirement().await.unwrap().is_none());
    let label = InferenceGateway::with_backend(Box::new(label), "Candle");
    label
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
    assert_eq!(state(&f.registry), before);
    let plain =
        InferenceGateway::with_backend(Box::new(crate::backend::CandleBackend::new()), "Candle");
    plain
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
    assert_eq!(state(&f.registry), before);
    f.gateway
        .stop_and_publish_cpu_retirement(&f.registry)
        .await
        .unwrap();
}
