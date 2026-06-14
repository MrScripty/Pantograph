use std::collections::BTreeSet;

use crate::scheduler::WorkflowSchedulerTaskOrchestrator;
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerReservationLeaseId, SchedulerRuntimeHandoff,
};

use super::runtime_dispatch_assignment::{
    WorkflowRuntimeDispatchAssignmentBatchClaim,
    WorkflowRuntimeDispatchAssignmentBatchClaimOutcome, WorkflowRuntimeDispatchAssignmentId,
    WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentState,
};
use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;
use super::runtime_task_attempt_fact::WorkflowRuntimeTaskAttemptFactRecord;

#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchExecutionOwner<'a, R>
where
    R: WorkflowRuntimeBranchBatchResponderFanOut + ?Sized,
{
    scheduler_task_orchestrator: &'a WorkflowSchedulerTaskOrchestrator,
    responder_fan_out: &'a R,
}

pub(super) trait WorkflowRuntimeBranchBatchResponderFanOut {
    fn ensure_assignment_responders_registered(
        &self,
        members: &[WorkflowRuntimeBranchBatchExecutionMember],
    ) -> Result<(), WorkflowRuntimeBranchBatchExecutionDiagnostic>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchExecutionPlan {
    pub(super) batch_execution_request_id: String,
    pub(super) batch_claim: WorkflowRuntimeDispatchAssignmentBatchClaim,
    pub(super) members: Vec<WorkflowRuntimeBranchBatchExecutionMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchExecutionMember {
    pub(super) assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) runtime_branch_event_id: String,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) scheduler_task_attempt_started_at_ms: u64,
    pub(super) task_attempt_generation: u64,
    pub(super) timeout_ms: Option<u64>,
    pub(super) selected_runtime_handoff: SchedulerRuntimeHandoff,
    pub(super) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    pub(super) reservation_lease_id: SchedulerReservationLeaseId,
    pub(super) selected_candidate_id: Option<SchedulerDispatchCandidateId>,
    pub(super) task_attempt_fact: WorkflowRuntimeTaskAttemptFactRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchMemberExecutionOutcome {
    pub(super) assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) session_id: String,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) state: WorkflowRuntimeBranchBatchMemberExecutionOutcomeState,
    pub(super) diagnostics: Vec<WorkflowRuntimeBranchBatchExecutionDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchBatchMemberExecutionOutcomeState {
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchExecutionFailure {
    pub(super) diagnostics: Vec<WorkflowRuntimeBranchBatchExecutionDiagnostic>,
    pub(super) member_outcomes: Vec<WorkflowRuntimeBranchBatchMemberExecutionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchExecutionDiagnostic {
    pub(super) code: WorkflowRuntimeBranchBatchExecutionDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchBatchExecutionDiagnosticCode {
    EmptyBatchClaim,
    AnchorAssignmentMissing,
    DuplicateAssignment,
    AssignmentBatchClaimMissing,
    AssignmentBatchClaimMismatch,
    AssignmentNotRunning,
    MissingTaskAttemptFact,
    TaskAttemptFactMismatch,
    ResponderFanOutUnavailable,
}

impl<'a, R> WorkflowRuntimeBranchBatchExecutionOwner<'a, R>
where
    R: WorkflowRuntimeBranchBatchResponderFanOut + ?Sized,
{
    pub(super) fn new(
        scheduler_task_orchestrator: &'a WorkflowSchedulerTaskOrchestrator,
        responder_fan_out: &'a R,
    ) -> Self {
        Self {
            scheduler_task_orchestrator,
            responder_fan_out,
        }
    }

    pub(super) fn prepare_claimed_batch(
        &self,
        claim_outcome: WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
    ) -> Result<WorkflowRuntimeBranchBatchExecutionPlan, WorkflowRuntimeBranchBatchExecutionFailure>
    {
        let _selected_batch_dispatcher_owner = self.scheduler_task_orchestrator;
        let members = validate_claimed_assignments(&claim_outcome)?;
        self.responder_fan_out
            .ensure_assignment_responders_registered(&members)
            .map_err(WorkflowRuntimeBranchBatchExecutionFailure::global)?;
        Ok(WorkflowRuntimeBranchBatchExecutionPlan {
            batch_execution_request_id: batch_execution_request_id(&claim_outcome.batch_claim),
            batch_claim: claim_outcome.batch_claim,
            members,
        })
    }
}

impl WorkflowRuntimeBranchBatchExecutionFailure {
    fn global(diagnostic: WorkflowRuntimeBranchBatchExecutionDiagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
            member_outcomes: Vec::new(),
        }
    }

    fn member(
        record: &WorkflowRuntimeDispatchAssignmentRecord,
        diagnostic: WorkflowRuntimeBranchBatchExecutionDiagnostic,
    ) -> Self {
        Self {
            diagnostics: vec![diagnostic.clone()],
            member_outcomes: vec![WorkflowRuntimeBranchBatchMemberExecutionOutcome {
                assignment_id: record.assignment_id.clone(),
                session_id: record.session_id.clone(),
                workflow_id: record.workflow_id.clone(),
                workflow_run_id: record.workflow_run_id.clone(),
                state: WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed,
                diagnostics: vec![diagnostic],
            }],
        }
    }
}

impl WorkflowRuntimeBranchBatchExecutionDiagnostic {
    fn new(
        code: WorkflowRuntimeBranchBatchExecutionDiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn validate_claimed_assignments(
    claim_outcome: &WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
) -> Result<
    Vec<WorkflowRuntimeBranchBatchExecutionMember>,
    WorkflowRuntimeBranchBatchExecutionFailure,
> {
    if claim_outcome.assignments.is_empty() {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::global(
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::EmptyBatchClaim,
                "runtime branch batch execution requires at least one claimed assignment",
            ),
        ));
    }

    let mut seen_assignment_ids = BTreeSet::new();
    let mut anchor_seen = false;
    let mut members = Vec::with_capacity(claim_outcome.assignments.len());
    for record in &claim_outcome.assignments {
        if !seen_assignment_ids.insert(record.assignment_id.clone()) {
            return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
                record,
                WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                    WorkflowRuntimeBranchBatchExecutionDiagnosticCode::DuplicateAssignment,
                    format!(
                        "runtime branch batch assignment '{}' appears more than once",
                        record.assignment_id.as_str()
                    ),
                ),
            ));
        }
        if record.assignment_id == claim_outcome.batch_claim.anchor_assignment_id {
            anchor_seen = true;
        }
        members.push(batch_execution_member_from_assignment(
            record,
            &claim_outcome.batch_claim,
        )?);
    }

    if !anchor_seen {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::global(
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::AnchorAssignmentMissing,
                format!(
                    "runtime branch batch anchor assignment '{}' is not present in the claimed assignment group",
                    claim_outcome.batch_claim.anchor_assignment_id.as_str()
                ),
            ),
        ));
    }

    Ok(members)
}

fn batch_execution_member_from_assignment(
    record: &WorkflowRuntimeDispatchAssignmentRecord,
    batch_claim: &WorkflowRuntimeDispatchAssignmentBatchClaim,
) -> Result<WorkflowRuntimeBranchBatchExecutionMember, WorkflowRuntimeBranchBatchExecutionFailure> {
    validate_assignment_ready_for_batch_finalization(record, batch_claim)?;
    let task_attempt_fact = record
        .task_attempt_fact
        .as_ref()
        .expect("assignment task-attempt fact is validated before member projection");
    validate_task_attempt_fact_matches_assignment(record, task_attempt_fact)?;
    Ok(WorkflowRuntimeBranchBatchExecutionMember {
        assignment_id: record.assignment_id.clone(),
        runtime_branch_event_id: record.runtime_branch_event_id.as_str().to_string(),
        session_id: record.session_id.clone(),
        workflow_id: record.workflow_id.clone(),
        workflow_run_id: record.workflow_run_id.clone(),
        scheduler_task_id: record.scheduler_task_id.clone(),
        scheduler_task_attempt_id: record.scheduler_task_attempt_id.clone(),
        scheduler_task_attempt_started_at_ms: record.scheduler_task_attempt_started_at_ms,
        task_attempt_generation: record.task_attempt_generation,
        timeout_ms: record.timeout_ms,
        selected_runtime_handoff: record.selected_runtime_handoff.clone(),
        selected_candidate_fact: record.selected_candidate_fact.clone(),
        reservation_lease_id: record.reservation_lease_id.clone(),
        selected_candidate_id: record.selected_candidate_id.clone(),
        task_attempt_fact: task_attempt_fact.clone(),
    })
}

fn validate_assignment_ready_for_batch_finalization(
    record: &WorkflowRuntimeDispatchAssignmentRecord,
    batch_claim: &WorkflowRuntimeDispatchAssignmentBatchClaim,
) -> Result<(), WorkflowRuntimeBranchBatchExecutionFailure> {
    if record.state != WorkflowRuntimeDispatchAssignmentState::Running {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
            record,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::AssignmentNotRunning,
                format!(
                    "runtime branch batch assignment '{}' is not running",
                    record.assignment_id.as_str()
                ),
            ),
        ));
    }
    let Some(record_batch_claim) = record.batch_claim.as_ref() else {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
            record,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::AssignmentBatchClaimMissing,
                format!(
                    "runtime branch batch assignment '{}' is missing its durable batch claim",
                    record.assignment_id.as_str()
                ),
            ),
        ));
    };
    if record_batch_claim != batch_claim {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
            record,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::AssignmentBatchClaimMismatch,
                format!(
                    "runtime branch batch assignment '{}' has a mismatched durable batch claim",
                    record.assignment_id.as_str()
                ),
            ),
        ));
    }
    if record.task_attempt_fact.is_none() {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
            record,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::MissingTaskAttemptFact,
                format!(
                    "runtime branch batch assignment '{}' is missing task-attempt facts required for finalization",
                    record.assignment_id.as_str()
                ),
            ),
        ));
    }
    Ok(())
}

fn validate_task_attempt_fact_matches_assignment(
    record: &WorkflowRuntimeDispatchAssignmentRecord,
    fact: &WorkflowRuntimeTaskAttemptFactRecord,
) -> Result<(), WorkflowRuntimeBranchBatchExecutionFailure> {
    let mismatch = if fact.workflow_id != record.workflow_id {
        Some((
            "workflow_id",
            fact.workflow_id.as_str(),
            record.workflow_id.as_str(),
        ))
    } else if fact.workflow_run_id != record.workflow_run_id {
        Some((
            "workflow_run_id",
            fact.workflow_run_id.as_str(),
            record.workflow_run_id.as_str(),
        ))
    } else if fact.scheduler_task_id != record.scheduler_task_id {
        Some((
            "scheduler_task_id",
            fact.scheduler_task_id.as_str(),
            record.scheduler_task_id.as_str(),
        ))
    } else if fact.scheduler_task_attempt_id != record.scheduler_task_attempt_id {
        Some((
            "scheduler_task_attempt_id",
            fact.scheduler_task_attempt_id.as_str(),
            record.scheduler_task_attempt_id.as_str(),
        ))
    } else if fact.task_attempt_generation != record.task_attempt_generation {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
            record,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::TaskAttemptFactMismatch,
                format!(
                    "runtime branch batch assignment '{}' has task-attempt generation {} but fact records {}",
                    record.assignment_id.as_str(),
                    record.task_attempt_generation,
                    fact.task_attempt_generation
                ),
            ),
        ));
    } else {
        None
    };
    if let Some((field, fact_value, assignment_value)) = mismatch {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::member(
            record,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::TaskAttemptFactMismatch,
                format!(
                    "runtime branch batch assignment '{}' has mismatched task-attempt fact {field}: fact '{fact_value}' assignment '{assignment_value}'",
                    record.assignment_id.as_str()
                ),
            ),
        ));
    }
    Ok(())
}

fn batch_execution_request_id(batch_claim: &WorkflowRuntimeDispatchAssignmentBatchClaim) -> String {
    format!(
        "workflow-runtime-branch-batch:{}",
        batch_claim.batch_claim_id.as_str()
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use pantograph_dependency_planning::{
        DependencyEnvironmentId, DependencyEnvironmentRef, DependencyReadinessProofEnvelope,
        DependencyReadinessProofId, DependencyReadinessWorkflowRunId, DependencyTaskId,
        DeviceIntentId, PumasModelRef,
    };
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerDispatchCandidateId, SchedulerDispatchDecision,
        SchedulerDispatchDiagnostic, SchedulerDispatchDiagnosticCode,
        SchedulerDispatchDiagnosticSeverity, SchedulerNodeId, SchedulerResourceFitAssessment,
        SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
        SchedulerRuntimeDeviceConstraints, SchedulerRuntimeHandoff, SchedulerRuntimeHandoffState,
        SchedulerRuntimeVariantId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION, SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
        SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
    };

    use super::super::runtime_branch_task_event::{
        WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventClaimLeaseId,
        WorkflowRuntimeBranchTaskEventClaimOwnerId, WorkflowRuntimeBranchTaskEventId,
    };
    use super::super::runtime_dispatch_assignment::{
        InMemoryWorkflowRuntimeDispatchAssignmentRepository,
        WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
        WorkflowRuntimeDispatchAssignmentRepository, WorkflowRuntimeDispatchAssignmentRequest,
    };
    use super::super::runtime_dispatch_selection::{
        WorkflowRuntimeDispatchCandidateFact, WorkflowRuntimeDispatchLoadState,
    };
    use super::super::WorkflowService;
    use super::*;
    use crate::graph::WorkflowRuntimeSourceContext;

    #[test]
    fn runtime_branch_batch_execution_owner_accepts_claimed_running_members_with_facts() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let claim_outcome = batch_claim_outcome();

        let plan = owner
            .prepare_claimed_batch(claim_outcome.clone())
            .expect("claimed batch execution plan");

        assert_eq!(plan.batch_claim, claim_outcome.batch_claim);
        assert_eq!(
            plan.batch_execution_request_id,
            format!(
                "workflow-runtime-branch-batch:{}",
                claim_outcome.batch_claim.batch_claim_id.as_str()
            )
        );
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.assignment_id.as_str())
                .collect::<Vec<_>>(),
            vec!["assignment.1", "assignment.2"]
        );
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.workflow_run_id.as_str())
                .collect::<Vec<_>>(),
            vec!["run.2026-05-22.001", "run.2026-05-22.002"]
        );
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.task_attempt_fact.workflow_run_id.as_str())
                .collect::<Vec<_>>(),
            vec!["run.2026-05-22.001", "run.2026-05-22.002"]
        );
        assert_eq!(
            plan.members
                .iter()
                .map(|member| {
                    member
                        .selected_runtime_handoff
                        .task_intent
                        .workflow_run_id
                        .as_str()
                })
                .collect::<Vec<_>>(),
            vec!["run.2026-05-22.001", "run.2026-05-22.002"]
        );
        assert_eq!(
            plan.members
                .iter()
                .map(|member| member.reservation_lease_id.as_str())
                .collect::<Vec<_>>(),
            vec!["reservation-lease.runtime.1", "reservation-lease.runtime.2"]
        );
        assert_eq!(
            responder_fan_out.observed_assignment_ids(),
            vec![vec!["assignment.1".to_string(), "assignment.2".to_string()]]
        );
    }

    #[test]
    fn runtime_branch_batch_execution_owner_fails_closed_when_member_lacks_task_attempt_fact() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let mut claim_outcome = batch_claim_outcome();
        claim_outcome.assignments[1].task_attempt_fact = None;

        let failure = owner
            .prepare_claimed_batch(claim_outcome)
            .expect_err("missing durable member fact must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::MissingTaskAttemptFact
        );
        assert_eq!(failure.member_outcomes.len(), 1);
        assert_eq!(
            failure.member_outcomes[0].assignment_id.as_str(),
            "assignment.2"
        );
        assert_eq!(
            failure.member_outcomes[0].state,
            WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed
        );
        assert!(
            responder_fan_out.observed_assignment_ids().is_empty(),
            "fan-out must not be consulted after durable fact validation fails"
        );
    }

    #[test]
    fn runtime_branch_batch_execution_owner_rejects_mismatched_task_attempt_fact_identity() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let mut claim_outcome = batch_claim_outcome();
        claim_outcome.assignments[1]
            .task_attempt_fact
            .as_mut()
            .expect("second assignment task-attempt fact")
            .workflow_run_id = "run.unrelated".to_string();

        let failure = owner
            .prepare_claimed_batch(claim_outcome)
            .expect_err("mismatched task-attempt fact must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::TaskAttemptFactMismatch
        );
        assert_eq!(failure.member_outcomes.len(), 1);
        assert_eq!(
            failure.member_outcomes[0].assignment_id.as_str(),
            "assignment.2"
        );
        assert!(
            responder_fan_out.observed_assignment_ids().is_empty(),
            "fan-out must not be consulted after durable fact validation fails"
        );
    }

    #[test]
    fn runtime_branch_batch_execution_owner_fails_closed_when_responder_is_missing() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::fail_with(
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ResponderFanOutUnavailable,
                "runtime branch batch responder for assignment.2 is not registered",
            ),
        );
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );

        let failure = owner
            .prepare_claimed_batch(batch_claim_outcome())
            .expect_err("missing responder must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ResponderFanOutUnavailable
        );
        assert!(failure.member_outcomes.is_empty());
        assert_eq!(
            responder_fan_out.observed_assignment_ids(),
            vec![vec!["assignment.1".to_string(), "assignment.2".to_string()]]
        );
    }

    #[derive(Default)]
    struct RecordingResponderFanOut {
        observed_assignment_ids: Mutex<Vec<Vec<String>>>,
        failure: Option<WorkflowRuntimeBranchBatchExecutionDiagnostic>,
    }

    impl RecordingResponderFanOut {
        fn fail_with(failure: WorkflowRuntimeBranchBatchExecutionDiagnostic) -> Self {
            Self {
                observed_assignment_ids: Mutex::new(Vec::new()),
                failure: Some(failure),
            }
        }

        fn observed_assignment_ids(&self) -> Vec<Vec<String>> {
            self.observed_assignment_ids
                .lock()
                .expect("observed assignment ids lock")
                .clone()
        }
    }

    impl WorkflowRuntimeBranchBatchResponderFanOut for RecordingResponderFanOut {
        fn ensure_assignment_responders_registered(
            &self,
            members: &[WorkflowRuntimeBranchBatchExecutionMember],
        ) -> Result<(), WorkflowRuntimeBranchBatchExecutionDiagnostic> {
            self.observed_assignment_ids
                .lock()
                .expect("observed assignment ids lock")
                .push(
                    members
                        .iter()
                        .map(|member| member.assignment_id.as_str().to_string())
                        .collect(),
                );
            if let Some(failure) = self.failure.clone() {
                return Err(failure);
            }
            Ok(())
        }
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

    fn batch_claim_outcome() -> WorkflowRuntimeDispatchAssignmentBatchClaimOutcome {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request_for_member(
                &DispatchAssignmentFixtureMember::first(),
            ))
            .expect("first assignment");
        let second = repository
            .create(assignment_request_for_member(
                &DispatchAssignmentFixtureMember::second(),
            ))
            .expect("second assignment");
        let first = repository
            .mark_running(&first.assignment_id, 130)
            .expect("first running assignment");
        let _second = repository
            .mark_running(&second.assignment_id, 131)
            .expect("second running assignment");
        repository
            .claim_compatible_running_batch(&first.assignment_id, batch_owner_id(), 140, 1_000, 8)
            .expect("compatible batch claim")
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

    fn batch_owner_id() -> WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId {
        WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId::parse("workflow-service.batch-claimer")
            .expect("batch owner id")
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
        reservation_lease_id: pantograph_scheduler::SchedulerReservationLeaseId,
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
                reservation_lease_id: pantograph_scheduler::SchedulerReservationLeaseId::parse(
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
}
