use std::collections::BTreeMap;

use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerReservationLeaseId, SchedulerRuntimeHandoff,
    SchedulerRuntimeHandoffState, ValidatedSchedulerRuntimeHandoff,
};
use uuid::Uuid;

use crate::graph::WorkflowRuntimeSourceContext;

use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchBatchEligibilityDiagnostic,
    WorkflowRuntimeBranchBatchEligibilityDiagnosticCode,
    WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile, WorkflowRuntimeBranchTaskEventClaim,
    WorkflowRuntimeBranchTaskEventId,
};
use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;
use super::runtime_task_attempt_fact::{
    WorkflowRuntimeTaskAttemptFactBuildRequest, WorkflowRuntimeTaskAttemptFactDiagnostic,
    WorkflowRuntimeTaskAttemptFactRecord, WorkflowRuntimeTaskAttemptSourceContext,
    WorkflowRuntimeTaskAttemptSourceContextRequest,
};

pub(super) const WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_SCHEMA_VERSION: u16 = 1;
pub(super) const WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_BATCH_BROKER_WAIT_WINDOW_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchClaimId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentRequest {
    pub(super) assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) runtime_branch_event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) scheduler_task_attempt_started_at_ms: u64,
    pub(super) task_attempt_generation: u64,
    pub(super) timeout_ms: Option<u64>,
    pub(super) runtime_source_context: WorkflowRuntimeSourceContext,
    pub(super) runtime_branch_claim: WorkflowRuntimeBranchTaskEventClaim,
    pub(super) readiness_proof: DependencyReadinessProofEnvelope,
    pub(super) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    pub(super) selected_runtime_handoff: SchedulerRuntimeHandoff,
    pub(super) reservation_lease_id: SchedulerReservationLeaseId,
    pub(super) selected_candidate_id: Option<SchedulerDispatchCandidateId>,
    pub(super) created_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchClaim {
    pub(super) batch_claim_id: WorkflowRuntimeDispatchAssignmentBatchClaimId,
    pub(super) owner_id: WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
    pub(super) anchor_assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) claimed_at_ms: u64,
    pub(super) lease_expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchClaimOutcome {
    pub(super) batch_claim: WorkflowRuntimeDispatchAssignmentBatchClaim,
    pub(super) assignments: Vec<WorkflowRuntimeDispatchAssignmentRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchBrokerRequest {
    pub(super) anchor_assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) now_ms: u64,
    pub(super) min_assignments: usize,
    pub(super) max_assignments: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeDispatchAssignmentBatchBrokerDecision {
    ReadyToClaim {
        assignments: Vec<WorkflowRuntimeDispatchAssignmentRecord>,
    },
    WaitingForPeers {
        anchor_assignment: WorkflowRuntimeDispatchAssignmentRecord,
        compatible_assignments: Vec<WorkflowRuntimeDispatchAssignmentRecord>,
        required_assignments: usize,
    },
    WaitWindowExpired {
        anchor_assignment: WorkflowRuntimeDispatchAssignmentRecord,
        expiry_diagnostic: WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnostic,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest {
    pub(super) decision: WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
    pub(super) owner_id: WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
    pub(super) now_ms: u64,
    pub(super) lease_duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest {
    pub(super) decision: WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
    pub(super) now_ms: u64,
    pub(super) wait_window_duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchBrokerWaitWindow {
    pub(super) waiting_since_ms: u64,
    pub(super) expires_at_ms: u64,
    pub(super) required_assignments: usize,
    pub(super) expiry_diagnostic: WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnostic {
    pub(super) code: WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode {
    BatchWindowExpired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentRecord {
    pub(super) schema_version: u16,
    pub(super) assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) runtime_branch_event_id: WorkflowRuntimeBranchTaskEventId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) scheduler_task_attempt_started_at_ms: u64,
    pub(super) task_attempt_generation: u64,
    pub(super) timeout_ms: Option<u64>,
    pub(super) runtime_source_context: WorkflowRuntimeSourceContext,
    pub(super) runtime_branch_claim: WorkflowRuntimeBranchTaskEventClaim,
    pub(super) readiness_proof: DependencyReadinessProofEnvelope,
    pub(super) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    pub(super) selected_runtime_handoff: SchedulerRuntimeHandoff,
    pub(super) reservation_lease_id: SchedulerReservationLeaseId,
    pub(super) selected_candidate_id: Option<SchedulerDispatchCandidateId>,
    pub(super) task_attempt_fact: Option<WorkflowRuntimeTaskAttemptFactRecord>,
    pub(super) batch_claim: Option<WorkflowRuntimeDispatchAssignmentBatchClaim>,
    pub(super) batch_broker_wait_window:
        Option<WorkflowRuntimeDispatchAssignmentBatchBrokerWaitWindow>,
    pub(super) state: WorkflowRuntimeDispatchAssignmentState,
    pub(super) created_at_ms: u64,
    pub(super) updated_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeDispatchAssignmentState {
    Prepared,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentDiagnostic {
    pub(super) code: WorkflowRuntimeDispatchAssignmentDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeDispatchAssignmentDiagnosticCode {
    InvalidAssignment,
    DuplicateAssignment,
    DuplicateActiveAssignment,
    AssignmentNotFound,
    InvalidTransition,
    TaskAttemptFactInvalid,
    InvalidBatchClaim,
    AssignmentNotRunning,
    AlreadyBatchClaimed,
    MissingTaskAttemptFact,
    BatchCompatibilityRejected,
    InvalidBatchBrokerWaitWindow,
}

pub(super) trait WorkflowRuntimeDispatchAssignmentRepository {
    fn create(
        &mut self,
        request: WorkflowRuntimeDispatchAssignmentRequest,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn mark_running(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn mark_completed(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn mark_cancelled(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn mark_failed(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn claim_compatible_running_batch(
        &mut self,
        anchor_assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        owner_id: WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
        max_assignments: usize,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    >;

    fn evaluate_running_batch_broker_decision(
        &self,
        request: WorkflowRuntimeDispatchAssignmentBatchBrokerRequest,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    >;

    fn claim_batch_broker_decision(
        &mut self,
        request: WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    >;

    fn record_batch_broker_waiting_decision(
        &mut self,
        request: WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn mark_batch_broker_wait_window_expired(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>;

    fn get(
        &self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    ) -> Option<WorkflowRuntimeDispatchAssignmentRecord>;
}

#[derive(Debug, Default)]
#[must_use]
pub(super) struct InMemoryWorkflowRuntimeDispatchAssignmentRepository {
    records: BTreeMap<WorkflowRuntimeDispatchAssignmentId, WorkflowRuntimeDispatchAssignmentRecord>,
}

impl InMemoryWorkflowRuntimeDispatchAssignmentRepository {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

impl WorkflowRuntimeDispatchAssignmentId {
    pub(super) fn new() -> Self {
        Self(format!("runtime-dispatch-assignment.{}", Uuid::new_v4()))
    }

    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeDispatchAssignmentDiagnostic> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment,
                "runtime dispatch assignment id must be non-empty",
            ));
        }
        Ok(Self(value))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeDispatchAssignmentBatchClaimId {
    fn new() -> Self {
        Self(format!(
            "runtime-dispatch-assignment-batch-claim.{}",
            Uuid::new_v4()
        ))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId {
    pub(super) fn parse(
        value: impl Into<String>,
    ) -> Result<Self, WorkflowRuntimeDispatchAssignmentDiagnostic> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
                "runtime dispatch assignment batch-claim owner id must be non-empty",
            ));
        }
        Ok(Self(value))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl WorkflowRuntimeDispatchAssignmentRepository
    for InMemoryWorkflowRuntimeDispatchAssignmentRepository
{
    fn create(
        &mut self,
        request: WorkflowRuntimeDispatchAssignmentRequest,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        validate_assignment_request(&request)?;
        if self.records.contains_key(&request.assignment_id) {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::DuplicateAssignment,
                "runtime dispatch assignment already exists",
            ));
        }
        if self.records.values().any(|record| {
            record.runtime_branch_event_id == request.runtime_branch_event_id
                && record.scheduler_task_attempt_id == request.scheduler_task_attempt_id
                && !record.state.is_terminal()
        }) {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::DuplicateActiveAssignment,
                "runtime branch task event already has an active dispatch assignment for this task attempt",
            ));
        }
        let record = WorkflowRuntimeDispatchAssignmentRecord {
            schema_version: WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_SCHEMA_VERSION,
            assignment_id: request.assignment_id,
            runtime_branch_event_id: request.runtime_branch_event_id,
            session_id: request.session_id,
            workflow_id: request.workflow_id,
            workflow_run_id: request.workflow_run_id,
            scheduler_task_id: request.scheduler_task_id,
            scheduler_task_attempt_id: request.scheduler_task_attempt_id,
            scheduler_task_attempt_started_at_ms: request.scheduler_task_attempt_started_at_ms,
            task_attempt_generation: request.task_attempt_generation,
            timeout_ms: request.timeout_ms,
            runtime_source_context: request.runtime_source_context,
            runtime_branch_claim: request.runtime_branch_claim,
            readiness_proof: request.readiness_proof,
            selected_candidate_fact: request.selected_candidate_fact,
            selected_runtime_handoff: request.selected_runtime_handoff,
            reservation_lease_id: request.reservation_lease_id,
            selected_candidate_id: request.selected_candidate_id,
            task_attempt_fact: None,
            batch_claim: None,
            batch_broker_wait_window: None,
            state: WorkflowRuntimeDispatchAssignmentState::Prepared,
            created_at_ms: request.created_at_ms,
            updated_at_ms: request.created_at_ms,
        };
        self.records
            .insert(record.assignment_id.clone(), record.clone());
        Ok(record)
    }

    fn mark_running(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        self.transition(
            assignment_id,
            now_ms,
            &[WorkflowRuntimeDispatchAssignmentState::Prepared],
            WorkflowRuntimeDispatchAssignmentState::Running,
            "runtime dispatch assignment must be prepared before running",
        )
    }

    fn mark_completed(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        self.transition(
            assignment_id,
            now_ms,
            &[WorkflowRuntimeDispatchAssignmentState::Running],
            WorkflowRuntimeDispatchAssignmentState::Completed,
            "runtime dispatch assignment must be running before completion",
        )
    }

    fn mark_cancelled(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        self.transition(
            assignment_id,
            now_ms,
            &[WorkflowRuntimeDispatchAssignmentState::Running],
            WorkflowRuntimeDispatchAssignmentState::Cancelled,
            "runtime dispatch assignment must be running before cancellation",
        )
    }

    fn mark_failed(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        self.transition(
            assignment_id,
            now_ms,
            &[
                WorkflowRuntimeDispatchAssignmentState::Prepared,
                WorkflowRuntimeDispatchAssignmentState::Running,
            ],
            WorkflowRuntimeDispatchAssignmentState::Failed,
            "runtime dispatch assignment must be active before failure",
        )
    }

    fn claim_compatible_running_batch(
        &mut self,
        anchor_assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        owner_id: WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
        max_assignments: usize,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    > {
        self.claim_compatible_running_batch_record(
            anchor_assignment_id,
            owner_id,
            now_ms,
            lease_duration_ms,
            max_assignments,
        )
    }

    fn get(
        &self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    ) -> Option<WorkflowRuntimeDispatchAssignmentRecord> {
        self.records.get(assignment_id).cloned()
    }

    fn evaluate_running_batch_broker_decision(
        &self,
        request: WorkflowRuntimeDispatchAssignmentBatchBrokerRequest,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    > {
        validate_batch_broker_request(&request)?;
        if let Some(decision) = self.expired_batch_broker_wait_window_decision(
            &request.anchor_assignment_id,
            request.now_ms,
        )? {
            return Ok(decision);
        }
        let selected_assignment_ids = self.select_compatible_running_batch_assignment_ids(
            &request.anchor_assignment_id,
            request.now_ms,
            request.max_assignments,
        )?;
        let selected_assignments = selected_assignment_ids
            .iter()
            .map(|assignment_id| {
                self.records.get(assignment_id).cloned().ok_or_else(|| {
                    WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                        WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                        "runtime dispatch assignment selected for broker decision was not found",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if selected_assignments.len() >= request.min_assignments {
            return Ok(
                WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::ReadyToClaim {
                    assignments: selected_assignments,
                },
            );
        }
        let anchor_assignment = selected_assignments
            .first()
            .cloned()
            .expect("batch broker selection always contains the anchor assignment");
        Ok(
            WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitingForPeers {
                anchor_assignment,
                compatible_assignments: selected_assignments.into_iter().skip(1).collect(),
                required_assignments: request.min_assignments,
            },
        )
    }

    fn claim_batch_broker_decision(
        &mut self,
        request: WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    > {
        validate_batch_claim_request(request.now_ms, request.lease_duration_ms, 1)?;
        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::ReadyToClaim { assignments } =
            request.decision
        else {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
                "runtime dispatch assignment batch broker cannot claim a non-ready decision",
            ));
        };
        if assignments.len() < 2 {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
                "runtime dispatch assignment batch broker requires at least two assignments before claiming",
            ));
        }
        let anchor_assignment_id = assignments
            .first()
            .expect("ready broker decision must include an anchor")
            .assignment_id
            .clone();
        let selected_assignment_ids = assignments
            .iter()
            .map(|assignment| assignment.assignment_id.clone())
            .collect::<Vec<_>>();
        let current_selection = self.select_compatible_running_batch_assignment_ids(
            &anchor_assignment_id,
            request.now_ms,
            selected_assignment_ids.len(),
        )?;
        if current_selection != selected_assignment_ids {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::BatchCompatibilityRejected,
                "runtime dispatch assignment batch broker selected assignments are no longer claimable",
            ));
        }
        self.claim_selected_running_batch_assignment_ids(
            selected_assignment_ids,
            request.owner_id,
            request.now_ms,
            request.lease_duration_ms,
            anchor_assignment_id,
        )
    }

    fn record_batch_broker_waiting_decision(
        &mut self,
        request: WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        validate_batch_broker_wait_request(request.now_ms, request.wait_window_duration_ms)?;
        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitingForPeers {
            anchor_assignment,
            required_assignments,
            ..
        } = request.decision
        else {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow,
                "runtime dispatch assignment batch broker cannot record a wait window for a non-waiting decision",
            ));
        };
        let record = self
            .records
            .get_mut(&anchor_assignment.assignment_id)
            .ok_or_else(|| {
                WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                    WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                    "runtime dispatch assignment batch broker wait anchor was not found",
                )
            })?;
        ensure_batch_claimable(record, request.now_ms)?;
        if request.now_ms >= record.runtime_branch_claim.lease_expires_at_ms {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow,
                "runtime dispatch assignment batch broker wait cannot start after the runtime branch claim lease expired",
            ));
        }
        if record.batch_broker_wait_window.is_none() {
            record.batch_broker_wait_window =
                Some(WorkflowRuntimeDispatchAssignmentBatchBrokerWaitWindow::new(
                    request.now_ms,
                    request.wait_window_duration_ms,
                    required_assignments,
                    record.runtime_branch_claim.lease_expires_at_ms,
                ));
            record.updated_at_ms = request.now_ms;
        }
        Ok(record.clone())
    }

    fn mark_batch_broker_wait_window_expired(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        validate_batch_broker_wait_transition_timestamp(now_ms)?;
        let record = self.records.get_mut(assignment_id).ok_or_else(|| {
            WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                "runtime dispatch assignment batch broker wait anchor was not found",
            )
        })?;
        if record.state != WorkflowRuntimeDispatchAssignmentState::Running {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidTransition,
                "runtime dispatch assignment must be running before wait-window expiry",
            ));
        }
        let wait_window = record.batch_broker_wait_window.as_ref().ok_or_else(|| {
            WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow,
                "runtime dispatch assignment has no batch broker wait window to expire",
            )
        })?;
        if now_ms < wait_window.expires_at_ms {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow,
                "runtime dispatch assignment batch broker wait window has not expired",
            ));
        }
        record.state = WorkflowRuntimeDispatchAssignmentState::Failed;
        record.updated_at_ms = now_ms;
        Ok(record.clone())
    }
}

impl InMemoryWorkflowRuntimeDispatchAssignmentRepository {
    fn claim_compatible_running_batch_record(
        &mut self,
        anchor_assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        owner_id: WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
        max_assignments: usize,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    > {
        validate_batch_claim_request(now_ms, lease_duration_ms, max_assignments)?;
        let selected_assignment_ids = self.select_compatible_running_batch_assignment_ids(
            anchor_assignment_id,
            now_ms,
            max_assignments,
        )?;
        self.claim_selected_running_batch_assignment_ids(
            selected_assignment_ids,
            owner_id,
            now_ms,
            lease_duration_ms,
            anchor_assignment_id.clone(),
        )
    }

    fn claim_selected_running_batch_assignment_ids(
        &mut self,
        selected_assignment_ids: Vec<WorkflowRuntimeDispatchAssignmentId>,
        owner_id: WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
        now_ms: u64,
        lease_duration_ms: u64,
        anchor_assignment_id: WorkflowRuntimeDispatchAssignmentId,
    ) -> Result<
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    > {
        let batch_claim = WorkflowRuntimeDispatchAssignmentBatchClaim {
            batch_claim_id: WorkflowRuntimeDispatchAssignmentBatchClaimId::new(),
            owner_id,
            anchor_assignment_id,
            claimed_at_ms: now_ms,
            lease_expires_at_ms: now_ms.saturating_add(lease_duration_ms),
        };
        let mut assignments = Vec::with_capacity(selected_assignment_ids.len());
        for assignment_id in selected_assignment_ids {
            let record = self.records.get_mut(&assignment_id).ok_or_else(|| {
                WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                    WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                    "runtime dispatch assignment selected for batch claim was not found",
                )
            })?;
            record.batch_claim = Some(batch_claim.clone());
            record.batch_broker_wait_window = None;
            record.updated_at_ms = now_ms;
            assignments.push(record.clone());
        }

        Ok(WorkflowRuntimeDispatchAssignmentBatchClaimOutcome {
            batch_claim,
            assignments,
        })
    }

    fn select_compatible_running_batch_assignment_ids(
        &self,
        anchor_assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
        max_assignments: usize,
    ) -> Result<Vec<WorkflowRuntimeDispatchAssignmentId>, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        let anchor = self.records.get(anchor_assignment_id).ok_or_else(|| {
            WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                "runtime dispatch assignment batch anchor was not found",
            )
        })?;
        ensure_batch_claimable(anchor, now_ms)?;
        let anchor_fact = task_attempt_fact(anchor)?;
        let mut selected_assignment_ids = vec![anchor_assignment_id.clone()];

        for (assignment_id, candidate) in &self.records {
            if selected_assignment_ids.len() >= max_assignments {
                break;
            }
            if assignment_id == anchor_assignment_id
                || candidate.state != WorkflowRuntimeDispatchAssignmentState::Running
                || candidate.has_active_batch_claim(now_ms)
                || candidate.has_expired_batch_broker_wait_window(now_ms)
            {
                continue;
            }
            let candidate_fact = task_attempt_fact(candidate)?;
            match WorkflowRuntimeBranchTaskAttemptBatchCompatibilityProfile::ensure_task_attempt_facts_compatible(
                Some(anchor_fact),
                Some(candidate_fact),
            ) {
                Ok(()) => selected_assignment_ids.push(assignment_id.clone()),
                Err(diagnostic) if is_batch_compatibility_mismatch(diagnostic.code) => {}
                Err(diagnostic) => {
                    return Err(batch_compatibility_error(diagnostic));
                }
            }
        }

        Ok(selected_assignment_ids)
    }

    fn expired_batch_broker_wait_window_decision(
        &self,
        anchor_assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
    ) -> Result<
        Option<WorkflowRuntimeDispatchAssignmentBatchBrokerDecision>,
        WorkflowRuntimeDispatchAssignmentDiagnostic,
    > {
        let anchor = self.records.get(anchor_assignment_id).ok_or_else(|| {
            WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                "runtime dispatch assignment batch anchor was not found",
            )
        })?;
        ensure_batch_claimable(anchor, now_ms)?;
        let Some(wait_window) = anchor.batch_broker_wait_window.as_ref() else {
            return Ok(None);
        };
        if now_ms < wait_window.expires_at_ms {
            return Ok(None);
        }
        Ok(Some(
            WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitWindowExpired {
                anchor_assignment: anchor.clone(),
                expiry_diagnostic: wait_window.expiry_diagnostic.clone(),
            },
        ))
    }

    fn transition(
        &mut self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
        allowed_states: &[WorkflowRuntimeDispatchAssignmentState],
        next_state: WorkflowRuntimeDispatchAssignmentState,
        message: &'static str,
    ) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentDiagnostic>
    {
        if now_ms == 0 {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment,
                "runtime dispatch assignment transition timestamp must be greater than zero",
            ));
        }
        let record = self.records.get_mut(assignment_id).ok_or_else(|| {
            WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound,
                "runtime dispatch assignment was not found",
            )
        })?;
        if !allowed_states.contains(&record.state) {
            return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidTransition,
                message,
            ));
        }
        if next_state == WorkflowRuntimeDispatchAssignmentState::Running {
            record.task_attempt_fact = Some(record.task_attempt_fact_record(now_ms).map_err(
                |diagnostic| {
                    WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                        WorkflowRuntimeDispatchAssignmentDiagnosticCode::TaskAttemptFactInvalid,
                        format!(
                            "runtime dispatch assignment task-attempt fact is invalid: {}",
                            diagnostic.message
                        ),
                    )
                },
            )?);
        }
        record.state = next_state;
        record.updated_at_ms = now_ms;
        Ok(record.clone())
    }
}

impl WorkflowRuntimeDispatchAssignmentBatchBrokerWaitWindow {
    fn new(
        waiting_since_ms: u64,
        wait_window_duration_ms: u64,
        required_assignments: usize,
        claim_lease_expires_at_ms: u64,
    ) -> Self {
        let policy_expires_at_ms = waiting_since_ms.saturating_add(wait_window_duration_ms);
        let claim_safe_expires_at_ms = claim_lease_expires_at_ms.saturating_sub(1);
        Self {
            waiting_since_ms,
            expires_at_ms: policy_expires_at_ms.min(claim_safe_expires_at_ms),
            required_assignments,
            expiry_diagnostic:
                WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnostic {
                    code: WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode::BatchWindowExpired,
                    message: format!(
                        "runtime dispatch assignment batch broker wait window expired before reaching {required_assignments} compatible assignments",
                    ),
                },
        }
    }
}

impl WorkflowRuntimeDispatchAssignmentRecord {
    pub(super) fn task_attempt_fact_record(
        &self,
        recorded_at_ms: u64,
    ) -> Result<WorkflowRuntimeTaskAttemptFactRecord, WorkflowRuntimeTaskAttemptFactDiagnostic>
    {
        let source_context = WorkflowRuntimeTaskAttemptSourceContext::new(
            WorkflowRuntimeTaskAttemptSourceContextRequest {
                workflow_id: self.workflow_id.clone(),
                workflow_run_id: self.workflow_run_id.clone(),
                scheduler_task_id: self.scheduler_task_id.clone(),
                task_attempt_generation: self.task_attempt_generation,
                timeout_ms: self.timeout_ms,
                runtime_source_context: self.runtime_source_context.clone(),
                selected_candidate_fact: self.selected_candidate_fact.clone(),
            },
        )?;
        WorkflowRuntimeTaskAttemptFactRecord::from_source_context(
            WorkflowRuntimeTaskAttemptFactBuildRequest {
                source_context,
                scheduler_task_attempt_id: self.scheduler_task_attempt_id.clone(),
                scheduler_task_attempt_started_at_ms: self.scheduler_task_attempt_started_at_ms,
                recorded_at_ms,
            },
        )
    }

    fn has_active_batch_claim(&self, now_ms: u64) -> bool {
        self.batch_claim
            .as_ref()
            .is_some_and(|claim| now_ms < claim.lease_expires_at_ms)
    }

    fn has_expired_batch_broker_wait_window(&self, now_ms: u64) -> bool {
        self.batch_broker_wait_window
            .as_ref()
            .is_some_and(|wait_window| now_ms >= wait_window.expires_at_ms)
    }
}

impl WorkflowRuntimeDispatchAssignmentState {
    fn is_terminal(self) -> bool {
        matches!(
            self,
            WorkflowRuntimeDispatchAssignmentState::Completed
                | WorkflowRuntimeDispatchAssignmentState::Cancelled
                | WorkflowRuntimeDispatchAssignmentState::Failed
        )
    }
}

impl WorkflowRuntimeDispatchAssignmentDiagnostic {
    fn new(
        code: WorkflowRuntimeDispatchAssignmentDiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn validate_assignment_request(
    request: &WorkflowRuntimeDispatchAssignmentRequest,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    validate_non_blank("session id", &request.session_id)?;
    validate_non_blank("workflow id", &request.workflow_id)?;
    validate_non_blank("workflow run id", &request.workflow_run_id)?;
    validate_non_blank("scheduler task id", &request.scheduler_task_id)?;
    validate_non_blank(
        "scheduler task attempt id",
        &request.scheduler_task_attempt_id,
    )?;
    if request.scheduler_task_attempt_started_at_ms == 0 {
        return invalid_assignment(
            "scheduler task attempt started timestamp must be greater than zero",
        );
    }
    if request.created_at_ms == 0 {
        return invalid_assignment(
            "runtime dispatch assignment created timestamp must be greater than zero",
        );
    }
    if request.timeout_ms == Some(0) {
        return invalid_assignment(
            "runtime dispatch assignment timeout must be greater than zero when present",
        );
    }
    validate_runtime_source_context(&request.runtime_source_context)?;
    if request.runtime_branch_claim.attempt_generation != request.task_attempt_generation {
        return invalid_assignment(
            "runtime branch claim generation must match task attempt generation",
        );
    }
    validate_candidate_correlation(request)?;
    validate_selected_runtime_handoff(request)?;
    Ok(())
}

fn validate_runtime_source_context(
    context: &WorkflowRuntimeSourceContext,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    validate_non_blank(
        "runtime source context operation type",
        &context.operation_type,
    )?;
    validate_non_blank(
        "runtime source context context shape key",
        &context.context_shape_key,
    )?;
    validate_non_blank(
        "runtime source context cancellation mode",
        &context.cancellation_mode,
    )?;
    Ok(())
}

fn validate_candidate_correlation(
    request: &WorkflowRuntimeDispatchAssignmentRequest,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if request
        .selected_candidate_fact
        .resource_fit_assessment
        .workflow_run_id
        .as_str()
        != request.workflow_run_id
    {
        return invalid_assignment(
            "selected candidate resource-fit workflow run does not match assignment",
        );
    }
    if request
        .selected_candidate_fact
        .resource_fit_assessment
        .task_id
        .as_str()
        != request.scheduler_task_id
    {
        return invalid_assignment(
            "selected candidate resource-fit task id does not match assignment",
        );
    }
    if request.selected_candidate_id.as_ref() != Some(&request.selected_candidate_fact.candidate_id)
    {
        return invalid_assignment("selected candidate id does not match selected candidate fact");
    }
    for reservation in &request.selected_candidate_fact.reservations {
        if reservation.workflow_run_id.as_str() != request.workflow_run_id {
            return invalid_assignment(
                "selected candidate reservation workflow run does not match assignment",
            );
        }
        if reservation.task_id.as_str() != request.scheduler_task_id {
            return invalid_assignment(
                "selected candidate reservation task id does not match assignment",
            );
        }
        if reservation.reservation_lease_id != request.reservation_lease_id {
            return invalid_assignment(
                "selected candidate reservation lease does not match assignment",
            );
        }
    }
    Ok(())
}

fn validate_selected_runtime_handoff(
    request: &WorkflowRuntimeDispatchAssignmentRequest,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    let _validated_handoff =
        ValidatedSchedulerRuntimeHandoff::try_from(request.selected_runtime_handoff.clone())
            .map_err(|error| {
                WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                    WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment,
                    format!("selected runtime handoff is invalid: {error}"),
                )
            })?;
    if request.selected_runtime_handoff.state != SchedulerRuntimeHandoffState::DispatchSelected {
        return invalid_assignment(
            "selected runtime handoff must carry a selected dispatch decision",
        );
    }
    if request.selected_runtime_handoff.workflow_run_id.as_str() != request.workflow_run_id {
        return invalid_assignment(
            "selected runtime handoff workflow run does not match assignment",
        );
    }
    if request.selected_runtime_handoff.task_id.as_str() != request.scheduler_task_id {
        return invalid_assignment("selected runtime handoff task id does not match assignment");
    }
    if request.selected_runtime_handoff.readiness_proof != request.readiness_proof {
        return invalid_assignment(
            "selected runtime handoff readiness proof does not match assignment",
        );
    }
    let dispatch_decision = request
        .selected_runtime_handoff
        .dispatch_decision
        .as_ref()
        .ok_or_else(|| {
            WorkflowRuntimeDispatchAssignmentDiagnostic::new(
                WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment,
                "selected runtime handoff is missing dispatch decision",
            )
        })?;
    if dispatch_decision.reservation_lease_id != request.reservation_lease_id {
        return invalid_assignment(
            "selected runtime handoff reservation lease does not match assignment",
        );
    }
    if dispatch_decision.workflow_run_id.as_str() != request.workflow_run_id {
        return invalid_assignment("dispatch decision workflow run does not match assignment");
    }
    if dispatch_decision.task_id.as_str() != request.scheduler_task_id {
        return invalid_assignment("dispatch decision task id does not match assignment");
    }
    if dispatch_decision.readiness_proof != request.readiness_proof {
        return invalid_assignment("dispatch decision readiness proof does not match assignment");
    }
    Ok(())
}

fn validate_non_blank(
    label: &'static str,
    value: &str,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if value.trim().is_empty() {
        return invalid_assignment(format!("{label} must be non-empty"));
    }
    Ok(())
}

fn invalid_assignment<T>(
    message: impl Into<String>,
) -> Result<T, WorkflowRuntimeDispatchAssignmentDiagnostic> {
    Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
        WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment,
        message,
    ))
}

fn validate_batch_claim_request(
    now_ms: u64,
    lease_duration_ms: u64,
    max_assignments: usize,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if now_ms == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
            "runtime dispatch assignment batch claim timestamp must be greater than zero",
        ));
    }
    if lease_duration_ms == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
            "runtime dispatch assignment batch claim lease duration must be greater than zero",
        ));
    }
    validate_batch_selection_size(max_assignments)
}

fn validate_batch_broker_request(
    request: &WorkflowRuntimeDispatchAssignmentBatchBrokerRequest,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if request.now_ms == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
            "runtime dispatch assignment batch broker timestamp must be greater than zero",
        ));
    }
    if request.min_assignments == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
            "runtime dispatch assignment batch broker minimum size must be greater than zero",
        ));
    }
    validate_batch_selection_size(request.max_assignments)?;
    if request.min_assignments > request.max_assignments {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
            "runtime dispatch assignment batch broker minimum size cannot exceed maximum size",
        ));
    }
    Ok(())
}

fn validate_batch_broker_wait_request(
    now_ms: u64,
    wait_window_duration_ms: u64,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    validate_batch_broker_wait_transition_timestamp(now_ms)?;
    if wait_window_duration_ms == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow,
            "runtime dispatch assignment batch broker wait duration must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_batch_broker_wait_transition_timestamp(
    now_ms: u64,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if now_ms == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow,
            "runtime dispatch assignment batch broker wait timestamp must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_batch_selection_size(
    max_assignments: usize,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if max_assignments == 0 {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim,
            "runtime dispatch assignment batch claim size must be greater than zero",
        ));
    }
    Ok(())
}

fn ensure_batch_claimable(
    record: &WorkflowRuntimeDispatchAssignmentRecord,
    now_ms: u64,
) -> Result<(), WorkflowRuntimeDispatchAssignmentDiagnostic> {
    if record.state != WorkflowRuntimeDispatchAssignmentState::Running {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotRunning,
            "runtime dispatch assignment batch anchor must be running",
        ));
    }
    if record.has_active_batch_claim(now_ms) {
        return Err(WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::AlreadyBatchClaimed,
            "runtime dispatch assignment batch anchor already has an active batch claim",
        ));
    }
    let _fact = task_attempt_fact(record)?;
    Ok(())
}

fn task_attempt_fact(
    record: &WorkflowRuntimeDispatchAssignmentRecord,
) -> Result<&WorkflowRuntimeTaskAttemptFactRecord, WorkflowRuntimeDispatchAssignmentDiagnostic> {
    record.task_attempt_fact.as_ref().ok_or_else(|| {
        WorkflowRuntimeDispatchAssignmentDiagnostic::new(
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::MissingTaskAttemptFact,
            "runtime dispatch assignment is missing task-attempt facts required for batch claiming",
        )
    })
}

fn is_batch_compatibility_mismatch(
    code: WorkflowRuntimeBranchBatchEligibilityDiagnosticCode,
) -> bool {
    matches!(
        code,
        WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ModelArtifactMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::RuntimeFamilyMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::BackendMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::RuntimeResidencyMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::MemoryEstimateMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ContextShapeMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::OperationTypeMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::CancellationModeMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::TimeoutMismatch
            | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMismatch
    )
}

fn batch_compatibility_error(
    diagnostic: WorkflowRuntimeBranchBatchEligibilityDiagnostic,
) -> WorkflowRuntimeDispatchAssignmentDiagnostic {
    let code = match diagnostic.code {
        WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::MissingTaskAttemptFact
        | WorkflowRuntimeBranchBatchEligibilityDiagnosticCode::ReservationProfileMissing => {
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::MissingTaskAttemptFact
        }
        _ => WorkflowRuntimeDispatchAssignmentDiagnosticCode::BatchCompatibilityRejected,
    };
    WorkflowRuntimeDispatchAssignmentDiagnostic::new(
        code,
        format!(
            "runtime dispatch assignment batch compatibility failed: {}",
            diagnostic.message
        ),
    )
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{
        DependencyEnvironmentId, DependencyEnvironmentRef, DependencyReadinessProofId,
        DependencyReadinessWorkflowRunId, DependencyTaskId, DeviceIntentId, PumasModelRef,
    };
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerDispatchDecision, SchedulerDispatchDiagnostic,
        SchedulerDispatchDiagnosticCode, SchedulerDispatchDiagnosticSeverity, SchedulerNodeId,
        SchedulerResourceFitAssessment, SchedulerResourceFitState, SchedulerResourceKind,
        SchedulerResourceReservation, SchedulerRuntimeDeviceConstraints, SchedulerRuntimeVariantId,
        SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION, SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
        SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
    };

    use super::super::runtime_branch_task_event::{
        WorkflowRuntimeBranchTaskEventClaimLeaseId, WorkflowRuntimeBranchTaskEventClaimOwnerId,
    };
    use super::super::runtime_dispatch_selection::WorkflowRuntimeDispatchLoadState;
    use super::*;

    #[test]
    fn runtime_dispatch_assignment_repository_creates_prepared_assignment() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let request = assignment_request("assignment.1");

        let record = repository.create(request.clone()).expect("assignment");

        assert_eq!(
            record.schema_version,
            WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_SCHEMA_VERSION
        );
        assert_eq!(record.assignment_id, request.assignment_id);
        assert_eq!(
            record.state,
            WorkflowRuntimeDispatchAssignmentState::Prepared
        );
        assert_eq!(record.timeout_ms, Some(30_000));
        assert!(record.task_attempt_fact.is_none());
        assert_eq!(
            record.runtime_source_context.context_shape_key,
            "txt2img.1024x1024.steps30"
        );
        assert_eq!(record.updated_at_ms, request.created_at_ms);
        assert_eq!(
            repository
                .get(&record.assignment_id)
                .expect("stored assignment")
                .selected_candidate_fact,
            request.selected_candidate_fact
        );
    }

    #[test]
    fn runtime_dispatch_assignment_mark_running_records_task_attempt_fact() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let record = repository
            .create(assignment_request("assignment.1"))
            .expect("assignment");

        let running = repository
            .mark_running(&record.assignment_id, 130)
            .expect("running assignment");

        assert_eq!(
            running.state,
            WorkflowRuntimeDispatchAssignmentState::Running
        );
        let fact = running
            .task_attempt_fact
            .as_ref()
            .expect("task attempt fact");
        assert_eq!(fact.scheduler_task_attempt_id, "attempt.image.1");
        assert_eq!(fact.recorded_at_ms, 130);
        assert_eq!(fact.reservations.len(), 1);
        assert_eq!(fact.reservations[0].device_id, "cuda:0");
        assert_eq!(
            repository
                .get(&record.assignment_id)
                .expect("stored assignment")
                .task_attempt_fact
                .as_ref()
                .expect("stored fact")
                .recorded_at_ms,
            130
        );
    }

    #[test]
    fn runtime_dispatch_assignment_multi_run_fixture_builder_produces_consistent_readiness_facts() {
        let first_member = DispatchAssignmentFixtureMember::first();
        let second_member = DispatchAssignmentFixtureMember::second();

        let first = assignment_request_for_member(&first_member);
        let second = assignment_request_for_member(&second_member);

        assert_ne!(first.workflow_run_id, second.workflow_run_id);
        assert_member_facts_match(&first, &first_member);
        assert_member_facts_match(&second, &second_member);
        first.readiness_proof.validate().expect("first proof");
        second.readiness_proof.validate().expect("second proof");
        let _first_handoff =
            ValidatedSchedulerRuntimeHandoff::try_from(first.selected_runtime_handoff.clone())
                .expect("first handoff");
        let _second_handoff =
            ValidatedSchedulerRuntimeHandoff::try_from(second.selected_runtime_handoff.clone())
                .expect("second handoff");

        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first_record = repository.create(first).expect("first assignment");
        let second_record = repository.create(second).expect("second assignment");

        assert_ne!(first_record.workflow_run_id, second_record.workflow_run_id);
        assert!(first_record.task_attempt_fact.is_none());
        assert!(second_record.task_attempt_fact.is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_repository_claims_compatible_cross_run_batch() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let second_member = DispatchAssignmentFixtureMember::second();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let second = repository
            .create(assignment_request_for_member(&second_member))
            .expect("second assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let second = repository
            .mark_running(&second.assignment_id, 131)
            .expect("second running");
        assert_ne!(first.workflow_run_id, second.workflow_run_id);

        let outcome = repository
            .claim_compatible_running_batch(&first.assignment_id, batch_owner_id(), 140, 1_000, 8)
            .expect("batch claim");

        assert_eq!(outcome.assignments.len(), 2);
        assert_eq!(
            outcome.batch_claim.anchor_assignment_id,
            first.assignment_id
        );
        assert_eq!(outcome.batch_claim.claimed_at_ms, 140);
        assert_eq!(outcome.batch_claim.lease_expires_at_ms, 1_140);
        assert!(
            !outcome
                .batch_claim
                .batch_claim_id
                .as_str()
                .trim()
                .is_empty(),
            "batch claim id must be stable"
        );
        assert_eq!(
            outcome.batch_claim.owner_id.as_str(),
            "workflow-service.batch-claimer"
        );
        assert_eq!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .batch_claim
                .as_ref(),
            Some(&outcome.batch_claim)
        );
        assert_eq!(
            repository
                .get(&second.assignment_id)
                .expect("stored second")
                .batch_claim
                .as_ref(),
            Some(&outcome.batch_claim)
        );
        assert_eq!(
            outcome
                .assignments
                .iter()
                .map(|record| record.assignment_id.as_str())
                .collect::<Vec<_>>(),
            vec![first.assignment_id.as_str(), second.assignment_id.as_str()]
        );
        assert_eq!(
            outcome
                .assignments
                .iter()
                .map(|record| record.workflow_run_id.as_str())
                .collect::<Vec<_>>(),
            vec!["run.2026-05-22.001", second_member.workflow_run_id]
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_ready_to_claim_is_non_mutating() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let second_member = DispatchAssignmentFixtureMember::second();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let second = repository
            .create(assignment_request_for_member(&second_member))
            .expect("second assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let second = repository
            .mark_running(&second.assignment_id, 131)
            .expect("second running");

        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::ReadyToClaim { assignments } =
            decision
        else {
            panic!("expected ready-to-claim decision");
        };
        assert_eq!(
            assignments
                .iter()
                .map(|record| record.assignment_id.as_str())
                .collect::<Vec<_>>(),
            vec![first.assignment_id.as_str(), second.assignment_id.as_str()]
        );
        assert!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .batch_claim
                .is_none(),
            "broker readiness must not claim the anchor"
        );
        assert!(
            repository
                .get(&second.assignment_id)
                .expect("stored second")
                .batch_claim
                .is_none(),
            "broker readiness must not claim compatible peers"
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_waits_without_claiming_anchor() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");

        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitingForPeers {
            anchor_assignment,
            compatible_assignments,
            required_assignments,
        } = decision
        else {
            panic!("expected waiting-for-peers decision");
        };
        assert_eq!(anchor_assignment.assignment_id, first.assignment_id);
        assert!(compatible_assignments.is_empty());
        assert_eq!(required_assignments, 2);
        assert!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .batch_claim
                .is_none(),
            "waiting decision must not claim one-member batches"
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_skips_incompatible_peers_without_mutating() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let anchor = repository
            .create(assignment_request("assignment.1"))
            .expect("anchor assignment");
        let second_member = DispatchAssignmentFixtureMember::second();
        let mut incompatible_request = assignment_request_for_member(&second_member);
        incompatible_request
            .runtime_source_context
            .context_shape_key = "txt2img.512x512.steps20".to_string();
        let incompatible = repository
            .create(incompatible_request)
            .expect("incompatible assignment");
        let anchor = repository
            .mark_running(&anchor.assignment_id, 130)
            .expect("anchor running");
        let incompatible = repository
            .mark_running(&incompatible.assignment_id, 131)
            .expect("incompatible running");

        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                anchor.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitingForPeers {
            compatible_assignments,
            ..
        } = decision
        else {
            panic!("expected waiting-for-peers decision");
        };
        assert!(
            compatible_assignments.is_empty(),
            "incompatible assignments must not satisfy broker readiness"
        );
        assert!(repository
            .get(&anchor.assignment_id)
            .expect("stored anchor")
            .batch_claim
            .is_none());
        assert!(repository
            .get(&incompatible.assignment_id)
            .expect("stored incompatible")
            .batch_claim
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_rejects_missing_peer_facts_without_mutating() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let anchor = repository
            .create(assignment_request("assignment.1"))
            .expect("anchor assignment");
        let second_member = DispatchAssignmentFixtureMember::second();
        let candidate = repository
            .create(assignment_request_for_member(&second_member))
            .expect("candidate assignment");
        let anchor = repository
            .mark_running(&anchor.assignment_id, 130)
            .expect("anchor running");
        let candidate = repository
            .mark_running(&candidate.assignment_id, 131)
            .expect("candidate running");
        repository
            .records
            .get_mut(&candidate.assignment_id)
            .expect("stored candidate")
            .task_attempt_fact = None;

        let error = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                anchor.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect_err("missing candidate facts must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::MissingTaskAttemptFact
        );
        assert!(repository
            .get(&anchor.assignment_id)
            .expect("stored anchor")
            .batch_claim
            .is_none());
        assert!(repository
            .get(&candidate.assignment_id)
            .expect("stored candidate")
            .batch_claim
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_claims_ready_decision() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let second_member = DispatchAssignmentFixtureMember::second();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let second = repository
            .create(assignment_request_for_member(&second_member))
            .expect("second assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let second = repository
            .mark_running(&second.assignment_id, 131)
            .expect("second running");
        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let outcome = repository
            .claim_batch_broker_decision(batch_broker_claim_request(decision, 150, 1_000))
            .expect("broker claim");

        assert_eq!(outcome.assignments.len(), 2);
        assert_eq!(
            outcome
                .assignments
                .iter()
                .map(|record| record.assignment_id.as_str())
                .collect::<Vec<_>>(),
            vec![first.assignment_id.as_str(), second.assignment_id.as_str()]
        );
        assert_eq!(
            outcome.batch_claim.anchor_assignment_id,
            first.assignment_id
        );
        assert_eq!(outcome.batch_claim.claimed_at_ms, 150);
        assert_eq!(outcome.batch_claim.lease_expires_at_ms, 1_150);
        assert_eq!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .batch_claim
                .as_ref(),
            Some(&outcome.batch_claim)
        );
        assert_eq!(
            repository
                .get(&second.assignment_id)
                .expect("stored second")
                .batch_claim
                .as_ref(),
            Some(&outcome.batch_claim)
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_rejects_waiting_claim_without_mutating() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let error = repository
            .claim_batch_broker_decision(batch_broker_claim_request(decision, 150, 1_000))
            .expect_err("waiting decision must not claim");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim
        );
        assert!(repository
            .get(&first.assignment_id)
            .expect("stored first")
            .batch_claim
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_records_wait_window_without_claiming_anchor() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let waiting = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(decision, 145, 500))
            .expect("record wait window");

        assert_eq!(waiting.assignment_id, first.assignment_id);
        assert_eq!(
            waiting.state,
            WorkflowRuntimeDispatchAssignmentState::Running
        );
        assert!(waiting.batch_claim.is_none());
        let wait_window = waiting
            .batch_broker_wait_window
            .as_ref()
            .expect("wait window");
        assert_eq!(wait_window.waiting_since_ms, 145);
        assert_eq!(wait_window.expires_at_ms, 645);
        assert_eq!(wait_window.required_assignments, 2);
        assert_eq!(
            wait_window.expiry_diagnostic.code,
            WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode::BatchWindowExpired
        );
        assert_eq!(
            wait_window.expiry_diagnostic.message,
            "runtime dispatch assignment batch broker wait window expired before reaching 2 compatible assignments"
        );
        assert_eq!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .batch_broker_wait_window
                .as_ref(),
            Some(wait_window)
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_repeated_wait_does_not_reset_window() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let first_decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("first broker decision");
        let _ = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(
                first_decision,
                145,
                500,
            ))
            .expect("record initial wait window");
        let second_decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                200,
                2,
                8,
            ))
            .expect("second broker decision");

        let waiting = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(
                second_decision,
                205,
                1_000,
            ))
            .expect("record repeated wait window");

        let wait_window = waiting
            .batch_broker_wait_window
            .as_ref()
            .expect("wait window");
        assert_eq!(wait_window.waiting_since_ms, 145);
        assert_eq!(wait_window.expires_at_ms, 645);
        assert_eq!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .updated_at_ms,
            145,
            "repeated waiting decisions must not extend the durable wait window"
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_expired_wait_wins_over_late_peer() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let first_decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("first broker decision");
        let _ = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(
                first_decision,
                145,
                500,
            ))
            .expect("record wait window");
        let second_member = DispatchAssignmentFixtureMember::second();
        let second = repository
            .create(assignment_request_for_member(&second_member))
            .expect("second assignment");
        let second = repository
            .mark_running(&second.assignment_id, 200)
            .expect("second running");

        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                645,
                2,
                8,
            ))
            .expect("expired broker decision");

        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitWindowExpired {
            anchor_assignment,
            expiry_diagnostic,
        } = decision
        else {
            panic!("expected expired wait-window decision");
        };
        assert_eq!(anchor_assignment.assignment_id, first.assignment_id);
        assert_eq!(
            expiry_diagnostic.code,
            WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode::BatchWindowExpired
        );
        assert!(repository
            .get(&first.assignment_id)
            .expect("stored first")
            .batch_claim
            .is_none());
        assert!(repository
            .get(&second.assignment_id)
            .expect("stored second")
            .batch_claim
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_skips_expired_wait_as_late_peer_candidate() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let first_decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("first broker decision");
        let _ = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(
                first_decision,
                145,
                500,
            ))
            .expect("record wait window");
        let second_member = DispatchAssignmentFixtureMember::second();
        let second = repository
            .create(assignment_request_for_member(&second_member))
            .expect("second assignment");
        let second = repository
            .mark_running(&second.assignment_id, 200)
            .expect("second running");

        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                second.assignment_id.clone(),
                645,
                2,
                8,
            ))
            .expect("late peer broker decision");

        let WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitingForPeers {
            anchor_assignment,
            compatible_assignments,
            ..
        } = decision
        else {
            panic!("expected second assignment to wait instead of claiming expired peer");
        };
        assert_eq!(anchor_assignment.assignment_id, second.assignment_id);
        assert!(
            compatible_assignments.is_empty(),
            "expired waiters must not satisfy late peer broker readiness"
        );
        assert!(repository
            .get(&first.assignment_id)
            .expect("stored first")
            .batch_claim
            .is_none());
        assert!(repository
            .get(&second.assignment_id)
            .expect("stored second")
            .batch_claim
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_marks_expired_wait_failed() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");
        let _ = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(decision, 145, 500))
            .expect("record wait window");

        let expired = repository
            .mark_batch_broker_wait_window_expired(&first.assignment_id, 645)
            .expect("mark expired");

        assert_eq!(
            expired.state,
            WorkflowRuntimeDispatchAssignmentState::Failed
        );
        assert_eq!(expired.updated_at_ms, 645);
        assert!(
            expired.batch_broker_wait_window.is_some(),
            "expired assignment retains wait-window facts for diagnostics"
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_rejects_unexpired_wait_transition() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");
        let _ = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(decision, 145, 500))
            .expect("record wait window");

        let error = repository
            .mark_batch_broker_wait_window_expired(&first.assignment_id, 644)
            .expect_err("unexpired wait must be rejected");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow
        );
        assert_eq!(
            repository
                .get(&first.assignment_id)
                .expect("stored first")
                .state,
            WorkflowRuntimeDispatchAssignmentState::Running
        );
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_rejects_ready_wait_window_without_mutating() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let second_member = DispatchAssignmentFixtureMember::second();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let second = repository
            .create(assignment_request_for_member(&second_member))
            .expect("second assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let _second = repository
            .mark_running(&second.assignment_id, 131)
            .expect("second running");
        let decision = repository
            .evaluate_running_batch_broker_decision(batch_broker_request(
                first.assignment_id.clone(),
                140,
                2,
                8,
            ))
            .expect("broker decision");

        let error = repository
            .record_batch_broker_waiting_decision(batch_broker_wait_request(decision, 145, 500))
            .expect_err("ready decision must not record wait window");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow
        );
        assert!(repository
            .get(&first.assignment_id)
            .expect("stored first")
            .batch_broker_wait_window
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_batch_broker_rejects_one_member_ready_claim_without_mutating() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("first assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running");
        let decision = WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::ReadyToClaim {
            assignments: vec![first.clone()],
        };

        let error = repository
            .claim_batch_broker_decision(batch_broker_claim_request(decision, 150, 1_000))
            .expect_err("one-member ready decision must not claim");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim
        );
        assert!(repository
            .get(&first.assignment_id)
            .expect("stored first")
            .batch_claim
            .is_none());
    }

    #[test]
    fn runtime_dispatch_assignment_repository_skips_incompatible_batch_candidates() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let anchor = repository
            .create(assignment_request("assignment.1"))
            .expect("anchor assignment");
        let second_member = DispatchAssignmentFixtureMember::second();
        let mut incompatible_request = assignment_request_for_member(&second_member);
        incompatible_request
            .runtime_source_context
            .context_shape_key = "txt2img.512x512.steps20".to_string();
        let incompatible = repository
            .create(incompatible_request)
            .expect("incompatible assignment");
        let anchor = repository
            .mark_running(&anchor.assignment_id, 130)
            .expect("anchor running");
        let incompatible = repository
            .mark_running(&incompatible.assignment_id, 131)
            .expect("incompatible running");
        assert_ne!(anchor.workflow_run_id, incompatible.workflow_run_id);

        let outcome = repository
            .claim_compatible_running_batch(&anchor.assignment_id, batch_owner_id(), 140, 1_000, 8)
            .expect("batch claim");

        assert_eq!(outcome.assignments.len(), 1);
        assert_eq!(outcome.assignments[0].assignment_id, anchor.assignment_id);
        assert!(
            repository
                .get(&incompatible.assignment_id)
                .expect("stored incompatible")
                .batch_claim
                .is_none(),
            "incompatible assignment must remain unclaimed"
        );
    }

    #[test]
    fn runtime_dispatch_assignment_repository_rejects_batch_claim_without_task_attempt_facts() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let record = repository
            .create(assignment_request("assignment.1"))
            .expect("assignment");
        let running = repository
            .mark_running(&record.assignment_id, 130)
            .expect("running assignment");
        repository
            .records
            .get_mut(&running.assignment_id)
            .expect("stored assignment")
            .task_attempt_fact = None;

        let error = repository
            .claim_compatible_running_batch(&running.assignment_id, batch_owner_id(), 140, 1_000, 8)
            .expect_err("missing task-attempt facts must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::MissingTaskAttemptFact
        );
    }

    #[test]
    fn runtime_dispatch_assignment_repository_rejects_cross_run_batch_candidate_without_task_attempt_facts(
    ) {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let anchor = repository
            .create(assignment_request("assignment.1"))
            .expect("anchor assignment");
        let second_member = DispatchAssignmentFixtureMember::second();
        let candidate = repository
            .create(assignment_request_for_member(&second_member))
            .expect("candidate assignment");
        let anchor = repository
            .mark_running(&anchor.assignment_id, 130)
            .expect("anchor running");
        let candidate = repository
            .mark_running(&candidate.assignment_id, 131)
            .expect("candidate running");
        repository
            .records
            .get_mut(&candidate.assignment_id)
            .expect("stored candidate")
            .task_attempt_fact = None;

        let error = repository
            .claim_compatible_running_batch(&anchor.assignment_id, batch_owner_id(), 140, 1_000, 8)
            .expect_err("missing candidate task-attempt facts must fail closed");

        assert_ne!(anchor.workflow_run_id, candidate.workflow_run_id);
        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::MissingTaskAttemptFact
        );
        assert!(
            repository
                .get(&anchor.assignment_id)
                .expect("stored anchor")
                .batch_claim
                .is_none(),
            "anchor must not be claimed when a cross-run candidate is missing facts"
        );
        assert!(
            repository
                .get(&candidate.assignment_id)
                .expect("stored candidate")
                .batch_claim
                .is_none(),
            "candidate must not be claimed when facts are missing"
        );
    }

    #[test]
    fn runtime_dispatch_assignment_repository_rejects_active_batch_claim_reentry() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let record = repository
            .create(assignment_request("assignment.1"))
            .expect("assignment");
        let running = repository
            .mark_running(&record.assignment_id, 130)
            .expect("running assignment");
        let _first_claim = repository
            .claim_compatible_running_batch(&running.assignment_id, batch_owner_id(), 140, 1_000, 8)
            .expect("first batch claim");

        let error = repository
            .claim_compatible_running_batch(&running.assignment_id, batch_owner_id(), 141, 1_000, 8)
            .expect_err("active batch claim must block reentry");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::AlreadyBatchClaimed
        );
    }

    #[test]
    fn runtime_dispatch_assignment_derives_task_attempt_fact_from_assignment_owned_evidence() {
        let record = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new()
            .create(assignment_request("assignment.1"))
            .expect("assignment");

        let fact = record
            .task_attempt_fact_record(130)
            .expect("task attempt fact");

        assert_eq!(fact.workflow_id, "workflow.image_generation");
        assert_eq!(fact.workflow_run_id, "run.2026-05-22.001");
        assert_eq!(fact.scheduler_task_id, "task.image_generation.001");
        assert_eq!(fact.scheduler_task_attempt_id, "attempt.image.1");
        assert_eq!(fact.task_attempt_generation, 1);
        assert_eq!(fact.selected_model_id, "pumas://models/juggernaut-xl-v10");
        assert_eq!(fact.selected_runtime_id, "diffusers-pytorch");
        assert_eq!(fact.selected_runtime_variant_id.as_deref(), Some("cuda"));
        assert_eq!(fact.backend_id, "backend.diffusers");
        assert_eq!(fact.runtime_family, "diffusers");
        assert_eq!(fact.load_target, "cuda:0");
        assert_eq!(
            fact.runtime_residency_key,
            "runtime.diffusers.model.sdxl.cuda0"
        );
        assert_eq!(fact.timeout_ms, Some(30_000));
        assert_eq!(fact.operation_type, "image-generation.txt2img");
        assert_eq!(fact.context_shape_key, "txt2img.1024x1024.steps30");
        assert_eq!(fact.cancellation_mode, "per-run-fanout");
        assert_eq!(fact.reservations.len(), 1);
        assert_eq!(
            fact.reservations[0].reservation_lease_id,
            "reservation-lease.runtime.1"
        );
        assert_eq!(fact.reservations[0].device_id, "cuda:0");
        assert_eq!(fact.recorded_at_ms, 130);
    }

    #[test]
    fn runtime_dispatch_assignment_repository_rejects_duplicate_active_assignment() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let _first = repository
            .create(assignment_request("assignment.1"))
            .expect("assignment");
        let error = repository
            .create(assignment_request("assignment.2"))
            .expect_err("duplicate active assignment");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::DuplicateActiveAssignment
        );
    }

    #[test]
    fn runtime_dispatch_assignment_repository_allows_new_assignment_after_terminal() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request("assignment.1"))
            .expect("assignment");
        let _failed = repository
            .mark_failed(&first.assignment_id, 200)
            .expect("fail assignment");

        let second = repository
            .create(assignment_request("assignment.2"))
            .expect("new assignment");

        assert_eq!(
            second.state,
            WorkflowRuntimeDispatchAssignmentState::Prepared
        );
    }

    #[test]
    fn runtime_dispatch_assignment_repository_enforces_lifecycle_order() {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let record = repository
            .create(assignment_request("assignment.1"))
            .expect("assignment");

        let error = repository
            .mark_completed(&record.assignment_id, 200)
            .expect_err("cannot complete before running");
        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidTransition
        );

        let running = repository
            .mark_running(&record.assignment_id, 210)
            .expect("running");
        assert_eq!(
            running.state,
            WorkflowRuntimeDispatchAssignmentState::Running
        );
        let completed = repository
            .mark_completed(&record.assignment_id, 220)
            .expect("completed");
        assert_eq!(
            completed.state,
            WorkflowRuntimeDispatchAssignmentState::Completed
        );
    }

    #[test]
    fn runtime_dispatch_assignment_rejects_mismatched_selected_candidate() {
        let mut request = assignment_request("assignment.1");
        request.selected_candidate_id =
            Some(SchedulerDispatchCandidateId::parse("candidate.other").expect("candidate id"));

        let error = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new()
            .create(request)
            .expect_err("candidate mismatch");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment
        );
        assert!(error.message.contains("selected candidate id"));
    }

    #[test]
    fn runtime_dispatch_assignment_rejects_mismatched_handoff() {
        let mut request = assignment_request("assignment.1");
        request.selected_runtime_handoff.workflow_run_id =
            SchedulerWorkflowRunId::parse("run.other").expect("run id");

        let error = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new()
            .create(request)
            .expect_err("handoff mismatch");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment
        );
        assert!(error.message.contains("selected runtime handoff"));
    }

    #[test]
    fn runtime_dispatch_assignment_rejects_missing_source_context() {
        let mut request = assignment_request("assignment.1");
        request.runtime_source_context.operation_type = " ".to_string();

        let error = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new()
            .create(request)
            .expect_err("source context is required");

        assert_eq!(
            error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment
        );
        assert!(
            error.message.contains("runtime source context"),
            "unexpected error message: {}",
            error.message
        );
    }

    #[derive(Debug, Clone, Copy)]
    struct DispatchAssignmentFixtureMember<'a> {
        assignment_id: &'a str,
        runtime_branch_event_id: &'a str,
        workflow_run_id: &'a str,
        scheduler_task_attempt_id: &'a str,
        reservation_lease_id: &'a str,
    }

    impl DispatchAssignmentFixtureMember<'_> {
        fn first() -> Self {
            Self {
                assignment_id: "assignment.1",
                runtime_branch_event_id:
                    "runtime-branch-task-event.run.2026-05-22.001.task.image_generation.001",
                workflow_run_id: "run.2026-05-22.001",
                scheduler_task_attempt_id: "attempt.image.1",
                reservation_lease_id: "reservation-lease.runtime.1",
            }
        }

        fn second() -> Self {
            Self {
                assignment_id: "assignment.2",
                runtime_branch_event_id:
                    "runtime-branch-task-event.run.2026-05-22.002.task.image_generation.001",
                workflow_run_id: "run.2026-05-22.002",
                scheduler_task_attempt_id: "attempt.image.2",
                reservation_lease_id: "reservation-lease.runtime.2",
            }
        }
    }

    fn assignment_request(assignment_id: &str) -> WorkflowRuntimeDispatchAssignmentRequest {
        let member = DispatchAssignmentFixtureMember {
            assignment_id,
            ..DispatchAssignmentFixtureMember::first()
        };
        assignment_request_for_member(&member)
    }

    fn assignment_request_for_member(
        member: &DispatchAssignmentFixtureMember<'_>,
    ) -> WorkflowRuntimeDispatchAssignmentRequest {
        let readiness_proof = readiness_proof_for_member(member);
        let selected_candidate_fact = selected_candidate_fact_for_member(member);
        let reservation_lease_id = selected_candidate_fact.reservations[0]
            .reservation_lease_id
            .clone();
        let selected_candidate_id = Some(selected_candidate_fact.candidate_id.clone());
        WorkflowRuntimeDispatchAssignmentRequest {
            assignment_id: WorkflowRuntimeDispatchAssignmentId::parse(member.assignment_id)
                .expect("assignment id"),
            runtime_branch_event_id: WorkflowRuntimeBranchTaskEventId::parse(
                member.runtime_branch_event_id,
            )
            .expect("event id"),
            session_id: "session.image.1".to_string(),
            workflow_id: "workflow.image_generation".to_string(),
            workflow_run_id: member.workflow_run_id.to_string(),
            scheduler_task_id: "task.image_generation.001".to_string(),
            scheduler_task_attempt_id: member.scheduler_task_attempt_id.to_string(),
            scheduler_task_attempt_started_at_ms: 100,
            task_attempt_generation: 1,
            timeout_ms: Some(30_000),
            runtime_source_context: runtime_source_context(),
            runtime_branch_claim: runtime_branch_claim(),
            selected_runtime_handoff: selected_runtime_handoff_for_member(
                member,
                readiness_proof.clone(),
                reservation_lease_id.clone(),
            ),
            readiness_proof,
            selected_candidate_fact,
            reservation_lease_id,
            selected_candidate_id,
            created_at_ms: 120,
        }
    }

    fn assignment_request_for_run(
        assignment_id: &str,
        runtime_branch_event_id: &str,
        workflow_run_id: &str,
        scheduler_task_attempt_id: &str,
        reservation_lease_id: &str,
    ) -> WorkflowRuntimeDispatchAssignmentRequest {
        assignment_request_for_member(&DispatchAssignmentFixtureMember {
            assignment_id,
            runtime_branch_event_id,
            workflow_run_id,
            scheduler_task_attempt_id,
            reservation_lease_id,
        })
    }

    fn batch_owner_id() -> WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId {
        WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId::parse("workflow-service.batch-claimer")
            .expect("batch owner id")
    }

    fn batch_broker_request(
        anchor_assignment_id: WorkflowRuntimeDispatchAssignmentId,
        now_ms: u64,
        min_assignments: usize,
        max_assignments: usize,
    ) -> WorkflowRuntimeDispatchAssignmentBatchBrokerRequest {
        WorkflowRuntimeDispatchAssignmentBatchBrokerRequest {
            anchor_assignment_id,
            now_ms,
            min_assignments,
            max_assignments,
        }
    }

    fn batch_broker_claim_request(
        decision: WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
        now_ms: u64,
        lease_duration_ms: u64,
    ) -> WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest {
        WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest {
            decision,
            owner_id: batch_owner_id(),
            now_ms,
            lease_duration_ms,
        }
    }

    fn batch_broker_wait_request(
        decision: WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
        now_ms: u64,
        wait_window_duration_ms: u64,
    ) -> WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest {
        WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest {
            decision,
            now_ms,
            wait_window_duration_ms,
        }
    }

    fn runtime_source_context() -> WorkflowRuntimeSourceContext {
        WorkflowRuntimeSourceContext {
            operation_type: "image-generation.txt2img".to_string(),
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
        }
    }

    fn runtime_branch_claim() -> WorkflowRuntimeBranchTaskEventClaim {
        WorkflowRuntimeBranchTaskEventClaim {
            owner_id: WorkflowRuntimeBranchTaskEventClaimOwnerId::parse(
                "workflow-service.task-execution-worker",
            )
            .expect("owner id"),
            lease_id: WorkflowRuntimeBranchTaskEventClaimLeaseId::parse("claim.1")
                .expect("lease id"),
            attempt_generation: 1,
            claimed_at_ms: 90,
            lease_expires_at_ms: 30_090,
        }
    }

    fn selected_runtime_handoff_for_member(
        member: &DispatchAssignmentFixtureMember<'_>,
        readiness_proof: DependencyReadinessProofEnvelope,
        reservation_lease_id: SchedulerReservationLeaseId,
    ) -> SchedulerRuntimeHandoff {
        let intent = task_intent_for_member(member);
        let environment_ref = environment_ref();
        let workflow_run_id =
            SchedulerWorkflowRunId::parse(member.workflow_run_id).expect("run id");
        SchedulerRuntimeHandoff {
            contract_version: SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: workflow_run_id.clone(),
            node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
            task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
            task_intent: intent.clone(),
            state: SchedulerRuntimeHandoffState::DispatchSelected,
            readiness_proof: readiness_proof.clone(),
            environment_ref: environment_ref.clone(),
            dispatch_decision: Some(SchedulerDispatchDecision {
                contract_version: SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
                workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                    .expect("workflow id"),
                workflow_run_id,
                node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
                task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
                task_intent: intent,
                selected_runtime_id: "diffusers-pytorch".parse().expect("runtime id"),
                selected_runtime_variant_id: Some(
                    SchedulerRuntimeVariantId::parse("cuda").expect("variant id"),
                ),
                selected_device_ids: vec![DeviceIntentId::parse("cuda:0").expect("device id")],
                selected_model_ref: model_ref(),
                readiness_proof,
                environment_ref,
                batching_group_id: None,
                reservation_lease_id,
                reservations: selected_candidate_fact_for_member(member).reservations,
                runtime_trait_settings: Vec::new(),
                diagnostics: vec![SchedulerDispatchDiagnostic {
                    severity: SchedulerDispatchDiagnosticSeverity::Info,
                    code: SchedulerDispatchDiagnosticCode::RuntimeSelected,
                    message: "runtime selected".to_string(),
                    hint: None,
                }],
            }),
            diagnostics: Vec::new(),
        }
    }

    fn task_intent_for_member(
        member: &DispatchAssignmentFixtureMember<'_>,
    ) -> SchedulableTaskIntent {
        SchedulableTaskIntent {
            contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(member.workflow_run_id)
                .expect("workflow run id"),
            node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
            task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
            fairness_key: None,
            task_type: DependencyTaskId::parse("image_generation").expect("task type"),
            model_ref: model_ref(),
            constraints: SchedulerRuntimeDeviceConstraints {
                requested_runtime_id: Some("diffusers-pytorch".parse().expect("runtime id")),
                requested_device_id: Some(DeviceIntentId::parse("cuda:0").expect("device id")),
            },
            trait_settings: Vec::new(),
            dependency_override_patches: Vec::new(),
            estimate_hints: Vec::new(),
        }
    }

    fn selected_candidate_fact_for_member(
        member: &DispatchAssignmentFixtureMember<'_>,
    ) -> WorkflowRuntimeDispatchCandidateFact {
        let workflow_run_id: SchedulerWorkflowRunId =
            member.workflow_run_id.parse().expect("run id");
        let task_id: SchedulerTaskId = "task.image_generation.001".parse().expect("task id");
        let device_id: DeviceIntentId = "cuda:0".parse().expect("device id");
        WorkflowRuntimeDispatchCandidateFact {
            candidate_id: SchedulerDispatchCandidateId::parse("candidate.diffusers.cuda0")
                .expect("candidate id"),
            selected_runtime_id: "diffusers-pytorch".parse().expect("runtime id"),
            selected_runtime_variant_id: Some(
                SchedulerRuntimeVariantId::parse("cuda").expect("variant id"),
            ),
            selected_backend_key: "backend.diffusers".to_string(),
            runtime_family: "diffusers".to_string(),
            resolved_load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.model.sdxl.cuda0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8_589_934_592,
            runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
            runtime_instance_id: Some("runtime.diffusers.001".to_string()),
            selected_device_ids: vec![device_id.clone()],
            selected_model_ref: model_ref(),
            runtime_trait_settings: Vec::new(),
            environment_ref: environment_ref(),
            reservations: vec![SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(
                    member.reservation_lease_id,
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

    fn model_ref() -> PumasModelRef {
        PumasModelRef {
            model_id: "pumas://models/juggernaut-xl-v10".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }
    }

    fn environment_ref() -> DependencyEnvironmentRef {
        DependencyEnvironmentRef {
            environment_id: DependencyEnvironmentId::parse("env.pytorch_diffusers")
                .expect("environment id"),
            manifest_id: Some("manifest.pytorch_diffusers".parse().expect("manifest id")),
        }
    }

    fn readiness_proof() -> DependencyReadinessProofEnvelope {
        let handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
            "../../../pantograph-scheduler/tests/fixtures/runtime_handoff_readiness_admitted.json"
        ))
        .expect("runtime handoff");
        handoff.readiness_proof
    }

    fn readiness_proof_for_member(
        member: &DispatchAssignmentFixtureMember<'_>,
    ) -> DependencyReadinessProofEnvelope {
        let mut proof = readiness_proof();
        proof.execution_context.workflow_run_id =
            DependencyReadinessWorkflowRunId::parse(member.workflow_run_id)
                .expect("readiness workflow run id");
        proof.readiness_proof_id =
            DependencyReadinessProofId::parse(format!("readiness-proof.{}", member.assignment_id))
                .expect("readiness proof id");
        proof.validate().expect("readiness proof");
        proof
    }

    fn assert_member_facts_match(
        request: &WorkflowRuntimeDispatchAssignmentRequest,
        member: &DispatchAssignmentFixtureMember<'_>,
    ) {
        assert_eq!(request.assignment_id.as_str(), member.assignment_id);
        assert_eq!(
            request.runtime_branch_event_id.as_str(),
            member.runtime_branch_event_id
        );
        assert_eq!(request.workflow_run_id, member.workflow_run_id);
        assert_eq!(
            request.scheduler_task_attempt_id,
            member.scheduler_task_attempt_id
        );
        assert_eq!(
            request.reservation_lease_id.as_str(),
            member.reservation_lease_id
        );
        assert_eq!(
            request
                .readiness_proof
                .execution_context
                .workflow_run_id
                .as_str(),
            member.workflow_run_id
        );
        assert_eq!(
            request.selected_runtime_handoff.workflow_run_id.as_str(),
            member.workflow_run_id
        );
        assert_eq!(
            request
                .selected_runtime_handoff
                .task_intent
                .workflow_run_id
                .as_str(),
            member.workflow_run_id
        );
        assert_eq!(
            request
                .selected_candidate_fact
                .resource_fit_assessment
                .workflow_run_id
                .as_str(),
            member.workflow_run_id
        );
        assert!(request
            .selected_candidate_fact
            .reservations
            .iter()
            .all(
                |reservation| reservation.workflow_run_id.as_str() == member.workflow_run_id
                    && reservation.reservation_lease_id.as_str() == member.reservation_lease_id
            ));
        let dispatch_decision = request
            .selected_runtime_handoff
            .dispatch_decision
            .as_ref()
            .expect("dispatch decision");
        assert_eq!(
            dispatch_decision.workflow_run_id.as_str(),
            member.workflow_run_id
        );
        assert_eq!(
            dispatch_decision.task_intent.workflow_run_id.as_str(),
            member.workflow_run_id
        );
        assert_eq!(
            dispatch_decision.reservation_lease_id.as_str(),
            member.reservation_lease_id
        );
        assert_eq!(dispatch_decision.readiness_proof, request.readiness_proof);
    }
}
