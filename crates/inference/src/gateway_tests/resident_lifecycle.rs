//! Real gateway orchestration with a portable backend; no Python/GPU/model downloads.
use super::*;
use crate::resident_lifecycle::ResidentAllocationState;
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind as Kind, RuntimeModelResidentEstimate, RuntimeRegistry,
    RuntimeRegistryError, RuntimeReservationRequest,
    RuntimeReservationRequirements as Requirements, RuntimeReservationResourceClaim as Claim,
    RuntimeResourceDomain, RuntimeResourceDomainBinding, RuntimeResourceDomainConfig,
    RuntimeRetentionHint,
};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Default)]
struct LifecycleFixture {
    ready: Arc<AtomicBool>,
    failure: Arc<AtomicUsize>,
    stop_failure: Arc<AtomicBool>,
}

struct FreshPyTorchFactory;
impl crate::backend::BackendFactory for FreshPyTorchFactory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        Ok(Box::new(LifecycleFixture::default()))
    }
    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "PyTorch".into(),
            backend_key: "pytorch".into(),
            description: "portable fresh owner".into(),
            capabilities: BackendCapabilities::default(),
            default_start_mode: crate::backend::BackendDefaultStartMode::Inference,
            active: false,
            available: true,
            unavailable_reason: None,
            can_install: false,
            runtime_binary_id: None,
        }
    }
}

#[async_trait]
impl InferenceBackend for LifecycleFixture {
    fn name(&self) -> &'static str {
        "PyTorch"
    }
    fn description(&self) -> &'static str {
        "portable lifecycle fixture"
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        match self.failure.load(Ordering::SeqCst) {
            1 => return Err(BackendError::StartupFailed("pre-effect rejection".into())),
            2 => {
                self.ready.store(false, Ordering::SeqCst);
                return Err(BackendError::StartupFailed("effectful load failure".into()));
            }
            _ => {}
        }
        self.ready.store(true, Ordering::SeqCst);
        Ok(BackendStartOutcome {
            runtime_reused: Some(self.failure.load(Ordering::SeqCst) == 3),
            lifecycle_decision_reason: None,
        })
    }
    async fn stop(&mut self) -> Result<(), BackendError> {
        if self.stop_failure.load(Ordering::SeqCst) {
            return Err(BackendError::StartupFailed(
                "unload was not acknowledged".into(),
            ));
        }
        self.ready.store(false, Ordering::SeqCst);
        Ok(())
    }
    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
    async fn health_check(&self) -> bool {
        self.is_ready()
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn chat_completion_stream(
        &self,
        _: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }
    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(vec![])
    }
    async fn rerank(&self, _: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: vec![],
            metadata: serde_json::Value::Null,
        })
    }
}

fn requirements(ram: u64, vram: Option<u64>) -> Requirements {
    let mut claims = vec![Claim::ram_bytes(ram)];
    if let Some(bytes) = vram {
        claims.push(Claim::vram_bytes(bytes));
    }
    Requirements::from_claims(claims)
}

fn registry(known_zero: bool) -> RuntimeRegistry {
    RuntimeResourceDomainConfig {
        runtime_resource_domains: vec![RuntimeResourceDomain {
            domain_id: "unified".into(),
            total_bytes: 100,
            safety_margin_bytes: 0,
            bindings: vec![
                RuntimeResourceDomainBinding {
                    runtime_id: "pytorch".into(),
                    resource_kind: Kind::RamBytes,
                },
                RuntimeResourceDomainBinding {
                    runtime_id: "pytorch".into(),
                    resource_kind: Kind::VramBytes,
                },
                RuntimeResourceDomainBinding {
                    runtime_id: "candle".into(),
                    resource_kind: Kind::RamBytes,
                },
            ],
        }],
        runtime_model_resident_estimates: vec![
            RuntimeModelResidentEstimate {
                runtime_id: "Torch".into(),
                model_id: "model-a".into(),
                requirements: requirements(40, known_zero.then_some(0)),
            },
            RuntimeModelResidentEstimate {
                runtime_id: "pytorch".into(),
                model_id: "model-b".into(),
                requirements: requirements(60, Some(0)),
            },
            RuntimeModelResidentEstimate {
                runtime_id: "pytorch".into(),
                model_id: "too-large".into(),
                requirements: requirements(101, Some(0)),
            },
        ],
    }
    .compose_registry()
    .unwrap()
}

fn task(runtime: &str, bytes: u64) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: runtime.into(),
        workflow_id: "workflow".into(),
        reservation_owner_id: None,
        model_id: None,
        usage_profile: None,
        pin_runtime: false,
        retention_hint: RuntimeRetentionHint::KeepAlive,
        requirements: Some(requirements(bytes, None)),
    }
}

fn available(registry: &RuntimeRegistry) -> Result<u64, RuntimeRegistryError> {
    registry
        .evaluate_reservation(task("candle", 0))
        .map(|evaluation| evaluation.observation().resource_domains[0].available_bytes)
}

fn config(model: &str) -> BackendConfig {
    BackendConfig {
        model_path: Some(model.into()),
        ..Default::default()
    }
}

async fn gateway() -> (InferenceGateway, LifecycleFixture) {
    let fixture = LifecycleFixture::default();
    let gateway = InferenceGateway::with_backend(Box::new(fixture.clone()), "PyTorch");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    (gateway, fixture)
}

async fn publish(gateway: &InferenceGateway, registry: &RuntimeRegistry) {
    gateway
        .resident_lifecycle_snapshot()
        .await
        .unwrap()
        .publish(registry)
        .unwrap();
}

#[tokio::test]
async fn start_retained_idle_replacement_and_full_peak_task_claims() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    let lease = registry.acquire_reservation(task("pytorch", 50)).unwrap();
    assert_eq!(available(&registry).unwrap(), 10); // 40 resident + full 50 peak
    registry.release_reservation(lease.reservation_id).unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 60); // idle retains weights
    gateway.start(&config("model-b")).await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 40);
}

#[tokio::test]
async fn delayed_old_stop_and_old_load_cannot_clear_or_shrink_reload() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    let old_load = gateway.resident_lifecycle_snapshot().await.unwrap();
    old_load.clone().publish(&registry).unwrap();
    gateway.stop().await.unwrap();
    let old_stop = gateway.resident_lifecycle_snapshot().await.unwrap();
    gateway.start(&config("model-b")).await.unwrap();
    publish(&gateway, &registry).await;
    assert!(old_stop.publish(&registry).is_err());
    assert!(old_load.publish(&registry).is_err());
    assert_eq!(available(&registry).unwrap(), 40);
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
}

#[tokio::test]
async fn rejected_replacement_preserves_old_identity_and_estimate() {
    let (gateway, fixture) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    let old = gateway.resident_lifecycle_snapshot().await.unwrap();
    old.clone().publish(&registry).unwrap();
    fixture.failure.store(1, Ordering::SeqCst);
    assert!(gateway.start(&config("model-b")).await.is_err());
    let retained = gateway.resident_lifecycle_snapshot().await.unwrap();
    assert_eq!(retained.model_target, old.model_target);
    assert_eq!(
        retained.lifecycle.runtime_instance_id,
        old.lifecycle.runtime_instance_id
    );
    assert_eq!(retained.allocation_state, ResidentAllocationState::Resident);
    retained.publish(&registry).unwrap();
    assert_eq!(available(&registry).unwrap(), 60);
}

#[tokio::test]
async fn effectful_failure_and_failed_unload_hold_capacity_until_acknowledged_stop() {
    let (gateway, fixture) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    fixture.failure.store(2, Ordering::SeqCst);
    assert!(gateway.start(&config("model-b")).await.is_err());
    let uncertain = gateway.resident_lifecycle_snapshot().await.unwrap();
    assert_eq!(uncertain.allocation_state, ResidentAllocationState::Unknown);
    uncertain.publish(&registry).unwrap();
    assert!(matches!(
        available(&registry),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
    ));
    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .unwrap();
    assert!(runtime.resident_resources_uncertain);
    assert_eq!(
        runtime
            .model_resource_residency
            .unwrap()
            .requirements
            .unwrap(),
        requirements(40, Some(0))
    );
    fixture.stop_failure.store(true, Ordering::SeqCst);
    assert!(gateway.stop().await.is_err());
    publish(&gateway, &registry).await;
    assert!(available(&registry).is_err());
    fixture.stop_failure.store(false, Ordering::SeqCst);
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
}

#[tokio::test]
async fn readiness_loss_is_unknown_until_owner_confirms_unload() {
    let (gateway, fixture) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    fixture.ready.store(false, Ordering::SeqCst); // portable process/worker-loss evidence
    publish(&gateway, &registry).await;
    assert!(available(&registry).is_err());
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
}

#[tokio::test]
async fn known_zero_vram_admits_cpu_residency_but_omitted_vram_blocks() {
    let (gateway, _) = gateway().await;
    gateway.start(&config("model-a")).await.unwrap();
    let known = registry(true);
    publish(&gateway, &known).await;
    assert_eq!(available(&known).unwrap(), 60);
    let unknown = registry(false);
    publish(&gateway, &unknown).await;
    assert!(matches!(
        available(&unknown),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable {
            resource_kind: "vram_bytes",
            ..
        })
    ));
}

#[tokio::test]
async fn missing_or_rejected_estimate_blocks_admission_without_restoring_old_model() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("missing-estimate")).await.unwrap();
    publish(&gateway, &registry).await;
    assert!(available(&registry).is_err());
    gateway.start(&config("too-large")).await.unwrap();
    let frame = gateway.resident_lifecycle_snapshot().await.unwrap();
    assert!(frame.publish(&registry).is_err());
    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .unwrap();
    assert_eq!(
        runtime.model_resource_residency.unwrap().model_id,
        "too-large"
    );
    assert!(available(&registry).is_err());
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
}

#[tokio::test]
async fn a_different_gateway_source_and_unsequenced_stop_cannot_replace_owner() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    let (other, _) = self::gateway().await;
    assert!(other
        .resident_lifecycle_snapshot()
        .await
        .unwrap()
        .publish(&registry)
        .is_err());
    registry.observe_runtime(pantograph_runtime_registry::RuntimeObservation {
        runtime_id: "pytorch".into(),
        display_name: "stale".into(),
        backend_keys: vec![],
        model_id: None,
        runtime_instance_id: None,
        status: pantograph_runtime_registry::RuntimeRegistryStatus::Stopped,
        last_error: None,
    });
    assert_eq!(available(&registry).unwrap(), 60);
    assert!(registry
        .transition_runtime(
            "pytorch",
            pantograph_runtime_registry::RuntimeTransition::Stopped
        )
        .is_err());
    registry.observe_runtimes(vec![]);
    assert_eq!(available(&registry).unwrap(), 60);
    assert_eq!(
        registry.reclaim_runtime("pytorch", false).unwrap().action,
        pantograph_runtime_registry::RuntimeReclaimAction::StopProducer
    );
    assert_eq!(available(&registry).unwrap(), 60);
}

#[tokio::test]
async fn failed_initial_load_is_unknown_and_never_measured_zero() {
    let (gateway, fixture) = gateway().await;
    let registry = registry(true);
    fixture.failure.store(2, Ordering::SeqCst);
    assert!(gateway.start(&config("model-a")).await.is_err());
    publish(&gateway, &registry).await;
    assert!(matches!(
        available(&registry),
        Err(RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. })
    ));
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
}

#[tokio::test]
async fn old_generation_is_rejected_even_with_reused_model_and_instance_labels() {
    let (gateway, fixture) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    let mut stale = gateway.resident_lifecycle_snapshot().await.unwrap();
    stale.clone().publish(&registry).unwrap();
    fixture.failure.store(3, Ordering::SeqCst);
    gateway.start(&config("model-a")).await.unwrap();
    let newer = gateway.resident_lifecycle_snapshot().await.unwrap();
    assert_eq!(
        stale.lifecycle.runtime_instance_id,
        newer.lifecycle.runtime_instance_id
    );
    newer.publish(&registry).unwrap();
    // A delayed stop carries the old token, even if its labels identify this
    // reused instance. The generation, rather than those labels, rejects it.
    stale.allocation_state = ResidentAllocationState::Released;
    stale.model_target = None;
    stale.lifecycle.active = false;
    assert!(stale.publish(&registry).is_err());
    assert_eq!(available(&registry).unwrap(), 60);
}

#[tokio::test]
async fn confirmed_unload_does_not_release_live_task_custody() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    let lease = registry.acquire_reservation(task("pytorch", 50)).unwrap();
    gateway.stop().await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 50);
    registry.release_reservation(lease.reservation_id).unwrap();
    assert_eq!(available(&registry).unwrap(), 100);
}

#[tokio::test]
async fn rejected_resident_publication_retries_after_full_peak_lease_release() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    let lease = registry.acquire_reservation(task("pytorch", 80)).unwrap();
    gateway.start(&config("model-a")).await.unwrap();
    assert!(gateway
        .resident_lifecycle_snapshot()
        .await
        .unwrap()
        .publish(&registry)
        .is_err());
    assert!(available(&registry).is_err());
    registry.release_reservation(lease.reservation_id).unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 60);
}

#[tokio::test]
async fn unconfigured_gateway_keeps_legacy_lifecycle_behavior() {
    let (gateway, _) = gateway().await;
    let registry = RuntimeRegistry::new();
    let initial = gateway
        .resident_lifecycle_snapshot()
        .await
        .unwrap()
        .publish(&registry)
        .unwrap();
    assert_eq!(
        initial.status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
    );
    assert!(!initial.resident_resources_uncertain);
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    registry.acquire_reservation(task("pytorch", 100)).unwrap();
}

#[tokio::test]
async fn matching_negative_health_restricts_dispatch_without_releasing_allocation() {
    let (gateway, _) = gateway().await;
    let registry = registry(true);
    gateway.start(&config("model-a")).await.unwrap();
    let frame = gateway.resident_lifecycle_snapshot().await.unwrap();
    frame.clone().publish(&registry).unwrap();
    registry.observe_runtime(pantograph_runtime_registry::RuntimeObservation {
        runtime_id: "pytorch".into(),
        display_name: "PyTorch".into(),
        backend_keys: vec!["pytorch".into()],
        model_id: frame.model_target,
        runtime_instance_id: frame.lifecycle.runtime_instance_id,
        status: pantograph_runtime_registry::RuntimeRegistryStatus::Failed,
        last_error: Some("health probe failed".into()),
    });
    assert!(registry.acquire_reservation(task("pytorch", 0)).is_err());
    assert_eq!(available(&registry).unwrap(), 60);
}

#[tokio::test]
async fn fresh_backend_switch_admits_first_load_and_switch_away_confirms_release() {
    let mut gateway =
        InferenceGateway::with_backend(Box::new(LifecycleFixture::default()), "candle");
    gateway
        .registry
        .register("pytorch", Box::new(FreshPyTorchFactory));
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    let registry = registry(true);
    gateway.switch_backend("pytorch").await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
    let peak = registry.acquire_reservation(task("pytorch", 50)).unwrap();
    gateway.start(&config("model-a")).await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 10);
    registry.release_reservation(peak.reservation_id).unwrap();
    gateway.switch_backend("llama.cpp").await.unwrap();
    publish(&gateway, &registry).await;
    assert_eq!(available(&registry).unwrap(), 100);
}
