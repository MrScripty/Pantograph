use std::collections::BTreeMap;
use std::sync::{Arc, Weak};

use uuid::Uuid;

use super::runtime_dispatch_assignment::WorkflowRuntimeDispatchAssignmentId;
use super::runtime_dispatch_selection::{
    ValidatedWorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateFact,
    WorkflowRuntimeDispatchCandidateFactBundle,
    WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
};
use super::runtime_task_attempt_fact::{
    WorkflowRuntimeTaskAttemptFactRecord, WorkflowRuntimeTaskAttemptResourceKind,
};
use super::WorkflowOutputTarget;
use crate::graph::WorkflowRuntimeSourceContext;

pub(super) const WORKFLOW_RUNTIME_BRANCH_TASK_EVENT_SCHEMA_VERSION: u16 = 3;

#[derive(Debug)]
pub(super) struct WorkflowRuntimeBranchOwnedEventClaim {
    pub(super) event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) claim: WorkflowRuntimeBranchTaskEventClaim,
    pub(super) proof: WorkflowRuntimeClaimOwnership,
}

/// The sole strong handle belongs to execution, never to a record snapshot.
#[derive(Debug)]
pub(super) struct WorkflowRuntimeClaimOwnership(Arc<()>);

#[derive(Debug)]
pub(super) struct WorkflowRuntimeClaimLiveness {
    owner: Weak<()>,
    dispatched: bool,
}

impl WorkflowRuntimeClaimLiveness {
    pub(super) fn new() -> (Self, WorkflowRuntimeClaimOwnership) {
        let owner = Arc::new(());
        (
            Self {
                owner: Arc::downgrade(&owner),
                dispatched: false,
            },
            WorkflowRuntimeClaimOwnership(owner),
        )
    }

    pub(super) fn is_live(&self) -> bool {
        self.owner.strong_count() != 0
    }

    pub(super) fn is_abandoned(&self) -> bool {
        self.dispatched && !self.is_live()
    }

    pub(super) fn owns(&self, proof: &WorkflowRuntimeClaimOwnership) -> bool {
        self.owner.ptr_eq(&Arc::downgrade(&proof.0))
    }

    pub(super) fn has_dispatched(&self) -> bool {
        self.dispatched
    }

    pub(super) fn mark_dispatched(&mut self, proof: &WorkflowRuntimeClaimOwnership) -> bool {
        if !self.owns(proof) || self.dispatched {
            return false;
        }
        self.dispatched = true;
        true
    }
}

#[derive(Debug)]
struct WorkflowRuntimeBranchEventOwnership {
    claim: WorkflowRuntimeBranchTaskEventClaim,
    liveness: WorkflowRuntimeClaimLiveness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaimOwnerId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaimLeaseId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventRequest {
    pub(super) event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: Option<String>,
    pub(super) attempt_generation: u64,
    pub(super) queued_input_keys: Vec<String>,
    pub(super) output_targets: Option<Vec<WorkflowOutputTarget>>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) batching_key: Option<String>,
    pub(super) runtime_source_context: WorkflowRuntimeSourceContext,
    pub(super) batch_eligibility: Option<WorkflowRuntimeBranchBatchEligibilityProfile>,
    pub(super) ready_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventRecord {
    pub(super) schema_version: u16,
    pub(super) event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: Option<String>,
    pub(super) attempt_generation: u64,
    pub(super) queued_input_keys: Vec<String>,
    pub(super) output_targets: Option<Vec<WorkflowOutputTarget>>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) batching_key: Option<String>,
    pub(super) runtime_source_context: WorkflowRuntimeSourceContext,
    pub(super) batch_eligibility: Option<WorkflowRuntimeBranchBatchEligibilityProfile>,
    pub(super) selected_candidate_fact: Option<WorkflowRuntimeDispatchCandidateFact>,
    pub(super) dispatch_assignment_link:
        Option<WorkflowRuntimeBranchTaskEventDispatchAssignmentLink>,
    pub(super) state: WorkflowRuntimeBranchTaskEventState,
    pub(super) claim: Option<WorkflowRuntimeBranchTaskEventClaim>,
    pub(super) ready_at_ms: u64,
    pub(super) dispatching_at_ms: Option<u64>,
    pub(super) running_at_ms: Option<u64>,
    pub(super) completed_at_ms: Option<u64>,
    pub(super) deferred_at_ms: Option<u64>,
    pub(super) failed_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaim {
    pub(super) owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
    pub(super) lease_id: WorkflowRuntimeBranchTaskEventClaimLeaseId,
    pub(super) attempt_generation: u64,
    pub(super) claimed_at_ms: u64,
    pub(super) lease_expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventDispatchAssignmentLink {
    pub(super) assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) claim_attempt_generation: u64,
    pub(super) linked_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchEligibilityProfile {
    pub(super) model_artifact_id: String,
    pub(super) runtime_family: String,
    pub(super) backend_id: String,
    pub(super) device_load_target: String,
    pub(super) runtime_residency_key: String,
    pub(super) estimated_loaded_runtime_bytes: u64,
    pub(super) context_shape_key: String,
    pub(super) operation_type: String,
    pub(super) cancellation_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile {
    pub(super) model_artifact_id: String,
    pub(super) runtime_family: String,
    pub(super) backend_id: String,
    pub(super) runtime_residency_key: String,
    pub(super) loaded_runtime_memory_estimate_bytes: u64,
    pub(super) operation_type: String,
    pub(super) context_shape_key: String,
    pub(super) cancellation_mode: String,
    pub(super) timeout_ms: Option<u64>,
    pub(super) reservations: Vec<WorkflowRuntimeBranchTaskAttemptReservationCompatibilityEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskAttemptReservationCompatibilityEntry {
    pub(super) device_id: String,
    pub(super) resource_kind: WorkflowRuntimeTaskAttemptResourceKind,
    pub(super) reserved_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchEligibilityDiagnostic {
    pub(super) code: WorkflowRuntimeBranchBatchEligibilityDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchBatchEligibilityDiagnosticCode {
    MissingEligibilityProfile,
    MissingTaskAttemptFact,
    ModelArtifactMismatch,
    RuntimeFamilyMismatch,
    BackendMismatch,
    RuntimeResidencyMismatch,
    MemoryEstimateMismatch,
    ContextShapeMismatch,
    OperationTypeMismatch,
    CancellationModeMismatch,
    TimeoutMismatch,
    ReservationProfileMissing,
    ReservationProfileMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchTaskEventState {
    Ready,
    Claimed,
    Dispatching,
    Running,
    Completed,
    Deferred,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventDiagnostic {
    pub(super) code: WorkflowRuntimeBranchTaskEventDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchTaskEventDiagnosticCode {
    InvalidEvent,
    DuplicateEvent,
    EventNotFound,
    AlreadyClaimed,
    LeaseExpired,
    MissingClaim,
    StaleClaim,
    TerminalEvent,
    InvalidTransition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchTaskEventClaimOutcome {
    pub(super) record: WorkflowRuntimeBranchTaskEventRecord,
    pub(super) claim: WorkflowRuntimeBranchTaskEventClaim,
}

pub(super) trait WorkflowRuntimeBranchTaskEventRepository {
    fn enqueue(
        &mut self,
        record: WorkflowRuntimeBranchTaskEventRecord,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn claim_event(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn claim_next_due(
        &mut self,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    >;

    #[cfg(test)]
    fn claim_next_due_for_workflow_run(
        &mut self,
        workflow_run_id: &str,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    >;

    fn complete(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        completed_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn mark_dispatching(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        dispatching_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn record_selected_candidate_fact(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn link_dispatch_assignment(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        assignment_id: WorkflowRuntimeDispatchAssignmentId,
        scheduler_task_attempt_id: String,
        linked_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn mark_running(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        running_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn defer(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn defer_until(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
        ready_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn mark_deferred_ready(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        ready_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn release_claim(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        ready_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn fail(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>;

    fn get(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Option<WorkflowRuntimeBranchTaskEventRecord>;
}

#[derive(Debug, Default)]
#[must_use]
pub(super) struct InMemoryWorkflowRuntimeBranchTaskEventRepository {
    records: BTreeMap<String, WorkflowRuntimeBranchTaskEventRecord>,
    ownership: BTreeMap<String, WorkflowRuntimeBranchEventOwnership>,
}

impl InMemoryWorkflowRuntimeBranchTaskEventRepository {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub(super) fn claim_owned_for_workflow_task(
        &mut self,
        workflow_run_id: &str,
        task_id: &str,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<(
            WorkflowRuntimeBranchTaskEventClaimOutcome,
            WorkflowRuntimeClaimOwnership,
        )>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    > {
        let event_id = self
            .records
            .values()
            .filter(|record| {
                record.workflow_run_id == workflow_run_id && record.scheduler_task_id == task_id
            })
            .filter(|record| record.is_due_for_claim(now_ms) && !self.is_owned_or_abandoned(record))
            .min_by_key(|record| (record.ready_at_ms, record.event_id.as_str()))
            .map(|record| record.event_id.clone());
        let Some(event_id) = event_id else {
            return Ok(None);
        };
        let outcome = self.claim_event(&event_id, owner_id, now_ms, lease_duration_ms)?;
        let (liveness, proof) = WorkflowRuntimeClaimLiveness::new();
        self.ownership.insert(
            event_id.as_str().to_owned(),
            WorkflowRuntimeBranchEventOwnership {
                claim: outcome.claim.clone(),
                liveness,
            },
        );
        Ok(Some((outcome, proof)))
    }

    #[cfg(test)]
    pub(super) fn claim_owned_for_workflow_run(
        &mut self,
        workflow_run_id: &str,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<(
            WorkflowRuntimeBranchTaskEventClaimOutcome,
            WorkflowRuntimeClaimOwnership,
        )>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    > {
        let Some(outcome) = self.claim_next_due_for_workflow_run(
            workflow_run_id,
            owner_id,
            now_ms,
            lease_duration_ms,
        )?
        else {
            return Ok(None);
        };
        let (liveness, proof) = WorkflowRuntimeClaimLiveness::new();
        self.ownership.insert(
            outcome.record.event_id.as_str().to_owned(),
            WorkflowRuntimeBranchEventOwnership {
                claim: outcome.claim.clone(),
                liveness,
            },
        );
        Ok(Some((outcome, proof)))
    }
}

impl WorkflowRuntimeBranchTaskEventId {
    pub(super) fn new() -> Self {
        Self(format!("runtime-branch-task-event.{}", Uuid::new_v4()))
    }

    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        parse_non_blank(value, "runtime branch task event id").map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeBranchTaskEventClaimOwnerId {
    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        parse_non_blank(value, "runtime branch task event claim owner id").map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeBranchTaskEventClaimLeaseId {
    fn new() -> Self {
        Self(format!(
            "runtime-branch-task-event-claim.{}",
            Uuid::new_v4()
        ))
    }

    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        parse_non_blank(value, "runtime branch task event claim lease id").map(Self)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeBranchTaskEventRepository for InMemoryWorkflowRuntimeBranchTaskEventRepository {
    fn enqueue(
        &mut self,
        record: WorkflowRuntimeBranchTaskEventRecord,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        let event_id = record.event_id.as_str().to_string();
        if self.records.contains_key(&event_id) {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::DuplicateEvent,
                "runtime branch task event is already enqueued",
            ));
        }
        self.records.insert(event_id, record);
        Ok(())
    }

    fn claim_event(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        if self.has_live_ownership(&record) {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                "runtime branch task event has a live execution owner",
            ));
        }
        let outcome = record.claim(owner_id, now_ms, lease_duration_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), outcome.record.clone());
        Ok(outcome)
    }

    fn claim_next_due(
        &mut self,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    > {
        let Some(event_id) = self.next_due_event_id(now_ms) else {
            return Ok(None);
        };
        self.claim_event(&event_id, owner_id, now_ms, lease_duration_ms)
            .map(Some)
    }

    #[cfg(test)]
    fn claim_next_due_for_workflow_run(
        &mut self,
        workflow_run_id: &str,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeBranchTaskEventClaimOutcome>,
        WorkflowRuntimeBranchTaskEventDiagnostic,
    > {
        let Some(event_id) = self.next_due_event_id_for_workflow_run(workflow_run_id, now_ms)
        else {
            if let Some(record) = self.active_event_for_workflow_run(workflow_run_id, now_ms) {
                return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                    WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                    format!(
                        "runtime branch task event '{}' is already active for workflow run '{}'",
                        record.event_id.as_str(),
                        workflow_run_id
                    ),
                ));
            }
            return Ok(None);
        };
        self.claim_event(&event_id, owner_id, now_ms, lease_duration_ms)
            .map(Some)
    }

    fn complete(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        completed_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.complete(claim, completed_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn mark_dispatching(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        dispatching_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.mark_dispatching(claim, dispatching_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn record_selected_candidate_fact(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        self.validate_transition_ownership(&record, proof)?;
        let updated = record.record_selected_candidate_fact(claim, selected_candidate_fact)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn link_dispatch_assignment(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        assignment_id: WorkflowRuntimeDispatchAssignmentId,
        scheduler_task_attempt_id: String,
        linked_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.link_dispatch_assignment(
            claim,
            assignment_id,
            scheduler_task_attempt_id,
            linked_at_ms,
            live_owned,
        )?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn mark_running(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        running_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.mark_running(claim, running_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn defer(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.defer(claim, deferred_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn defer_until(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
        ready_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.defer_until(claim, deferred_at_ms, ready_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn mark_deferred_ready(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        ready_at_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let updated = record.mark_deferred_ready(ready_at_ms)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn fail(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.fail(claim, failed_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn release_claim(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        ready_at_ms: u64,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        let record = self.record(event_id)?;
        let live_owned = self.validate_transition_ownership(&record, proof)?;
        let updated = record.release_claim(claim, ready_at_ms, live_owned)?;
        self.records
            .insert(event_id.as_str().to_string(), updated.clone());
        Ok(updated)
    }

    fn get(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Option<WorkflowRuntimeBranchTaskEventRecord> {
        self.records.get(event_id.as_str()).cloned()
    }
}

impl InMemoryWorkflowRuntimeBranchTaskEventRepository {
    fn current_ownership(
        &self,
        record: &WorkflowRuntimeBranchTaskEventRecord,
    ) -> Option<&WorkflowRuntimeBranchEventOwnership> {
        self.ownership
            .get(record.event_id.as_str())
            .filter(|owned| record.claim.as_ref() == Some(&owned.claim))
    }

    fn validate_transition_ownership(
        &self,
        record: &WorkflowRuntimeBranchTaskEventRecord,
        proof: Option<&WorkflowRuntimeClaimOwnership>,
    ) -> Result<bool, WorkflowRuntimeBranchTaskEventDiagnostic> {
        match (self.current_ownership(record), proof) {
            (Some(owned), Some(proof)) if owned.liveness.owns(proof) => Ok(true),
            (None, None) => Ok(false),
            _ => Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim,
                "runtime branch transition requires the current scoped ownership proof",
            )),
        }
    }

    fn has_live_ownership(&self, record: &WorkflowRuntimeBranchTaskEventRecord) -> bool {
        self.current_ownership(record)
            .is_some_and(|owned| owned.liveness.is_live())
    }

    fn is_owned_or_abandoned(&self, record: &WorkflowRuntimeBranchTaskEventRecord) -> bool {
        self.current_ownership(record)
            .is_some_and(|owned| owned.liveness.is_live() || owned.liveness.is_abandoned())
    }

    pub(super) fn fail_abandoned(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        let abandoned = self
            .records
            .get(event_id.as_str())
            .and_then(|record| self.current_ownership(record))
            .is_some_and(|owned| &owned.claim == claim && !owned.liveness.is_live());
        if !abandoned {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                "runtime branch claim has not been abandoned after dispatch",
            ));
        }
        let record = self
            .records
            .get_mut(event_id.as_str())
            .expect("abandoned record exists");
        if matches!(
            record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
                | WorkflowRuntimeBranchTaskEventState::Dispatching
                | WorkflowRuntimeBranchTaskEventState::Running
        ) {
            record.state = WorkflowRuntimeBranchTaskEventState::Failed;
            record.failed_at_ms = Some(failed_at_ms);
        }
        Ok(())
    }

    pub(super) fn validate_owned_running(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        proof: &WorkflowRuntimeClaimOwnership,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        let record = self.record(event_id)?;
        record.validate_current_claim_for_state(
            claim,
            &[WorkflowRuntimeBranchTaskEventState::Running],
            "owned event must be running",
        )?;
        self.validate_transition_ownership(&record, Some(proof))?;
        Ok(())
    }

    pub(super) fn mark_owned_dispatch(
        &mut self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        proof: &WorkflowRuntimeClaimOwnership,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        let record = self.record(event_id)?;
        record.validate_current_claim_for_state(
            claim,
            &[WorkflowRuntimeBranchTaskEventState::Running],
            "owned runtime branch must be running before host dispatch",
        )?;
        let owned = self
            .ownership
            .get_mut(event_id.as_str())
            .filter(|owned| &owned.claim == claim && owned.liveness.owns(proof))
            .ok_or_else(|| {
                WorkflowRuntimeBranchTaskEventDiagnostic::new(
                    WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim,
                    "runtime branch dispatch does not own the current claim",
                )
            })?;
        if !owned.liveness.mark_dispatched(proof) {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                "owned event has already crossed host dispatch",
            ));
        }
        Ok(())
    }

    fn record(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        if self
            .records
            .get(event_id.as_str())
            .and_then(|record| self.current_ownership(record))
            .is_some_and(|owned| owned.liveness.is_abandoned())
        {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent, "runtime branch host dispatch ownership was abandoned; replay and old results are fenced"));
        }
        self.records.get(event_id.as_str()).cloned().ok_or_else(|| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::EventNotFound,
                "runtime branch task event was not found",
            )
        })
    }

    fn next_due_event_id(&self, now_ms: u64) -> Option<WorkflowRuntimeBranchTaskEventId> {
        self.records
            .values()
            .filter(|record| record.is_due_for_claim(now_ms) && !self.is_owned_or_abandoned(record))
            .min_by(|left, right| {
                left.ready_at_ms
                    .cmp(&right.ready_at_ms)
                    .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
            })
            .map(|record| record.event_id.clone())
    }

    #[cfg(test)]
    fn next_due_event_id_for_workflow_run(
        &self,
        workflow_run_id: &str,
        now_ms: u64,
    ) -> Option<WorkflowRuntimeBranchTaskEventId> {
        self.records
            .values()
            .filter(|record| record.workflow_run_id == workflow_run_id)
            .filter(|record| record.is_due_for_claim(now_ms) && !self.is_owned_or_abandoned(record))
            .min_by(|left, right| {
                left.ready_at_ms
                    .cmp(&right.ready_at_ms)
                    .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
            })
            .map(|record| record.event_id.clone())
    }

    #[cfg(test)]
    fn active_event_for_workflow_run(
        &self,
        workflow_run_id: &str,
        now_ms: u64,
    ) -> Option<WorkflowRuntimeBranchTaskEventRecord> {
        self.records
            .values()
            .filter(|record| record.workflow_run_id == workflow_run_id)
            .filter(|record| {
                record.has_active_unexpired_claim(now_ms) || self.has_live_ownership(record)
            })
            .min_by(|left, right| {
                left.claim
                    .as_ref()
                    .map(|claim| claim.claimed_at_ms)
                    .cmp(&right.claim.as_ref().map(|claim| claim.claimed_at_ms))
                    .then_with(|| left.event_id.as_str().cmp(right.event_id.as_str()))
            })
            .cloned()
    }
}

impl WorkflowRuntimeBranchTaskEventRecord {
    pub(super) fn ready(
        request: WorkflowRuntimeBranchTaskEventRequest,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        validate_request(&request)?;
        Ok(Self {
            schema_version: WORKFLOW_RUNTIME_BRANCH_TASK_EVENT_SCHEMA_VERSION,
            event_id: request.event_id,
            session_id: request.session_id,
            workflow_id: request.workflow_id,
            workflow_run_id: request.workflow_run_id,
            scheduler_task_id: request.scheduler_task_id,
            scheduler_task_attempt_id: request.scheduler_task_attempt_id,
            attempt_generation: request.attempt_generation,
            queued_input_keys: request.queued_input_keys,
            output_targets: request.output_targets,
            timeout_ms: request.timeout_ms,
            batching_key: request.batching_key,
            runtime_source_context: request.runtime_source_context,
            batch_eligibility: request.batch_eligibility,
            selected_candidate_fact: None,
            dispatch_assignment_link: None,
            state: WorkflowRuntimeBranchTaskEventState::Ready,
            claim: None,
            ready_at_ms: request.ready_at_ms,
            dispatching_at_ms: None,
            running_at_ms: None,
            completed_at_ms: None,
            deferred_at_ms: None,
            failed_at_ms: None,
        })
    }

    pub(super) fn claim(
        mut self,
        owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> Result<WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventDiagnostic>
    {
        if lease_duration_ms == 0 {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                "runtime branch task event claim lease duration must be greater than zero",
            ));
        }

        match self.state {
            WorkflowRuntimeBranchTaskEventState::Ready => {}
            WorkflowRuntimeBranchTaskEventState::Claimed => {
                let claim = self.claim.as_ref().ok_or_else(|| {
                    WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim,
                        "claimed runtime branch task event is missing claim details",
                    )
                })?;
                if now_ms < claim.lease_expires_at_ms {
                    return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                        "runtime branch task event is already claimed by an active lease",
                    ));
                }
                self.attempt_generation = self.attempt_generation.saturating_add(1);
            }
            WorkflowRuntimeBranchTaskEventState::Dispatching
            | WorkflowRuntimeBranchTaskEventState::Running => {
                let claim = self.claim.as_ref().ok_or_else(|| {
                    WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim,
                        "active runtime branch task event is missing claim details",
                    )
                })?;
                if now_ms < claim.lease_expires_at_ms {
                    return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                        "runtime branch task event is already active under an unexpired lease",
                    ));
                }
                self.attempt_generation = self.attempt_generation.saturating_add(1);
            }
            WorkflowRuntimeBranchTaskEventState::Deferred => {
                if self.ready_at_ms > now_ms {
                    return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                        WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed,
                        "deferred runtime branch task event is not ready for retry",
                    ));
                }
                self.attempt_generation = self.attempt_generation.saturating_add(1);
            }
            WorkflowRuntimeBranchTaskEventState::Completed
            | WorkflowRuntimeBranchTaskEventState::Failed => {
                return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                    WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent,
                    "terminal runtime branch task event cannot be claimed",
                ));
            }
        }

        let claim = WorkflowRuntimeBranchTaskEventClaim {
            owner_id,
            lease_id: WorkflowRuntimeBranchTaskEventClaimLeaseId::new(),
            attempt_generation: self.attempt_generation,
            claimed_at_ms: now_ms,
            lease_expires_at_ms: now_ms.saturating_add(lease_duration_ms),
        };
        self.state = WorkflowRuntimeBranchTaskEventState::Claimed;
        self.claim = Some(claim.clone());
        self.scheduler_task_attempt_id = None;
        self.selected_candidate_fact = None;
        self.dispatch_assignment_link = None;
        self.dispatching_at_ms = None;
        self.running_at_ms = None;
        Ok(WorkflowRuntimeBranchTaskEventClaimOutcome {
            record: self,
            claim,
        })
    }

    pub(super) fn mark_dispatching(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        dispatching_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim_for_state(
            claim,
            dispatching_at_ms,
            &[WorkflowRuntimeBranchTaskEventState::Claimed],
            "runtime branch task event must be claimed before dispatching",
            live_owned,
        )?;
        self.state = WorkflowRuntimeBranchTaskEventState::Dispatching;
        self.dispatching_at_ms = Some(dispatching_at_ms);
        Ok(self)
    }

    pub(super) fn record_selected_candidate_fact(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_current_claim_for_state(
            claim,
            &[
                WorkflowRuntimeBranchTaskEventState::Claimed,
                WorkflowRuntimeBranchTaskEventState::Dispatching,
            ],
            "runtime branch task event must be claimed before recording selected candidate evidence",
        )?;
        validate_selected_candidate_fact(&selected_candidate_fact)?;
        if let Some(batch_eligibility) = &self.batch_eligibility {
            ensure_selected_candidate_matches_batch_profile(
                &selected_candidate_fact,
                batch_eligibility,
            )?;
        }
        if self
            .selected_candidate_fact
            .as_ref()
            .is_some_and(|existing| existing != &selected_candidate_fact)
        {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                "runtime branch task event already carries different selected candidate evidence",
            ));
        }
        self.selected_candidate_fact = Some(selected_candidate_fact);
        Ok(self)
    }

    pub(super) fn link_dispatch_assignment(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        assignment_id: WorkflowRuntimeDispatchAssignmentId,
        scheduler_task_attempt_id: String,
        linked_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim_for_state(
            claim,
            linked_at_ms,
            &[
                WorkflowRuntimeBranchTaskEventState::Claimed,
                WorkflowRuntimeBranchTaskEventState::Dispatching,
            ],
            "runtime branch task event must be claimed or dispatching before linking dispatch assignment", live_owned)?;
        validate_non_blank("scheduler task attempt id", &scheduler_task_attempt_id)?;
        if linked_at_ms < claim.claimed_at_ms {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                "runtime branch dispatch assignment link time cannot precede claim time",
            ));
        }
        let link = WorkflowRuntimeBranchTaskEventDispatchAssignmentLink {
            assignment_id,
            scheduler_task_attempt_id,
            claim_attempt_generation: claim.attempt_generation,
            linked_at_ms,
        };
        if let Some(existing) = &self.dispatch_assignment_link {
            if existing != &link {
                return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                    WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                    "runtime branch task event already carries different dispatch assignment facts",
                ));
            }
            return Ok(self);
        }
        if self
            .scheduler_task_attempt_id
            .as_ref()
            .is_some_and(|existing| existing != &link.scheduler_task_attempt_id)
        {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                "runtime branch task event scheduler attempt projection does not match dispatch assignment",
            ));
        }
        self.scheduler_task_attempt_id = Some(link.scheduler_task_attempt_id.clone());
        self.dispatch_assignment_link = Some(link);
        Ok(self)
    }

    pub(super) fn mark_running(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        running_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim_for_state(
            claim,
            running_at_ms,
            &[WorkflowRuntimeBranchTaskEventState::Dispatching],
            "runtime branch task event must be dispatching before running",
            live_owned,
        )?;
        self.state = WorkflowRuntimeBranchTaskEventState::Running;
        self.running_at_ms = Some(running_at_ms);
        Ok(self)
    }

    pub(super) fn complete(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        completed_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, completed_at_ms, live_owned)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Completed;
        self.completed_at_ms = Some(completed_at_ms);
        Ok(self)
    }

    pub(super) fn defer(
        self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.defer_until(claim, deferred_at_ms, deferred_at_ms, live_owned)
    }

    pub(super) fn defer_until(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        deferred_at_ms: u64,
        ready_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, deferred_at_ms, live_owned)?;
        if ready_at_ms < deferred_at_ms {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                "runtime branch task event retry ready time cannot be before deferred time",
            ));
        }
        self.state = WorkflowRuntimeBranchTaskEventState::Deferred;
        self.claim = None;
        self.scheduler_task_attempt_id = None;
        self.selected_candidate_fact = None;
        self.dispatch_assignment_link = None;
        self.ready_at_ms = ready_at_ms;
        self.deferred_at_ms = Some(deferred_at_ms);
        Ok(self)
    }

    pub(super) fn mark_deferred_ready(
        mut self,
        ready_at_ms: u64,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        if self.state != WorkflowRuntimeBranchTaskEventState::Deferred {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                "only deferred runtime branch task events can be marked ready",
            ));
        }
        self.ready_at_ms = ready_at_ms;
        Ok(self)
    }

    pub(super) fn fail(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        failed_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, failed_at_ms, live_owned)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Failed;
        self.failed_at_ms = Some(failed_at_ms);
        Ok(self)
    }

    pub(super) fn release_claim(
        mut self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        ready_at_ms: u64,
        live_owned: bool,
    ) -> Result<Self, WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim(claim, ready_at_ms, live_owned)?;
        self.state = WorkflowRuntimeBranchTaskEventState::Ready;
        self.claim = None;
        self.scheduler_task_attempt_id = None;
        self.selected_candidate_fact = None;
        self.dispatch_assignment_link = None;
        self.ready_at_ms = ready_at_ms;
        self.dispatching_at_ms = None;
        self.running_at_ms = None;
        Ok(self)
    }

    fn validate_active_claim(
        &self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        now_ms: u64,
        live_owned: bool,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        self.validate_active_claim_for_state(
            claim,
            now_ms,
            &[
                WorkflowRuntimeBranchTaskEventState::Claimed,
                WorkflowRuntimeBranchTaskEventState::Dispatching,
                WorkflowRuntimeBranchTaskEventState::Running,
            ],
            "runtime branch task event must be active before terminal transition",
            live_owned,
        )
    }

    fn validate_current_claim_for_state(
        &self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        allowed_states: &[WorkflowRuntimeBranchTaskEventState],
        invalid_message: &'static str,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        if !allowed_states.contains(&self.state) {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                invalid_message,
            ));
        }
        let current = self.claim.as_ref().ok_or_else(|| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim,
                "claimed runtime branch task event is missing claim details",
            )
        })?;
        if current != claim {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim,
                "runtime branch task event claim does not match the current lease",
            ));
        }
        Ok(())
    }

    fn validate_active_claim_for_state(
        &self,
        claim: &WorkflowRuntimeBranchTaskEventClaim,
        now_ms: u64,
        allowed_states: &[WorkflowRuntimeBranchTaskEventState],
        invalid_message: &'static str,
        live_owned: bool,
    ) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
        if !allowed_states.contains(&self.state) {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition,
                invalid_message,
            ));
        }
        let current = self.claim.as_ref().ok_or_else(|| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim,
                "claimed runtime branch task event is missing claim details",
            )
        })?;
        if current != claim {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim,
                "runtime branch task event claim does not match the current lease",
            ));
        }
        if !live_owned && now_ms >= current.lease_expires_at_ms {
            return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::LeaseExpired,
                "runtime branch task event claim lease expired before terminal transition",
            ));
        }
        Ok(())
    }

    fn is_due_for_claim(&self, now_ms: u64) -> bool {
        match self.state {
            WorkflowRuntimeBranchTaskEventState::Ready => self.ready_at_ms <= now_ms,
            WorkflowRuntimeBranchTaskEventState::Deferred => self.ready_at_ms <= now_ms,
            WorkflowRuntimeBranchTaskEventState::Claimed
            | WorkflowRuntimeBranchTaskEventState::Dispatching
            | WorkflowRuntimeBranchTaskEventState::Running => self
                .claim
                .as_ref()
                .is_none_or(|claim| claim.lease_expires_at_ms <= now_ms),
            WorkflowRuntimeBranchTaskEventState::Completed
            | WorkflowRuntimeBranchTaskEventState::Failed => false,
        }
    }

    fn has_active_unexpired_claim(&self, now_ms: u64) -> bool {
        matches!(
            self.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
                | WorkflowRuntimeBranchTaskEventState::Dispatching
                | WorkflowRuntimeBranchTaskEventState::Running
        ) && self
            .claim
            .as_ref()
            .is_some_and(|claim| now_ms < claim.lease_expires_at_ms)
    }
}

impl WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile {
    pub(super) fn ensure_task_attempt_facts_compatible(
        left_fact: Option<&WorkflowRuntimeTaskAttemptFactRecord>,
        right_fact: Option<&WorkflowRuntimeTaskAttemptFactRecord>,
    ) -> Result<(), WorkflowRuntimeBranchBatchEligibilityDiagnostic> {
        let left_fact = left_fact.ok_or_else(|| {
            WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
                WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::MissingTaskAttemptFact,
                "left runtime branch candidate is missing task-attempt facts",
            )
        })?;
        let right_fact = right_fact.ok_or_else(|| {
            WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
                WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::MissingTaskAttemptFact,
                "right runtime branch candidate is missing task-attempt facts",
            )
        })?;
        let left = Self::from_task_attempt_fact(left_fact)?;
        let right = Self::from_task_attempt_fact(right_fact)?;
        left.ensure_compatible_with(&right)
    }

    pub(super) fn from_task_attempt_fact(
        fact: &WorkflowRuntimeTaskAttemptFactRecord,
    ) -> Result<Self, WorkflowRuntimeBranchBatchEligibilityDiagnostic> {
        if fact.reservations.is_empty() {
            return Err(WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
                WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMissing,
                "runtime task-attempt fact has no reservation compatibility evidence",
            ));
        }
        let mut reservations = fact
            .reservations
            .iter()
            .map(
                |reservation| WorkflowRuntimeBranchTaskAttemptReservationCompatibilityEntry {
                    device_id: reservation.device_id.clone(),
                    resource_kind: reservation.resource_kind,
                    reserved_bytes: reservation.reserved_bytes,
                },
            )
            .collect::<Vec<_>>();
        reservations.sort_by(|left, right| {
            left.device_id
                .cmp(&right.device_id)
                .then_with(|| {
                    task_attempt_resource_kind_rank(left.resource_kind)
                        .cmp(&task_attempt_resource_kind_rank(right.resource_kind))
                })
                .then_with(|| left.reserved_bytes.cmp(&right.reserved_bytes))
        });

        Ok(Self {
            model_artifact_id: fact.selected_artifact_id.clone(),
            runtime_family: fact.runtime_family.clone(),
            backend_id: fact.backend_id.clone(),
            runtime_residency_key: fact.runtime_residency_key.clone(),
            loaded_runtime_memory_estimate_bytes: fact.loaded_runtime_memory_estimate_bytes,
            operation_type: fact.operation_type.clone(),
            context_shape_key: fact.context_shape_key.clone(),
            cancellation_mode: fact.cancellation_mode.clone(),
            timeout_ms: fact.timeout_ms,
            reservations,
        })
    }

    pub(super) fn ensure_compatible_with(
        &self,
        other: &Self,
    ) -> Result<(), WorkflowRuntimeBranchBatchEligibilityDiagnostic> {
        ensure_batch_field_matches(
            "model artifact",
            &self.model_artifact_id,
            &other.model_artifact_id,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ModelArtifactMismatch,
        )?;
        ensure_batch_field_matches(
            "runtime family",
            &self.runtime_family,
            &other.runtime_family,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::RuntimeFamilyMismatch,
        )?;
        ensure_batch_field_matches(
            "backend",
            &self.backend_id,
            &other.backend_id,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::BackendMismatch,
        )?;
        ensure_batch_field_matches(
            "runtime residency",
            &self.runtime_residency_key,
            &other.runtime_residency_key,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::RuntimeResidencyMismatch,
        )?;
        if self.loaded_runtime_memory_estimate_bytes != other.loaded_runtime_memory_estimate_bytes {
            return Err(WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
                WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::MemoryEstimateMismatch,
                "runtime branch task attempts have incompatible loaded-runtime memory estimates",
            ));
        }
        ensure_batch_field_matches(
            "operation type",
            &self.operation_type,
            &other.operation_type,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::OperationTypeMismatch,
        )?;
        ensure_batch_field_matches(
            "context shape",
            &self.context_shape_key,
            &other.context_shape_key,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ContextShapeMismatch,
        )?;
        ensure_batch_field_matches(
            "cancellation mode",
            &self.cancellation_mode,
            &other.cancellation_mode,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::CancellationModeMismatch,
        )?;
        if self.timeout_ms != other.timeout_ms {
            return Err(WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
                WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::TimeoutMismatch,
                "runtime branch task attempts have incompatible timeout policies",
            ));
        }
        if self.reservations != other.reservations {
            return Err(WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
                WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMismatch,
                "runtime branch task attempts have incompatible reservation profiles",
            ));
        }
        Ok(())
    }
}

impl WorkflowRuntimeBranchTaskEventDiagnostic {
    fn new(code: WorkflowRuntimeBranchTaskEventDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl WorkflowRuntimeBranchBatchEligibilityDiagnostic {
    fn new(
        code: WorkflowRuntimeBranchBatchEligibilityDiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn validate_request(
    request: &WorkflowRuntimeBranchTaskEventRequest,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    validate_non_blank("session id", &request.session_id)?;
    validate_non_blank("workflow id", &request.workflow_id)?;
    validate_non_blank("workflow run id", &request.workflow_run_id)?;
    validate_non_blank("scheduler task id", &request.scheduler_task_id)?;
    if let Some(scheduler_task_attempt_id) = &request.scheduler_task_attempt_id {
        validate_non_blank("scheduler task attempt id", scheduler_task_attempt_id)?;
    }
    for queued_input_key in &request.queued_input_keys {
        validate_non_blank("queued input key", queued_input_key)?;
    }
    if let Some(batching_key) = &request.batching_key {
        validate_non_blank("runtime branch task event batching key", batching_key)?;
    }
    if let Some(batch_eligibility) = &request.batch_eligibility {
        validate_batch_eligibility_profile(batch_eligibility)?;
    }
    validate_runtime_source_context(&request.runtime_source_context)?;
    Ok(())
}

fn validate_runtime_source_context(
    context: &WorkflowRuntimeSourceContext,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    validate_non_blank(
        "runtime branch source context operation type",
        &context.operation_type,
    )?;
    validate_non_blank(
        "runtime branch source context shape key",
        &context.context_shape_key,
    )?;
    validate_non_blank(
        "runtime branch source context cancellation mode",
        &context.cancellation_mode,
    )?;
    Ok(())
}

fn validate_batch_eligibility_profile(
    profile: &WorkflowRuntimeBranchBatchEligibilityProfile,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    validate_non_blank(
        "runtime branch batch eligibility model artifact id",
        &profile.model_artifact_id,
    )?;
    validate_non_blank(
        "runtime branch batch eligibility runtime family",
        &profile.runtime_family,
    )?;
    validate_non_blank(
        "runtime branch batch eligibility backend id",
        &profile.backend_id,
    )?;
    validate_non_blank(
        "runtime branch batch eligibility device load target",
        &profile.device_load_target,
    )?;
    validate_non_blank(
        "runtime branch batch eligibility runtime residency key",
        &profile.runtime_residency_key,
    )?;
    if profile.estimated_loaded_runtime_bytes == 0 {
        return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
            "runtime branch batch eligibility loaded-runtime memory estimate must be greater than zero",
        ));
    }
    validate_non_blank(
        "runtime branch batch eligibility context shape key",
        &profile.context_shape_key,
    )?;
    validate_non_blank(
        "runtime branch batch eligibility operation type",
        &profile.operation_type,
    )?;
    validate_non_blank(
        "runtime branch batch eligibility cancellation mode",
        &profile.cancellation_mode,
    )?;
    Ok(())
}

fn validate_selected_candidate_fact(
    fact: &WorkflowRuntimeDispatchCandidateFact,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    let bundle = WorkflowRuntimeDispatchCandidateFactBundle {
        contract_version: WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
        facts: vec![fact.clone()],
        diagnostics: Vec::new(),
    };

    ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
        .map(|_| ())
        .map_err(|error| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                format!("runtime branch selected candidate fact is invalid: {error}"),
            )
        })
}

fn ensure_selected_candidate_matches_batch_profile(
    fact: &WorkflowRuntimeDispatchCandidateFact,
    profile: &WorkflowRuntimeBranchBatchEligibilityProfile,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    let selected_artifact_id = fact
        .selected_model_ref
        .selected_artifact_id
        .as_deref()
        .ok_or_else(|| {
            WorkflowRuntimeBranchTaskEventDiagnostic::new(
                WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
                "runtime branch selected candidate fact is missing selected artifact id",
            )
        })?;
    ensure_selected_candidate_field_matches(
        "model artifact id",
        selected_artifact_id,
        &profile.model_artifact_id,
    )?;
    ensure_selected_candidate_field_matches(
        "runtime family",
        &fact.runtime_family,
        &profile.runtime_family,
    )?;
    ensure_selected_candidate_field_matches(
        "backend id",
        &fact.selected_backend_key,
        &profile.backend_id,
    )?;
    ensure_selected_candidate_field_matches(
        "device load target",
        &fact.resolved_load_target,
        &profile.device_load_target,
    )?;
    ensure_selected_candidate_field_matches(
        "runtime residency key",
        &fact.runtime_residency_key,
        &profile.runtime_residency_key,
    )?;
    if fact.loaded_runtime_memory_estimate_bytes != profile.estimated_loaded_runtime_bytes {
        return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
            "runtime branch selected candidate fact loaded-runtime memory estimate does not match batch eligibility profile",
        ));
    }
    Ok(())
}

fn ensure_selected_candidate_field_matches(
    label: &str,
    selected: &str,
    profile: &str,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    if selected != profile {
        return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
            format!(
                "runtime branch selected candidate fact {label} does not match batch eligibility profile"
            ),
        ));
    }
    Ok(())
}

fn ensure_batch_field_matches(
    label: &str,
    left: &str,
    right: &str,
    code: WorkflowRuntimeBranchBatchEligibilityDiagnosticCode,
) -> Result<(), WorkflowRuntimeBranchBatchEligibilityDiagnostic> {
    if left != right {
        return Err(WorkflowRuntimeBranchBatchEligibilityDiagnostic::new(
            code,
            format!("runtime branch task events have incompatible {label} facts"),
        ));
    }
    Ok(())
}

fn task_attempt_resource_kind_rank(kind: WorkflowRuntimeTaskAttemptResourceKind) -> u8 {
    match kind {
        WorkflowRuntimeTaskAttemptResourceKind::SystemRam => 0,
        WorkflowRuntimeTaskAttemptResourceKind::SystemSwap => 1,
        WorkflowRuntimeTaskAttemptResourceKind::DeviceVram => 2,
        WorkflowRuntimeTaskAttemptResourceKind::DeviceSharedMemory => 3,
    }
}

fn parse_non_blank(
    value: impl Into<String>,
    label: &str,
) -> Result<String, WorkflowRuntimeBranchTaskEventDiagnostic> {
    let value = value.into();
    validate_non_blank(label, &value)?;
    Ok(value)
}

fn validate_non_blank(
    label: &str,
    value: &str,
) -> Result<(), WorkflowRuntimeBranchTaskEventDiagnostic> {
    if value.trim().is_empty() {
        return Err(WorkflowRuntimeBranchTaskEventDiagnostic::new(
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent,
            format!("{label} must not be blank"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{
        DependencyEnvironmentId, DependencyEnvironmentRef, DeviceIntentId, PumasModelRef,
    };
    use pantograph_scheduler::{
        SchedulerDispatchCandidateId, SchedulerReservationLeaseId, SchedulerResourceFitAssessment,
        SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
        SchedulerRuntimeVariantId, SchedulerTaskId, SchedulerWorkflowRunId,
    };

    use super::super::runtime_dispatch_assignment::WorkflowRuntimeDispatchAssignmentId;
    use super::super::runtime_dispatch_selection::WorkflowRuntimeDispatchLoadState;
    use super::super::runtime_task_attempt_fact::{
        WorkflowRuntimeTaskAttemptFactRequest, WorkflowRuntimeTaskAttemptReservationFact,
        WorkflowRuntimeTaskAttemptResourceFitFacts, WorkflowRuntimeTaskAttemptResourceFitState,
    };
    use super::*;

    #[test]
    fn runtime_branch_task_event_copied_claim_and_wrong_proof_cannot_mutate_owned_event() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("enqueue");
        let (claimed, ownership) = repository
            .claim_owned_for_workflow_run("run.test", owner_id("worker.alpha"), 100, 30_000)
            .expect("claim")
            .expect("event");
        let id = &claimed.record.event_id;
        let (_, wrong_proof) = WorkflowRuntimeClaimLiveness::new();
        for proof in [None, Some(&wrong_proof)] {
            assert_eq!(
                repository
                    .release_claim(id, &claimed.claim, 30_101, proof)
                    .expect_err("copied identity cannot release")
                    .code,
                WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
            );
            assert!(repository.defer(id, &claimed.claim, 30_101, proof).is_err());
            assert!(repository
                .complete(id, &claimed.claim, 30_101, proof)
                .is_err());
            assert!(repository.fail(id, &claimed.claim, 30_101, proof).is_err());
        }
        assert_eq!(repository.get(id).expect("unchanged event"), claimed.record);
        let _completed = repository
            .complete(id, &claimed.claim, 30_102, Some(&ownership))
            .expect("real owner completes");
    }

    #[test]
    fn runtime_branch_task_event_snapshot_does_not_retain_pre_dispatch_ownership() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("enqueue");
        let (claimed, ownership) = repository
            .claim_owned_for_workflow_run("run.test", owner_id("worker.alpha"), 100, 30_000)
            .expect("claim")
            .expect("due event");
        let snapshot = claimed.record.clone();
        drop(ownership);
        let replacement = repository
            .claim_event(&snapshot.event_id, owner_id("worker.beta"), 30_101, 30_000)
            .expect("pre-dispatch abandoned claim expires");
        assert_eq!(
            replacement.claim.attempt_generation,
            claimed.claim.attempt_generation + 1
        );
        assert_eq!(
            repository
                .complete(&snapshot.event_id, &claimed.claim, 30_102, None)
                .expect_err("old fence rejected")
                .code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
    }

    #[test]
    fn runtime_branch_task_event_abandoned_host_dispatch_is_failed_and_not_replayed() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("enqueue");
        let (claimed, ownership) = repository
            .claim_owned_for_workflow_run("run.test", owner_id("worker.alpha"), 100, 30_000)
            .expect("claim")
            .expect("due event");
        let id = &claimed.record.event_id;
        let _record = repository
            .mark_dispatching(id, &claimed.claim, 101, Some(&ownership))
            .expect("dispatching");
        let _record = repository
            .mark_running(id, &claimed.claim, 102, Some(&ownership))
            .expect("running");
        repository
            .mark_owned_dispatch(id, &claimed.claim, &ownership)
            .expect("host dispatch owned");
        drop(ownership);
        assert_eq!(
            repository.get(id).expect("event").state,
            WorkflowRuntimeBranchTaskEventState::Running
        );
        assert_eq!(
            repository
                .claim_event(id, owner_id("worker.beta"), 30_101, 30_000)
                .expect_err("abandoned host work cannot replay")
                .code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent
        );
        assert!(
            repository.complete(id, &claimed.claim, 103, None).is_err(),
            "old success cannot publish even before expiry"
        );
        repository
            .fail_abandoned(id, &claimed.claim, 30_102)
            .expect("supervised failure observed");
        let failed = repository.get(id).expect("failed event");
        assert_eq!(failed.state, WorkflowRuntimeBranchTaskEventState::Failed);
        assert_eq!(failed.failed_at_ms, Some(30_102));
    }

    #[test]
    fn runtime_branch_task_event_live_owner_excludes_expired_reclaim_and_completes_once() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("enqueue");
        let (claimed, _ownership) = repository
            .claim_owned_for_workflow_run("run.test", owner_id("worker.alpha"), 100, 30_000)
            .expect("claim")
            .expect("due event");
        let snapshot = claimed.record.clone();
        let error = repository
            .claim_event(&snapshot.event_id, owner_id("worker.beta"), 30_101, 30_000)
            .expect_err("live execution excludes reclaim beyond deadline");
        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
        let _record = repository
            .complete(
                &snapshot.event_id,
                &claimed.claim,
                30_102,
                Some(&_ownership),
            )
            .expect("live completion beyond deadline");
        assert!(repository
            .complete(
                &snapshot.event_id,
                &claimed.claim,
                30_103,
                Some(&_ownership)
            )
            .is_err());
    }

    #[test]
    fn runtime_branch_task_event_starts_ready_with_stable_contract_fields() {
        let record = ready_record();

        assert_eq!(
            record.schema_version,
            WORKFLOW_RUNTIME_BRANCH_TASK_EVENT_SCHEMA_VERSION
        );
        assert_eq!(record.event_id.as_str(), "runtime-branch-task-event.test");
        assert_eq!(record.session_id, "session.test");
        assert_eq!(record.workflow_id, "workflow.image");
        assert_eq!(record.workflow_run_id, "run.test");
        assert_eq!(record.scheduler_task_id, "image-task");
        assert_eq!(
            record.scheduler_task_attempt_id.as_deref(),
            Some("attempt.1")
        );
        assert_eq!(record.attempt_generation, 1);
        assert_eq!(record.queued_input_keys, vec!["prompt".to_string()]);
        assert_eq!(record.timeout_ms, Some(30_000));
        assert_eq!(
            record.batching_key.as_deref(),
            Some("runtime.diffusers.cuda0")
        );
        assert_eq!(record.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(record.claim.is_none());
        assert!(record.dispatch_assignment_link.is_none());
        assert_eq!(record.dispatching_at_ms, None);
        assert_eq!(record.running_at_ms, None);
    }

    #[test]
    fn runtime_branch_task_event_rejects_blank_identity_fields() {
        let mut request = ready_request();
        request.workflow_run_id = " ".to_string();

        let error = WorkflowRuntimeBranchTaskEventRecord::ready(request)
            .expect_err("blank run id must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent
        );
        assert!(error.message.contains("workflow run id"));
    }

    #[test]
    fn runtime_branch_task_attempt_batch_compatibility_compares_canonical_facts() {
        let left = task_attempt_fact(default_reservation_facts());
        let mut right = task_attempt_fact(default_reservation_facts());
        right.runtime_residency_key = "runtime.diffusers.loaded-model-1".to_string();

        let error = WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible(
            Some(&left),
            Some(&right),
        )
            .expect_err("runtime residency mismatch must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::RuntimeResidencyMismatch
        );

        let right = task_attempt_fact(default_reservation_facts());
        WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible(
            Some(&left),
            Some(&right),
        )
            .expect("matching canonical facts are batch compatible");
    }

    #[test]
    fn runtime_branch_task_attempt_batch_compatibility_rejects_missing_fact() {
        let right = task_attempt_fact(default_reservation_facts());

        let error = WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible(
            None,
            Some(&right),
        )
        .expect_err("missing canonical task-attempt fact must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::MissingTaskAttemptFact
        );
    }

    #[test]
    fn runtime_branch_task_attempt_batch_compatibility_rejects_timeout_mismatch() {
        let left = task_attempt_fact(default_reservation_facts());
        let mut right = task_attempt_fact(default_reservation_facts());
        right.timeout_ms = Some(60_000);

        assert_task_attempt_compatibility_error(
            &left,
            &right,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::TimeoutMismatch,
        );
    }

    #[test]
    fn runtime_branch_task_attempt_batch_compatibility_rejects_source_context_mismatches() {
        let left = task_attempt_fact(default_reservation_facts());

        let mut right = task_attempt_fact(default_reservation_facts());
        right.operation_type = "image-generation.img2img".to_string();
        assert_task_attempt_compatibility_error(
            &left,
            &right,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::OperationTypeMismatch,
        );

        let mut right = task_attempt_fact(default_reservation_facts());
        right.context_shape_key = "txt2img.512x512.steps20".to_string();
        assert_task_attempt_compatibility_error(
            &left,
            &right,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ContextShapeMismatch,
        );

        let mut right = task_attempt_fact(default_reservation_facts());
        right.cancellation_mode = "whole-batch".to_string();
        assert_task_attempt_compatibility_error(
            &left,
            &right,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::CancellationModeMismatch,
        );
    }

    #[test]
    fn runtime_branch_task_attempt_batch_compatibility_ignores_matching_provisional_key() {
        let left_event = ready_record();
        let right_event = ready_record_with_id("runtime-branch-task-event.other");
        let left_fact = task_attempt_fact(default_reservation_facts());
        let mut right_fact = task_attempt_fact(default_reservation_facts());
        right_fact.backend_id = "backend.other".to_string();

        assert_eq!(left_event.batching_key, right_event.batching_key);
        let error = WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible(
            Some(&left_fact),
            Some(&right_fact),
        )
            .expect_err("matching provisional batching key must not authorize batching");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::BackendMismatch
        );
    }

    #[test]
    fn runtime_branch_task_event_batch_eligibility_validates_profile_fields() {
        let mut request = ready_request();
        let mut profile = batch_profile();
        profile.estimated_loaded_runtime_bytes = 0;
        request.batch_eligibility = Some(profile);

        let error = WorkflowRuntimeBranchTaskEventRecord::ready(request)
            .expect_err("zero memory estimate must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent
        );
        assert!(
            error
                .message
                .contains("loaded-runtime memory estimate must be greater than zero"),
            "unexpected error: {}",
            error.message
        );
    }

    #[test]
    fn runtime_branch_task_attempt_batch_profile_derives_reservation_level_facts() {
        let fact = task_attempt_fact(vec![
            reservation_fact(
                "reservation.gpu",
                "cuda:0",
                WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                6_442_450_944,
            ),
            reservation_fact(
                "reservation.ram",
                "system",
                WorkflowRuntimeTaskAttemptResourceKind::SystemRam,
                2_147_483_648,
            ),
        ]);

        let profile =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::from_task_attempt_fact(
                &fact,
            )
            .expect("compatibility profile derives");

        assert_eq!(profile.model_artifact_id, "artifact.stable-diffusion-xl");
        assert_eq!(profile.runtime_family, "diffusers");
        assert_eq!(profile.backend_id, "backend.cuda");
        assert_eq!(
            profile.runtime_residency_key,
            "runtime.diffusers.loaded-model-0"
        );
        assert_eq!(profile.loaded_runtime_memory_estimate_bytes, 8_589_934_592);
        assert_eq!(profile.operation_type, "image-generation.txt2img");
        assert_eq!(profile.context_shape_key, "txt2img.1024x1024.steps30");
        assert_eq!(profile.cancellation_mode, "per-run-fanout");
        assert_eq!(profile.timeout_ms, Some(30_000));
        assert_eq!(
            profile.reservations,
            vec![
                WorkflowRuntimeBranchTaskAttemptReservationCompatibilityEntry {
                    device_id: "cuda:0".to_string(),
                    resource_kind: WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                    reserved_bytes: 6_442_450_944,
                },
                WorkflowRuntimeBranchTaskAttemptReservationCompatibilityEntry {
                    device_id: "system".to_string(),
                    resource_kind: WorkflowRuntimeTaskAttemptResourceKind::SystemRam,
                    reserved_bytes: 2_147_483_648,
                },
            ]
        );
    }

    #[test]
    fn runtime_branch_task_attempt_batch_profile_ignores_reservation_lease_ids() {
        let left =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::from_task_attempt_fact(
                &task_attempt_fact(vec![
                    reservation_fact(
                        "reservation.left.gpu",
                        "cuda:0",
                        WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                        6_442_450_944,
                    ),
                    reservation_fact(
                        "reservation.left.ram",
                        "system",
                        WorkflowRuntimeTaskAttemptResourceKind::SystemRam,
                        2_147_483_648,
                    ),
                ]),
            )
            .expect("left profile");
        let right =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::from_task_attempt_fact(
                &task_attempt_fact(vec![
                    reservation_fact(
                        "reservation.right.ram",
                        "system",
                        WorkflowRuntimeTaskAttemptResourceKind::SystemRam,
                        2_147_483_648,
                    ),
                    reservation_fact(
                        "reservation.right.gpu",
                        "cuda:0",
                        WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                        6_442_450_944,
                    ),
                ]),
            )
            .expect("right profile");

        assert_eq!(left, right);
        left.ensure_compatible_with(&right)
            .expect("matching reservation requirements are compatible");
    }

    #[test]
    fn runtime_branch_task_attempt_batch_profile_rejects_missing_reservations() {
        let fact = task_attempt_fact(Vec::new());

        let error =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::from_task_attempt_fact(
                &fact,
            )
            .expect_err("reservation evidence is required");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMissing
        );
    }

    #[test]
    fn runtime_branch_task_attempt_batch_profile_rejects_reservation_mismatch() {
        let left =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::from_task_attempt_fact(
                &task_attempt_fact(vec![reservation_fact(
                    "reservation.left.gpu",
                    "cuda:0",
                    WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                    6_442_450_944,
                )]),
            )
            .expect("left profile");
        let right =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::from_task_attempt_fact(
                &task_attempt_fact(vec![reservation_fact(
                    "reservation.right.gpu",
                    "cuda:1",
                    WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                    6_442_450_944,
                )]),
            )
            .expect("right profile");

        let error = left
            .ensure_compatible_with(&right)
            .expect_err("different device reservation must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMismatch
        );

        let left = task_attempt_fact(vec![reservation_fact(
            "reservation.left.gpu",
            "cuda:0",
            WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
            6_442_450_944,
        )]);
        let right = task_attempt_fact(vec![reservation_fact(
            "reservation.right.shared",
            "cuda:0",
            WorkflowRuntimeTaskAttemptResourceKind::DeviceSharedMemory,
            6_442_450_944,
        )]);
        assert_task_attempt_compatibility_error(
            &left,
            &right,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMismatch,
        );

        let right = task_attempt_fact(vec![reservation_fact(
            "reservation.right.gpu",
            "cuda:0",
            WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
            3_221_225_472,
        )]);
        assert_task_attempt_compatibility_error(
            &left,
            &right,
            WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMismatch,
        );
    }

    #[test]
    fn runtime_branch_task_event_records_selected_candidate_fact_under_current_claim() {
        let claimed = ready_record_with_batch_profile(batch_profile())
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let selected_candidate_fact = selected_candidate_fact();

        let recorded = claimed
            .record
            .record_selected_candidate_fact(&claimed.claim, selected_candidate_fact.clone())
            .expect("selected candidate fact records");

        assert_eq!(
            recorded.selected_candidate_fact.as_ref(),
            Some(&selected_candidate_fact)
        );
        let recorded_again = recorded
            .clone()
            .record_selected_candidate_fact(&claimed.claim, selected_candidate_fact.clone())
            .expect("recording identical selected evidence is idempotent");
        assert_eq!(
            recorded_again.selected_candidate_fact.as_ref(),
            Some(&selected_candidate_fact)
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_selected_candidate_fact_for_stale_claim() {
        let first_claimed = ready_record_with_batch_profile(batch_profile())
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let replay_claimed = first_claimed
            .record
            .claim(owner_id("worker.beta"), 150, 50)
            .expect("expired event reclaims");

        let error = replay_claimed
            .record
            .record_selected_candidate_fact(&first_claimed.claim, selected_candidate_fact())
            .expect_err("stale claim cannot record selected evidence");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_selected_candidate_fact_profile_mismatch() {
        let claimed = ready_record_with_batch_profile(batch_profile())
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let mut selected_candidate_fact = selected_candidate_fact();
        selected_candidate_fact.selected_backend_key = "backend.other".to_string();

        let error = claimed
            .record
            .record_selected_candidate_fact(&claimed.claim, selected_candidate_fact)
            .expect_err("candidate profile mismatch must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent
        );
        assert!(
            error.message.contains("backend id"),
            "unexpected diagnostic: {}",
            error.message
        );
    }

    #[test]
    fn runtime_branch_task_event_reclaim_clears_selected_candidate_fact() {
        let claimed = ready_record_with_batch_profile(batch_profile())
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let recorded = claimed
            .record
            .record_selected_candidate_fact(&claimed.claim, selected_candidate_fact())
            .expect("selected candidate fact records");

        let replay = recorded
            .claim(owner_id("worker.beta"), 150, 50)
            .expect("expired event reclaims");

        assert!(replay.record.selected_candidate_fact.is_none());
    }

    #[test]
    fn runtime_branch_task_event_links_dispatch_assignment_under_current_claim() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 80)
            .expect("ready event claims");
        let assignment_id = dispatch_assignment_id("runtime-dispatch-assignment.1");

        let linked = claimed
            .record
            .link_dispatch_assignment(
                &claimed.claim,
                assignment_id.clone(),
                "scheduler-task-attempt.1".to_string(),
                110,
                false,
            )
            .expect("dispatch assignment links");

        assert_eq!(
            linked.scheduler_task_attempt_id.as_deref(),
            Some("scheduler-task-attempt.1")
        );
        let link = linked
            .dispatch_assignment_link
            .clone()
            .expect("assignment link");
        assert_eq!(link.assignment_id, assignment_id);
        assert_eq!(link.scheduler_task_attempt_id, "scheduler-task-attempt.1");
        assert_eq!(
            link.claim_attempt_generation,
            claimed.claim.attempt_generation
        );
        assert_eq!(link.linked_at_ms, 110);

        let linked_again = linked
            .link_dispatch_assignment(
                &claimed.claim,
                dispatch_assignment_id("runtime-dispatch-assignment.1"),
                "scheduler-task-attempt.1".to_string(),
                110,
                false,
            )
            .expect("identical dispatch assignment link is idempotent");
        assert_eq!(linked_again.dispatch_assignment_link, Some(link));
    }

    #[test]
    fn runtime_branch_task_event_rejects_dispatch_assignment_for_stale_claim() {
        let first_claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let replay_claimed = first_claimed
            .record
            .claim(owner_id("worker.beta"), 150, 50)
            .expect("expired event reclaims");

        let error = replay_claimed
            .record
            .link_dispatch_assignment(
                &first_claimed.claim,
                dispatch_assignment_id("runtime-dispatch-assignment.1"),
                "scheduler-task-attempt.1".to_string(),
                160,
                false,
            )
            .expect_err("stale claim cannot link dispatch assignment");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_different_dispatch_assignment_link() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 80)
            .expect("ready event claims");
        let linked = claimed
            .record
            .link_dispatch_assignment(
                &claimed.claim,
                dispatch_assignment_id("runtime-dispatch-assignment.1"),
                "scheduler-task-attempt.1".to_string(),
                110,
                false,
            )
            .expect("dispatch assignment links");

        let error = linked
            .link_dispatch_assignment(
                &claimed.claim,
                dispatch_assignment_id("runtime-dispatch-assignment.2"),
                "scheduler-task-attempt.1".to_string(),
                110,
                false,
            )
            .expect_err("different assignment link must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent
        );
        assert!(error.message.contains("different dispatch assignment"));
    }

    #[test]
    fn runtime_branch_task_event_reclaim_clears_dispatch_assignment_link() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let linked = claimed
            .record
            .link_dispatch_assignment(
                &claimed.claim,
                dispatch_assignment_id("runtime-dispatch-assignment.1"),
                "scheduler-task-attempt.1".to_string(),
                110,
                false,
            )
            .expect("dispatch assignment links");

        let replay = linked
            .claim(owner_id("worker.beta"), 150, 50)
            .expect("expired event reclaims");

        assert!(replay.record.dispatch_assignment_link.is_none());
        assert_eq!(replay.record.scheduler_task_attempt_id, None);
    }

    #[test]
    fn runtime_branch_task_event_claims_ready_event_with_lease() {
        let owner = owner_id("worker.alpha");
        let outcome = ready_record()
            .claim(owner.clone(), 100, 50)
            .expect("ready event claims");

        assert_eq!(
            outcome.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(outcome.claim.owner_id, owner);
        assert_eq!(outcome.claim.attempt_generation, 1);
        assert_eq!(outcome.claim.claimed_at_ms, 100);
        assert_eq!(outcome.claim.lease_expires_at_ms, 150);
        assert_eq!(
            outcome.record.claim.as_ref().expect("stored claim"),
            &outcome.claim
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_duplicate_active_claim() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims")
            .record;

        let error = claimed
            .claim(owner_id("worker.beta"), 149, 50)
            .expect_err("active lease blocks duplicate claim");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
    }

    #[test]
    fn runtime_branch_task_event_reclaims_after_lease_expiry_with_new_generation() {
        let first = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let second = first
            .record
            .claim(owner_id("worker.beta"), 150, 60)
            .expect("expired lease reclaims");

        assert_eq!(
            second.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(second.record.attempt_generation, 2);
        assert_eq!(second.claim.attempt_generation, 2);
        assert_eq!(second.claim.owner_id.as_str(), "worker.beta");
        assert_ne!(second.claim.lease_id, first.claim.lease_id);
    }

    #[test]
    fn runtime_branch_task_event_completes_only_with_current_claim() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let completed = claimed
            .record
            .complete(&claimed.claim, 120, false)
            .expect("current claim completes");

        assert_eq!(
            completed.state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(completed.completed_at_ms, Some(120));
    }

    #[test]
    fn runtime_branch_task_event_records_dispatching_and_running_before_completion() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 80)
            .expect("ready event claims");
        let dispatching = claimed
            .record
            .mark_dispatching(&claimed.claim, 110, false)
            .expect("current claim marks dispatching");
        assert_eq!(
            dispatching.state,
            WorkflowRuntimeBranchTaskEventState::Dispatching
        );
        assert_eq!(dispatching.dispatching_at_ms, Some(110));
        assert_eq!(dispatching.running_at_ms, None);

        let running = dispatching
            .mark_running(&claimed.claim, 120, false)
            .expect("current claim marks running");
        assert_eq!(running.state, WorkflowRuntimeBranchTaskEventState::Running);
        assert_eq!(running.dispatching_at_ms, Some(110));
        assert_eq!(running.running_at_ms, Some(120));

        let completed = running
            .complete(&claimed.claim, 130, false)
            .expect("running event completes");
        assert_eq!(
            completed.state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(completed.dispatching_at_ms, Some(110));
        assert_eq!(completed.running_at_ms, Some(120));
        assert_eq!(completed.completed_at_ms, Some(130));
    }

    #[test]
    fn runtime_branch_task_event_rejects_running_before_dispatching() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 80)
            .expect("ready event claims");

        let error = claimed
            .record
            .mark_running(&claimed.claim, 110, false)
            .expect_err("claimed event cannot skip dispatching");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition
        );
        assert!(error.message.contains("dispatching before running"));
    }

    #[test]
    fn runtime_branch_task_event_rejects_stale_claim_terminal_transition() {
        let first = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let second = first
            .record
            .claim(owner_id("worker.beta"), 150, 60)
            .expect("expired lease reclaims");

        let error = second
            .record
            .complete(&first.claim, 160, false)
            .expect_err("stale claim cannot complete");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
    }

    #[test]
    fn runtime_branch_task_event_rejects_expired_claim_terminal_transition() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let error = claimed
            .record
            .complete(&claimed.claim, 150, false)
            .expect_err("expired claim cannot complete");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::LeaseExpired
        );
    }

    #[test]
    fn runtime_branch_task_event_records_deferred_retry_and_failed_terminal_states() {
        let deferred_claim = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let deferred = deferred_claim
            .record
            .defer(&deferred_claim.claim, 120, false)
            .expect("current claim defers");
        assert_eq!(
            deferred.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert_eq!(deferred.deferred_at_ms, Some(120));
        assert_eq!(deferred.ready_at_ms, 120);
        assert!(deferred.claim.is_none());

        let not_due = deferred
            .clone()
            .claim(owner_id("worker.beta"), 119, 50)
            .expect_err("deferred event is not due before ready_at");
        assert_eq!(
            not_due.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
        let retry = deferred
            .claim(owner_id("worker.beta"), 120, 50)
            .expect("deferred event reclaims when due");
        assert_eq!(
            retry.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(retry.claim.attempt_generation, 2);

        let failed_claim = ready_record()
            .claim(owner_id("worker.beta"), 200, 50)
            .expect("ready event claims");
        let failed = failed_claim
            .record
            .fail(&failed_claim.claim, 220, false)
            .expect("current claim fails");
        assert_eq!(failed.state, WorkflowRuntimeBranchTaskEventState::Failed);
        assert_eq!(failed.failed_at_ms, Some(220));
    }

    #[test]
    fn runtime_branch_task_event_defer_until_separates_deferred_and_retry_ready_times() {
        let deferred_claim = ready_record()
            .claim(owner_id("worker.alpha"), 100, 80)
            .expect("ready event claims");
        let deferred = deferred_claim
            .record
            .defer_until(&deferred_claim.claim, 120, 180, false)
            .expect("current claim defers until retry time");

        assert_eq!(
            deferred.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert_eq!(deferred.deferred_at_ms, Some(120));
        assert_eq!(deferred.ready_at_ms, 180);
        assert!(deferred.claim.is_none());

        let invalid_claimed = ready_record()
            .claim(owner_id("worker.beta"), 200, 80)
            .expect("ready event claims");
        let error = invalid_claimed
            .record
            .defer_until(&invalid_claimed.claim, 220, 219, false)
            .expect_err("retry ready time cannot precede deferred time");
        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition
        );
    }

    #[test]
    fn runtime_branch_task_event_marks_deferred_event_ready_for_recovery() {
        let deferred_claim = ready_record()
            .claim(owner_id("worker.alpha"), 100, 80)
            .expect("ready event claims");
        let deferred = deferred_claim
            .record
            .defer_until(&deferred_claim.claim, 120, 180, false)
            .expect("current claim defers until retry time");

        let ready = deferred
            .mark_deferred_ready(130)
            .expect("deferred event marks ready");

        assert_eq!(ready.state, WorkflowRuntimeBranchTaskEventState::Deferred);
        assert_eq!(ready.ready_at_ms, 130);
        assert!(ready.is_due_for_claim(130));
    }

    #[test]
    fn runtime_branch_task_event_rejects_mark_ready_for_non_deferred_event() {
        let error = ready_record()
            .mark_deferred_ready(130)
            .expect_err("ready event cannot be marked deferred-ready");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition
        );
    }

    #[test]
    fn runtime_branch_task_event_releases_claim_back_to_ready() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");

        let released = claimed
            .record
            .mark_dispatching(&claimed.claim, 110, false)
            .expect("event marks dispatching")
            .release_claim(&claimed.claim, 120, false)
            .expect("current claim releases");

        assert_eq!(released.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(released.claim.is_none());
        assert_eq!(released.ready_at_ms, 120);
        assert_eq!(released.dispatching_at_ms, None);
        assert_eq!(released.running_at_ms, None);
        assert_eq!(released.deferred_at_ms, None);
        assert_eq!(released.failed_at_ms, None);
        assert_eq!(released.completed_at_ms, None);
    }

    #[test]
    fn runtime_branch_task_event_rejects_claim_after_terminal_state() {
        let claimed = ready_record()
            .claim(owner_id("worker.alpha"), 100, 50)
            .expect("ready event claims");
        let completed = claimed
            .record
            .complete(&claimed.claim, 120, false)
            .expect("current claim completes");

        let error = completed
            .claim(owner_id("worker.beta"), 130, 50)
            .expect_err("terminal event cannot be claimed");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_enqueues_and_claims_due_event() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("event enqueues");

        let outcome = repository
            .claim_next_due(owner_id("worker.alpha"), 42, 50)
            .expect("claim next succeeds")
            .expect("due event exists");

        assert_eq!(
            outcome.record.event_id.as_str(),
            "runtime-branch-task-event.test"
        );
        assert_eq!(
            outcome.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        let stored = repository
            .get(&outcome.record.event_id)
            .expect("claimed event is stored");
        assert_eq!(stored.claim.as_ref(), Some(&outcome.claim));
    }

    #[test]
    fn runtime_branch_task_event_repository_rejects_duplicate_enqueue() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository.enqueue(ready_record()).expect("event enqueues");

        let error = repository
            .enqueue(ready_record())
            .expect_err("duplicate event fails");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::DuplicateEvent
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_returns_none_without_due_event() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let mut request = ready_request();
        request.ready_at_ms = 200;
        repository
            .enqueue(
                WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("future event ready"),
            )
            .expect("event enqueues");

        let claimed = repository
            .claim_next_due(owner_id("worker.alpha"), 199, 50)
            .expect("claim next succeeds");

        assert!(claimed.is_none());
    }

    #[test]
    fn runtime_branch_task_event_repository_claims_next_due_for_workflow_run_only() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        repository
            .enqueue(ready_record_with_id_and_run(
                "runtime-branch-task-event.other",
                "run.other",
                10,
            ))
            .expect("other event enqueues");
        repository
            .enqueue(ready_record_with_id_and_run(
                "runtime-branch-task-event.target",
                "run.target",
                20,
            ))
            .expect("target event enqueues");

        let claimed = repository
            .claim_next_due_for_workflow_run("run.target", owner_id("worker.alpha"), 25, 50)
            .expect("claim next succeeds")
            .expect("target event is due");

        assert_eq!(
            claimed.record.event_id.as_str(),
            "runtime-branch-task-event.target"
        );
        assert_eq!(
            repository
                .get(&event_id("runtime-branch-task-event.other"))
                .expect("other event")
                .state,
            WorkflowRuntimeBranchTaskEventState::Ready
        );
    }

    #[test]
    fn runtime_branch_task_event_claims_selected_task_without_claiming_earlier_downstream_event() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        for (id, run, task, ready_at) in [
            ("event.downstream", "run.target", "task.downstream", 1),
            ("event.other", "run.other", "task.ready", 2),
            ("event.ready", "run.target", "task.ready", 3),
        ] {
            let mut record = ready_record_with_id_and_run(id, run, ready_at);
            record.scheduler_task_id = task.to_string();
            repository.enqueue(record).expect("enqueue event");
        }
        let (claimed, proof) = repository
            .claim_owned_for_workflow_task(
                "run.target",
                "task.ready",
                owner_id("worker.alpha"),
                10,
                20,
            )
            .expect("selected task claim")
            .expect("ready event");
        assert_eq!(claimed.record.event_id.as_str(), "event.ready");
        for id in ["event.downstream", "event.other"] {
            let record = repository.get(&event_id(id)).expect("unselected event");
            assert_eq!(record.state, WorkflowRuntimeBranchTaskEventState::Ready);
            assert!(record.claim.is_none());
        }
        assert!(repository
            .claim_owned_for_workflow_task(
                "run.target",
                "task.ready",
                owner_id("worker.beta"),
                31,
                20,
            )
            .expect("live ownership excludes expired competitor")
            .is_none());
        let completed = repository
            .complete(&claimed.record.event_id, &claimed.claim, 32, Some(&proof))
            .expect("owner settles after lease expiry");
        assert_eq!(
            completed.state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_reclaims_expired_claim() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let first = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");

        let second = repository
            .claim_next_due(owner_id("worker.beta"), 150, 60)
            .expect("claim next succeeds")
            .expect("expired event is due");

        assert_eq!(second.record.attempt_generation, 2);
        assert_eq!(second.claim.owner_id.as_str(), "worker.beta");
        assert_ne!(second.claim.lease_id, first.claim.lease_id);
    }

    #[test]
    fn runtime_branch_task_event_repository_rejects_duplicate_active_dispatch_for_run() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 80)
            .expect("event claims");
        let dispatching = repository
            .mark_dispatching(&event_id, &claimed.claim, 110, None)
            .expect("event marks dispatching");
        assert_eq!(
            dispatching.state,
            WorkflowRuntimeBranchTaskEventState::Dispatching
        );

        let error = repository
            .claim_next_due_for_workflow_run("run.test", owner_id("worker.beta"), 120, 80)
            .expect_err("duplicate active dispatch is rejected");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
        assert!(
            error.message.contains("already active for workflow run"),
            "unexpected diagnostic: {}",
            error.message
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_reclaims_expired_running_event_for_replay() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let first = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 80)
            .expect("event claims");
        let dispatching = repository
            .mark_dispatching(&event_id, &first.claim, 110, None)
            .expect("event marks dispatching");
        assert_eq!(
            dispatching.state,
            WorkflowRuntimeBranchTaskEventState::Dispatching
        );
        let running = repository
            .mark_running(&event_id, &first.claim, 120, None)
            .expect("event marks running");
        assert_eq!(running.state, WorkflowRuntimeBranchTaskEventState::Running);

        let replay = repository
            .claim_next_due_for_workflow_run("run.test", owner_id("worker.replay"), 180, 90)
            .expect("claim next succeeds")
            .expect("expired running event is replayable");

        assert_eq!(replay.record.event_id, event_id);
        assert_eq!(replay.record.attempt_generation, 2);
        assert_eq!(replay.claim.attempt_generation, 2);
        assert_eq!(replay.claim.owner_id.as_str(), "worker.replay");
        assert_ne!(replay.claim.lease_id, first.claim.lease_id);
        assert_eq!(
            replay.record.state,
            WorkflowRuntimeBranchTaskEventState::Claimed
        );
        assert_eq!(replay.record.dispatching_at_ms, None);
        assert_eq!(replay.record.running_at_ms, None);
    }

    #[test]
    fn runtime_branch_task_event_repository_persists_terminal_completion() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");

        let completed = repository
            .complete(&event_id, &claimed.claim, 120, None)
            .expect("event completes");

        assert_eq!(
            completed.state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(completed.completed_at_ms, Some(120));
        assert_eq!(
            repository.get(&event_id).expect("stored event").state,
            WorkflowRuntimeBranchTaskEventState::Completed
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_persists_dispatching_and_running_states() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 80)
            .expect("event claims");

        let dispatching = repository
            .mark_dispatching(&event_id, &claimed.claim, 110, None)
            .expect("event marks dispatching");
        assert_eq!(
            dispatching.state,
            WorkflowRuntimeBranchTaskEventState::Dispatching
        );
        assert_eq!(dispatching.dispatching_at_ms, Some(110));

        let running = repository
            .mark_running(&event_id, &claimed.claim, 120, None)
            .expect("event marks running");
        assert_eq!(running.state, WorkflowRuntimeBranchTaskEventState::Running);
        assert_eq!(running.running_at_ms, Some(120));
        assert_eq!(
            repository.get(&event_id).expect("stored event").state,
            WorkflowRuntimeBranchTaskEventState::Running
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_records_selected_candidate_fact() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository
            .enqueue(ready_record_with_batch_profile(batch_profile()))
            .expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 80)
            .expect("event claims");
        let selected_candidate_fact = selected_candidate_fact();

        let recorded = repository
            .record_selected_candidate_fact(
                &event_id,
                &claimed.claim,
                selected_candidate_fact.clone(),
                None,
            )
            .expect("repository records selected candidate fact");

        assert_eq!(
            recorded.selected_candidate_fact.as_ref(),
            Some(&selected_candidate_fact)
        );
        assert_eq!(
            repository
                .get(&event_id)
                .expect("stored event")
                .selected_candidate_fact
                .as_ref(),
            Some(&selected_candidate_fact)
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_persists_dispatch_assignment_link() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 80)
            .expect("event claims");
        let assignment_id = dispatch_assignment_id("runtime-dispatch-assignment.1");

        let linked = repository
            .link_dispatch_assignment(
                &event_id,
                &claimed.claim,
                assignment_id.clone(),
                "scheduler-task-attempt.1".to_string(),
                110,
                None,
            )
            .expect("repository links dispatch assignment");

        assert_eq!(
            linked.dispatch_assignment_link.as_ref().map(|link| (
                link.assignment_id.clone(),
                link.scheduler_task_attempt_id.as_str(),
            )),
            Some((assignment_id, "scheduler-task-attempt.1"))
        );
        assert_eq!(
            repository
                .get(&event_id)
                .expect("stored event")
                .dispatch_assignment_link,
            linked.dispatch_assignment_link
        );
    }

    #[test]
    fn runtime_branch_task_event_repository_does_not_mutate_on_stale_terminal_claim() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let first = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let second = repository
            .claim_event(&event_id, owner_id("worker.beta"), 150, 60)
            .expect("expired event reclaims");

        let error = repository
            .complete(&event_id, &first.claim, 160, None)
            .expect_err("stale claim cannot complete");

        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim
        );
        let stored = repository.get(&event_id).expect("stored event");
        assert_eq!(stored.claim.as_ref(), Some(&second.claim));
        assert_eq!(stored.state, WorkflowRuntimeBranchTaskEventState::Claimed);
    }

    #[test]
    fn runtime_branch_task_event_repository_persists_deferred_and_failed_events() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let deferred_id = event_id("runtime-branch-task-event.defer");
        repository
            .enqueue(ready_record_with_id("runtime-branch-task-event.defer"))
            .expect("deferred event enqueues");
        let deferred_claim = repository
            .claim_event(&deferred_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");
        let deferred = repository
            .defer(&deferred_id, &deferred_claim.claim, 120, None)
            .expect("event defers");
        assert_eq!(
            deferred.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert!(deferred.claim.is_none());
        assert_eq!(
            repository
                .claim_next_due_for_workflow_run("run.test", owner_id("worker.retry"), 119, 50)
                .expect("claim next succeeds"),
            None
        );
        let retry_claim = repository
            .claim_next_due_for_workflow_run("run.test", owner_id("worker.retry"), 120, 50)
            .expect("claim next succeeds")
            .expect("deferred event is due");
        assert_eq!(retry_claim.record.event_id, deferred_id);
        assert_eq!(retry_claim.claim.attempt_generation, 2);

        let failed_id = event_id("runtime-branch-task-event.fail");
        repository
            .enqueue(ready_record_with_id("runtime-branch-task-event.fail"))
            .expect("failed event enqueues");
        let failed_claim = repository
            .claim_event(&failed_id, owner_id("worker.beta"), 200, 50)
            .expect("event claims");
        let failed = repository
            .fail(&failed_id, &failed_claim.claim, 220, None)
            .expect("event fails");
        assert_eq!(failed.state, WorkflowRuntimeBranchTaskEventState::Failed);
    }

    #[test]
    fn runtime_branch_task_event_repository_releases_claimed_event_to_ready() {
        let mut repository = InMemoryWorkflowRuntimeBranchTaskEventRepository::new();
        let event_id = event_id("runtime-branch-task-event.test");
        repository.enqueue(ready_record()).expect("event enqueues");
        let claimed = repository
            .claim_event(&event_id, owner_id("worker.alpha"), 100, 50)
            .expect("event claims");

        let released = repository
            .release_claim(&event_id, &claimed.claim, 120, None)
            .expect("event claim releases");

        assert_eq!(released.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(released.claim.is_none());
        assert_eq!(
            repository.get(&event_id).expect("stored event").state,
            WorkflowRuntimeBranchTaskEventState::Ready
        );
        let reclaimed = repository
            .claim_next_due_for_workflow_run("run.test", owner_id("worker.beta"), 121, 50)
            .expect("claim next succeeds")
            .expect("released event is due");
        assert_eq!(reclaimed.record.event_id, event_id);
    }

    fn ready_record() -> WorkflowRuntimeBranchTaskEventRecord {
        WorkflowRuntimeBranchTaskEventRecord::ready(ready_request()).expect("ready record")
    }

    fn ready_record_with_id(event_id: &str) -> WorkflowRuntimeBranchTaskEventRecord {
        let mut request = ready_request();
        request.event_id = WorkflowRuntimeBranchTaskEventId::parse(event_id).expect("event id");
        WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("ready record")
    }

    fn ready_record_with_batch_profile(
        profile: WorkflowRuntimeBranchBatchEligibilityProfile,
    ) -> WorkflowRuntimeBranchTaskEventRecord {
        let mut request = ready_request();
        request.batch_eligibility = Some(profile);
        WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("ready record")
    }

    fn ready_record_with_id_and_batch_profile(
        event_id: &str,
        profile: WorkflowRuntimeBranchBatchEligibilityProfile,
    ) -> WorkflowRuntimeBranchTaskEventRecord {
        let mut request = ready_request();
        request.event_id = WorkflowRuntimeBranchTaskEventId::parse(event_id).expect("event id");
        request.batch_eligibility = Some(profile);
        WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("ready record")
    }

    fn ready_record_with_id_and_run(
        event_id: &str,
        workflow_run_id: &str,
        ready_at_ms: u64,
    ) -> WorkflowRuntimeBranchTaskEventRecord {
        let mut request = ready_request();
        request.event_id = WorkflowRuntimeBranchTaskEventId::parse(event_id).expect("event id");
        request.workflow_run_id = workflow_run_id.to_string();
        request.ready_at_ms = ready_at_ms;
        WorkflowRuntimeBranchTaskEventRecord::ready(request).expect("ready record")
    }

    fn ready_request() -> WorkflowRuntimeBranchTaskEventRequest {
        WorkflowRuntimeBranchTaskEventRequest {
            event_id: WorkflowRuntimeBranchTaskEventId::parse("runtime-branch-task-event.test")
                .expect("event id"),
            session_id: "session.test".to_string(),
            workflow_id: "workflow.image".to_string(),
            workflow_run_id: "run.test".to_string(),
            scheduler_task_id: "image-task".to_string(),
            scheduler_task_attempt_id: Some("attempt.1".to_string()),
            attempt_generation: 1,
            queued_input_keys: vec!["prompt".to_string()],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
            }]),
            timeout_ms: Some(30_000),
            batching_key: Some("runtime.diffusers.cuda0".to_string()),
            runtime_source_context: runtime_source_context(),
            batch_eligibility: None,
            ready_at_ms: 42,
        }
    }

    fn runtime_source_context() -> crate::graph::WorkflowRuntimeSourceContext {
        crate::graph::WorkflowRuntimeSourceContext {
            operation_type: "image-generation.txt2img".to_string(),
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
        }
    }

    fn batch_profile() -> WorkflowRuntimeBranchBatchEligibilityProfile {
        WorkflowRuntimeBranchBatchEligibilityProfile {
            model_artifact_id: "artifact.stable-diffusion-xl".to_string(),
            runtime_family: "diffusers".to_string(),
            backend_id: "backend.cuda".to_string(),
            device_load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.loaded-model-0".to_string(),
            estimated_loaded_runtime_bytes: 8_589_934_592,
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            operation_type: "image-generation.txt2img".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
        }
    }

    fn task_attempt_fact(
        reservations: Vec<WorkflowRuntimeTaskAttemptReservationFact>,
    ) -> WorkflowRuntimeTaskAttemptFactRecord {
        WorkflowRuntimeTaskAttemptFactRecord::new(WorkflowRuntimeTaskAttemptFactRequest {
            workflow_id: "workflow.image".to_string(),
            workflow_run_id: "run.test".to_string(),
            scheduler_task_id: "image-task".to_string(),
            scheduler_task_attempt_id: "attempt.1".to_string(),
            task_attempt_generation: 1,
            selected_model_id: "model.stable-diffusion-xl".to_string(),
            selected_artifact_id: "artifact.stable-diffusion-xl".to_string(),
            selected_runtime_id: "runtime.diffusers".to_string(),
            selected_runtime_variant_id: Some("cuda".to_string()),
            backend_id: "backend.cuda".to_string(),
            runtime_family: "diffusers".to_string(),
            load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.loaded-model-0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8_589_934_592,
            resource_fit: WorkflowRuntimeTaskAttemptResourceFitFacts {
                state: WorkflowRuntimeTaskAttemptResourceFitState::Fits,
                diagnostic_codes: Vec::new(),
            },
            reservations,
            operation_type: "image-generation.txt2img".to_string(),
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
            timeout_ms: Some(30_000),
            recorded_at_ms: 200,
        })
        .expect("task-attempt fact")
    }

    fn default_reservation_facts() -> Vec<WorkflowRuntimeTaskAttemptReservationFact> {
        vec![
            reservation_fact(
                "reservation.gpu",
                "cuda:0",
                WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                6_442_450_944,
            ),
            reservation_fact(
                "reservation.ram",
                "system",
                WorkflowRuntimeTaskAttemptResourceKind::SystemRam,
                2_147_483_648,
            ),
        ]
    }

    fn assert_task_attempt_compatibility_error(
        left: &WorkflowRuntimeTaskAttemptFactRecord,
        right: &WorkflowRuntimeTaskAttemptFactRecord,
        expected_code: WorkflowRuntimeBranchBatchEligibilityDiagnosticCode,
    ) {
        let error =
            WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible(
                Some(left),
                Some(right),
            )
            .expect_err("task-attempt facts must fail compatibility");

        assert_eq!(error.code, expected_code);
    }

    fn reservation_fact(
        reservation_lease_id: &str,
        device_id: &str,
        resource_kind: WorkflowRuntimeTaskAttemptResourceKind,
        reserved_bytes: u64,
    ) -> WorkflowRuntimeTaskAttemptReservationFact {
        WorkflowRuntimeTaskAttemptReservationFact {
            reservation_lease_id: reservation_lease_id.to_string(),
            device_id: device_id.to_string(),
            resource_kind,
            reserved_bytes,
        }
    }

    fn selected_candidate_fact() -> WorkflowRuntimeDispatchCandidateFact {
        let workflow_run_id: SchedulerWorkflowRunId = "run.test".parse().expect("run id");
        let task_id: SchedulerTaskId = "image-task".parse().expect("task id");
        let device_id: DeviceIntentId = "cuda:0".parse().expect("device id");
        WorkflowRuntimeDispatchCandidateFact {
            candidate_id: SchedulerDispatchCandidateId::parse("candidate.diffusers.cuda0")
                .expect("candidate id"),
            selected_runtime_id: "runtime.diffusers".parse().expect("runtime id"),
            selected_runtime_variant_id: Some(
                SchedulerRuntimeVariantId::parse("cuda").expect("runtime variant id"),
            ),
            selected_backend_key: "backend.cuda".to_string(),
            runtime_family: "diffusers".to_string(),
            resolved_load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.loaded-model-0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8_589_934_592,
            runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
            runtime_instance_id: Some("runtime.diffusers.001".to_string()),
            selected_device_ids: vec![device_id.clone()],
            selected_model_ref: PumasModelRef {
                model_id: "model.sdxl".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("artifact.stable-diffusion-xl".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            runtime_trait_settings: Vec::new(),
            environment_ref: DependencyEnvironmentRef {
                environment_id: DependencyEnvironmentId::parse("env.runtime")
                    .expect("environment id"),
                manifest_id: None,
            },
            reservations: vec![SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(
                    "reservation-lease.runtime.1",
                )
                .expect("reservation lease id"),
                workflow_run_id: workflow_run_id.clone(),
                task_id: task_id.clone(),
                device_id,
                resource_kind: SchedulerResourceKind::DeviceVram,
                reserved_bytes: 8_589_934_592,
            }],
            resource_fit_assessment: SchedulerResourceFitAssessment {
                workflow_run_id,
                task_id,
                state: SchedulerResourceFitState::Fits,
                diagnostics: Vec::new(),
            },
            batching_group_id: None,
        }
    }

    fn owner_id(value: &str) -> WorkflowRuntimeBranchTaskEventClaimOwnerId {
        WorkflowRuntimeBranchTaskEventClaimOwnerId::parse(value).expect("owner id")
    }

    fn event_id(value: &str) -> WorkflowRuntimeBranchTaskEventId {
        WorkflowRuntimeBranchTaskEventId::parse(value).expect("event id")
    }

    fn dispatch_assignment_id(value: &str) -> WorkflowRuntimeDispatchAssignmentId {
        WorkflowRuntimeDispatchAssignmentId::parse(value).expect("assignment id")
    }
}
