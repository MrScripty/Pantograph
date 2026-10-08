//! Synchronous retained-owner release. This never stops or reclaims a producer.
#[cfg(test)]
#[path = "retained_cleanup_tests.rs"]
mod tests;
use crate::{
    canonical_runtime_id, remove_reservation_locked, reservation_claim_from_requirements,
    RuntimeRegistry, RuntimeRegistryError, RuntimeRegistryStatus, RuntimeReservationLease,
    RuntimeRetentionDisposition, RuntimeRetentionHint, RuntimeRetentionReason,
};

/// Actual facts sampled under the producer's held exclusive backend guard.
/// These labels alone are not an execution or cleanup capability.
pub struct RuntimeRetainedOwnerIdentity<'a> {
    pub runtime_id: &'a str,
    pub source_id: &'a str,
    pub runtime_instance_id: &'a str,
    pub model_target: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRetainedCleanupRefusal {
    PendingCustody,
    ChangedLease,
    OwnerMismatch,
    UnknownResidentAccounting,
    LastLease,
    RetentionRequired,
}

/// Additive opt-in API error; existing registry error contracts are unchanged.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeRetainedCleanupError {
    #[error("retained owner cleanup refused for runtime '{runtime_id}': {reason:?}")]
    Refused {
        runtime_id: String,
        reason: RuntimeRetainedCleanupRefusal,
    },
    #[error(transparent)]
    Registry(#[from] RuntimeRegistryError),
}

impl RuntimeRegistry {
    /// Caller must hold the actual exclusive producer guard through this call.
    /// Every check precedes mutation under one admission lock. A different live
    /// lease (including provisional custody) must keep the producer retained.
    /// Last-lease eviction is unsupported and leaves the full task claim intact.
    /// Work does not scan other leases: map/set operations are logarithmic in
    /// registry size, and identity comparisons are bounded by their byte lengths.
    pub fn release_retained_reservation_for_owner(
        &self,
        expected: &RuntimeReservationLease,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRetainedCleanupError> {
        self.check_retained_reservation_for_owner(expected, owner, true, None)
    }

    /// Read-only admission preflight for an actually held native attempt owner.
    /// Cleanup must repeat this check; this result does not reserve exclusion.
    pub fn validate_retained_reservation_for_owner(
        &self,
        expected: &RuntimeReservationLease,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<(), RuntimeRetainedCleanupError> {
        self.check_retained_reservation_for_owner(expected, owner, false, None)
            .map(|_| ())
    }

    pub(crate) fn check_retained_reservation_for_owner(
        &self,
        expected: &RuntimeReservationLease,
        owner: RuntimeRetainedOwnerIdentity<'_>,
        release: bool,
        execution_token: Option<u64>,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRetainedCleanupError> {
        let runtime_id = canonical_runtime_id(owner.runtime_id);
        let refuse = |reason| RuntimeRetainedCleanupError::Refused {
            runtime_id: runtime_id.clone(),
            reason,
        };
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        validate_retained_owner_locked(&state, expected, &owner, execution_token)?;
        let record = state.runtimes.get(&runtime_id).expect("validated runtime");
        // At most two IDs are inspected: only one can be the released lease.
        let successor = record
            .active_reservations
            .iter()
            .find(|id| **id != expected.reservation_id);
        let Some(successor) = successor
            .and_then(|id| state.reservations.get(id))
            .filter(|lease| lease.runtime_id == runtime_id)
        else {
            return Err(refuse(RuntimeRetainedCleanupRefusal::LastLease));
        };
        let reason = if successor.retention_hint == RuntimeRetentionHint::KeepAlive {
            RuntimeRetentionReason::KeepAliveReservation
        } else {
            RuntimeRetentionReason::ActiveReservations
        };
        // The successor remains charged; resident accounting was established
        // before releasing this task's peak claim. No stop or callback follows.
        if release {
            state
                .executing_reservations
                .remove(&expected.reservation_id);
            remove_reservation_locked(&mut state, expected.reservation_id)
                .expect("validated live reservation under the same lock");
        }
        Ok(RuntimeRetentionDisposition::retain(runtime_id, reason))
    }
}

/// Shared exact owner/accounting checks; caller holds the one admission lock.
pub(crate) fn validate_retained_owner_locked(
    state: &crate::RuntimeRegistryState,
    expected: &RuntimeReservationLease,
    owner: &RuntimeRetainedOwnerIdentity<'_>,
    execution_token: Option<u64>,
) -> Result<String, RuntimeRetainedCleanupError> {
    let runtime_id = canonical_runtime_id(owner.runtime_id);
    let refuse = |reason| RuntimeRetainedCleanupError::Refused {
        runtime_id: runtime_id.clone(),
        reason,
    };
    if state.evicting_runtimes.contains_key(&runtime_id) {
        return Err(refuse(RuntimeRetainedCleanupRefusal::PendingCustody));
    }
    if state
        .executing_reservations
        .get(&expected.reservation_id)
        .copied()
        != execution_token
    {
        return Err(refuse(RuntimeRetainedCleanupRefusal::PendingCustody));
    }
    if state
        .pending_reservations
        .contains_key(&expected.reservation_id)
    {
        return Err(refuse(RuntimeRetainedCleanupRefusal::PendingCustody));
    }
    let lease = state
        .reservations
        .get(&expected.reservation_id)
        .ok_or_else(|| refuse(RuntimeRetainedCleanupRefusal::ChangedLease))?;
    if lease.reservation_id != expected.reservation_id
        || lease.runtime_id != expected.runtime_id
        || lease.workflow_id != expected.workflow_id
        || lease.reservation_owner_id != expected.reservation_owner_id
        || lease.usage_profile != expected.usage_profile
        || lease.model_id != expected.model_id
        || lease.pin_runtime != expected.pin_runtime
        || lease.retention_hint != expected.retention_hint
        || lease.created_at_ms != expected.created_at_ms
        || expected.runtime_id != runtime_id
    {
        return Err(refuse(RuntimeRetainedCleanupRefusal::ChangedLease));
    }
    let record = state
        .runtimes
        .get(&runtime_id)
        .ok_or_else(|| refuse(RuntimeRetainedCleanupRefusal::OwnerMismatch))?;
    if state
        .producer_observations
        .get(&runtime_id)
        .map(|(source, _)| source.as_str())
        != Some(owner.source_id)
        || record.runtime_instance_id.as_deref() != Some(owner.runtime_instance_id)
        || !matches!(
            record.status,
            RuntimeRegistryStatus::Ready | RuntimeRegistryStatus::Busy
        )
        || !record
            .active_reservations
            .contains(&expected.reservation_id)
    {
        return Err(refuse(RuntimeRetainedCleanupRefusal::OwnerMismatch));
    }
    let accounted = record
        .model_resource_residency
        .as_ref()
        .is_some_and(|resident| {
            resident.model_id == owner.model_target
                && resident.runtime_instance_id.as_deref() == Some(owner.runtime_instance_id)
                && resident
                    .requirements
                    .as_ref()
                    .is_some_and(|r| !r.claims.is_empty())
        });
    if record.resident_resources_uncertain || !accounted {
        return Err(refuse(
            RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
        ));
    }
    let resident_claim = reservation_claim_from_requirements(
        &runtime_id,
        record
            .model_resource_residency
            .as_ref()
            .and_then(|resident| resident.requirements.as_ref()),
    )?;
    // A retained native CPU allocation requires explicit RAM accounting.
    // Every kind charged by the completed task must remain explicitly known.
    if resident_claim.ram_bytes.is_none()
        || (lease.claim.vram_bytes.is_some() && resident_claim.vram_bytes.is_none())
    {
        return Err(refuse(
            RuntimeRetainedCleanupRefusal::UnknownResidentAccounting,
        ));
    }
    Ok(runtime_id)
}
