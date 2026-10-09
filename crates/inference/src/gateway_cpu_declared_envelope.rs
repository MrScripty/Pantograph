//! Additive native-only declared envelope and selected-native no-start branch.
use super::*;
use pantograph_runtime_registry::{
    RuntimeRetainedExecutionEnvelope, RuntimeUnstartedEnvelopeSettlement,
};
use std::sync::Mutex;

type Recovery = Arc<Mutex<Option<RuntimeRetainedExecutionEnvelope>>>;
pub(super) struct DeclaredPreparation {
    pub successor: RuntimeReservationLease,
    pub recovery: Recovery,
}
pub(super) struct DeclaredCustody {
    pub guard: Option<RuntimeRetainedExecutionEnvelope>,
    pub recovery: Recovery,
}
impl Drop for DeclaredCustody {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            if guard.started() {
                drop(guard);
            } else if let Ok(mut slot) = self.recovery.lock() {
                *slot = Some(guard);
            }
            // Poisoned recovery cannot attest settlement; guard Drop retains claims.
        }
    }
}
/// Native operation outcome. Before-preparation failures and uncertain starts
/// remain Err, charged, with no no-start/release assertion.
#[must_use]
pub enum CandleCpuDeclaredAttemptOutcome {
    Drained {
        result: InferenceExecutionResult,
        receipt: CandleCpuDrainedAttempt,
    },
    NoStart {
        error: GatewayError,
        receipt: CandleCpuDeclaredNoStart,
    },
}
/// Sealed, non-Clone, supervised selected-native no-start capability. Does not
/// certify resolver/filesystem/setup effects, prior-job drain or Worker recovery.
#[must_use]
pub struct CandleCpuDeclaredNoStart {
    binding: AttemptBinding,
}
#[must_use]
pub struct CandleCpuVerifiedNoStartSettlement {
    gateway: Arc<InferenceGateway>,
    registry: Arc<RuntimeRegistry>,
    identity: [String; 7],
    input: [u8; 32],
    settlement: RuntimeUnstartedEnvelopeSettlement,
    model_ref: crate::PumasModelRef,
}
impl CandleCpuVerifiedNoStartSettlement {
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
            reservation_lease_id: self.settlement.task().reservation_id,
        }
    }
    pub fn model_ref(&self) -> &crate::PumasModelRef {
        &self.model_ref
    }

    pub fn task_lease(&self) -> &RuntimeReservationLease {
        self.settlement.task()
    }
    pub fn retaining_lease(&self) -> &RuntimeReservationLease {
        self.settlement.successor()
    }
    pub fn task_id(&self) -> &str {
        &self.identity[3]
    }
    pub fn attempt_id(&self) -> &str {
        &self.identity[4]
    }
    pub fn execution_request_id(&self) -> &str {
        &self.identity[5]
    }
    pub fn input_fingerprint(&self) -> [u8; 32] {
        self.input
    }
    pub fn matches_authority(
        &self,
        gateway: &Arc<InferenceGateway>,
        registry: &Arc<RuntimeRegistry>,
    ) -> bool {
        Arc::ptr_eq(&self.gateway, gateway) && Arc::ptr_eq(&self.registry, registry)
    }
}
impl CandleCpuDeclaredNoStart {
    pub async fn settle_retained(self) -> Result<CandleCpuVerifiedNoStartSettlement, GatewayError> {
        let mut b = self.binding;
        let gateway = b.gateway.clone();
        let backend = gateway.backend.write().await;
        validate_actual(&b, backend.as_ref()).await?;
        let guard = b
            .declared_custody
            .as_mut()
            .and_then(|c| c.guard.take())
            .ok_or_else(refusal)?;
        let settlement = guard
            .settle_unstarted(owner(&b)?)
            .map_err(|e| GatewayError::SwitchFailed(e.to_string()))?;
        drop(backend); // No native/registry lock survives into an owner observer.
        Ok(CandleCpuVerifiedNoStartSettlement {
            gateway: b.gateway,
            registry: b.registry,
            identity: b.identity,
            input: *b.input.as_bytes(),
            settlement,
            model_ref: b.owner.model_ref().clone(),
        })
    }
}
fn owner(b: &AttemptBinding) -> Result<RuntimeRetainedOwnerIdentity<'_>, GatewayError> {
    Ok(RuntimeRetainedOwnerIdentity {
        runtime_id: "candle",
        source_id: &b.gateway.resident_source_id,
        runtime_instance_id: b.runtime_instance_id.as_deref().ok_or_else(refusal)?,
        model_target: &b.model_target,
    })
}
async fn validate_actual(
    b: &AttemptBinding,
    backend: &dyn InferenceBackend,
) -> Result<(), GatewayError> {
    b.current(backend)?;
    let lifecycle = b.gateway.runtime_lifecycle.read().await;
    let config = b.gateway.current_runtime_config.read().await;
    if !lifecycle.active
        || lifecycle.runtime_id.as_deref() != Some("candle")
        || lifecycle.runtime_instance_id != b.runtime_instance_id
        || config.as_ref().and_then(config_model_target).as_deref() != Some(&b.model_target)
        || config.as_ref().and_then(|c| c.model_name.as_deref())
            != Some(b.owner.model_ref().model_id.as_str())
    {
        return Err(refusal());
    }
    b.current(backend)
}
impl AttemptBinding {
    pub(super) async fn release_declared(
        mut self,
    ) -> Result<pantograph_runtime_registry::RuntimeRetentionDisposition, GatewayError> {
        let gateway = self.gateway.clone();
        let backend = gateway.backend.write().await;
        validate_actual(&self, backend.as_ref()).await?;
        let guard = self
            .declared_custody
            .as_mut()
            .and_then(|c| c.guard.take())
            .ok_or_else(refusal)?;
        guard
            .release_drained(owner(&self)?)
            .map_err(|e| GatewayError::SwitchFailed(e.to_string()))
    }
}
impl InferenceGateway {
    /// Explicit native-only path, qualified only for complete DECLARED RAM/VRAM
    /// charges. Requires SAME frozen task-start authority and exact named retaining lease.
    /// Does not install full-envelope provider or reopen generic Worker admission.
    pub async fn execute_declared_envelope_cpu_warm_attempt(
        self: &Arc<Self>,
        registry: Arc<RuntimeRegistry>,
        input: CandleCpuWarmAttemptRequest<'_>,
        retaining_lease: &RuntimeReservationLease,
        host: InferenceExecutionCancellationHandle,
    ) -> Result<CandleCpuDeclaredAttemptOutcome, GatewayError> {
        if !CandleCpuWarmAttemptRequest::metadata_is_bounded(&(input.lease, retaining_lease))
            || host.task_start_disposition().is_none()
            || !host.task_start_matches_attempt(input.identity.task_id, input.identity.attempt_id)
        {
            return Err(refusal());
        }
        let mut seed = AttemptBinding::bind(self.clone(), registry.clone(), &input)?;
        let recovery = Arc::new(Mutex::new(None));
        let preparation = DeclaredPreparation {
            successor: retaining_lease.clone(),
            recovery: recovery.clone(),
        };
        let result = self
            .execute_cpu_warm_attempt_with_custody(
                registry,
                input,
                host.clone(),
                true,
                Some(preparation),
            )
            .await;
        match result {
            Ok((result, receipt)) => {
                Ok(CandleCpuDeclaredAttemptOutcome::Drained { result, receipt })
            }
            Err(error) => {
                host.revoke_task_start();
                if host.task_start_disposition()
                    != Some(crate::InferenceTaskStartDisposition::NoStartAuthorized)
                {
                    return Err(error);
                }
                let guard = recovery.lock().map_err(|_| refusal())?.take();
                let Some(guard) = guard else {
                    return Err(error);
                };
                // Runtime identity was validated under the held writer when the
                // envelope was prepared; settlement repeats actual owner checks.
                seed.runtime_instance_id = guard.retained_instance_id().map(str::to_owned);
                seed.declared_custody = Some(DeclaredCustody {
                    guard: Some(guard),
                    recovery,
                });
                Ok(CandleCpuDeclaredAttemptOutcome::NoStart {
                    error,
                    receipt: CandleCpuDeclaredNoStart { binding: seed },
                })
            }
        }
    }
}
