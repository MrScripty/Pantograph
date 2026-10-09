mod admission;
mod model_resources;
mod producer_resources;
pub use model_resources::RuntimeModelResourceResidency;
pub use producer_resources::{
    RuntimeModelResidentEstimate, RuntimeProducerAllocationState, RuntimeProducerObservation,
};
mod execution_custody;
mod retained_envelope;
pub use retained_envelope::{
    RuntimeRetainedExecutionEnvelope, RuntimeUnstartedEnvelopeSettlement,
    RETAINED_ENVELOPE_MAX_RESERVATIONS, RETAINED_ENVELOPE_MAX_RUNTIMES,
};
mod last_lease_eviction;
pub use last_lease_eviction::RuntimeLastLeaseEviction;
mod observation;
mod reclaim;
mod registry_queries;
mod reservation;
mod reservation_custody;
pub use execution_custody::RuntimeReservationExecutionCustody;
mod reservation_evaluation;
mod resource_domain;
use reservation_custody::{
    check_observed_runtime_identity, prospective_reservation, PendingReservation,
};
pub use reservation_custody::{RuntimeReservationCustody, RuntimeReservationPublicationError};
pub use resource_domain::{
    RuntimeHostRamCapacitySource, RuntimeResourceDomain, RuntimeResourceDomainBinding,
    RuntimeResourceDomainConfig, RuntimeResourceDomainObservation,
};
mod retention;
mod runtime_selection_policy;
mod snapshot;
mod state;
pub mod technical_fit;
mod warmup;

use admission::RuntimeReservationClaim;
use registry_queries::{runtime_is_evictable, runtime_snapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub use admission::{
    RuntimeAdmissionBudget, RuntimeAdmissionFailure, RuntimeAdmissionResourceBudget,
    RuntimeAdmissionResourceKind, RuntimeReservationRequirements, RuntimeReservationResourceClaim,
};
pub use observation::{observed_runtime_status_from_lifecycle, RuntimeObservation};
use pantograph_runtime_identity::canonical_runtime_id;
pub use reclaim::{RuntimeReclaimAction, RuntimeReclaimDisposition};
use reservation::RuntimeReservationRecord;
pub use reservation::{RuntimeReservationLease, RuntimeReservationRequest, RuntimeRetentionHint};
pub use reservation_evaluation::{
    RuntimeReservationAdmissionObservation, RuntimeReservationEvaluation,
    RuntimeReservationResourceObservation,
};
pub use retention::{
    RuntimeRetentionDecision, RuntimeRetentionDisposition, RuntimeRetentionReason,
};
pub use snapshot::{
    RuntimeActiveReservationClaim, RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot,
};
use state::RuntimeTransition as Transition;
pub use state::{
    RuntimeDispatchIdentity, RuntimeDispatchIdentityError, RuntimeModelResidencyRecord,
    RuntimeRegistryRecord, RuntimeRegistryStatus, RuntimeTransition,
};
pub use technical_fit::{
    select_runtime_technical_fit, RuntimeTechnicalFitCandidate,
    RuntimeTechnicalFitCandidateHistorySummary, RuntimeTechnicalFitCandidateSetSummary,
    RuntimeTechnicalFitCandidateSourceKind, RuntimeTechnicalFitCompatibilityIssue,
    RuntimeTechnicalFitCompatibilityReport, RuntimeTechnicalFitDecision,
    RuntimeTechnicalFitDecisionCode, RuntimeTechnicalFitDependencyReadinessFact,
    RuntimeTechnicalFitDependencyReadinessResolverOwner,
    RuntimeTechnicalFitDependencyReadinessState, RuntimeTechnicalFitDependencyReadinessSubjectKind,
    RuntimeTechnicalFitDeviceClass, RuntimeTechnicalFitDeviceDiagnostic,
    RuntimeTechnicalFitDeviceDiagnosticCode, RuntimeTechnicalFitDeviceDiagnosticSeverity,
    RuntimeTechnicalFitDevicePolicy, RuntimeTechnicalFitFactor,
    RuntimeTechnicalFitHistoryThresholdState, RuntimeTechnicalFitObservedThroughputHint,
    RuntimeTechnicalFitOverride, RuntimeTechnicalFitPolicyPhase, RuntimeTechnicalFitReason,
    RuntimeTechnicalFitReasonCode, RuntimeTechnicalFitRequest, RuntimeTechnicalFitResidencyState,
    RuntimeTechnicalFitResourceEstimate, RuntimeTechnicalFitResourceEstimateDiagnostic,
    RuntimeTechnicalFitResourceEstimateDiagnosticCode,
    RuntimeTechnicalFitResourceEstimateDiagnosticSeverity, RuntimeTechnicalFitResourceEstimateKind,
    RuntimeTechnicalFitResourceEstimateState, RuntimeTechnicalFitResourcePressure,
    RuntimeTechnicalFitSelectionMode, RuntimeTechnicalFitSelectionPolicyTrace,
    RuntimeTechnicalFitUnavailableResourceEstimateState, RuntimeTechnicalFitWarmupState,
};
pub use warmup::{RuntimeWarmupDecision, RuntimeWarmupDisposition, RuntimeWarmupReason};

pub type SharedRuntimeRegistry = Arc<RuntimeRegistry>;

mod retained_cleanup;
pub use retained_cleanup::{
    RuntimeRetainedCleanupError, RuntimeRetainedCleanupRefusal, RuntimeRetainedOwnerIdentity,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeRegistryError {
    #[error("runtime '{0}' model/producer identity changed before resident resource publication")]
    ModelResidencyObservationChanged(String),

    #[error("runtime '{runtime_id}' model '{model_id}' has no declared resident {resource_kind} estimate")]
    ModelResidencyResourcesUnavailable {
        runtime_id: String,
        model_id: String,
        resource_kind: &'static str,
    },

    #[error("runtime '{runtime_id}' resident resource declaration is invalid: {reason}")]
    InvalidModelResidencyResources {
        runtime_id: String,
        reason: &'static str,
    },
    #[error("resource domain '{domain_id}' is invalid: {reason}")]
    InvalidResourceDomain {
        domain_id: String,
        reason: &'static str,
    },

    #[error("resource domain '{domain_id}' byte accounting overflowed")]
    ResourceDomainAccountingOverflow { domain_id: String },

    #[error("runtime '{runtime_id}' cannot reserve {requested_bytes} bytes in resource domain '{domain_id}': {available_bytes} bytes available")]
    ResourceDomainAdmissionRejected {
        runtime_id: String,
        domain_id: String,
        requested_bytes: u64,
        available_bytes: u64,
    },
    #[error("runtime '{0}' is not registered")]
    RuntimeNotFound(String),

    #[error("runtime '{runtime_id}' cannot transition from {from:?} to {to:?}")]
    InvalidTransition {
        runtime_id: String,
        from: RuntimeRegistryStatus,
        to: RuntimeRegistryStatus,
    },

    #[error("reservation '{0}' was not found")]
    ReservationNotFound(u64),

    #[error("reservation identity sequence is exhausted")]
    ReservationSequenceExhausted,

    #[error("reservation '{0}' is awaiting custody transfer")]
    ReservationCustodyPending(u64),

    #[error("runtime '{0}' changed after reservation observation")]
    ReservationObservationChanged(String),

    #[error("runtime '{0}' cannot accept reservations while stopping or failed")]
    ReservationRejected(String),

    #[error(
        "reservation owner '{owner_id}' is already bound to runtime '{existing_runtime_id}', not '{requested_runtime_id}'"
    )]
    ReservationOwnerConflict {
        owner_id: String,
        existing_runtime_id: String,
        requested_runtime_id: String,
    },

    #[error("runtime '{runtime_id}' admission rejected reservation: {failure}")]
    AdmissionRejected {
        runtime_id: String,
        failure: RuntimeAdmissionFailure,
    },

    #[error(
        "runtime '{runtime_id}' reservation resource accounting overflowed for {resource_kind}"
    )]
    ResourceAccountingOverflow {
        runtime_id: String,
        resource_kind: &'static str,
    },

    #[error(
        "runtime '{runtime_id}' admission budget underflowed for {resource_kind}: total={total_bytes} bytes safety_margin={safety_margin_bytes} bytes reserved={reserved_bytes} bytes"
    )]
    ResourceBudgetUnderflow {
        runtime_id: String,
        resource_kind: &'static str,
        total_bytes: u64,
        safety_margin_bytes: u64,
        reserved_bytes: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRegistration {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub dispatch_identity: Option<RuntimeDispatchIdentity>,
    pub admission_budget: Option<RuntimeAdmissionBudget>,
}

impl RuntimeRegistration {
    pub fn new(runtime_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            display_name: display_name.into(),
            backend_keys: Vec::new(),
            dispatch_identity: None,
            admission_budget: None,
        }
    }

    pub fn with_backend_keys(mut self, backend_keys: Vec<String>) -> Self {
        self.backend_keys = backend_keys;
        self
    }

    pub fn with_dispatch_identity(mut self, dispatch_identity: RuntimeDispatchIdentity) -> Self {
        self.dispatch_identity = Some(dispatch_identity);
        self
    }

    pub fn with_admission_budget(mut self, admission_budget: RuntimeAdmissionBudget) -> Self {
        self.admission_budget = Some(admission_budget);
        self
    }
}

#[derive(Debug, Default)]
struct RuntimeRegistryState {
    resource_domains: BTreeMap<String, RuntimeResourceDomain>,
    host_ram_capacity_source: Option<Arc<dyn RuntimeHostRamCapacitySource>>,
    resident_estimates: BTreeMap<(String, String), RuntimeReservationRequirements>,
    producer_observations: BTreeMap<String, (String, u64)>,
    runtimes: BTreeMap<String, RuntimeRegistryRecord>,
    reservations: BTreeMap<u64, RuntimeReservationRecord>,
    pending_reservations: BTreeMap<u64, PendingReservation>,
    // Exact predecessor fence while rollback can restore its original ID.
    pending_predecessors: BTreeMap<u64, u64>,
    executing_reservations: BTreeMap<u64, u64>,
    retained_envelope_fences: BTreeMap<String, u64>,
    evicting_runtimes: BTreeMap<String, u64>,
}

#[derive(Debug, Default)]
pub struct RuntimeRegistry {
    state: Mutex<RuntimeRegistryState>,
    reservation_sequence: AtomicU64,
}

impl RuntimeRegistry {
    pub(crate) fn next_reservation_token(&self) -> Result<u64, RuntimeRegistryError> {
        self.reservation_sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map(|previous| previous + 1)
            .map_err(|_| RuntimeRegistryError::ReservationSequenceExhausted)
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_runtime(
        &self,
        registration: RuntimeRegistration,
    ) -> RuntimeRegistryRuntimeSnapshot {
        let runtime_id = canonical_runtime_id(&registration.runtime_id);
        let now_ms = unix_timestamp_ms();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let record = guard.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
            RuntimeRegistryRecord::new(&runtime_id, &registration.display_name, now_ms)
        });

        let RuntimeRegistration {
            runtime_id: _,
            display_name,
            backend_keys,
            dispatch_identity,
            admission_budget,
        } = registration;

        record.display_name = display_name.trim().to_string();
        record.set_backend_keys(backend_keys);
        record.set_dispatch_identity(dispatch_identity);
        record.runtime_id = runtime_id.clone();
        if let Some(admission_budget) = admission_budget {
            record.admission_budget = Some(admission_budget);
        }
        runtime_snapshot(record)
    }

    pub fn transition_runtime(
        &self,
        runtime_id: &str,
        transition: RuntimeTransition,
    ) -> Result<RuntimeRegistryRuntimeSnapshot, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(runtime_id);
        let now_ms = unix_timestamp_ms();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        reject_eviction_pending(&guard, &runtime_id)?;
        if guard.producer_observations.contains_key(&runtime_id) {
            let current_instance = guard
                .runtimes
                .get(&runtime_id)
                .and_then(|record| record.runtime_instance_id.as_ref());
            let changes_identity = match &transition {
                Transition::Stopped => true,
                Transition::WarmupStarted {
                    runtime_instance_id,
                }
                | Transition::Ready {
                    runtime_instance_id,
                }
                | Transition::Busy {
                    runtime_instance_id,
                } => runtime_instance_id
                    .as_ref()
                    .is_some_and(|instance| Some(instance) != current_instance),
                _ => false,
            };
            if changes_identity {
                return Err(RuntimeRegistryError::ModelResidencyObservationChanged(
                    runtime_id,
                ));
            }
        }
        let record = guard
            .runtimes
            .get_mut(&runtime_id)
            .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

        if !transition.can_transition_from(record.status) {
            return Err(RuntimeRegistryError::InvalidTransition {
                runtime_id,
                from: record.status,
                to: transition.target_status(),
            });
        }

        record.status = transition.target_status();
        record.last_transition_at_ms = now_ms;

        match transition {
            Transition::WarmupStarted {
                runtime_instance_id,
            }
            | Transition::Ready {
                runtime_instance_id,
            }
            | Transition::Busy {
                runtime_instance_id,
            } => {
                if let Some(runtime_instance_id) = runtime_instance_id {
                    if let Some(resident) = record.model_resource_residency.as_mut() {
                        if resident.runtime_instance_id.as_ref() != Some(&runtime_instance_id) {
                            resident.runtime_instance_id = Some(runtime_instance_id.clone());
                            resident.requirements = None;
                            record.models.clear();
                        }
                    }
                    record.runtime_instance_id = Some(runtime_instance_id);
                }
                if !matches!(
                    record.status,
                    RuntimeRegistryStatus::Unhealthy | RuntimeRegistryStatus::Failed
                ) {
                    record.last_error = None;
                }
            }
            Transition::Unhealthy { message } | Transition::Failed { message } => {
                record.last_error = Some(message);
            }
            Transition::StopRequested => {}
            Transition::Stopped => {
                record.runtime_instance_id = None;
                record.last_error = None;
                record.models.clear();
                record.model_resource_residency = None;
            }
        }

        Ok(runtime_snapshot(record))
    }

    pub fn acquire_reservation(
        &self,
        request: RuntimeReservationRequest,
    ) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
        self.acquire_reservation_observed(request, None)
    }

    fn acquire_reservation_observed(
        &self,
        request: RuntimeReservationRequest,
        expected: Option<&RuntimeReservationAdmissionObservation>,
    ) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
        let next_id = self.next_reservation_token()?;
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        if let Some(expected) = expected {
            check_observed_runtime_identity(&state, &request, expected)?;
        }
        let (reservation, previous) = prospective_reservation(&state, request, next_id)?;
        if let Some(previous) = previous {
            remove_reservation_locked(&mut state, previous.reservation_id);
        }
        state
            .runtimes
            .get_mut(&reservation.runtime_id)
            .expect("validated runtime")
            .active_reservations
            .insert(reservation.reservation_id);
        state
            .reservations
            .insert(reservation.reservation_id, reservation.clone());
        Ok(reservation.into_lease())
    }

    pub fn can_acquire_reservation(
        &self,
        request: &RuntimeReservationRequest,
    ) -> Result<(), RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(&request.runtime_id);
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");

        if let Some(owner_id) = request.reservation_owner_id.as_deref() {
            if let Some(existing_reservation_id) = guard
                .reservations
                .values()
                .find(|reservation| reservation.reservation_owner_id.as_deref() == Some(owner_id))
                .map(|reservation| reservation.reservation_id)
            {
                let existing_runtime_id = guard
                    .reservations
                    .get(&existing_reservation_id)
                    .expect("existing reservation should still exist")
                    .runtime_id
                    .clone();
                if existing_runtime_id != runtime_id {
                    return Err(RuntimeRegistryError::ReservationOwnerConflict {
                        owner_id: owner_id.to_string(),
                        existing_runtime_id,
                        requested_runtime_id: runtime_id,
                    });
                }

                return validate_reservation_request(
                    &guard,
                    &runtime_id,
                    request.requirements.as_ref(),
                    Some(existing_reservation_id),
                );
            }
        }

        validate_reservation_request(&guard, &runtime_id, request.requirements.as_ref(), None)
    }

    /// Exact selected-lease query; does not scan the registry population.
    pub fn reservation_lease(&self, reservation_id: u64) -> Option<RuntimeReservationLease> {
        self.state
            .lock()
            .expect("runtime registry state lock poisoned")
            .reservations
            .get(&reservation_id)
            .cloned()
            .map(RuntimeReservationRecord::into_lease)
    }

    pub fn release_reservation(&self, reservation_id: u64) -> Result<(), RuntimeRegistryError> {
        self.release_reservation_with_disposition(reservation_id)
            .map(|_| ())
    }

    pub fn release_reservation_if_present(
        &self,
        reservation_id: u64,
    ) -> Result<Option<RuntimeRetentionDisposition>, RuntimeRegistryError> {
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        release_reservation_locked(&mut guard, reservation_id)
    }

    pub fn release_reservation_with_disposition(
        &self,
        reservation_id: u64,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRegistryError> {
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        release_reservation_locked(&mut guard, reservation_id)?
            .ok_or(RuntimeRegistryError::ReservationNotFound(reservation_id))
    }

    pub fn update_reservation_retention_hint_if_present(
        &self,
        reservation_id: u64,
        retention_hint: RuntimeRetentionHint,
    ) -> Result<Option<RuntimeReservationLease>, RuntimeRegistryError> {
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        if guard.pending_predecessors.contains_key(&reservation_id)
            || guard.pending_reservations.contains_key(&reservation_id)
            || guard.executing_reservations.contains_key(&reservation_id)
        {
            return Err(RuntimeRegistryError::ReservationCustodyPending(
                reservation_id,
            ));
        }
        let Some(reservation) = guard.reservations.get_mut(&reservation_id) else {
            return Ok(None);
        };

        reservation.retention_hint = retention_hint;
        Ok(Some(reservation.clone().into_lease()))
    }

    pub fn update_reservation_retention_hint(
        &self,
        reservation_id: u64,
        retention_hint: RuntimeRetentionHint,
    ) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
        self.update_reservation_retention_hint_if_present(reservation_id, retention_hint)?
            .ok_or(RuntimeRegistryError::ReservationNotFound(reservation_id))
    }

    pub fn retention_disposition(
        &self,
        runtime_id: &str,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRegistryError> {
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        runtime_retention_disposition(runtime_id, &guard)
    }

    pub fn reclaim_runtime(
        &self,
        runtime_id: &str,
        producer_active: bool,
    ) -> Result<RuntimeReclaimDisposition, RuntimeRegistryError> {
        let now_ms = unix_timestamp_ms();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        runtime_reclaim(runtime_id, producer_active, &mut guard, now_ms)
    }

    pub fn warmup_disposition(
        &self,
        runtime_id: &str,
    ) -> Result<RuntimeWarmupDisposition, RuntimeRegistryError> {
        let guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        runtime_warmup_disposition(runtime_id, &guard)
    }

    pub fn observe_runtimes(
        &self,
        observations: Vec<RuntimeObservation>,
    ) -> Vec<RuntimeRegistryRuntimeSnapshot> {
        let now_ms = unix_timestamp_ms();
        let observed_runtime_ids = observations
            .iter()
            .map(|observation| canonical_runtime_id(&observation.runtime_id))
            .collect::<BTreeSet<_>>();
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");

        for observation in observations {
            apply_runtime_observation(&mut guard, observation, now_ms);
        }

        let bound_runtime_ids = guard
            .resource_domains
            .values()
            .flat_map(|domain| {
                domain
                    .bindings
                    .iter()
                    .map(|binding| binding.runtime_id.clone())
            })
            .collect::<BTreeSet<_>>();
        let owned_runtime_ids = guard
            .producer_observations
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        for record in guard.runtimes.values_mut() {
            if owned_runtime_ids.contains(&record.runtime_id)
                || observed_runtime_ids.contains(&record.runtime_id)
                || !record.active_reservations.is_empty()
                || record
                    .model_resource_residency
                    .as_ref()
                    .is_some_and(|resident| {
                        resident.requirements.is_some()
                            || bound_runtime_ids.contains(&record.runtime_id)
                    })
            {
                continue;
            }

            record.status = RuntimeRegistryStatus::Stopped;
            record.runtime_instance_id = None;
            record.last_error = None;
            record.models.clear();
            record.model_resource_residency = None;
            record.last_transition_at_ms = now_ms;
        }

        let mut snapshots = guard
            .runtimes
            .values()
            .map(runtime_snapshot)
            .collect::<Vec<_>>();
        snapshots.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        snapshots
    }

    pub fn observe_runtime(
        &self,
        observation: RuntimeObservation,
    ) -> RuntimeRegistryRuntimeSnapshot {
        let now_ms = unix_timestamp_ms();
        let runtime_id = canonical_runtime_id(&observation.runtime_id);
        let mut guard = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");

        apply_runtime_observation(&mut guard, observation, now_ms);

        let record = guard
            .runtimes
            .get(&runtime_id)
            .expect("observed runtime must exist after observation");
        runtime_snapshot(record)
    }
}

fn runtime_retention_disposition(
    runtime_id: &str,
    state: &RuntimeRegistryState,
) -> Result<RuntimeRetentionDisposition, RuntimeRegistryError> {
    let runtime_id = canonical_runtime_id(runtime_id);
    let record = state
        .runtimes
        .get(&runtime_id)
        .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

    if record.active_reservations.iter().any(|reservation_id| {
        state
            .reservations
            .get(reservation_id)
            .map(|reservation| reservation.retention_hint == RuntimeRetentionHint::KeepAlive)
            .unwrap_or(false)
    }) {
        return Ok(RuntimeRetentionDisposition::retain(
            runtime_id,
            RuntimeRetentionReason::KeepAliveReservation,
        ));
    }

    if !record.active_reservations.is_empty() {
        return Ok(RuntimeRetentionDisposition::retain(
            runtime_id,
            RuntimeRetentionReason::ActiveReservations,
        ));
    }

    if record.models.values().any(|model| model.pinned) {
        return Ok(RuntimeRetentionDisposition::retain(
            runtime_id,
            RuntimeRetentionReason::PinnedModel,
        ));
    }

    if runtime_is_evictable(record) {
        return Ok(RuntimeRetentionDisposition::evict(runtime_id));
    }

    Ok(RuntimeRetentionDisposition::retain(
        runtime_id,
        RuntimeRetentionReason::Status(record.status),
    ))
}

fn runtime_warmup_disposition(
    runtime_id: &str,
    state: &RuntimeRegistryState,
) -> Result<RuntimeWarmupDisposition, RuntimeRegistryError> {
    let runtime_id = canonical_runtime_id(runtime_id);
    let record = state
        .runtimes
        .get(&runtime_id)
        .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

    let disposition = match record.status {
        RuntimeRegistryStatus::Stopped => RuntimeWarmupDisposition::start(
            runtime_id,
            RuntimeWarmupReason::NoLoadedInstance,
            record.status,
        ),
        RuntimeRegistryStatus::Failed | RuntimeRegistryStatus::Unhealthy => {
            RuntimeWarmupDisposition::start(
                runtime_id,
                RuntimeWarmupReason::RecoveryRequired,
                record.status,
            )
        }
        RuntimeRegistryStatus::Ready => RuntimeWarmupDisposition::reuse(
            runtime_id,
            RuntimeWarmupReason::LoadedInstanceReady,
            record.status,
            record.runtime_instance_id.clone(),
        ),
        RuntimeRegistryStatus::Busy => RuntimeWarmupDisposition::reuse(
            runtime_id,
            RuntimeWarmupReason::LoadedInstanceBusy,
            record.status,
            record.runtime_instance_id.clone(),
        ),
        RuntimeRegistryStatus::Warming => RuntimeWarmupDisposition::wait(
            runtime_id,
            RuntimeWarmupReason::WarmupInProgress,
            record.status,
            record.runtime_instance_id.clone(),
        ),
        RuntimeRegistryStatus::Stopping => RuntimeWarmupDisposition::wait(
            runtime_id,
            RuntimeWarmupReason::StopInProgress,
            record.status,
            record.runtime_instance_id.clone(),
        ),
    };

    Ok(disposition)
}

fn runtime_reclaim(
    runtime_id: &str,
    producer_active: bool,
    state: &mut RuntimeRegistryState,
    now_ms: u64,
) -> Result<RuntimeReclaimDisposition, RuntimeRegistryError> {
    reject_eviction_pending(state, &canonical_runtime_id(runtime_id))?;
    let retention = runtime_retention_disposition(runtime_id, state)?;
    let runtime_id = retention.runtime_id.clone();
    let record = state
        .runtimes
        .get_mut(&runtime_id)
        .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

    if retention.decision == RuntimeRetentionDecision::Retain {
        return Ok(RuntimeReclaimDisposition::no_action(
            runtime_id,
            retention.reason,
            record.status,
        ));
    }

    if producer_active
        || (state.producer_observations.contains_key(&runtime_id)
            && (record.model_resource_residency.is_some() || record.resident_resources_uncertain))
    {
        if record.status != RuntimeRegistryStatus::Stopping {
            record.status = RuntimeRegistryStatus::Stopping;
            record.last_transition_at_ms = now_ms;
        }

        return Ok(RuntimeReclaimDisposition::stop_producer(
            runtime_id,
            record.status,
        ));
    }

    if record.status != RuntimeRegistryStatus::Stopped {
        record.status = RuntimeRegistryStatus::Stopped;
        record.runtime_instance_id = None;
        record.last_error = None;
        record.models.clear();
        record.model_resource_residency = None;
        record.last_transition_at_ms = now_ms;
    }

    Ok(RuntimeReclaimDisposition::no_action(
        runtime_id,
        RuntimeRetentionReason::Evictable,
        record.status,
    ))
}

fn release_reservation_locked(
    state: &mut RuntimeRegistryState,
    reservation_id: u64,
) -> Result<Option<RuntimeRetentionDisposition>, RuntimeRegistryError> {
    if state.pending_predecessors.contains_key(&reservation_id)
        || state.executing_reservations.contains_key(&reservation_id)
    {
        return Err(RuntimeRegistryError::ReservationCustodyPending(
            reservation_id,
        ));
    }
    let Some(reservation) = remove_reservation_locked(state, reservation_id) else {
        return Ok(None);
    };
    runtime_retention_disposition(&reservation.runtime_id, state).map(Some)
}

fn remove_reservation_locked(
    state: &mut RuntimeRegistryState,
    reservation_id: u64,
) -> Option<RuntimeReservationRecord> {
    // Explicit release ends this lease's lineage. A later custody drop must not
    // resurrect an owner-ended predecessor; only custody rollback restores it.
    if let Some(pending) = state.pending_reservations.remove(&reservation_id) {
        if let Some(previous) = pending.previous {
            state.pending_predecessors.remove(&previous.reservation_id);
        }
    }
    let reservation = state.reservations.remove(&reservation_id)?;

    if let Some(runtime) = state.runtimes.get_mut(&reservation.runtime_id) {
        runtime.active_reservations.remove(&reservation_id);
    }

    Some(reservation)
}

fn admission_failure(
    record: &RuntimeRegistryRecord,
    claim: RuntimeReservationClaim,
    reservations: &BTreeMap<u64, RuntimeReservationRecord>,
    excluded_reservation_id: Option<u64>,
) -> Result<Option<RuntimeAdmissionFailure>, RuntimeRegistryError> {
    let Some(budget) = record.admission_budget.as_ref() else {
        return Ok(None);
    };

    if let Some(requested_ram_bytes) = claim.ram_bytes {
        let resource_kind = RuntimeAdmissionResourceKind::RamBytes.resource_label();
        let reserved_ram_bytes = total_reserved_resource_bytes(
            record,
            resource_kind,
            reservations,
            excluded_reservation_id,
            |reservation| reservation.claim.ram_bytes,
        )?;
        let ram_budget = budget.resource_budget(RuntimeAdmissionResourceKind::RamBytes);
        let available_ram_bytes = available_budget_bytes(
            &record.runtime_id,
            resource_kind,
            ram_budget,
            reserved_ram_bytes,
        )?;
        if requested_ram_bytes > available_ram_bytes {
            return Ok(Some(RuntimeAdmissionFailure::InsufficientRam {
                requested_bytes: requested_ram_bytes,
                available_bytes: available_ram_bytes,
                reserved_bytes: reserved_ram_bytes,
                total_bytes: ram_budget
                    .and_then(|budget| budget.total_bytes)
                    .unwrap_or(0),
                safety_margin_bytes: ram_budget
                    .map(|budget| budget.safety_margin_bytes)
                    .unwrap_or(0),
            }));
        }
    }

    if let Some(requested_vram_bytes) = claim.vram_bytes {
        let resource_kind = RuntimeAdmissionResourceKind::VramBytes.resource_label();
        let reserved_vram_bytes = total_reserved_resource_bytes(
            record,
            resource_kind,
            reservations,
            excluded_reservation_id,
            |reservation| reservation.claim.vram_bytes,
        )?;
        let vram_budget = budget.resource_budget(RuntimeAdmissionResourceKind::VramBytes);
        let available_vram_bytes = available_budget_bytes(
            &record.runtime_id,
            resource_kind,
            vram_budget,
            reserved_vram_bytes,
        )?;
        if requested_vram_bytes > available_vram_bytes {
            return Ok(Some(RuntimeAdmissionFailure::InsufficientVram {
                requested_bytes: requested_vram_bytes,
                available_bytes: available_vram_bytes,
                reserved_bytes: reserved_vram_bytes,
                total_bytes: vram_budget
                    .and_then(|budget| budget.total_bytes)
                    .unwrap_or(0),
                safety_margin_bytes: vram_budget
                    .map(|budget| budget.safety_margin_bytes)
                    .unwrap_or(0),
            }));
        }
    }

    Ok(None)
}

fn validate_reservation_request(
    state: &RuntimeRegistryState,
    runtime_id: &str,
    requirements: Option<&RuntimeReservationRequirements>,
    existing_reservation_id: Option<u64>,
) -> Result<(), RuntimeRegistryError> {
    reject_eviction_pending(state, runtime_id)?;
    let record = state
        .runtimes
        .get(runtime_id)
        .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.to_string()))?;

    if matches!(
        record.status,
        RuntimeRegistryStatus::Stopping | RuntimeRegistryStatus::Failed
    ) {
        return Err(RuntimeRegistryError::ReservationRejected(
            runtime_id.to_string(),
        ));
    }

    let claim = reservation_claim_from_requirements(runtime_id, requirements)?;
    if let Some(failure) =
        admission_failure(record, claim, &state.reservations, existing_reservation_id)?
    {
        return Err(RuntimeRegistryError::AdmissionRejected {
            runtime_id: runtime_id.to_string(),
            failure,
        });
    }
    resource_domain::validate_domain_admission(state, runtime_id, claim, existing_reservation_id)
}

fn reservation_claim_from_requirements(
    runtime_id: &str,
    requirements: Option<&RuntimeReservationRequirements>,
) -> Result<RuntimeReservationClaim, RuntimeRegistryError> {
    let Some(requirements) = requirements else {
        return Ok(RuntimeReservationClaim::default());
    };

    let mut claim = RuntimeReservationClaim::default();
    for resource_claim in &requirements.claims {
        match resource_claim.kind {
            RuntimeAdmissionResourceKind::RamBytes => {
                claim.ram_bytes = Some(add_optional_resource_claim_bytes(
                    runtime_id,
                    RuntimeAdmissionResourceKind::RamBytes.resource_label(),
                    claim.ram_bytes,
                    resource_claim.bytes,
                )?);
            }
            RuntimeAdmissionResourceKind::VramBytes => {
                claim.vram_bytes = Some(add_optional_resource_claim_bytes(
                    runtime_id,
                    RuntimeAdmissionResourceKind::VramBytes.resource_label(),
                    claim.vram_bytes,
                    resource_claim.bytes,
                )?);
            }
        }
    }

    Ok(claim)
}

fn add_optional_resource_claim_bytes(
    runtime_id: &str,
    resource_kind: &'static str,
    current_bytes: Option<u64>,
    additional_bytes: u64,
) -> Result<u64, RuntimeRegistryError> {
    current_bytes
        .unwrap_or(0)
        .checked_add(additional_bytes)
        .ok_or_else(|| RuntimeRegistryError::ResourceAccountingOverflow {
            runtime_id: runtime_id.to_string(),
            resource_kind,
        })
}

fn total_reserved_resource_bytes<F>(
    record: &RuntimeRegistryRecord,
    resource_kind: &'static str,
    reservations: &BTreeMap<u64, RuntimeReservationRecord>,
    excluded_reservation_id: Option<u64>,
    claim_bytes: F,
) -> Result<u64, RuntimeRegistryError>
where
    F: Fn(&RuntimeReservationRecord) -> Option<u64>,
{
    let runtime_id = record.runtime_id.as_str();
    let resident_bytes = model_resources::resident_bytes(
        record,
        match resource_kind {
            "ram_bytes" => RuntimeAdmissionResourceKind::RamBytes,
            "vram_bytes" => RuntimeAdmissionResourceKind::VramBytes,
            _ => unreachable!("known resource kind"),
        },
        false,
    )?;
    reservations
        .values()
        .filter(|reservation| reservation.runtime_id == runtime_id)
        .filter(|reservation| Some(reservation.reservation_id) != excluded_reservation_id)
        .filter_map(claim_bytes)
        .try_fold(resident_bytes, |total, claim_bytes| {
            total.checked_add(claim_bytes).ok_or_else(|| {
                RuntimeRegistryError::ResourceAccountingOverflow {
                    runtime_id: runtime_id.to_string(),
                    resource_kind,
                }
            })
        })
}

fn available_budget_bytes(
    runtime_id: &str,
    resource_kind: &'static str,
    budget: Option<&RuntimeAdmissionResourceBudget>,
    reserved_bytes: u64,
) -> Result<u64, RuntimeRegistryError> {
    let total_bytes = budget
        .and_then(|budget| budget.total_bytes)
        .unwrap_or(u64::MAX);
    let safety_margin_bytes = budget.map(|budget| budget.safety_margin_bytes).unwrap_or(0);
    let after_safety = total_bytes
        .checked_sub(safety_margin_bytes)
        .ok_or_else(|| RuntimeRegistryError::ResourceBudgetUnderflow {
            runtime_id: runtime_id.to_string(),
            resource_kind,
            total_bytes,
            safety_margin_bytes,
            reserved_bytes,
        })?;
    after_safety.checked_sub(reserved_bytes).ok_or_else(|| {
        RuntimeRegistryError::ResourceBudgetUnderflow {
            runtime_id: runtime_id.to_string(),
            resource_kind,
            total_bytes,
            safety_margin_bytes,
            reserved_bytes,
        }
    })
}

fn apply_runtime_observation(
    state: &mut RuntimeRegistryState,
    observation: RuntimeObservation,
    now_ms: u64,
) {
    let runtime_id = canonical_runtime_id(&observation.runtime_id);
    if state.evicting_runtimes.contains_key(&runtime_id) {
        return;
    }
    if state.producer_observations.contains_key(&runtime_id) {
        // Matching health assessments may make dispatch less permissive, but
        // unsequenced projections never replace identity or release allocation.
        if let Some(record) = state.runtimes.get_mut(&runtime_id) {
            if matches!(
                observation.status,
                RuntimeRegistryStatus::Unhealthy | RuntimeRegistryStatus::Failed
            ) && observation.runtime_instance_id.is_some()
                && observation.runtime_instance_id == record.runtime_instance_id
                && observation.model_id.as_ref()
                    == record
                        .model_resource_residency
                        .as_ref()
                        .map(|resident| &resident.model_id)
            {
                record.status = observation.status;
                record.last_error = observation.last_error;
            }
        }
        return;
    }
    apply_producer_runtime_observation(state, observation, now_ms);
}

fn apply_producer_runtime_observation(
    state: &mut RuntimeRegistryState,
    observation: RuntimeObservation,
    now_ms: u64,
) {
    let runtime_id = canonical_runtime_id(&observation.runtime_id);
    let record = state.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
        RuntimeRegistryRecord::new(&runtime_id, &observation.display_name, now_ms)
    });

    model_resources::observe_model_resources(record, &observation);

    record.runtime_id = runtime_id;
    record.display_name = observation.display_name;
    record.set_backend_keys(observation.backend_keys);
    record.status = observation.status;
    record.runtime_instance_id = match observation.status {
        RuntimeRegistryStatus::Stopped => None,
        _ => observation.runtime_instance_id,
    };
    record.last_error = observation.last_error;
    record.last_transition_at_ms = now_ms;
    sync_observed_models(record, observation.model_id, observation.status, now_ms);
}

fn sync_observed_models(
    record: &mut RuntimeRegistryRecord,
    model_id: Option<String>,
    status: RuntimeRegistryStatus,
    now_ms: u64,
) {
    if matches!(
        status,
        RuntimeRegistryStatus::Stopped | RuntimeRegistryStatus::Failed
    ) {
        record.models.clear();
        return;
    }

    let Some(model_id) = model_id else {
        record.models.clear();
        return;
    };

    let existing_loaded_at_ms = record
        .models
        .get(&model_id)
        .map(|model| model.loaded_at_ms)
        .unwrap_or(now_ms);
    record.models.clear();
    record.models.insert(
        model_id.clone(),
        RuntimeModelResidencyRecord {
            model_id,
            usage_profile: None,
            pinned: false,
            loaded_at_ms: existing_loaded_at_ms,
        },
    );
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

fn reject_eviction_pending(
    state: &RuntimeRegistryState,
    runtime_id: &str,
) -> Result<(), RuntimeRegistryError> {
    if state.evicting_runtimes.contains_key(runtime_id) {
        Err(RuntimeRegistryError::ReservationRejected(runtime_id.into()))
    } else {
        Ok(())
    }
}
