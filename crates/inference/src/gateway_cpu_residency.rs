//! Native linear idle residency. No forecast, snapshot or caller stamp can
//! reconstruct a lost load-started guard or substitute for physical stop ACK.
use super::*;
use crate::CandleCpuCleanupOwner;
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeObservation, RuntimeProducerAllocationState,
    RuntimeProducerObservation, RuntimeRegistry, RuntimeRegistryStatus,
    RuntimeReservationExecutionCustody, RuntimeReservationLease, RuntimeRetainedOwnerIdentity,
    RuntimeRetentionDisposition, RuntimeRetentionHint,
};
use std::time::{Duration, Instant};
use tokio::sync::OwnedRwLockWriteGuard;

type BackendGuard = OwnedRwLockWriteGuard<Box<dyn InferenceBackend>>;

pub struct CandleCpuColdResidencyRequest<'a> {
    pub request: InferenceExecutionRequest,
    pub target: crate::PumasArtifactLoadTarget,
    pub decision: crate::BackendExecutionDecision,
    pub guard: &'a RuntimeReservationLease,
}

struct OwnedColdLoad {
    guard: RuntimeReservationLease,
    request: InferenceExecutionRequest,
    target: crate::PumasArtifactLoadTarget,
    decision: crate::BackendExecutionDecision,
    config: BackendConfig,
}

/// The expected native owner is supplied by the consumed idle capability.
pub struct CandleCpuResidencyWarmAttemptRequest<'a> {
    pub request: InferenceExecutionRequest,
    pub target: crate::PumasArtifactLoadTarget,
    pub decision: crate::BackendExecutionDecision,
    pub identity: CandleCpuWarmAttemptIdentity<'a>,
    pub lease: &'a RuntimeReservationLease,
}

/// Actual native load/drain and guard custody, unavailable while a task runs.
/// The owning facade must exclude writable gateway/registry aliases. Dropping
/// this capability never stops a backend or releases its uncertain charge.
#[must_use]
pub struct CandleCpuIdleResidency {
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    guard: RuntimeReservationLease,
    custody: RuntimeReservationExecutionCustody,
    owner: CandleCpuCleanupOwner,
    target: crate::PumasArtifactLoadTarget,
    runtime_instance_id: String,
    load_duration: Duration,
    publication_duration: Duration,
}

/// Native physical stop is measured separately from registry acknowledgement.
pub struct CandleCpuResidencyStopOutcome {
    pub disposition: RuntimeRetentionDisposition,
    pub stop_duration: Duration,
    pub acknowledgement_duration: Duration,
}

impl CandleCpuIdleResidency {
    pub fn owner_facts(&self) -> crate::CandleCpuSerialOwnerFacts {
        self.owner.serial_facts()
    }
    pub fn matches_target(&self, target: &crate::PumasArtifactLoadTarget) -> bool {
        self.target == *target
    }
    /// Native load submission through the actual loader join/drain. Excludes
    /// gateway lifecycle and registry accounting publication.
    pub fn load_duration(&self) -> Duration {
        self.load_duration
    }
    pub fn publication_duration(&self) -> Duration {
        self.publication_duration
    }
    pub(super) fn registry(&self) -> &Arc<RuntimeRegistry> {
        &self.registry
    }
    pub(super) fn cleanup_owner(&self) -> &CandleCpuCleanupOwner {
        &self.owner
    }
    pub(super) fn belongs_to(&self, gateway: &Arc<InferenceGateway>) -> bool {
        Arc::ptr_eq(&self.gateway, gateway) && self.custody.matches(&self.registry, &self.guard)
    }
    pub(super) async fn validate_live(
        &self,
        backend: &dyn InferenceBackend,
    ) -> Result<(), GatewayError> {
        validate_current_owner(
            &self.gateway,
            backend,
            &self.owner,
            &self.runtime_instance_id,
            &self.target.local_load_path,
        )
        .await?;
        // Unlike retained-task validation, a sole guard needs no successor.
        // Custody excludes exact-lease replacement; the private facade excludes
        // registry writers while the actual producer writer is held.
        if !self.custody.matches(&self.registry, &self.guard)
            || self
                .registry
                .reservation_lease(self.guard.reservation_id)
                .as_ref()
                != Some(&self.guard)
        {
            return Err(refusal());
        }
        Ok(())
    }
    /// Called only after the consumed actual task receipt's release ACK under
    /// the same writer. Move its private current owner; never stamp a generation.
    pub(super) fn after_task_ack(
        mut self,
        current: CandleCpuCleanupOwner,
    ) -> Result<Self, GatewayError> {
        let old = self.owner.serial_facts();
        let new = current.serial_facts();
        if old.loaded_instance != new.loaded_instance
            || old.loaded_profile != new.loaded_profile
            || old.effective_settings != new.effective_settings
            || old.cpu_threads != new.cpu_threads
            || !Arc::ptr_eq(&old.generation_handle, &new.generation_handle)
            || old.generation.checked_add(2) != Some(new.generation)
            || self.owner.model_ref() != current.model_ref()
        {
            return Err(refusal());
        }
        self.owner = current;
        Ok(self)
    }
    /// Consume original load-started guard custody through genuine physical
    /// stop and existing sole-lease ACK. Failed/lost outcomes stay fenced.
    pub async fn stop(
        self,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<CandleCpuResidencyStopOutcome, GatewayError> {
        let gateway = self.gateway.clone();
        gateway.embedding_replacement.drain().await?;
        let backend = gateway.backend.clone().write_owned().await;
        self.validate_live(backend.as_ref()).await?;
        reject_cancelled_execution_handle("native idle CPU stop", &host)?;
        let ticket = self
            .custody
            .begin_last_lease_eviction(RuntimeRetainedOwnerIdentity {
                runtime_id: "candle",
                source_id: &gateway.resident_source_id,
                runtime_instance_id: &self.runtime_instance_id,
                model_target: &self.target.local_load_path,
            })
            .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?;
        cpu_warm_attempt::stop_last_lease_owned(gateway, backend, ticket, self.owner, host).await
    }
}

impl InferenceGateway {
    /// Load only on the facade-owned fresh or exactly acknowledged stopped
    /// calibrated CPU backend. Caller owns immutable resident/load estimates and
    /// checks the full additive future resident+guard+task envelope beforehand.
    /// The guard's complete declared peak remains charged throughout residency.
    pub async fn load_cpu_idle_residency(
        self: &Arc<Self>,
        registry: Arc<RuntimeRegistry>,
        input: CandleCpuColdResidencyRequest<'_>,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<CandleCpuIdleResidency, GatewayError> {
        if !crate::service_timing::bounds::within_budget(&(
            &input.request,
            &input.target,
            &input.decision,
            input.guard,
        )) {
            return Err(refusal());
        }
        let crate::InferenceExecutionInput::Embedding { texts } = &input.request.input else {
            return Err(refusal());
        };
        if !matches!(texts.as_slice(), [text] if !text.is_empty() && text.len() <= 4096)
            || input.request.generation_options.is_some()
            || !(input.request.extra_options.is_null()
                || input
                    .request
                    .extra_options
                    .as_object()
                    .is_some_and(|options| options.is_empty()))
            || input.guard.runtime_id != "candle"
            || input.guard.retention_hint != RuntimeRetentionHint::Ephemeral
            || input.guard.pin_runtime
            || input.guard.model_id.as_deref() != Some(input.target.model_ref.model_id.as_str())
            || self.candle_cpu_calibration.is_none()
        {
            return Err(refusal());
        }
        let selected = crate::selected_embedding_execution::SelectedEmbeddingLoad::validate(
            &input.request,
            &input.target,
            &input.decision,
        )
        .await?;
        let config = BackendConfig {
            model_path: Some(PathBuf::from(&selected.target.local_load_path)),
            model_name: Some(selected.package.model_ref.model_id.clone()),
            embedding_mode: true,
            device: Some(BackendStartupDeviceIntent::CanonicalDevice(
                selected.device.clone(),
            )),
            ..Default::default()
        };
        let custody = registry
            .acquire_execution_custody(input.guard)
            .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?;
        validate_cold_guard(&registry, input.guard)?;
        self.embedding_replacement.drain().await?;
        let backend = self.backend.clone().write_owned().await;
        let lifecycle = self.runtime_lifecycle.read().await;
        if canonical_backend_key(backend.name()) != "candle"
            || backend.is_ready()
            || backend.resident_cpu_calibration_instance().is_some()
            || lifecycle.active
            || self.current_runtime_config.read().await.is_some()
        {
            return Err(refusal());
        }
        drop(lifecycle);
        reject_cancelled_execution_handle("native CPU cold preparation", &host)?;
        let gateway = self.clone();
        let owned = OwnedColdLoad {
            guard: input.guard.clone(),
            request: input.request,
            target: input.target,
            decision: input.decision,
            config,
        };
        let (signal, mut cancel) = cpu_warm_attempt::caller_signal(host);
        let task = tokio::spawn(async move {
            load_cpu_idle_owned(gateway, registry, backend, custody, owned, signal).await
        });
        let result = task.await.map_err(|error| {
            GatewayError::SwitchFailed(format!("native CPU load supervision failed: {error}"))
        })??;
        cancel.disarm();
        Ok(result)
    }
}

fn validate_cold_guard(
    registry: &RuntimeRegistry,
    lease: &RuntimeReservationLease,
) -> Result<(), GatewayError> {
    let snapshot = registry.snapshot();
    let runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "candle")
        .ok_or_else(refusal)?;
    if runtime.model_resource_residency.is_some()
        || runtime.resident_resources_uncertain
        || runtime.active_reservation_ids != [lease.reservation_id]
        || snapshot
            .reservations
            .iter()
            .find(|actual| actual.reservation_id == lease.reservation_id)
            != Some(lease)
    {
        return Err(refusal());
    }
    let claims = &runtime
        .active_reservation_claims
        .iter()
        .find(|claim| claim.reservation_id == lease.reservation_id)
        .ok_or_else(refusal)?
        .claims;
    for kind in [
        RuntimeAdmissionResourceKind::RamBytes,
        RuntimeAdmissionResourceKind::VramBytes,
    ] {
        if !claims.iter().any(|claim| claim.kind == kind)
            || runtime.admission_budget.as_ref().is_none_or(|budget| {
                !budget
                    .resources
                    .iter()
                    .any(|resource| resource.kind == kind && resource.total_bytes.is_some())
            })
        {
            return Err(refusal());
        }
    }
    Ok(())
}

// All inputs are owned by this supervisor before load begins. Collector loss
// signals cancellation but cannot drop the writer/custody out from under work.
async fn load_cpu_idle_owned(
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    mut backend: BackendGuard,
    mut custody: RuntimeReservationExecutionCustody,
    input: OwnedColdLoad,
    signal: InferenceExecutionCancellationHandle,
) -> Result<CandleCpuIdleResidency, GatewayError> {
    let OwnedColdLoad {
        guard,
        request,
        target,
        decision,
        config,
    } = input;
    reject_cancelled_execution_handle("native CPU cold load", &signal)?;
    let started = Instant::now();
    custody.begin_execution();
    let outcome = backend
        .load_selected_embedding_with_cancellation(&request, &target, &decision, signal.clone())
        .await?;
    let load_duration = started.elapsed();
    if outcome.runtime_reused != Some(false) {
        return Err(refusal());
    }
    #[cfg(test)]
    residency_test_phase(&gateway, "residency_after_load").await?;
    reject_cancelled_execution_handle("native CPU load publication", &signal)?;
    let publication_started = Instant::now();
    let (owner, runtime_instance_id) =
        publish_cold_owner_locked(&gateway, &registry, backend.as_ref(), config, &target).await?;
    if !custody.matches(&registry, &guard)
        || registry.reservation_lease(guard.reservation_id).as_ref() != Some(&guard)
    {
        return Err(refusal());
    }
    Ok(CandleCpuIdleResidency {
        gateway,
        registry,
        guard,
        custody,
        owner,
        target,
        runtime_instance_id,
        load_duration,
        publication_duration: publication_started.elapsed(),
    })
}

async fn publish_cold_owner_locked(
    gateway: &InferenceGateway,
    registry: &RuntimeRegistry,
    backend: &dyn InferenceBackend,
    config: BackendConfig,
    target: &crate::PumasArtifactLoadTarget,
) -> Result<(CandleCpuCleanupOwner, String), GatewayError> {
    let instance = backend
        .resident_cpu_calibration_instance()
        .ok_or_else(refusal)?;
    let calibration = gateway
        .candle_cpu_calibration
        .as_ref()
        .ok_or_else(refusal)?;
    let owner = calibration.cleanup_owner(instance).ok_or_else(refusal)?;
    if canonical_backend_key(backend.name()) != "candle"
        || !backend.is_ready()
        || owner.model_ref() != &target.model_ref
        || !calibration.matches_cleanup_owner(&owner, instance)
    {
        return Err(refusal());
    }
    let runtime_instance_id = gateway.allocate_runtime_instance_id("candle");
    *gateway.current_runtime_config.write().await = Some(config);
    *gateway.embedding_mode.write().await = true;
    *gateway.reranking_mode.write().await = false;
    *gateway.external_mode.write().await = false;
    *gateway.runtime_lifecycle.write().await = RuntimeLifecycleSnapshot {
        runtime_id: Some("candle".into()),
        runtime_instance_id: Some(runtime_instance_id.clone()),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("native_cpu_load_only_preparation".into()),
        active: true,
        ..Default::default()
    };
    let sequence = next_observation_sequence(gateway)?;
    let published = registry
        .observe_runtime_producer(RuntimeProducerObservation {
            source_id: gateway.resident_source_id.clone(),
            sequence,
            allocation_state: RuntimeProducerAllocationState::Resident,
            observation: RuntimeObservation {
                runtime_id: "candle".into(),
                display_name: "Candle".into(),
                backend_keys: vec!["candle".into()],
                model_id: Some(target.local_load_path.clone()),
                runtime_instance_id: Some(runtime_instance_id.clone()),
                status: RuntimeRegistryStatus::Ready,
                last_error: None,
            },
        })
        .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?;
    let residency = published.model_resource_residency.ok_or_else(refusal)?;
    let resources = residency.requirements.ok_or_else(refusal)?;
    if residency.model_id != target.local_load_path
        || residency.runtime_instance_id.as_deref() != Some(&runtime_instance_id)
        || published.resident_resources_uncertain
        || [
            RuntimeAdmissionResourceKind::RamBytes,
            RuntimeAdmissionResourceKind::VramBytes,
        ]
        .into_iter()
        .any(|kind| !resources.claims.iter().any(|claim| claim.kind == kind))
    {
        return Err(refusal());
    }
    Ok((owner, runtime_instance_id))
}

pub(super) async fn validate_current_owner(
    gateway: &InferenceGateway,
    backend: &dyn InferenceBackend,
    owner: &CandleCpuCleanupOwner,
    runtime_instance_id: &str,
    target: &str,
) -> Result<(), GatewayError> {
    let instance = backend
        .resident_cpu_calibration_instance()
        .ok_or_else(refusal)?;
    let lifecycle = gateway.runtime_lifecycle.read().await;
    let config = gateway.current_runtime_config.read().await;
    if canonical_backend_key(backend.name()) != "candle"
        || !backend.is_ready()
        || !lifecycle.active
        || lifecycle.runtime_id.as_deref() != Some("candle")
        || lifecycle.runtime_instance_id.as_deref() != Some(runtime_instance_id)
        || config.as_ref().and_then(config_model_target).as_deref() != Some(target)
        || config
            .as_ref()
            .and_then(|config| config.model_name.as_deref())
            != Some(owner.model_ref().model_id.as_str())
        || !gateway
            .candle_cpu_calibration
            .as_ref()
            .is_some_and(|calibration| calibration.matches_cleanup_owner(owner, instance))
    {
        return Err(refusal());
    }
    Ok(())
}

pub(super) fn next_observation_sequence(gateway: &InferenceGateway) -> Result<u64, GatewayError> {
    gateway
        .resident_observation_sequence
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |sequence| {
            sequence.checked_add(1)
        })
        .map(|previous| previous + 1)
        .map_err(|_| refusal())
}

fn refusal() -> GatewayError {
    GatewayError::SwitchFailed("native CPU residency requires exact load-started guard custody, current owner and known accounting".into())
}

#[cfg(test)]
async fn residency_test_phase(
    gateway: &InferenceGateway,
    phase: &'static str,
) -> Result<(), GatewayError> {
    let owner = gateway
        .candle_cpu_calibration
        .as_ref()
        .ok_or_else(refusal)?
        .clone();
    tokio::task::spawn_blocking(move || owner.test_attempt_phase(phase))
        .await
        .map_err(|_| refusal())
}

#[cfg(test)]
#[path = "gateway_cpu_residency_tests.rs"]
mod tests;
