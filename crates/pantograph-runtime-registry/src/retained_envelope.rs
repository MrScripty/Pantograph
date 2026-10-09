//! Explicit declared retained envelope. Not measured peak/growth or global admission.
use crate::{
    admission::RuntimeReservationClaim, retained_cleanup::validate_retained_owner_locked,
    RuntimeAdmissionResourceKind, RuntimeModelResourceResidency,
    RuntimeReservationExecutionCustody, RuntimeReservationLease, RuntimeRetainedCleanupError,
    RuntimeRetainedCleanupRefusal, RuntimeRetainedOwnerIdentity, RuntimeRetentionDisposition,
    RuntimeRetentionHint, RuntimeRetentionReason,
};

pub const RETAINED_ENVELOPE_MAX_RESERVATIONS: usize = 64;
pub const RETAINED_ENVELOPE_MAX_RUNTIMES: usize = 16;

/// SAME task custody plus exact named KeepAlive successor and resident charge.
/// Non-Clone. Started abandonment fences both claims and resident accounting.
#[must_use]
pub struct RuntimeRetainedExecutionEnvelope {
    pub(crate) task: RuntimeReservationExecutionCustody,
    successor: RuntimeReservationExecutionCustody,
    resident: RuntimeModelResourceResidency,
}
/// Sealed registry fact only: the exact unstarted task claim was removed.
/// This alone is not a native no-effect, result or full-resource release proof.
#[must_use]
pub struct RuntimeUnstartedEnvelopeSettlement {
    task: RuntimeReservationLease,
    successor: RuntimeReservationLease,
}
impl RuntimeUnstartedEnvelopeSettlement {
    pub fn task(&self) -> &RuntimeReservationLease {
        &self.task
    }
    pub fn successor(&self) -> &RuntimeReservationLease {
        &self.successor
    }
}
fn refused(runtime: &str, reason: RuntimeRetainedCleanupRefusal) -> RuntimeRetainedCleanupError {
    RuntimeRetainedCleanupError::Refused {
        runtime_id: if runtime.len() <= 128 {
            runtime.into()
        } else {
            "retained-envelope".into()
        },
        reason,
    }
}
fn lease_is_bounded(lease: &RuntimeReservationLease) -> bool {
    lease.runtime_id.len() <= 128
        && lease.workflow_id.len() <= 128
        && lease
            .reservation_owner_id
            .as_ref()
            .is_none_or(|s| s.len() <= 128)
        && lease.usage_profile.as_ref().is_none_or(|s| s.len() <= 128)
        && lease.model_id.as_ref().is_none_or(|s| s.len() <= 4096)
}
fn owner_is_bounded(owner: &RuntimeRetainedOwnerIdentity<'_>) -> bool {
    owner.runtime_id.len() <= 128
        && owner.source_id.len() <= 128
        && owner.runtime_instance_id.len() <= 128
        && owner.model_target.len() <= 4096
}
fn bounded(state: &crate::RuntimeRegistryState) -> bool {
    if state.reservations.len() > RETAINED_ENVELOPE_MAX_RESERVATIONS
        || state.pending_reservations.len() > RETAINED_ENVELOPE_MAX_RESERVATIONS
        || state.runtimes.len() > RETAINED_ENVELOPE_MAX_RUNTIMES
        || state.resource_domains.len() > RETAINED_ENVELOPE_MAX_RUNTIMES
    {
        return false;
    }
    state.reservations.values().all(|r| {
        r.runtime_id.len() <= 128
            && r.workflow_id.len() <= 128
            && r.reservation_owner_id
                .as_ref()
                .is_none_or(|s| s.len() <= 128)
            && r.usage_profile.as_ref().is_none_or(|s| s.len() <= 128)
            && r.model_id.as_ref().is_none_or(|s| s.len() <= 4096)
    }) && state.runtimes.iter().all(|(id, r)| {
        id.len() <= 128
            && r.admission_budget
                .as_ref()
                .is_none_or(|b| b.resources.len() <= 2)
            && r.model_resource_residency.as_ref().is_none_or(|resident| {
                resident.model_id.len() <= 4096
                    && resident
                        .runtime_instance_id
                        .as_ref()
                        .is_none_or(|id| id.len() <= 128)
                    && resident
                        .requirements
                        .as_ref()
                        .is_none_or(|r| r.claims.len() <= 2)
            })
    }) && state.reservations.len() <= RETAINED_ENVELOPE_MAX_RESERVATIONS
        && state.pending_reservations.len() <= RETAINED_ENVELOPE_MAX_RESERVATIONS
        && state.runtimes.len() <= RETAINED_ENVELOPE_MAX_RUNTIMES
        && state.resource_domains.len() <= RETAINED_ENVELOPE_MAX_RUNTIMES
        && state.resource_domains.iter().all(|(key, d)| {
            key.len() <= 128
                && d.domain_id.len() <= 128
                && d.bindings.len() <= RETAINED_ENVELOPE_MAX_RUNTIMES * 2
                && d.bindings.iter().all(|b| b.runtime_id.len() <= 128)
        })
}
fn capacity(
    state: &crate::RuntimeRegistryState,
    runtime: &str,
) -> Result<(), RuntimeRetainedCleanupError> {
    if !bounded(state) {
        return Err(refused(
            runtime,
            RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
        ));
    }
    let record = state.runtimes.get(runtime).expect("validated runtime");
    let mut live = RuntimeReservationClaim {
        ram_bytes: None,
        vram_bytes: None,
    };
    for kind in [
        RuntimeAdmissionResourceKind::RamBytes,
        RuntimeAdmissionResourceKind::VramBytes,
    ] {
        let used = crate::reservation_claim_from_requirements(
            runtime,
            record
                .model_resource_residency
                .as_ref()
                .and_then(|r| r.requirements.as_ref()),
        )?;
        let charged = match kind {
            RuntimeAdmissionResourceKind::RamBytes => used.ram_bytes,
            RuntimeAdmissionResourceKind::VramBytes => used.vram_bytes,
        };
        let task_charged = state
            .reservations
            .values()
            .filter(|r| r.runtime_id == runtime)
            .any(|r| match kind {
                RuntimeAdmissionResourceKind::RamBytes => r.claim.ram_bytes.is_some(),
                RuntimeAdmissionResourceKind::VramBytes => r.claim.vram_bytes.is_some(),
            });
        if charged.is_none() && !task_charged {
            continue;
        }
        if task_charged && charged.is_none() {
            return Err(refused(
                runtime,
                RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
            ));
        }
        match kind {
            RuntimeAdmissionResourceKind::RamBytes => live.ram_bytes = Some(0),
            RuntimeAdmissionResourceKind::VramBytes => live.vram_bytes = Some(0),
        };
        let local_known = record
            .admission_budget
            .as_ref()
            .and_then(|b| b.resource_budget(kind))
            .and_then(|b| b.total_bytes)
            .is_some();
        let domain_known = state.resource_domains.values().any(|d| {
            d.bindings
                .iter()
                .any(|b| b.runtime_id == runtime && b.resource_kind == kind)
        });
        if !local_known && !domain_known {
            return Err(refused(
                runtime,
                RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
            ));
        }
    }
    if crate::admission_failure(record, live, &state.reservations, None)?.is_some() {
        return Err(refused(
            runtime,
            RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
        ));
    }
    crate::resource_domain::validate_retained_domain_ledger(state, runtime)?;
    Ok(())
}
impl RuntimeReservationExecutionCustody {
    /// Caller holds the actual backend writer. No callback, await or physical work.
    /// Explicit successor identity is never inferred from any unrelated task.
    pub fn protect_retained_envelope(
        self,
        successor: &RuntimeReservationLease,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeRetainedExecutionEnvelope, RuntimeRetainedCleanupError> {
        let mut state = self.registry.state.lock().expect("registry lock poisoned");
        if !bounded(&state)
            || !lease_is_bounded(&self.lease)
            || !lease_is_bounded(successor)
            || !owner_is_bounded(&owner)
        {
            return Err(refused(
                &self.lease.runtime_id,
                RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
            ));
        }
        let runtime =
            validate_retained_owner_locked(&state, &self.lease, &owner, Some(self.token))?;
        if self.started
            || !bounded(&state)
            || successor.runtime_id != runtime
            || successor.reservation_id == self.lease.reservation_id
            || successor.retention_hint != RuntimeRetentionHint::KeepAlive
            || successor.model_id != self.lease.model_id
            || state.retained_envelope_fences.contains_key(&runtime)
            || state
                .executing_reservations
                .contains_key(&successor.reservation_id)
            || state
                .pending_reservations
                .contains_key(&successor.reservation_id)
            || state
                .pending_predecessors
                .contains_key(&successor.reservation_id)
            || state
                .reservations
                .get(&successor.reservation_id)
                .is_none_or(|r| r.clone().into_lease() != *successor)
            || state
                .reservations
                .get(&self.lease.reservation_id)
                .is_none_or(|r| r.claim.ram_bytes.is_none())
            || state
                .reservations
                .get(&successor.reservation_id)
                .is_none_or(|r| r.claim.ram_bytes.is_none())
        {
            return Err(refused(
                &runtime,
                RuntimeRetainedCleanupRefusal::PendingCustody,
            ));
        }
        capacity(&state, &runtime)?;
        let resident = state.runtimes[&runtime]
            .model_resource_residency
            .clone()
            .expect("validated resident");
        let token = self.registry.next_reservation_token()?;
        state
            .executing_reservations
            .insert(successor.reservation_id, token);
        state.retained_envelope_fences.insert(runtime, self.token);
        let retaining = RuntimeReservationExecutionCustody {
            registry: self.registry.clone(),
            lease: successor.clone(),
            token,
            started: false,
        };
        drop(state);
        Ok(RuntimeRetainedExecutionEnvelope {
            task: self,
            successor: retaining,
            resident,
        })
    }
}
impl RuntimeRetainedExecutionEnvelope {
    pub fn retained_instance_id(&self) -> Option<&str> {
        self.resident.runtime_instance_id.as_deref()
    }
    pub fn started(&self) -> bool {
        self.task.started
    }
    pub fn validate_retained_owner(
        &self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<(), RuntimeRetainedCleanupError> {
        let state = self
            .task
            .registry
            .state
            .lock()
            .expect("registry lock poisoned");
        let runtime = self.validate(&state, &owner)?;
        if self.started() {
            capacity(&state, &runtime)?;
        }
        Ok(())
    }

    fn validate(
        &self,
        state: &crate::RuntimeRegistryState,
        owner: &RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<String, RuntimeRetainedCleanupError> {
        if !bounded(state) || !owner_is_bounded(owner) {
            return Err(refused(
                &self.task.lease.runtime_id,
                RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
            ));
        }
        let runtime =
            validate_retained_owner_locked(state, &self.task.lease, owner, Some(self.task.token))?;
        if state.retained_envelope_fences.get(&runtime) != Some(&self.task.token)
            || state
                .executing_reservations
                .get(&self.successor.lease.reservation_id)
                != Some(&self.successor.token)
            || state
                .reservations
                .get(&self.successor.lease.reservation_id)
                .is_none_or(|r| r.clone().into_lease() != self.successor.lease)
            || state.runtimes[&runtime].model_resource_residency.as_ref() != Some(&self.resident)
        {
            return Err(refused(
                &runtime,
                RuntimeRetainedCleanupRefusal::ChangedLease,
            ));
        }
        Ok(runtime)
    }
    /// Final bounded producer-only callback must be a fixed-work atomic start
    /// operation: no registry/store reentry, allocation, native call or await.
    /// Fresh complete declared ledger is checked under SAME admission lock.
    pub fn authorize_start(
        &mut self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
        authorize: impl FnOnce() -> bool,
    ) -> Result<bool, RuntimeRetainedCleanupError> {
        let state = self
            .task
            .registry
            .state
            .lock()
            .expect("registry lock poisoned");
        let runtime = self.validate(&state, &owner)?;
        capacity(&state, &runtime)?;
        if self.task.started || !authorize() {
            return Ok(false);
        }
        self.task.started = true;
        self.successor.started = true;
        Ok(true)
    }
    fn release(
        &mut self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRetainedCleanupError> {
        let mut state = self
            .task
            .registry
            .state
            .lock()
            .expect("registry lock poisoned");
        let runtime = self.validate(&state, &owner)?;
        if self.started() {
            capacity(&state, &runtime)?;
        }
        state
            .executing_reservations
            .remove(&self.task.lease.reservation_id);
        state
            .executing_reservations
            .remove(&self.successor.lease.reservation_id);
        state.retained_envelope_fences.remove(&runtime);
        crate::remove_reservation_locked(&mut state, self.task.lease.reservation_id)
            .expect("same exact lock");
        Ok(RuntimeRetentionDisposition::retain(
            runtime,
            RuntimeRetentionReason::KeepAliveReservation,
        ))
    }
    /// Trusted producer boundary only after actual drain. No default retry.
    pub fn release_drained(
        mut self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRetainedCleanupError> {
        if !self.started() {
            return Err(refused(
                &self.task.lease.runtime_id,
                RuntimeRetainedCleanupRefusal::PendingCustody,
            ));
        }
        self.release(owner)
    }
    /// Trusted supervisor supplies native no-start separately. Requires no
    /// successful envelope start; performs exact release under SAME lock.
    pub fn settle_unstarted(
        mut self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeUnstartedEnvelopeSettlement, RuntimeRetainedCleanupError> {
        if self.started() {
            return Err(refused(
                &self.task.lease.runtime_id,
                RuntimeRetainedCleanupRefusal::PendingCustody,
            ));
        }
        self.release(owner)?;
        Ok(RuntimeUnstartedEnvelopeSettlement {
            task: self.task.lease.clone(),
            successor: self.successor.lease.clone(),
        })
    }
}
impl Drop for RuntimeRetainedExecutionEnvelope {
    fn drop(&mut self) {
        if !self.started() {
            let mut state = self
                .task
                .registry
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state
                .retained_envelope_fences
                .get(&self.task.lease.runtime_id)
                == Some(&self.task.token)
            {
                state
                    .retained_envelope_fences
                    .remove(&self.task.lease.runtime_id);
            }
        }
        // Existing component Drop rules remove unstarted custody only; after
        // start both original claims stay fenced until verified recovery.
    }
}

#[cfg(test)]
#[path = "retained_envelope_tests.rs"]
mod tests;
