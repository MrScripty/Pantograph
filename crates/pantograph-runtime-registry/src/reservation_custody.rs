use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::reservation::RuntimeReservationRecord;
use crate::{
    admission_failure, canonical_runtime_id, reservation_claim_from_requirements,
    validate_reservation_request, RuntimeRegistry, RuntimeRegistryError, RuntimeRegistryState,
    RuntimeReservationAdmissionObservation, RuntimeReservationLease, RuntimeReservationRequest,
    RuntimeRetentionHint,
};

#[derive(Debug)]
pub enum RuntimeReservationPublicationError<E> {
    Registry(RuntimeRegistryError),
    Validation(E),
}

#[derive(Debug)]
pub(crate) struct PendingReservation {
    token: u64,
    previous: Option<RuntimeReservationRecord>,
    committed: RuntimeReservationRecord,
}

/// Owns a provisional claim until another owner records its cleanup intent.
/// Drop rolls it back. A failed replacement restores the entire previous lease.
/// This token cannot be cloned or serialized; candidate facts are not custody.
#[must_use]
pub struct RuntimeReservationCustody {
    registry: Arc<RuntimeRegistry>,
    reservation_id: u64,
    token: Option<u64>,
}

impl std::fmt::Debug for RuntimeReservationCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeReservationCustody")
            .field("reservation_id", &self.reservation_id)
            .finish_non_exhaustive()
    }
}

impl RuntimeReservationCustody {
    /// Transfer only after another owner has recorded its cleanup intent.
    pub fn transfer(mut self) -> Result<(), RuntimeRegistryError> {
        let mut state = self
            .registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let Some(pending) = state.pending_reservations.get(&self.reservation_id) else {
            return Err(RuntimeRegistryError::ReservationNotFound(
                self.reservation_id,
            ));
        };
        if Some(pending.token) != self.token {
            return Err(RuntimeRegistryError::ReservationCustodyPending(
                self.reservation_id,
            ));
        }
        let pending = state
            .pending_reservations
            .remove(&self.reservation_id)
            .expect("checked pending lease");
        state
            .reservations
            .insert(self.reservation_id, pending.committed);
        self.token = None;
        Ok(())
    }
}

impl Drop for RuntimeReservationCustody {
    fn drop(&mut self) {
        let Some(token) = self.token.take() else {
            return;
        };
        // Only restore this token's owned mutation, including during unwinding.
        let mut state = self
            .registry
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state
            .pending_reservations
            .get(&self.reservation_id)
            .is_some_and(|pending| pending.token == token)
        {
            rollback_pending_reservation(&mut state, self.reservation_id);
        }
    }
}

impl RuntimeRegistry {
    /// Validate the final publication under the same lock as admission/identity.
    /// The callback receives a prospective lease and must do synchronous, pure
    /// validation; it must not call the registry or publish external side effects.
    /// No reservation mutation occurs unless it succeeds. Returned custody keeps
    /// both old and new capacity protected until transfer or rollback.
    pub fn acquire_reservation_provisional<T, E>(
        self: &Arc<Self>,
        request: RuntimeReservationRequest,
        expected: &RuntimeReservationAdmissionObservation,
        validate: impl FnOnce(&RuntimeReservationLease) -> Result<T, E>,
    ) -> Result<(T, RuntimeReservationCustody), RuntimeReservationPublicationError<E>> {
        let token = self.reservation_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        check_observed_runtime_identity(&state, &request, expected)
            .map_err(RuntimeReservationPublicationError::Registry)?;
        let (committed, previous) = prospective_reservation(&state, request, token)
            .map_err(RuntimeReservationPublicationError::Registry)?;
        let reservation_id = committed.reservation_id;
        let mut held = committed.clone();
        if let Some(previous) = &previous {
            // A shrinking replacement must not free the predecessor's capacity
            // while rollback still promises to restore it.
            held.claim.ram_bytes = held.claim.ram_bytes.max(previous.claim.ram_bytes);
            held.claim.vram_bytes = held.claim.vram_bytes.max(previous.claim.vram_bytes);
            held.pin_runtime |= previous.pin_runtime;
            if previous.retention_hint == RuntimeRetentionHint::KeepAlive {
                held.retention_hint = RuntimeRetentionHint::KeepAlive;
            }
        }
        let runtime = state
            .runtimes
            .get(&held.runtime_id)
            .expect("validated runtime");
        if let Some(failure) = admission_failure(
            runtime,
            held.claim,
            &state.reservations,
            previous.as_ref().map(|record| record.reservation_id),
        )
        .map_err(RuntimeReservationPublicationError::Registry)?
        {
            return Err(RuntimeReservationPublicationError::Registry(
                RuntimeRegistryError::AdmissionRejected {
                    runtime_id: held.runtime_id.clone(),
                    failure,
                },
            ));
        }
        crate::resource_domain::validate_domain_admission(
            &state,
            &held.runtime_id,
            held.claim,
            previous.as_ref().map(|record| record.reservation_id),
        )
        .map_err(RuntimeReservationPublicationError::Registry)?;
        let output = validate(&committed.clone().into_lease())
            .map_err(RuntimeReservationPublicationError::Validation)?;
        state
            .runtimes
            .get_mut(&held.runtime_id)
            .expect("validated runtime")
            .active_reservations
            .insert(reservation_id);
        state.reservations.insert(reservation_id, held);
        state.pending_reservations.insert(
            reservation_id,
            PendingReservation {
                token,
                previous,
                committed,
            },
        );
        Ok((
            output,
            RuntimeReservationCustody {
                registry: self.clone(),
                reservation_id,
                token: Some(token),
            },
        ))
    }
}

pub(crate) fn check_observed_runtime_identity(
    state: &RuntimeRegistryState,
    request: &RuntimeReservationRequest,
    expected: &RuntimeReservationAdmissionObservation,
) -> Result<(), RuntimeRegistryError> {
    let runtime_id = canonical_runtime_id(&request.runtime_id);
    let runtime = state
        .runtimes
        .get(&runtime_id)
        .ok_or_else(|| RuntimeRegistryError::RuntimeNotFound(runtime_id.clone()))?;
    if matches!(
        runtime.status,
        crate::RuntimeRegistryStatus::Stopping | crate::RuntimeRegistryStatus::Failed
    ) {
        return Err(RuntimeRegistryError::ReservationRejected(runtime_id));
    }
    if expected.runtime_id != runtime_id
        || expected.runtime_status != runtime.status
        || expected.runtime_instance_id != runtime.runtime_instance_id
    {
        return Err(RuntimeRegistryError::ReservationObservationChanged(
            runtime_id,
        ));
    }
    Ok(())
}

pub(crate) fn prospective_reservation(
    state: &RuntimeRegistryState,
    request: RuntimeReservationRequest,
    next_id: u64,
) -> Result<(RuntimeReservationRecord, Option<RuntimeReservationRecord>), RuntimeRegistryError> {
    let runtime_id = canonical_runtime_id(&request.runtime_id);
    let previous = request
        .reservation_owner_id
        .as_deref()
        .and_then(|owner| {
            state
                .reservations
                .values()
                .find(|record| record.reservation_owner_id.as_deref() == Some(owner))
        })
        .cloned();
    if let Some(previous) = &previous {
        if previous.runtime_id != runtime_id {
            return Err(RuntimeRegistryError::ReservationOwnerConflict {
                owner_id: previous
                    .reservation_owner_id
                    .clone()
                    .expect("matched owner"),
                existing_runtime_id: previous.runtime_id.clone(),
                requested_runtime_id: runtime_id,
            });
        }
        if state
            .pending_reservations
            .contains_key(&previous.reservation_id)
        {
            return Err(RuntimeRegistryError::ReservationCustodyPending(
                previous.reservation_id,
            ));
        }
    }
    validate_reservation_request(
        state,
        &runtime_id,
        request.requirements.as_ref(),
        previous.as_ref().map(|record| record.reservation_id),
    )?;
    let claim = reservation_claim_from_requirements(&runtime_id, request.requirements.as_ref())?;
    Ok((
        RuntimeReservationRecord {
            reservation_id: previous
                .as_ref()
                .map(|record| record.reservation_id)
                .unwrap_or(next_id),
            created_at_ms: previous
                .as_ref()
                .map(|record| record.created_at_ms)
                .unwrap_or_else(crate::unix_timestamp_ms),
            runtime_id,
            workflow_id: request.workflow_id,
            reservation_owner_id: request.reservation_owner_id,
            usage_profile: request.usage_profile,
            model_id: request.model_id,
            pin_runtime: request.pin_runtime,
            retention_hint: request.retention_hint,
            claim,
        },
        previous,
    ))
}

pub(crate) fn rollback_pending_reservation(
    state: &mut RuntimeRegistryState,
    reservation_id: u64,
) -> Option<String> {
    let pending = state.pending_reservations.remove(&reservation_id)?;
    let runtime_id = pending.committed.runtime_id;
    if let Some(previous) = pending.previous {
        state.reservations.insert(reservation_id, previous);
    } else {
        state.reservations.remove(&reservation_id);
        if let Some(runtime) = state.runtimes.get_mut(&runtime_id) {
            runtime.active_reservations.remove(&reservation_id);
        }
    }
    Some(runtime_id)
}
