//! Opt-in actual native warm attempt/drain receipts; no generic serial activation.
use super::*;
use crate::CandleCpuCleanupOwner;
use pantograph_runtime_registry::{
    RuntimeRegistry, RuntimeReservationExecutionCustody, RuntimeReservationLease,
    RuntimeRetainedOwnerIdentity, RuntimeRetentionDisposition,
};
use std::sync::atomic::AtomicBool;
use tokio::sync::OwnedRwLockWriteGuard;

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
}
impl CandleCpuDrainedAttempt {
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

pub(super) struct AttemptBinding {
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    lease: RuntimeReservationLease,
    owner: CandleCpuCleanupOwner,
    previous_owner: CandleCpuCleanupOwner,
    custody: Option<RuntimeReservationExecutionCustody>,
    identity: [String; 7],
    input: blake3::Hash,
    model_target: String,
    runtime_instance_id: Option<String>,
}
impl AttemptBinding {
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
        match &self.custody {
            Some(custody) => custody.validate_retained_owner(identity),
            None => self
                .registry
                .validate_retained_reservation_for_owner(&self.lease, identity),
        }
        .map_err(|error| GatewayError::SwitchFailed(error.to_string()))
    }
    pub(super) fn begin_custodied_execution(&mut self) {
        if let Some(custody) = &mut self.custody {
            custody.begin_execution();
        }
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
    ) -> Result<CandleCpuDrainedAttempt, GatewayError> {
        self.current(backend)?;
        self.validate_lease_owner(RuntimeRetainedOwnerIdentity {
            runtime_id: "candle",
            source_id: &self.gateway.resident_source_id,
            runtime_instance_id: self.runtime_instance_id.as_deref().ok_or_else(refusal)?,
            model_target: &self.model_target,
        })?;
        self.current(backend)?;
        Ok(CandleCpuDrainedAttempt { binding: self })
    }
}

struct CallerSignal {
    dropped: Arc<AtomicBool>,
    host: InferenceExecutionCancellationHandle,
}
impl crate::InferenceExecutionCancellationSignal for CallerSignal {
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
struct CancelOnDrop(Option<Arc<AtomicBool>>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if let Some(flag) = &self.0 {
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
        self.execute_cpu_warm_attempt_with_custody(registry, input, host, false)
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
        self.execute_cpu_warm_attempt_with_custody(registry, input, host, true)
            .await
    }

    async fn execute_cpu_warm_attempt_with_custody(
        self: &Arc<Self>,
        registry: Arc<RuntimeRegistry>,
        input: CandleCpuWarmAttemptRequest<'_>,
        host: InferenceExecutionCancellationHandle,
        require_custody: bool,
    ) -> Result<(InferenceExecutionResult, CandleCpuDrainedAttempt), GatewayError> {
        let mut binding = AttemptBinding::bind(self.clone(), registry.clone(), &input)?;
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
        let dropped = Arc::new(AtomicBool::new(false));
        let mut cancel = CancelOnDrop(Some(dropped.clone()));
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
        cancel.0 = None;
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
