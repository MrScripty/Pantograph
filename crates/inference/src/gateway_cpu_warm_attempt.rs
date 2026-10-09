//! Opt-in actual native warm attempt/drain receipts; no generic serial activation.
use super::*;
use crate::CandleCpuCleanupOwner;
use pantograph_runtime_registry::{
    RuntimeRegistry, RuntimeReservationExecutionCustody, RuntimeReservationLease,
    RuntimeRetainedOwnerIdentity, RuntimeRetentionDisposition,
};
use std::sync::atomic::AtomicBool;
use tokio::sync::OwnedRwLockWriteGuard;

#[path = "gateway_cpu_declared_envelope.rs"]
mod declared_envelope;
pub use declared_envelope::{
    CandleCpuDeclaredAttemptOutcome, CandleCpuDeclaredNoStart, CandleCpuVerifiedNoStartSettlement,
};
use declared_envelope::{DeclaredCustody, DeclaredPreparation};
type BackendGuard = OwnedRwLockWriteGuard<Box<dyn InferenceBackend>>;

#[path = "gateway_cpu_last_lease_eviction.rs"]
mod last_lease_eviction;

/// Caller-owned tags. Native request/lease fields are checked exactly; the
/// native layer cannot independently certify run/node/attempt/candidate custody.
pub struct CandleCpuWarmAttemptIdentity<'a> {
    pub workflow_id: &'a str,
    pub workflow_run_id: &'a str,
    pub node_id: &'a str,
    pub task_id: &'a str,
    pub attempt_id: &'a str,
    pub execution_request_id: &'a str,
    pub candidate_id: &'a str,
    pub reservation_lease_id: u64,
}
pub struct CandleCpuWarmAttemptRequest<'a> {
    pub request: InferenceExecutionRequest,
    pub target: crate::PumasArtifactLoadTarget,
    pub decision: crate::BackendExecutionDecision,
    pub identity: CandleCpuWarmAttemptIdentity<'a>,
    pub lease: &'a RuntimeReservationLease,
    pub expected_owner: &'a CandleCpuCleanupOwner,
}

impl CandleCpuWarmAttemptRequest<'_> {
    /// Shared borrowed byte/node/depth preflight for the opt-in host boundary.
    /// Does not bound resolver filesystem I/O or physical worker elapsed time.
    pub fn metadata_is_bounded(value: &impl serde::Serialize) -> bool {
        crate::service_timing::bounds::within_budget(value)
    }
}

/// Linear actual success/drain capability, bound to its original gateway,
/// registry, request and exact lease. No public constructor or Clone exists.
#[must_use]
pub struct CandleCpuDrainedAttempt {
    binding: AttemptBinding,
    output: Option<[u8; 32]>,
}

/// Borrowed producer fact, available only from an actual custodied drain.
/// This proves a bounded embedding vector was produced, not that an owner
/// accepted it or that the task reservation was released.
pub struct CandleCpuVerifiedTaskOutput<'a> {
    receipt: &'a CandleCpuDrainedAttempt,
    fingerprint: [u8; 32],
}
impl CandleCpuVerifiedTaskOutput<'_> {
    pub fn identity(&self) -> CandleCpuWarmAttemptIdentity<'_> {
        self.receipt.identity()
    }
    pub fn input_fingerprint(&self) -> [u8; 32] {
        self.receipt.input_fingerprint()
    }
    pub fn vector_fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn model_ref(&self) -> &crate::PumasModelRef {
        self.receipt.binding.owner.model_ref()
    }
    pub fn matches_authority(
        &self,
        gateway: &Arc<InferenceGateway>,
        registry: &Arc<RuntimeRegistry>,
    ) -> bool {
        Arc::ptr_eq(&self.receipt.binding.gateway, gateway)
            && Arc::ptr_eq(&self.receipt.binding.registry, registry)
    }
}

/// Canonical bounded numeric vector digest shared with the result owner.
/// All values must be finite; f32 native values are represented exactly as f64.
pub fn candle_cpu_vector_fingerprint(
    values: impl ExactSizeIterator<Item = f64>,
) -> Option<[u8; 32]> {
    let length = values.len();
    if !(1..=4096).contains(&length) {
        return None;
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"pantograph.candle.cpu.embedding-vector.v1\0");
    hash.update(&(length as u64).to_le_bytes());
    let mut count = 0;
    for value in values.take(length + 1) {
        count += 1;
        if count > length || !value.is_finite() {
            return None;
        }
        hash.update(&value.to_bits().to_le_bytes());
    }
    (count == length).then(|| *hash.finalize().as_bytes())
}
impl CandleCpuDrainedAttempt {
    pub fn verified_task_output(&self) -> Result<CandleCpuVerifiedTaskOutput<'_>, GatewayError> {
        if self.binding.custody.is_none() && self.binding.declared_custody.is_none() {
            return Err(refusal());
        }
        Ok(CandleCpuVerifiedTaskOutput {
            receipt: self,
            fingerprint: self.output.ok_or_else(refusal)?,
        })
    }
    pub fn previous_owner_facts(&self) -> crate::CandleCpuSerialOwnerFacts {
        self.binding.previous_owner.serial_facts()
    }
    pub fn current_owner_facts(&self) -> crate::CandleCpuSerialOwnerFacts {
        self.binding.owner.serial_facts()
    }
    pub fn identity(&self) -> CandleCpuWarmAttemptIdentity<'_> {
        let f = &self.binding.identity;
        CandleCpuWarmAttemptIdentity {
            workflow_id: &f[0],
            workflow_run_id: &f[1],
            node_id: &f[2],
            task_id: &f[3],
            attempt_id: &f[4],
            execution_request_id: &f[5],
            candidate_id: &f[6],
            reservation_lease_id: self.binding.lease.reservation_id,
        }
    }
    pub fn execution_request_id(&self) -> &str {
        &self.binding.identity[5]
    }
    pub fn input_fingerprint(&self) -> [u8; 32] {
        *self.binding.input.as_bytes()
    }
    /// Consume actual drain proof; mismatch cannot use ordinary cleanup fallback.
    pub async fn release_retained(self) -> Result<RuntimeRetentionDisposition, GatewayError> {
        if self.binding.declared_custody.is_some() {
            return self.binding.release_declared().await;
        }
        self.binding
            .gateway
            .release_retained_cpu_reservation_owned(
                &self.binding.registry,
                &self.binding.lease,
                &self.binding.owner,
                self.binding.custody,
            )
            .await
    }
}

/// Sealed historical fact: this exact native attempt drained and its task
/// reservation was released. Resident model/session charges remain in place.
/// This is neither accepted output nor a full episode/resource-envelope ACK.
#[must_use]
pub struct CandleCpuVerifiedTaskRelease {
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    identity: [String; 7],
    reservation_id: u64,
    input: [u8; 32],
    model_ref: crate::PumasModelRef,
}
impl CandleCpuVerifiedTaskRelease {
    pub fn identity(&self) -> CandleCpuWarmAttemptIdentity<'_> {
        let f = &self.identity;
        CandleCpuWarmAttemptIdentity {
            workflow_id: &f[0],
            workflow_run_id: &f[1],
            node_id: &f[2],
            task_id: &f[3],
            attempt_id: &f[4],
            execution_request_id: &f[5],
            candidate_id: &f[6],
            reservation_lease_id: self.reservation_id,
        }
    }
    pub fn input_fingerprint(&self) -> [u8; 32] {
        self.input
    }
    pub fn model_ref(&self) -> &crate::PumasModelRef {
        &self.model_ref
    }
    pub fn matches_authority(
        &self,
        gateway: &Arc<InferenceGateway>,
        registry: &Arc<RuntimeRegistry>,
    ) -> bool {
        Arc::ptr_eq(&self.gateway, gateway) && Arc::ptr_eq(&self.registry, registry)
    }
}
impl CandleCpuDrainedAttempt {
    /// Mint only after actual retained cleanup succeeds. Returning from the
    /// awaited cleanup drops its backend/lifecycle/registry guards before a
    /// consumer can call an observer. A failed release produces no proof.
    pub async fn release_retained_verified(
        self,
    ) -> Result<CandleCpuVerifiedTaskRelease, GatewayError> {
        let b = self.binding;
        if b.custody.is_none() && b.declared_custody.is_none() {
            return Err(refusal());
        }
        if b.declared_custody.is_some() {
            return Err(refusal()); // Declared release uses its own envelope result contract.
        }
        b.gateway
            .release_retained_cpu_reservation_owned(&b.registry, &b.lease, &b.owner, b.custody)
            .await?;
        Ok(CandleCpuVerifiedTaskRelease {
            gateway: b.gateway,
            registry: b.registry,
            identity: b.identity,
            reservation_id: b.lease.reservation_id,
            input: *b.input.as_bytes(),
            model_ref: b.owner.model_ref().clone(),
        })
    }
}

pub(super) struct AttemptBinding {
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    lease: RuntimeReservationLease,
    owner: CandleCpuCleanupOwner,
    previous_owner: CandleCpuCleanupOwner,
    custody: Option<RuntimeReservationExecutionCustody>,
    declared_preparation: Option<DeclaredPreparation>,
    declared_custody: Option<DeclaredCustody>,
    identity: [String; 7],
    input: blake3::Hash,
    model_target: String,
    runtime_instance_id: Option<String>,
}
impl AttemptBinding {
    #[cfg(test)]
    pub(super) fn has_declared_envelope(&self) -> bool {
        self.declared_custody.is_some()
    }
    fn bind(
        gateway: Arc<InferenceGateway>,
        registry: Arc<RuntimeRegistry>,
        input: &CandleCpuWarmAttemptRequest<'_>,
    ) -> Result<Self, GatewayError> {
        // One borrowed byte/node/depth budget precedes cloning and validation.
        if !crate::service_timing::bounds::within_budget(&(
            &input.request,
            &input.target,
            &input.decision,
        )) {
            return Err(refusal());
        }
        let id = &input.identity;
        let tags = [
            id.workflow_id,
            id.workflow_run_id,
            id.node_id,
            id.task_id,
            id.attempt_id,
            id.execution_request_id,
            id.candidate_id,
        ];
        let bounded = |tag: &str| {
            !tag.is_empty()
                && tag.len() <= 128
                && tag
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':'))
        };
        let lease = input.lease;
        if !tags.into_iter().all(bounded)
            || lease.workflow_id != id.workflow_id
            || lease.reservation_id != id.reservation_lease_id
            || input.request.request_id.as_deref() != Some(id.execution_request_id)
            || input.request.task_id != crate::InferenceTaskId::Embedding
            || lease.model_id.as_deref() != Some(input.expected_owner.model_ref().model_id.as_str())
            || [&lease.runtime_id, &lease.workflow_id]
                .into_iter()
                .any(|s| s.len() > 128)
            || [
                &lease.reservation_owner_id,
                &lease.usage_profile,
                &lease.model_id,
            ]
            .into_iter()
            .any(|s| s.as_ref().is_some_and(|s| s.len() > 1024))
        {
            return Err(refusal());
        }
        let crate::InferenceExecutionInput::Embedding { texts } = &input.request.input else {
            return Err(refusal());
        };
        let [text] = texts.as_slice() else {
            return Err(refusal());
        };
        if text.is_empty() || text.len() > 4096 {
            return Err(refusal());
        }
        Ok(Self {
            gateway,
            registry,
            lease: lease.clone(),
            owner: input.expected_owner.clone(),
            previous_owner: input.expected_owner.clone(),
            custody: None,
            declared_preparation: None,
            declared_custody: None,
            identity: tags.map(str::to_owned),
            input: blake3::hash(text.as_bytes()),
            model_target: input.target.local_load_path.clone(),
            runtime_instance_id: None,
        })
    }

    pub(super) async fn preflight(
        &mut self,
        backend: &dyn InferenceBackend,
        target: &crate::PumasArtifactLoadTarget,
    ) -> Result<(), GatewayError> {
        if canonical_backend_key(backend.name()) != "candle"
            || !backend.is_ready()
            || target.model_ref != *self.owner.model_ref()
        {
            return Err(refusal());
        }
        let instance = backend
            .resident_cpu_calibration_instance()
            .ok_or_else(refusal)?;
        let lifecycle = self.gateway.runtime_lifecycle.read().await;
        let config = self.gateway.current_runtime_config.read().await;
        if lifecycle.runtime_id.as_deref() != Some("candle")
            || !lifecycle.active
            || config.as_ref().and_then(|c| c.model_name.as_deref())
                != Some(self.owner.model_ref().model_id.as_str())
            || config.as_ref().and_then(config_model_target).as_deref()
                != Some(target.local_load_path.as_str())
            || !self
                .gateway
                .candle_cpu_calibration
                .as_ref()
                .is_some_and(|owner| owner.matches_cleanup_owner(&self.owner, instance))
        {
            return Err(refusal());
        }
        let runtime_instance_id = lifecycle
            .runtime_instance_id
            .as_deref()
            .ok_or_else(refusal)?;
        if runtime_instance_id.len() > 128 {
            return Err(refusal());
        }
        self.runtime_instance_id = Some(runtime_instance_id.to_owned());
        if let Some(preparation) = self.declared_preparation.take() {
            let guard = self
                .custody
                .take()
                .ok_or_else(refusal)?
                .protect_retained_envelope(
                    &preparation.successor,
                    RuntimeRetainedOwnerIdentity {
                        runtime_id: "candle",
                        source_id: &self.gateway.resident_source_id,
                        runtime_instance_id,
                        model_target: &target.local_load_path,
                    },
                )
                .map_err(|e| GatewayError::SwitchFailed(e.to_string()))?;
            self.declared_custody = Some(DeclaredCustody {
                guard: Some(guard),
                recovery: preparation.recovery,
            });
        }
        self.validate_lease_owner(RuntimeRetainedOwnerIdentity {
            runtime_id: "candle",
            source_id: &self.gateway.resident_source_id,
            runtime_instance_id: lifecycle
                .runtime_instance_id
                .as_deref()
                .ok_or_else(refusal)?,
            model_target: &target.local_load_path,
        })
    }
    fn validate_lease_owner(
        &self,
        identity: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<(), GatewayError> {
        if let Some(custody) = &self.declared_custody {
            return custody
                .guard
                .as_ref()
                .ok_or_else(refusal)?
                .validate_retained_owner(identity)
                .map_err(|e| GatewayError::SwitchFailed(e.to_string()));
        }
        match &self.custody {
            Some(custody) => custody.validate_retained_owner(identity),
            None => self
                .registry
                .validate_retained_reservation_for_owner(&self.lease, identity),
        }
        .map_err(|error| GatewayError::SwitchFailed(error.to_string()))
    }
    pub(super) fn begin_custodied_execution(
        &mut self,
        cancellation: &InferenceExecutionCancellationHandle,
    ) -> Result<(), GatewayError> {
        if let Some(custody) = &mut self.declared_custody {
            let started = custody
                .guard
                .as_mut()
                .ok_or_else(refusal)?
                .authorize_start(
                    RuntimeRetainedOwnerIdentity {
                        runtime_id: "candle",
                        source_id: &self.gateway.resident_source_id,
                        runtime_instance_id: self
                            .runtime_instance_id
                            .as_deref()
                            .ok_or_else(refusal)?,
                        model_target: &self.model_target,
                    },
                    || {
                        cancellation.commit_task_start(&self.identity[3], &self.identity[4])
                            == Some(true)
                    },
                )
                .map_err(|e| GatewayError::SwitchFailed(e.to_string()))?;
            return if started { Ok(()) } else { Err(refusal()) };
        }
        if cancellation.commit_task_start(&self.identity[3], &self.identity[4]) == Some(false) {
            return Err(refusal());
        }
        if let Some(custody) = &mut self.custody {
            custody.begin_execution();
        }
        Ok(())
    }
    pub(super) fn adopt_verified_load(
        &mut self,
        backend: &mut dyn InferenceBackend,
    ) -> Result<(), GatewayError> {
        self.owner = backend
            .take_verified_cpu_warm_load()
            .and_then(|receipt| receipt.adopt(&self.owner))
            .ok_or_else(refusal)?;
        self.current(backend)
    }
    fn current(&self, backend: &dyn InferenceBackend) -> Result<(), GatewayError> {
        let instance = backend
            .resident_cpu_calibration_instance()
            .ok_or_else(refusal)?;
        if self
            .gateway
            .candle_cpu_calibration
            .as_ref()
            .is_some_and(|owner| owner.matches_cleanup_owner(&self.owner, instance))
        {
            Ok(())
        } else {
            Err(refusal())
        }
    }
    pub(super) fn drained(
        self,
        backend: &dyn InferenceBackend,
        embeddings: &[crate::InferenceEmbeddingResult],
    ) -> Result<CandleCpuDrainedAttempt, GatewayError> {
        self.current(backend)?;
        self.validate_lease_owner(RuntimeRetainedOwnerIdentity {
            runtime_id: "candle",
            source_id: &self.gateway.resident_source_id,
            runtime_instance_id: self.runtime_instance_id.as_deref().ok_or_else(refusal)?,
            model_target: &self.model_target,
        })?;
        self.current(backend)?;
        let output = match embeddings {
            [embedding] if embedding.index == Some(0) && embedding.token_count.is_some() => {
                candle_cpu_vector_fingerprint(embedding.vector.iter().map(|v| f64::from(*v)))
            }
            _ => None,
        };
        Ok(CandleCpuDrainedAttempt {
            binding: self,
            output,
        })
    }
}

struct CallerSignal {
    dropped: Arc<AtomicBool>,
    host: InferenceExecutionCancellationHandle,
}
impl crate::InferenceExecutionCancellationSignal for CallerSignal {
    fn task_start_matches_attempt(&self, task_id: &str, attempt_id: &str) -> bool {
        self.host.task_start_matches_attempt(task_id, attempt_id)
    }
    fn task_start_disposition(&self) -> Option<crate::InferenceTaskStartDisposition> {
        self.host.task_start_disposition()
    }
    fn commit_task_start(&self, task_id: &str, attempt_id: &str) -> Option<bool> {
        self.host.commit_task_start(task_id, attempt_id)
    }
    fn revoke_task_start(&self) {
        self.host.revoke_task_start();
    }
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if self.dropped.load(Ordering::Acquire) {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "native warm attempt collector dropped".into(),
            ))
        } else {
            self.host.snapshot()
        }
    }
}
// Preserve the existing eviction supervisor's independent collector contract.
struct CancelOnDrop(Option<Arc<AtomicBool>>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(flag) = &self.0 {
            flag.store(true, Ordering::Release);
        }
    }
}
struct WarmCancelOnDrop {
    dropped: Option<Arc<AtomicBool>>,
    host: InferenceExecutionCancellationHandle,
}
impl Drop for WarmCancelOnDrop {
    fn drop(&mut self) {
        if let Some(flag) = &self.dropped {
            self.host.revoke_task_start();
            flag.store(true, Ordering::Release);
        }
    }
}

impl InferenceGateway {
    /// Success-only retained warm execution. A supervised task owns the actual
    /// backend writer until load/forward/drain terminate, including collector loss.
    /// Caller must exclusively own this lease's admission/session lifecycle
    /// until receipt consumption. This API does not pin arbitrary registry
    /// mutations or certify caller-supplied workflow attempt custody.
    pub async fn execute_retained_cpu_warm_attempt(
        self: &Arc<Self>,
        registry: Arc<RuntimeRegistry>,
        input: CandleCpuWarmAttemptRequest<'_>,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<(InferenceExecutionResult, CandleCpuDrainedAttempt), GatewayError> {
        self.execute_cpu_warm_attempt_with_custody(registry, input, host, false, None)
            .await
    }

    /// Opt-in actual dispatch route. Same-lease mutations are excluded from
    /// preflight through receipt cleanup; after physical start, abandonment
    /// leaves the charge fenced. No ordinary cleanup or cold fallback exists.
    pub async fn execute_custodied_cpu_warm_attempt(
        self: &Arc<Self>,
        registry: Arc<RuntimeRegistry>,
        input: CandleCpuWarmAttemptRequest<'_>,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<(InferenceExecutionResult, CandleCpuDrainedAttempt), GatewayError> {
        self.execute_cpu_warm_attempt_with_custody(registry, input, host, true, None)
            .await
    }

    async fn execute_cpu_warm_attempt_with_custody(
        self: &Arc<Self>,
        registry: Arc<RuntimeRegistry>,
        input: CandleCpuWarmAttemptRequest<'_>,
        host: InferenceExecutionCancellationHandle,
        require_custody: bool,
        declared: Option<DeclaredPreparation>,
    ) -> Result<(InferenceExecutionResult, CandleCpuDrainedAttempt), GatewayError> {
        let mut binding = AttemptBinding::bind(self.clone(), registry.clone(), &input)?;
        binding.declared_preparation = declared;
        let dropped = Arc::new(AtomicBool::new(false));
        let mut cancel = WarmCancelOnDrop {
            dropped: Some(dropped.clone()),
            host: host.clone(),
        };
        if require_custody {
            binding.custody = Some(
                registry
                    .acquire_execution_custody(input.lease)
                    .map_err(|error| GatewayError::SwitchFailed(error.to_string()))?,
            );
        }
        self.embedding_replacement.drain().await?;
        let backend: BackendGuard = self.backend.clone().write_owned().await;
        reject_cancelled_execution_handle("native warm attempt", &host)?;
        let cancellation =
            InferenceExecutionCancellationHandle::with_signal(Arc::new(CallerSignal {
                dropped,
                host,
            }));
        let gateway = self.clone();
        let task = tokio::spawn(async move {
            gateway
                .execute_selected_embedding_owned(
                    input.request,
                    input.target,
                    input.decision,
                    cancellation,
                    Some(backend),
                    Some(binding),
                )
                .await
        });
        let outcome = task.await.map_err(|error| {
            GatewayError::SwitchFailed(format!("native warm attempt supervision failed: {error}"))
        })??;
        cancel.dropped = None;
        Ok((outcome.result, outcome.drained_attempt.ok_or_else(refusal)?))
    }
}
fn refusal() -> GatewayError {
    GatewayError::SwitchFailed(
        "native retained warm attempt requires exact live custody and actual verified reuse/drain"
            .into(),
    )
}

#[cfg(test)]
#[path = "gateway_cpu_warm_attempt_tests.rs"]
mod tests;
