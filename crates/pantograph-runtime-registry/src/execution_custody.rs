//! Exclusive custody of an already admitted lease during actual execution.
use crate::{
    RuntimeRegistry, RuntimeRegistryError, RuntimeReservationLease, RuntimeRetainedCleanupError,
    RuntimeRetainedOwnerIdentity, RuntimeRetentionDisposition,
};
use std::sync::{atomic::Ordering, Arc};

/// Non-Clone guard. Ordinary release, replacement and retention changes refuse
/// while it lives. Before execution, Drop ends exclusion without freeing a claim. Once actual
/// work starts, abandonment leaves the claim fenced until verified recovery.
#[must_use]
pub struct RuntimeReservationExecutionCustody {
    pub(crate) registry: Arc<RuntimeRegistry>,
    pub(crate) lease: RuntimeReservationLease,
    pub(crate) token: u64,
    started: bool,
}
impl RuntimeReservationExecutionCustody {
    /// Trusted held producer boundary, immediately before starting physical work.
    pub fn begin_execution(&mut self) {
        self.started = true;
    }
    pub fn matches(
        &self,
        registry: &Arc<RuntimeRegistry>,
        lease: &RuntimeReservationLease,
    ) -> bool {
        Arc::ptr_eq(&self.registry, registry) && self.lease == *lease
    }
    pub fn validate_retained_owner(
        &self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<(), RuntimeRetainedCleanupError> {
        self.registry
            .check_retained_reservation_for_owner(&self.lease, owner, false, Some(self.token))
            .map(|_| ())
    }
    pub fn release_retained(
        self,
        owner: RuntimeRetainedOwnerIdentity<'_>,
    ) -> Result<RuntimeRetentionDisposition, RuntimeRetainedCleanupError> {
        self.registry.check_retained_reservation_for_owner(
            &self.lease,
            owner,
            true,
            Some(self.token),
        )
    }
}
impl Drop for RuntimeReservationExecutionCustody {
    fn drop(&mut self) {
        if self.started {
            return;
        }
        let mut state = self
            .registry
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.executing_reservations.get(&self.lease.reservation_id) == Some(&self.token) {
            state
                .executing_reservations
                .remove(&self.lease.reservation_id);
        }
    }
}
impl RuntimeRegistry {
    /// Pin the exact selected live lease. Pending admission cannot be captured.
    /// The actual native supervisor must hold this guard through physical drain,
    /// then transfer it into its linear cleanup receipt.
    pub fn acquire_execution_custody(
        self: &Arc<Self>,
        expected: &RuntimeReservationLease,
    ) -> Result<RuntimeReservationExecutionCustody, RuntimeRegistryError> {
        let mut state = self
            .state
            .lock()
            .expect("runtime registry state lock poisoned");
        if state
            .pending_reservations
            .contains_key(&expected.reservation_id)
            || state
                .executing_reservations
                .contains_key(&expected.reservation_id)
        {
            return Err(RuntimeRegistryError::ReservationCustodyPending(
                expected.reservation_id,
            ));
        }
        let actual = state.reservations.get(&expected.reservation_id).ok_or(
            RuntimeRegistryError::ReservationNotFound(expected.reservation_id),
        )?;
        if actual.clone().into_lease() != *expected {
            return Err(RuntimeRegistryError::ReservationObservationChanged(
                expected.runtime_id.clone(),
            ));
        }
        let token = self.reservation_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        state
            .executing_reservations
            .insert(expected.reservation_id, token);
        Ok(RuntimeReservationExecutionCustody {
            registry: self.clone(),
            lease: expected.clone(),
            token,
            started: false,
        })
    }
}
