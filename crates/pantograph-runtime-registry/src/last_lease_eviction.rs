//! Linear last-lease stop handoff. No Drop path reclaims an unacknowledged owner.
use crate::{
    remove_reservation_locked, retained_cleanup::validate_retained_owner_locked, unix_timestamp_ms,
    RuntimeRegistryError, RuntimeRegistryStatus, RuntimeReservationExecutionCustody,
    RuntimeRetainedCleanupError, RuntimeRetainedCleanupRefusal, RuntimeRetainedOwnerIdentity,
    RuntimeRetentionDisposition, RuntimeRetentionHint,
};

/// Captured actual-owner stop obligation. Not Clone; failed/lost acknowledgement
/// leaves the runtime-wide fence and all accounting intact. The trusted producer
/// must retain its backend writer until physical stop and acknowledgement finish.
#[must_use]
pub struct RuntimeLastLeaseEviction {
    custody: RuntimeReservationExecutionCustody,
    source_id: String,
    instance_id: String,
    model_target: String,
    observed_sequence: u64,
    resident: crate::RuntimeModelResourceResidency,
}
impl RuntimeReservationExecutionCustody {
    /// Caller is the trusted actual producer, with successful/drained execution
    /// proof and its writer held. This operation never proves physical release.
    pub fn begin_last_lease_eviction(
        self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeLastLeaseEviction, RuntimeRetainedCleanupError> {
        let mut state = self
            .registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let id = validate_retained_owner_locked(&state, &self.lease, &owner, Some(self.token))?;
        let record = state.runtimes.get(&id).expect("validated runtime");
        let refuse = || RuntimeRetainedCleanupError::Refused {
            runtime_id: id.clone(),
            reason: RuntimeRetainedCleanupRefusal::RetentionRequired,
        };
        // Exact singleton checks avoid a global successor/pinned-model scan.
        if !self.started
            || record.active_reservations.len() != 1
            || self.lease.retention_hint != RuntimeRetentionHint::Ephemeral
            || self.lease.pin_runtime
            || record.models.len() != 1
            || record
                .models
                .get(owner.model_target)
                .is_none_or(|model| model.pinned)
        {
            return Err(refuse());
        }
        let resident = record
            .model_resource_residency
            .clone()
            .expect("validated accounting");
        let sequence = state
            .producer_observations
            .get(&id)
            .expect("validated source")
            .1;
        state.evicting_runtimes.insert(id.clone(), self.token);
        let record = state.runtimes.get_mut(&id).expect("validated runtime");
        record.status = RuntimeRegistryStatus::Stopping;
        record.last_transition_at_ms = unix_timestamp_ms();
        drop(state);
        Ok(RuntimeLastLeaseEviction {
            custody: self,
            source_id: owner.source_id.into(),
            instance_id: owner.runtime_instance_id.into(),
            model_target: owner.model_target.into(),
            observed_sequence: sequence,
            resident,
        })
    }
}
impl RuntimeLastLeaseEviction {
    /// Consume actual acknowledged stop. Sequence MUST be allocated by the
    /// producer after physical stop, under the SAME backend writer; it must
    /// exceed every previously sampled frame, including undelivered frames.
    /// Labels or this registry ticket alone are not physical release evidence.
    pub fn acknowledge_stopped(
        self,
        sequence: u64,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRegistryError> {
        let id = &self.custody.lease.runtime_id;
        let mut state = self
            .custody
            .registry
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        let invalid = || RuntimeRegistryError::ModelResidencyObservationChanged(id.clone());
        if sequence <= self.observed_sequence
            || state.evicting_runtimes.get(id) != Some(&self.custody.token)
            || state
                .executing_reservations
                .get(&self.custody.lease.reservation_id)
                != Some(&self.custody.token)
            || state.producer_observations.get(id)
                != Some(&(self.source_id.clone(), self.observed_sequence))
            || state
                .reservations
                .get(&self.custody.lease.reservation_id)
                .map(|r| r.clone().into_lease())
                != Some(self.custody.lease.clone())
        {
            return Err(invalid());
        }
        let record = state.runtimes.get(id).ok_or_else(invalid)?;
        if record.model_resource_residency.as_ref() != Some(&self.resident)
            || record.active_reservations.len() != 1
            || !record
                .active_reservations
                .contains(&self.custody.lease.reservation_id)
            || record.runtime_instance_id.as_deref() != Some(&self.instance_id)
            || record.model_resource_residency.as_ref().is_none_or(|r| {
                r.model_id != self.model_target
                    || r.runtime_instance_id.as_deref() != Some(&self.instance_id)
            })
        {
            return Err(invalid());
        }
        // Only this exact obligation can clear the two charges and the fence.
        state
            .executing_reservations
            .remove(&self.custody.lease.reservation_id);
        remove_reservation_locked(&mut state, self.custody.lease.reservation_id)
            .expect("validated lease");
        let record = state.runtimes.get_mut(id).expect("validated runtime");
        record.status = RuntimeRegistryStatus::Stopped;
        record.runtime_instance_id = None;
        record.model_resource_residency = None;
        record.resident_resources_uncertain = false;
        record.models.clear();
        record.last_error = None;
        record.last_transition_at_ms = unix_timestamp_ms();
        state
            .producer_observations
            .insert(id.clone(), (self.source_id.clone(), sequence));
        state.evicting_runtimes.remove(id);
        Ok(RuntimeRetentionDisposition::evict(id.clone()))
    }
}
