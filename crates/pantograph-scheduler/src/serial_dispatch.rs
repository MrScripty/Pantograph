//! Attempt-bound serial execution and cleanup. No runtime/port ownership is
//! invented here: the adapter supplies its actual exclusive owner lease and
//! observations from the existing drained host and reservation lifecycle paths.
use crate::SchedulerSerialDispatch;

/// Borrowed identities from an owned attempt and selected runtime request.
/// All eight fields are bounded before any cloning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerSerialAttemptIdentity<'a> {
    pub workflow_id: &'a str,
    pub workflow_run_id: &'a str,
    pub node_id: &'a str,
    pub task_id: &'a str,
    pub attempt_id: &'a str,
    pub execution_request_id: &'a str,
    pub candidate_id: &'a str,
    pub reservation_lease_id: &'a str,
}

impl<'a> SchedulerSerialAttemptIdentity<'a> {
    fn fields(self) -> [&'a str; 8] {
        [
            self.workflow_id,
            self.workflow_run_id,
            self.node_id,
            self.task_id,
            self.attempt_id,
            self.execution_request_id,
            self.candidate_id,
            self.reservation_lease_id,
        ]
    }
}

/// Exact local CPU execution domain, not an aggregate resource-capacity claim.
/// Profile/settings digests must come from the actual owner, not request labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerSerialOwnerSnapshot {
    pub loaded_instance: [u8; 16],
    pub loaded_profile: [u8; 32],
    pub effective_settings: [u8; 32],
    pub generation: u64,
    pub cpu_threads: u8,
}

impl SchedulerSerialOwnerSnapshot {
    fn valid(self) -> bool {
        self.loaded_instance != [0; 16]
            && self.generation > 0
            && self.generation.is_multiple_of(2)
            && (1..=4).contains(&self.cpu_threads)
    }
}

/// Implement only on an ACTUALLY HELD exclusive loaded-runtime owner guard.
/// The guard prevents stop/reload/replacement until the execution future drains;
/// snapshot probes effective settings at acquisition and after actual drain.
/// A borrowed gateway or a generation-only advisory comparison is not a lease.
pub trait SchedulerSerialRuntimeOwnerLease: Sync {
    fn snapshot(&self) -> SchedulerSerialOwnerSnapshot;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerSerialDispatchRefusal {
    InvalidIdentity,
    InvalidOwner,
    OwnerChanged,
    ForeignResponse,
    NotDrained,
    ForeignCleanup,
    CleanupFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerSerialDrainState {
    /// Terminal host response after actual worker drain; not mere acceptance.
    Completed,
    Failed,
    Rejected,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerSerialCleanupState {
    Applied,
    AlreadyApplied,
    Failed,
}

/// Expected owned lifecycle release event, captured BEFORE calling the port.
/// The adapter binds attempt/request from its custody, since the existing event
/// DTO carries workflow/task/candidate/lease but has no attempt/request fields.
#[derive(Debug, Clone, Copy)]
pub struct SchedulerSerialCleanupEvent<'a> {
    pub identity: SchedulerSerialAttemptIdentity<'a>,
    pub lifecycle_event_id: &'a str,
    pub releases_reservation: bool,
}

#[derive(Debug)]
#[must_use]
pub struct SchedulerSerialBoundDispatch {
    dispatch: SchedulerSerialDispatch,
    identity: [String; 8],
    expected_owner: SchedulerSerialOwnerSnapshot,
}

#[must_use]
pub struct SchedulerSerialExecutingDispatch<'a> {
    bound: SchedulerSerialBoundDispatch,
    owner_lease: &'a dyn SchedulerSerialRuntimeOwnerLease,
}

#[derive(Debug)]
#[must_use]
pub struct SchedulerSerialDrainedDispatch {
    bound: SchedulerSerialBoundDispatch,
}

#[derive(Debug)]
#[must_use]
pub struct SchedulerSerialCleanupPending {
    drained: SchedulerSerialDrainedDispatch,
    lifecycle_event_id: String,
}

impl SchedulerSerialDispatch {
    /// Bind after the actual attempt and selected lease/request exist. Promotion
    /// still happens before the first claim. Binding failure poisons exclusion.
    pub fn bind_attempt(
        self,
        identity: SchedulerSerialAttemptIdentity<'_>,
        expected_owner: SchedulerSerialOwnerSnapshot,
    ) -> Result<SchedulerSerialBoundDispatch, SchedulerSerialDispatchRefusal> {
        if !identity.fields().into_iter().all(bounded_identifier) {
            return Err(SchedulerSerialDispatchRefusal::InvalidIdentity);
        }
        if !expected_owner.valid() {
            return Err(SchedulerSerialDispatchRefusal::InvalidOwner);
        }
        Ok(SchedulerSerialBoundDispatch {
            dispatch: self,
            identity: identity.fields().map(str::to_owned),
            expected_owner,
        })
    }
}

impl SchedulerSerialBoundDispatch {
    fn matches(&self, identity: SchedulerSerialAttemptIdentity<'_>) -> bool {
        self.identity
            .iter()
            .map(String::as_str)
            .eq(identity.fields())
    }

    /// Validate when execution acquires the actual owner, and retain that lease
    /// borrow through drain. Mismatch refuses before any backend forward call.
    pub fn acquire_owner(
        self,
        owner_lease: &dyn SchedulerSerialRuntimeOwnerLease,
    ) -> Result<SchedulerSerialExecutingDispatch<'_>, SchedulerSerialDispatchRefusal> {
        if owner_lease.snapshot() != self.expected_owner {
            return Err(SchedulerSerialDispatchRefusal::OwnerChanged);
        }
        Ok(SchedulerSerialExecutingDispatch {
            bound: self,
            owner_lease,
        })
    }
}

impl SchedulerSerialExecutingDispatch<'_> {
    /// Trusted host boundary: call only after the execution future AND actual
    /// worker drain finish. `Accepted` never mints a drain proof. Terminal DTOs
    /// alone cannot establish drain; the adapter owns that real-world obligation.
    pub fn record_drained_response(
        self,
        identity: SchedulerSerialAttemptIdentity<'_>,
        state: SchedulerSerialDrainState,
    ) -> Result<SchedulerSerialDrainedDispatch, SchedulerSerialDispatchRefusal> {
        if !self.bound.matches(identity) {
            return Err(SchedulerSerialDispatchRefusal::ForeignResponse);
        }
        if state == SchedulerSerialDrainState::Accepted {
            return Err(SchedulerSerialDispatchRefusal::NotDrained);
        }
        if self.owner_lease.snapshot() != self.bound.expected_owner {
            return Err(SchedulerSerialDispatchRefusal::OwnerChanged);
        }
        Ok(SchedulerSerialDrainedDispatch { bound: self.bound })
    }
}

impl SchedulerSerialDrainedDispatch {
    pub fn expect_cleanup(
        self,
        event: SchedulerSerialCleanupEvent<'_>,
    ) -> Result<SchedulerSerialCleanupPending, SchedulerSerialDispatchRefusal> {
        if !self.bound.matches(event.identity)
            || !bounded_identifier(event.lifecycle_event_id)
            || !event.releases_reservation
        {
            return Err(SchedulerSerialDispatchRefusal::ForeignCleanup);
        }
        Ok(SchedulerSerialCleanupPending {
            drained: self,
            lifecycle_event_id: event.lifecycle_event_id.to_owned(),
        })
    }
}

impl SchedulerSerialCleanupPending {
    /// Match the EXACT event and selected lease echoed by the lifecycle port.
    /// The non-Clone pending ticket already owns the validated attempt/request,
    /// terminal drain proof, and expected release event. Wrong/failed responses
    /// consume the ticket and poison admission rather than reopening execution.
    pub fn acknowledge_cleanup(
        self,
        lifecycle_event_id: &str,
        reservation_lease_id: &str,
        state: SchedulerSerialCleanupState,
    ) -> Result<(), SchedulerSerialDispatchRefusal> {
        if lifecycle_event_id != self.lifecycle_event_id
            || reservation_lease_id != self.drained.bound.identity[7]
        {
            return Err(SchedulerSerialDispatchRefusal::ForeignCleanup);
        }
        if state == SchedulerSerialCleanupState::Failed {
            return Err(SchedulerSerialDispatchRefusal::CleanupFailed);
        }
        self.drained.bound.dispatch.release_after_bound_cleanup();
        Ok(())
    }
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b':'))
}

#[cfg(test)]
#[path = "serial_dispatch_tests.rs"]
mod tests;
