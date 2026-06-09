use std::collections::BTreeMap;

use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerReservationLeaseId, SchedulerRuntimeHandoff,
    SchedulerRuntimeHandoffState, ValidatedSchedulerRuntimeHandoff,
};
use uuid::Uuid;

use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventId,
};
use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;

pub(super) const WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[must_use]
pub(super) struct WorkflowRuntimeDispatchAssignmentId(String);

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
    pub(super) runtime_branch_claim: WorkflowRuntimeBranchTaskEventClaim,
    pub(super) readiness_proof: DependencyReadinessProofEnvelope,
    pub(super) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    pub(super) selected_runtime_handoff: SchedulerRuntimeHandoff,
    pub(super) reservation_lease_id: SchedulerReservationLeaseId,
    pub(super) selected_candidate_id: Option<SchedulerDispatchCandidateId>,
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
            runtime_branch_claim: request.runtime_branch_claim,
            readiness_proof: request.readiness_proof,
            selected_candidate_fact: request.selected_candidate_fact,
            selected_runtime_handoff: request.selected_runtime_handoff,
            reservation_lease_id: request.reservation_lease_id,
            selected_candidate_id: request.selected_candidate_id,
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

    fn get(
        &self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    ) -> Option<WorkflowRuntimeDispatchAssignmentRecord> {
        self.records.get(assignment_id).cloned()
    }
}

impl InMemoryWorkflowRuntimeDispatchAssignmentRepository {
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
        record.state = next_state;
        record.updated_at_ms = now_ms;
        Ok(record.clone())
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
    if request.runtime_branch_claim.attempt_generation != request.task_attempt_generation {
        return invalid_assignment(
            "runtime branch claim generation must match task attempt generation",
        );
    }
    validate_candidate_correlation(request)?;
    validate_selected_runtime_handoff(request)?;
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

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{
        DependencyEnvironmentId, DependencyEnvironmentRef, DependencyTaskId, DeviceIntentId,
        PumasModelRef,
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

    fn assignment_request(assignment_id: &str) -> WorkflowRuntimeDispatchAssignmentRequest {
        let readiness_proof = readiness_proof();
        let selected_candidate_fact = selected_candidate_fact();
        let reservation_lease_id = selected_candidate_fact.reservations[0]
            .reservation_lease_id
            .clone();
        let selected_candidate_id = Some(selected_candidate_fact.candidate_id.clone());
        WorkflowRuntimeDispatchAssignmentRequest {
            assignment_id: WorkflowRuntimeDispatchAssignmentId::parse(assignment_id)
                .expect("assignment id"),
            runtime_branch_event_id: WorkflowRuntimeBranchTaskEventId::parse(
                "runtime-branch-task-event.run.2026-05-22.001.task.image_generation.001",
            )
            .expect("event id"),
            session_id: "session.image.1".to_string(),
            workflow_id: "workflow.image_generation".to_string(),
            workflow_run_id: "run.2026-05-22.001".to_string(),
            scheduler_task_id: "task.image_generation.001".to_string(),
            scheduler_task_attempt_id: "attempt.image.1".to_string(),
            scheduler_task_attempt_started_at_ms: 100,
            task_attempt_generation: 1,
            runtime_branch_claim: runtime_branch_claim(),
            selected_runtime_handoff: selected_runtime_handoff(
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

    fn selected_runtime_handoff(
        readiness_proof: DependencyReadinessProofEnvelope,
        reservation_lease_id: SchedulerReservationLeaseId,
    ) -> SchedulerRuntimeHandoff {
        let intent = task_intent();
        let environment_ref = environment_ref();
        SchedulerRuntimeHandoff {
            contract_version: SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run.2026-05-22.001").expect("run id"),
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
                workflow_run_id: SchedulerWorkflowRunId::parse("run.2026-05-22.001")
                    .expect("run id"),
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
                reservations: selected_candidate_fact().reservations,
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

    fn task_intent() -> SchedulableTaskIntent {
        SchedulableTaskIntent {
            contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run.2026-05-22.001").expect("run id"),
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

    fn selected_candidate_fact() -> WorkflowRuntimeDispatchCandidateFact {
        let workflow_run_id: SchedulerWorkflowRunId = "run.2026-05-22.001".parse().expect("run id");
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
}
