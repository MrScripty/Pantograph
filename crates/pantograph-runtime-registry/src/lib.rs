mod admission;
mod observation;
mod reclaim;
mod registry_queries;
mod reservation;
mod retention;
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
    RuntimeAdmissionBudget, RuntimeAdmissionFailure, RuntimeReservationRequirements,
};
pub use observation::{observed_runtime_status_from_lifecycle, RuntimeObservation};
use pantograph_runtime_identity::canonical_runtime_id;
pub use reclaim::{RuntimeReclaimAction, RuntimeReclaimDisposition};
use reservation::RuntimeReservationRecord;
pub use reservation::{RuntimeReservationLease, RuntimeReservationRequest, RuntimeRetentionHint};
pub use retention::{
    RuntimeRetentionDecision, RuntimeRetentionDisposition, RuntimeRetentionReason,
};
pub use snapshot::{RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot};
use state::RuntimeTransition as Transition;
pub use state::{
    RuntimeModelResidencyRecord, RuntimeRegistryRecord, RuntimeRegistryStatus, RuntimeTransition,
};
pub use technical_fit::{
    select_runtime_technical_fit, RuntimeTechnicalFitCandidate,
    RuntimeTechnicalFitCandidateSourceKind, RuntimeTechnicalFitDecision, RuntimeTechnicalFitFactor,
    RuntimeTechnicalFitOverride, RuntimeTechnicalFitReason, RuntimeTechnicalFitReasonCode,
    RuntimeTechnicalFitRequest, RuntimeTechnicalFitResidencyState,
    RuntimeTechnicalFitResourcePressure, RuntimeTechnicalFitSelectionMode,
    RuntimeTechnicalFitWarmupState,
};
pub use warmup::{RuntimeWarmupDecision, RuntimeWarmupDisposition, RuntimeWarmupReason};

pub type SharedRuntimeRegistry = Arc<RuntimeRegistry>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeRegistryError {
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRegistration {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub admission_budget: Option<RuntimeAdmissionBudget>,
}

impl RuntimeRegistration {
    pub fn new(runtime_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            display_name: display_name.into(),
            backend_keys: Vec::new(),
            admission_budget: None,
        }
    }

    pub fn with_backend_keys(mut self, backend_keys: Vec<String>) -> Self {
        self.backend_keys = backend_keys;
        self
    }

    pub fn with_admission_budget(mut self, admission_budget: RuntimeAdmissionBudget) -> Self {
        self.admission_budget = Some(admission_budget);
        self
    }
}

#[derive(Debug, Default)]
struct RuntimeRegistryState {
    runtimes: BTreeMap<String, RuntimeRegistryRecord>,
    reservations: BTreeMap<u64, RuntimeReservationRecord>,
}

#[derive(Debug, Default)]
pub struct RuntimeRegistry {
    state: Mutex<RuntimeRegistryState>,
    reservation_sequence: AtomicU64,
}

impl RuntimeRegistry {
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
            admission_budget,
        } = registration;

        record.display_name = display_name.trim().to_string();
        record.set_backend_keys(backend_keys);
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
            }
        }

        Ok(runtime_snapshot(record))
    }

    pub fn acquire_reservation(
        &self,
        request: RuntimeReservationRequest,
    ) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
        let runtime_id = canonical_runtime_id(&request.runtime_id);
        let created_at_ms = unix_timestamp_ms();
        let reservation_id = self.reservation_sequence.fetch_add(1, Ordering::Relaxed) + 1;

        let mut guard = self
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
                if existing_runtime_id == runtime_id {
                    return update_existing_reservation_from_request(
                        &mut guard,
                        existing_reservation_id,
                        request,
                    );
                }

                return Err(RuntimeRegistryError::ReservationOwnerConflict {
                    owner_id: owner_id.to_string(),
                    existing_runtime_id,
                    requested_runtime_id: runtime_id,
                });
            }
        }
        let record = guard
            .runtimes
            .get(&runtime_id)
            .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

        if matches!(
            record.status,
            RuntimeRegistryStatus::Stopping | RuntimeRegistryStatus::Failed
        ) {
            return Err(RuntimeRegistryError::ReservationRejected(runtime_id));
        }

        let claim = RuntimeReservationClaim::from_requirements(request.requirements.as_ref());
        if let Some(failure) = admission_failure(record, claim, &guard.reservations, None) {
            return Err(RuntimeRegistryError::AdmissionRejected {
                runtime_id,
                failure,
            });
        }

        let record = guard
            .runtimes
            .get_mut(&runtime_id)
            .expect("runtime must exist after admission check");

        let reservation = RuntimeReservationRecord {
            reservation_id,
            runtime_id: runtime_id.clone(),
            workflow_id: request.workflow_id,
            reservation_owner_id: request.reservation_owner_id,
            usage_profile: request.usage_profile,
            model_id: request.model_id,
            pin_runtime: request.pin_runtime,
            retention_hint: request.retention_hint,
            created_at_ms,
            claim,
        };

        record.active_reservations.insert(reservation_id);
        guard
            .reservations
            .insert(reservation_id, reservation.clone());

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

        for record in guard.runtimes.values_mut() {
            if observed_runtime_ids.contains(&record.runtime_id)
                || !record.active_reservations.is_empty()
            {
                continue;
            }

            record.status = RuntimeRegistryStatus::Stopped;
            record.runtime_instance_id = None;
            record.last_error = None;
            record.models.clear();
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

    if producer_active {
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
    let Some(reservation) = state.reservations.remove(&reservation_id) else {
        return Ok(None);
    };

    if let Some(runtime) = state.runtimes.get_mut(&reservation.runtime_id) {
        runtime.active_reservations.remove(&reservation_id);
    }

    runtime_retention_disposition(&reservation.runtime_id, state).map(Some)
}

fn admission_failure(
    record: &RuntimeRegistryRecord,
    claim: RuntimeReservationClaim,
    reservations: &BTreeMap<u64, RuntimeReservationRecord>,
    excluded_reservation_id: Option<u64>,
) -> Option<RuntimeAdmissionFailure> {
    let budget = record.admission_budget.as_ref()?;

    if let Some(requested_ram_mb) = claim.ram_mb {
        let reserved_ram_mb = total_reserved_resource_mb(
            &record.runtime_id,
            reservations,
            excluded_reservation_id,
            |reservation| reservation.claim.ram_mb,
        );
        let available_ram_mb = available_budget_mb(
            budget.total_ram_mb,
            budget.safety_margin_ram_mb,
            reserved_ram_mb,
        );
        if requested_ram_mb > available_ram_mb {
            return Some(RuntimeAdmissionFailure::InsufficientRam {
                requested_mb: requested_ram_mb,
                available_mb: available_ram_mb,
                reserved_mb: reserved_ram_mb,
                total_mb: budget.total_ram_mb.unwrap_or(0),
                safety_margin_mb: budget.safety_margin_ram_mb,
            });
        }
    }

    if let Some(requested_vram_mb) = claim.vram_mb {
        let reserved_vram_mb = total_reserved_resource_mb(
            &record.runtime_id,
            reservations,
            excluded_reservation_id,
            |reservation| reservation.claim.vram_mb,
        );
        let available_vram_mb = available_budget_mb(
            budget.total_vram_mb,
            budget.safety_margin_vram_mb,
            reserved_vram_mb,
        );
        if requested_vram_mb > available_vram_mb {
            return Some(RuntimeAdmissionFailure::InsufficientVram {
                requested_mb: requested_vram_mb,
                available_mb: available_vram_mb,
                reserved_mb: reserved_vram_mb,
                total_mb: budget.total_vram_mb.unwrap_or(0),
                safety_margin_mb: budget.safety_margin_vram_mb,
            });
        }
    }

    None
}

fn validate_reservation_request(
    state: &RuntimeRegistryState,
    runtime_id: &str,
    requirements: Option<&RuntimeReservationRequirements>,
    existing_reservation_id: Option<u64>,
) -> Result<(), RuntimeRegistryError> {
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

    let claim = RuntimeReservationClaim::from_requirements(requirements);
    if let Some(failure) =
        admission_failure(record, claim, &state.reservations, existing_reservation_id)
    {
        return Err(RuntimeRegistryError::AdmissionRejected {
            runtime_id: runtime_id.to_string(),
            failure,
        });
    }

    Ok(())
}

fn total_reserved_resource_mb<F>(
    runtime_id: &str,
    reservations: &BTreeMap<u64, RuntimeReservationRecord>,
    excluded_reservation_id: Option<u64>,
    claim_mb: F,
) -> u64
where
    F: Fn(&RuntimeReservationRecord) -> Option<u64>,
{
    reservations
        .values()
        .filter(|reservation| reservation.runtime_id == runtime_id)
        .filter(|reservation| Some(reservation.reservation_id) != excluded_reservation_id)
        .filter_map(claim_mb)
        .sum()
}

fn available_budget_mb(total_mb: Option<u64>, safety_margin_mb: u64, reserved_mb: u64) -> u64 {
    total_mb
        .unwrap_or(u64::MAX)
        .saturating_sub(safety_margin_mb)
        .saturating_sub(reserved_mb)
}

fn update_existing_reservation_from_request(
    state: &mut RuntimeRegistryState,
    reservation_id: u64,
    request: RuntimeReservationRequest,
) -> Result<RuntimeReservationLease, RuntimeRegistryError> {
    let runtime_id = canonical_runtime_id(&request.runtime_id);
    let claim = RuntimeReservationClaim::from_requirements(request.requirements.as_ref());
    let record = state
        .runtimes
        .get(&runtime_id)
        .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;

    if matches!(
        record.status,
        RuntimeRegistryStatus::Stopping | RuntimeRegistryStatus::Failed
    ) {
        return Err(RuntimeRegistryError::ReservationRejected(runtime_id));
    }

    if let Some(failure) =
        admission_failure(record, claim, &state.reservations, Some(reservation_id))
    {
        return Err(RuntimeRegistryError::AdmissionRejected {
            runtime_id,
            failure,
        });
    }

    let reservation = state
        .reservations
        .get_mut(&reservation_id)
        .expect("existing reservation should still exist");
    reservation.workflow_id = request.workflow_id;
    reservation.usage_profile = request.usage_profile;
    reservation.model_id = request.model_id;
    reservation.pin_runtime = request.pin_runtime;
    reservation.retention_hint = request.retention_hint;
    reservation.claim = claim;

    Ok(reservation.clone().into_lease())
}

fn apply_runtime_observation(
    state: &mut RuntimeRegistryState,
    observation: RuntimeObservation,
    now_ms: u64,
) {
    let runtime_id = canonical_runtime_id(&observation.runtime_id);
    let record = state.runtimes.entry(runtime_id.clone()).or_insert_with(|| {
        RuntimeRegistryRecord::new(&runtime_id, &observation.display_name, now_ms)
    });

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
