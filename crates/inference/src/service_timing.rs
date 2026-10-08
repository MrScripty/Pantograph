//! Selected-text service phase observations; no collection or ranking by default.

use std::sync::Arc;
use std::time::Instant;

use pantograph_timing_contracts::{
    RuntimeServiceTimingAttempt, RuntimeServiceTimingCapture, RuntimeServiceTimingClockSnapshot,
    RuntimeServiceTimingIdentity, RuntimeServiceTimingLoadDisposition, RuntimeServiceTimingOutcome,
    RuntimeServiceTimingOwnerProvenance, RuntimeServiceTimingPhase,
    RuntimeServiceTimingPhaseEvidence, RuntimeServiceTimingProfile,
    RuntimeServiceTimingUnavailableReason, RuntimeServiceTimingValue,
};
use serde::Serialize;
#[path = "service_timing_bounds.rs"]
pub(crate) mod bounds;

/// Fresh facts from the loaded backend owner, not scheduler-selected labels.
/// Tokens cover implementation, effective load configuration (including resolved
/// defaults), and the actual device. An owner without these facts returns None.
#[derive(Debug, Clone, Serialize)]
pub struct RuntimeServiceTimingOwnerFacts {
    pub implementation_fingerprint: String,
    pub effective_configuration_fingerprint: String,
    pub physical_device_fingerprint: String,
    pub device_id: crate::InferenceDeviceId,
}

/// Optional consumer. Implementations must be bounded, nonblocking, nonpanicking
/// and must not reenter the gateway. Saturation returns false and drops a sample;
/// timing evidence cannot alter execution results or create an unbounded queue.
pub trait RuntimeServiceTimingRecorder: Send + Sync {
    fn try_record(&self, attempt: RuntimeServiceTimingAttempt) -> bool;
}

pub(crate) trait ServiceTimingClock: Send + Sync {
    fn now_ns(&self) -> u64;
}
struct SteadyClock(Instant);
impl ServiceTimingClock for SteadyClock {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

pub(crate) struct ServiceTimingInstrumentation {
    recorder: Arc<dyn RuntimeServiceTimingRecorder>,
    clock: Arc<dyn ServiceTimingClock>,
    clock_epoch: String,
    controlled_clock: bool,
}
impl ServiceTimingInstrumentation {
    pub(crate) fn new(recorder: Arc<dyn RuntimeServiceTimingRecorder>) -> Self {
        Self {
            recorder,
            clock: Arc::new(SteadyClock(Instant::now())),
            clock_epoch: uuid::Uuid::new_v4().to_string(),
            controlled_clock: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn with_clock(
        recorder: Arc<dyn RuntimeServiceTimingRecorder>,
        clock: Arc<dyn ServiceTimingClock>,
    ) -> Self {
        Self {
            recorder,
            clock,
            clock_epoch: uuid::Uuid::new_v4().to_string(),
            controlled_clock: true,
        }
    }
    pub(crate) fn clock_snapshot(&self) -> RuntimeServiceTimingClockSnapshot {
        RuntimeServiceTimingClockSnapshot {
            clock_epoch: self.clock_epoch.clone(),
            now_ns: self.clock.now_ns(),
        }
    }
}

pub(crate) struct SelectedTextServiceTimingAttempt<'a> {
    instrumentation: &'a ServiceTimingInstrumentation,
    owner_epoch: String,
    request_digest: Option<String>,
    selected_device: Option<crate::InferenceDeviceId>,
    pending: Option<(RuntimeServiceTimingPhase, u64)>,
    record: Option<RuntimeServiceTimingAttempt>,
    owner_provenance: RuntimeServiceTimingOwnerProvenance,
    load_disposition: RuntimeServiceTimingLoadDisposition,
    last_clock_ns: Option<u64>,
    clock_valid: bool,
}

impl<'a> SelectedTextServiceTimingAttempt<'a> {
    pub(crate) fn new(
        instrumentation: &'a ServiceTimingInstrumentation,
        owner_provenance: RuntimeServiceTimingOwnerProvenance,
        owner_epoch: &str,
        request: &crate::InferenceExecutionRequest,
        target: &crate::PumasArtifactLoadTarget,
        decision: &crate::BackendExecutionDecision,
    ) -> Self {
        // Never substitute model_id, revision labels, file size or a local path
        // for the authoritative owner's immutable content identity.
        let request_digest = target
            .content_fingerprint
            .as_deref()
            .filter(|token| valid_token(token))
            .and_then(|content| {
                bounds::digest(&(
                    2,
                    "selected_text_service_phases",
                    (
                        content,
                        &target.model_ref,
                        &target.artifact_kind,
                        &target.local_load_path,
                        &target.library_root_id,
                        target.package_facts_contract_version,
                    ),
                    (
                        &decision.selected_backend_id,
                        &decision.selected_runtime_variant_id,
                        &decision.selected_device_class,
                        &decision.selected_device_id,
                    ),
                    (
                        &request.task_id,
                        &request.input,
                        &request.generation_options,
                        &request.extra_options,
                        &request.resolved_model_package_facts,
                    ),
                ))
            });
        let reason = if request_digest.is_some() {
            RuntimeServiceTimingUnavailableReason::RuntimeGenerationUnavailable
        } else if target
            .content_fingerprint
            .as_deref()
            .is_some_and(valid_token)
        {
            RuntimeServiceTimingUnavailableReason::IdentityBudgetExceeded
        } else {
            RuntimeServiceTimingUnavailableReason::ModelContentIdentityUnavailable
        };
        Self {
            instrumentation,
            owner_epoch: owner_epoch.to_owned(),
            request_digest,
            selected_device: decision.selected_device_id.clone(),
            pending: None,
            owner_provenance: if instrumentation.controlled_clock {
                RuntimeServiceTimingOwnerProvenance::Injected
            } else {
                owner_provenance
            },
            load_disposition: RuntimeServiceTimingLoadDisposition::Unknown,
            last_clock_ns: None,
            clock_valid: true,
            record: Some(RuntimeServiceTimingAttempt {
                capture: None,
                attempt_id: uuid::Uuid::new_v4().to_string(),
                // Validation may reject the call after this guard is created.
                // Never retain or normalize arbitrary caller identity payloads.
                execution_request_id_digest: request
                    .request_id
                    .as_ref()
                    .filter(|id| id.len() <= 64 * 1024)
                    .map(|id| blake3::hash(id.as_bytes()).to_hex().to_string()),
                identity: RuntimeServiceTimingIdentity::Unknown { reason },
                outcome: RuntimeServiceTimingOutcome::Abandoned,
                phases: [
                    RuntimeServiceTimingPhase::GatewayCustodyWait,
                    RuntimeServiceTimingPhase::SelectedModelLoad,
                    RuntimeServiceTimingPhase::TextExecution,
                    RuntimeServiceTimingPhase::WorkerCleanup,
                ]
                .into_iter()
                .map(|phase| RuntimeServiceTimingPhaseEvidence {
                    phase,
                    value: RuntimeServiceTimingValue::Unknown {
                        reason: RuntimeServiceTimingUnavailableReason::PhaseNotReached,
                    },
                })
                .collect(),
            }),
        }
    }

    pub(crate) fn bind_load_outcome(&mut self, outcome: &crate::backend::BackendStartOutcome) {
        self.load_disposition = match outcome.runtime_reused {
            Some(false) => RuntimeServiceTimingLoadDisposition::Reloaded,
            Some(true) => RuntimeServiceTimingLoadDisposition::Reused,
            None => RuntimeServiceTimingLoadDisposition::Unknown,
        };
    }

    fn read_clock(&mut self) -> u64 {
        let now = self.instrumentation.clock.now_ns();
        if now == u64::MAX || self.last_clock_ns.is_some_and(|last| now < last) {
            self.clock_valid = false;
        }
        self.last_clock_ns = Some(now);
        now
    }

    pub(crate) fn bind_owner(
        &mut self,
        runtime_instance: Option<&str>,
        facts: Option<RuntimeServiceTimingOwnerFacts>,
    ) {
        let Some(request_digest) = self.request_digest.as_ref() else {
            return;
        };
        let Some(instance) = runtime_instance else {
            return;
        };
        let Some(facts) = facts.filter(|facts| {
            Some(&facts.device_id) == self.selected_device.as_ref()
                && [
                    &facts.implementation_fingerprint,
                    &facts.effective_configuration_fingerprint,
                    &facts.physical_device_fingerprint,
                ]
                .into_iter()
                .all(|token| valid_token(token))
        }) else {
            self.record.as_mut().unwrap().identity = RuntimeServiceTimingIdentity::Unknown {
                reason: RuntimeServiceTimingUnavailableReason::RuntimeOwnerFactsUnavailable,
            };
            return;
        };
        let identity = serde_json::to_vec(&(request_digest, facts))
            .ok()
            .and_then(|payload| {
                RuntimeServiceTimingProfile::new(
                    self.owner_epoch.clone(),
                    instance.to_owned(),
                    blake3::hash(&payload).to_hex().to_string(),
                )
                .ok()
            });
        if let Some(profile) = identity {
            self.record.as_mut().unwrap().identity =
                RuntimeServiceTimingIdentity::Exact { profile };
        }
    }

    pub(crate) fn begin(&mut self, phase: RuntimeServiceTimingPhase) {
        let started = self.read_clock();
        self.pending = Some((phase, started));
    }
    pub(crate) fn end(&mut self, outcome: RuntimeServiceTimingOutcome) {
        let Some((phase, started)) = self.pending.take() else {
            return;
        };
        let elapsed = self.read_clock().checked_sub(started);
        let value = match elapsed.filter(|_| self.clock_valid) {
            Some(elapsed_ns) => RuntimeServiceTimingValue::Observed {
                elapsed_ns,
                outcome,
            },
            None => RuntimeServiceTimingValue::Unknown {
                reason: RuntimeServiceTimingUnavailableReason::ClockDiscontinuity,
            },
        };
        self.record
            .as_mut()
            .unwrap()
            .phases
            .iter_mut()
            .find(|item| item.phase == phase)
            .unwrap()
            .value = value;
    }
    pub(crate) fn finish(&mut self, succeeded: bool) {
        let outcome = if succeeded {
            RuntimeServiceTimingOutcome::Completed
        } else {
            RuntimeServiceTimingOutcome::Failed
        };
        self.end(outcome);
        self.record.as_mut().unwrap().outcome = outcome;
    }
}
impl Drop for SelectedTextServiceTimingAttempt<'_> {
    fn drop(&mut self) {
        // Dropping the calling future is not an acknowledged backend stop.
        self.end(RuntimeServiceTimingOutcome::Abandoned);
        let observed_at_ns = self.last_clock_ns.map(|_| self.read_clock());
        if let Some(record) = self.record.take() {
            let mut record = record;
            if let Some(observed_at_ns) = observed_at_ns.filter(|_| self.clock_valid) {
                record.capture = Some(RuntimeServiceTimingCapture {
                    clock_epoch: self.instrumentation.clock_epoch.clone(),
                    observed_at_ns,
                    owner_provenance: self.owner_provenance,
                    load_disposition: self.load_disposition,
                });
            }
            let _ = self.instrumentation.recorder.try_record(record);
        }
    }
}
fn valid_token(token: &str) -> bool {
    token.len() <= 256 && !token.trim().is_empty() && !token.chars().any(char::is_control)
}
