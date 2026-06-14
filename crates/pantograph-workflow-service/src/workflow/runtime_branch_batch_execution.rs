use std::collections::BTreeSet;

use crate::scheduler::task_orchestrator::{
    SelectedRuntimeTaskDispatch, StartedRuntimeTaskBatchMember,
    WorkflowSchedulerRuntimeBatchMemberMutation, WorkflowSchedulerTaskOrchestrator,
    WorkflowSchedulerTaskOrchestratorError,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionMemberResponse,
    RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionResponse, RuntimeHostBatchMemberFailurePolicy,
    RuntimeHostBatchMemberReservationPolicy, RuntimeHostExecutionCancellationContext,
    ValidatedRuntimeHostBatchExecutionRequest, RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerReservationLeaseId, SchedulerRuntimeHandoff,
    SchedulerTaskStateRecord,
};

use super::runtime_dispatch_assignment::{
    WorkflowRuntimeDispatchAssignmentBatchClaim,
    WorkflowRuntimeDispatchAssignmentBatchClaimOutcome, WorkflowRuntimeDispatchAssignmentId,
    WorkflowRuntimeDispatchAssignmentRecord, WorkflowRuntimeDispatchAssignmentState,
};
use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;
use super::runtime_task_attempt_fact::WorkflowRuntimeTaskAttemptFactRecord;
use super::{
    materialize_runtime_host_inputs, WorkflowSchedulerTask, WorkflowSchedulerTaskResult,
    WorkflowService,
};

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
    pub(super) active_run_members: Vec<WorkflowRuntimeBranchBatchActiveRunMember>,
    pub(super) runtime_host_request: RuntimeHostBatchExecutionRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchActiveRunMember {
    pub(super) assignment_id: WorkflowRuntimeDispatchAssignmentId,
    pub(super) started_batch_member: StartedRuntimeTaskBatchMember,
    pub(super) runtime_task: WorkflowSchedulerTask,
    pub(super) running_task_record: SchedulerTaskStateRecord,
    pub(super) materialized_results: Vec<WorkflowSchedulerTaskResult>,
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
    Completed,
    Cancelled,
    Deferred,
    Retryable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchBatchResponseMutationOutcome {
    pub(super) member_outcomes: Vec<WorkflowRuntimeBranchBatchMemberExecutionOutcome>,
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
    ActiveRunTaskStateUnavailable,
    ActiveRunTaskUnavailable,
    ActiveRunTaskNotRunning,
    ActiveRunTaskAttemptMismatch,
    ActiveRunMaterializedInputsUnavailable,
    RuntimeHostBatchMemberInputMappingInvalid,
    RuntimeHostBatchMemberHandoffMismatch,
    RuntimeHostBatchRequestInvalid,
    RuntimeHostBatchResponseMismatch,
    RuntimeHostBatchMemberResponseMissing,
    RuntimeHostBatchMemberResponseUnknown,
    RuntimeHostBatchMemberMutationInvalid,
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
        service: &WorkflowService,
        claim_outcome: WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
    ) -> Result<WorkflowRuntimeBranchBatchExecutionPlan, WorkflowRuntimeBranchBatchExecutionFailure>
    {
        let members = validate_claimed_assignments(&claim_outcome)?;
        let active_run_members = rehydrate_batch_members_from_active_runs(
            self.scheduler_task_orchestrator,
            service,
            &members,
        )?;
        let runtime_host_request = runtime_host_batch_request_from_active_members(
            &claim_outcome.batch_claim,
            &members,
            &active_run_members,
        )?;
        self.responder_fan_out
            .ensure_assignment_responders_registered(&members)
            .map_err(WorkflowRuntimeBranchBatchExecutionFailure::global)?;
        Ok(WorkflowRuntimeBranchBatchExecutionPlan {
            batch_execution_request_id: batch_execution_request_id(&claim_outcome.batch_claim),
            batch_claim: claim_outcome.batch_claim,
            members,
            active_run_members,
            runtime_host_request,
        })
    }

    pub(super) fn apply_batch_response_mutations(
        &self,
        service: &WorkflowService,
        plan: &WorkflowRuntimeBranchBatchExecutionPlan,
        response: &RuntimeHostBatchExecutionResponse,
    ) -> Result<
        WorkflowRuntimeBranchBatchResponseMutationOutcome,
        WorkflowRuntimeBranchBatchExecutionFailure,
    > {
        validate_batch_response_matches_plan(plan, response)?;
        let mut store = service.session_store_guard().map_err(|error| {
            WorkflowRuntimeBranchBatchExecutionFailure::global(
                WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                    WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskStateUnavailable,
                    format!("runtime branch batch execution could not write session store: {error}"),
                ),
            )
        })?;
        let mut outcomes = Vec::with_capacity(plan.active_run_members.len());
        for active_member in &plan.active_run_members {
            let member = plan
                .members
                .iter()
                .find(|member| member.assignment_id == active_member.assignment_id)
                .expect("active-run member is built from validated plan members");
            let response_member =
                response_member_for_assignment(response, active_member.assignment_id.as_str())
                    .ok_or_else(|| {
                        WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
                            member,
                            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberResponseMissing,
                                format!(
                                    "runtime branch batch response is missing member '{}'",
                                    active_member.assignment_id.as_str()
                                ),
                            ),
                        )
                    })?;
            let mutation = self
                .scheduler_task_orchestrator
                .apply_runtime_batch_member_response_mutation(
                    &mut store,
                    &member.session_id,
                    &member.workflow_run_id,
                    &active_member.started_batch_member,
                    response_member,
                )
                .map_err(|error| {
                    WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
                        member,
                        WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberMutationInvalid,
                            format!(
                                "runtime branch batch member '{}' scheduler mutation failed: {error}",
                                member.assignment_id.as_str()
                            ),
                        ),
                    )
                })?;
            outcomes.push(member_outcome_from_scheduler_mutation(
                member,
                response_member,
                &mutation,
            ));
        }
        Ok(WorkflowRuntimeBranchBatchResponseMutationOutcome {
            member_outcomes: outcomes,
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

    fn active_run_member(
        member: &WorkflowRuntimeBranchBatchExecutionMember,
        diagnostic: WorkflowRuntimeBranchBatchExecutionDiagnostic,
    ) -> Self {
        Self {
            diagnostics: vec![diagnostic.clone()],
            member_outcomes: vec![WorkflowRuntimeBranchBatchMemberExecutionOutcome {
                assignment_id: member.assignment_id.clone(),
                session_id: member.session_id.clone(),
                workflow_id: member.workflow_id.clone(),
                workflow_run_id: member.workflow_run_id.clone(),
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

fn rehydrate_batch_members_from_active_runs(
    scheduler_task_orchestrator: &WorkflowSchedulerTaskOrchestrator,
    service: &WorkflowService,
    members: &[WorkflowRuntimeBranchBatchExecutionMember],
) -> Result<
    Vec<WorkflowRuntimeBranchBatchActiveRunMember>,
    WorkflowRuntimeBranchBatchExecutionFailure,
> {
    let mut store = service.session_store_guard().map_err(|error| {
        WorkflowRuntimeBranchBatchExecutionFailure::global(
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskStateUnavailable,
                format!("runtime branch batch execution could not read session store: {error}"),
            ),
        )
    })?;
    let mut active_run_members = Vec::with_capacity(members.len());
    for member in members {
        let started = scheduler_task_orchestrator
            .rehydrate_running_runtime_task(
                &mut store,
                &member.session_id,
                &member.workflow_run_id,
                &member.scheduler_task_id,
                &member.scheduler_task_attempt_id,
                member.scheduler_task_attempt_started_at_ms,
            )
            .map_err(|error| rehydration_failure_for_member(member, error))?;
        let selected_dispatch = SelectedRuntimeTaskDispatch::new(
            member.selected_runtime_handoff.clone(),
            member.reservation_lease_id.clone(),
            member.selected_candidate_id.clone(),
        );
        let started_batch_member = StartedRuntimeTaskBatchMember::new(
            member.assignment_id.as_str().to_string(),
            started.clone(),
            selected_dispatch,
        );
        active_run_members.push(WorkflowRuntimeBranchBatchActiveRunMember {
            assignment_id: member.assignment_id.clone(),
            started_batch_member,
            runtime_task: started.task().clone(),
            running_task_record: started.running_record().clone(),
            materialized_results: started.materialized_results.clone(),
        });
    }
    Ok(active_run_members)
}

fn rehydration_failure_for_member(
    member: &WorkflowRuntimeBranchBatchExecutionMember,
    error: WorkflowSchedulerTaskOrchestratorError,
) -> WorkflowRuntimeBranchBatchExecutionFailure {
    let message = error.to_string();
    let code = if message.contains("does not match")
        || message.contains("active task attempt")
        || message.contains("active attempt fact")
    {
        WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskAttemptMismatch
    } else if message.contains("must be running") {
        WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskNotRunning
    } else if message.contains("not a runtime inference task")
        || message.contains("not in active workflow run")
        || message.contains("active task-state record")
    {
        WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskUnavailable
    } else {
        WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskStateUnavailable
    };
    WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
        member,
        WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
            code,
            format!(
                "runtime branch batch member '{}' could not be rehydrated from scheduler active-run state: {message}",
                member.assignment_id.as_str()
            ),
        ),
    )
}

fn runtime_host_batch_request_from_active_members(
    batch_claim: &WorkflowRuntimeDispatchAssignmentBatchClaim,
    members: &[WorkflowRuntimeBranchBatchExecutionMember],
    active_run_members: &[WorkflowRuntimeBranchBatchActiveRunMember],
) -> Result<RuntimeHostBatchExecutionRequest, WorkflowRuntimeBranchBatchExecutionFailure> {
    let request_members = members
        .iter()
        .map(|member| {
            let active_member = active_run_members
                .iter()
                .find(|active_member| active_member.assignment_id == member.assignment_id)
                .ok_or_else(|| {
                    WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
                        member,
                        WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskUnavailable,
                            format!(
                                "runtime branch batch member '{}' has no active-run projection",
                                member.assignment_id.as_str()
                            ),
                        ),
                    )
                })?;
            runtime_host_batch_member_request_from_active_member(member, active_member)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let batch_execution_request_id = batch_execution_request_id(batch_claim);
    let anchor_execution_request_id =
        runtime_batch_member_execution_request_id(batch_claim.anchor_assignment_id.as_str());
    let request = RuntimeHostBatchExecutionRequest {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: batch_execution_request_id.clone(),
        anchor_execution_request_id,
        cancellation_context: RuntimeHostExecutionCancellationContext::workflow_service(
            &batch_execution_request_id,
        ),
        members: request_members,
    };
    let validated =
        ValidatedRuntimeHostBatchExecutionRequest::try_from(request).map_err(|error| {
            WorkflowRuntimeBranchBatchExecutionFailure::global(
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchRequestInvalid,
                format!("runtime branch batch request is invalid: {error}"),
            ),
        )
        })?;
    Ok(validated.into_inner())
}

fn runtime_host_batch_member_request_from_active_member(
    member: &WorkflowRuntimeBranchBatchExecutionMember,
    active_member: &WorkflowRuntimeBranchBatchActiveRunMember,
) -> Result<RuntimeHostBatchExecutionMemberRequest, WorkflowRuntimeBranchBatchExecutionFailure> {
    validate_runtime_handoff_matches_active_member(member, active_member)?;
    let materialized_inputs = materialize_runtime_host_inputs(
        &active_member.runtime_task,
        &active_member.materialized_results,
    )
    .map_err(|error| {
        WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
            member,
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberInputMappingInvalid,
                format!(
                    "runtime branch batch member '{}' runtime-host inputs are invalid: {error}",
                    member.assignment_id.as_str()
                ),
            ),
        )
    })?;
    Ok(RuntimeHostBatchExecutionMemberRequest {
        execution_request_id: runtime_batch_member_execution_request_id(
            member.assignment_id.as_str(),
        ),
        assignment_id: member.assignment_id.as_str().to_string(),
        handoff: member.selected_runtime_handoff.clone(),
        materialized_inputs,
        timeout_ms: member.timeout_ms,
        failure_policy: RuntimeHostBatchMemberFailurePolicy::Retryable,
        reservation_policy: RuntimeHostBatchMemberReservationPolicy::DeferToScheduler,
    })
}

fn validate_runtime_handoff_matches_active_member(
    member: &WorkflowRuntimeBranchBatchExecutionMember,
    active_member: &WorkflowRuntimeBranchBatchActiveRunMember,
) -> Result<(), WorkflowRuntimeBranchBatchExecutionFailure> {
    let handoff = &member.selected_runtime_handoff;
    let task = &active_member.runtime_task;
    if handoff.workflow_id != task.workflow_id
        || handoff.workflow_run_id != task.workflow_run_id
        || handoff.node_id != task.node_id
        || handoff.task_id != task.task_id
    {
        return Err(
            WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
                member,
                WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                    WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberHandoffMismatch,
                    format!(
                        "runtime branch batch member '{}' scheduler handoff does not match the active runtime task",
                        member.assignment_id.as_str()
                    ),
                ),
            ),
        );
    }
    Ok(())
}

fn validate_batch_response_matches_plan(
    plan: &WorkflowRuntimeBranchBatchExecutionPlan,
    response: &RuntimeHostBatchExecutionResponse,
) -> Result<(), WorkflowRuntimeBranchBatchExecutionFailure> {
    if response.batch_execution_request_id != plan.batch_execution_request_id {
        return Err(WorkflowRuntimeBranchBatchExecutionFailure::global(
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchResponseMismatch,
                format!(
                    "runtime branch batch response id '{}' does not match plan '{}'",
                    response.batch_execution_request_id, plan.batch_execution_request_id
                ),
            ),
        ));
    }
    for response_member in &response.members {
        if plan
            .members
            .iter()
            .all(|member| member.assignment_id.as_str() != response_member.assignment_id)
        {
            return Err(WorkflowRuntimeBranchBatchExecutionFailure::global(
                WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                    WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberResponseUnknown,
                    format!(
                        "runtime branch batch response includes unknown assignment '{}'",
                        response_member.assignment_id
                    ),
                ),
            ));
        }
    }
    for member in &plan.members {
        if response_member_for_assignment(response, member.assignment_id.as_str()).is_none() {
            return Err(
                WorkflowRuntimeBranchBatchExecutionFailure::active_run_member(
                    member,
                    WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                        WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberResponseMissing,
                        format!(
                            "runtime branch batch response is missing member '{}'",
                            member.assignment_id.as_str()
                        ),
                    ),
                ),
            );
        }
    }
    Ok(())
}

fn response_member_for_assignment<'a>(
    response: &'a RuntimeHostBatchExecutionResponse,
    assignment_id: &str,
) -> Option<&'a RuntimeHostBatchExecutionMemberResponse> {
    response
        .members
        .iter()
        .find(|member| member.assignment_id == assignment_id)
}

fn member_outcome_from_scheduler_mutation(
    member: &WorkflowRuntimeBranchBatchExecutionMember,
    response: &RuntimeHostBatchExecutionMemberResponse,
    mutation: &WorkflowSchedulerRuntimeBatchMemberMutation,
) -> WorkflowRuntimeBranchBatchMemberExecutionOutcome {
    let state = match mutation {
        WorkflowSchedulerRuntimeBatchMemberMutation::Terminal(_mutation) => match response.state {
            RuntimeHostBatchExecutionMemberState::Completed => {
                WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Completed
            }
            RuntimeHostBatchExecutionMemberState::Cancelled => {
                WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Cancelled
            }
            _ => WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed,
        },
        WorkflowSchedulerRuntimeBatchMemberMutation::Deferred(_record) => {
            WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Deferred
        }
        WorkflowSchedulerRuntimeBatchMemberMutation::Retryable(_record) => {
            WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Retryable
        }
    };
    WorkflowRuntimeBranchBatchMemberExecutionOutcome {
        assignment_id: member.assignment_id.clone(),
        session_id: member.session_id.clone(),
        workflow_id: member.workflow_id.clone(),
        workflow_run_id: member.workflow_run_id.clone(),
        state,
        diagnostics: Vec::new(),
    }
}

fn runtime_batch_member_execution_request_id(assignment_id: &str) -> String {
    format!("workflow-runtime-batch-member.{assignment_id}")
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
        "workflow-runtime-branch-batch.{}",
        batch_claim.anchor_assignment_id.as_str()
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
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionMemberResponse,
        RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionResponse,
        RuntimeHostBatchExecutionState, RuntimeHostBatchMemberFailurePolicy,
        RuntimeHostBatchMemberReservationDisposition, RuntimeHostBatchMemberReservationPolicy,
        RuntimeHostBatchMemberRetryDisposition, RuntimeHostExecutionInputValue,
    };
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerDispatchCandidateId, SchedulerDispatchDecision,
        SchedulerDispatchDiagnostic, SchedulerDispatchDiagnosticCode,
        SchedulerDispatchDiagnosticSeverity, SchedulerNodeId, SchedulerResourceFitAssessment,
        SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
        SchedulerRuntimeDeviceConstraints, SchedulerRuntimeHandoff, SchedulerRuntimeHandoffState,
        SchedulerRuntimeVariantId, SchedulerTaskExecutionIntent, SchedulerTaskId,
        SchedulerTaskState, SchedulerTaskStateKind, SchedulerTaskStateRecord,
        SchedulerTaskStateTransition, SchedulerTaskStateTransitionId, SchedulerWorkflowId,
        SchedulerWorkflowRunId, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
        SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION, SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
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
    use super::super::{
        WorkflowExecutionSessionRunRequest, WorkflowOutputTarget, WorkflowPortBinding,
        WorkflowSchedulerSourceInputTemplate, WorkflowSchedulerTask,
        WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
        WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskResult,
        WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
        WorkflowSchedulerTaskResultValue, WorkflowService,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
    };
    use super::*;
    use crate::graph::WorkflowRuntimeSourceContext;
    use crate::scheduler::WorkflowSchedulerTaskAttemptId;

    #[test]
    fn runtime_branch_batch_execution_owner_accepts_claimed_running_members_with_facts() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let members = active_batch_members(&service);
        let claim_outcome = batch_claim_outcome(&members);

        let plan = owner
            .prepare_claimed_batch(&service, claim_outcome.clone())
            .expect("claimed batch execution plan");

        assert_eq!(plan.batch_claim, claim_outcome.batch_claim);
        assert_eq!(
            plan.batch_execution_request_id,
            "workflow-runtime-branch-batch.assignment.1"
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
            plan.active_run_members
                .iter()
                .map(|member| member.assignment_id.as_str())
                .collect::<Vec<_>>(),
            vec!["assignment.1", "assignment.2"]
        );
        assert_eq!(
            plan.active_run_members
                .iter()
                .map(|member| member.runtime_task.workflow_run_id.as_str())
                .collect::<Vec<_>>(),
            vec!["run.2026-05-22.001", "run.2026-05-22.002"]
        );
        assert_eq!(
            plan.active_run_members
                .iter()
                .map(|member| member.running_task_record.state.kind())
                .collect::<Vec<_>>(),
            vec![
                SchedulerTaskStateKind::Running,
                SchedulerTaskStateKind::Running
            ]
        );
        assert_eq!(
            plan.active_run_members
                .iter()
                .map(|member| prompt_text_from_materialized_results(&member.materialized_results))
                .collect::<Vec<_>>(),
            vec![
                "prompt owned by run.2026-05-22.001".to_string(),
                "prompt owned by run.2026-05-22.002".to_string(),
            ]
        );
        assert_eq!(
            plan.active_run_members
                .iter()
                .map(|member| { member.started_batch_member.started().attempt_id().as_str() })
                .collect::<Vec<_>>(),
            vec![
                plan.members[0].scheduler_task_attempt_id.as_str(),
                plan.members[1].scheduler_task_attempt_id.as_str(),
            ]
        );
        assert_eq!(
            plan.active_run_members
                .iter()
                .map(|member| {
                    member
                        .started_batch_member
                        .selected_dispatch()
                        .reservation_lease_id()
                        .as_str()
                })
                .collect::<Vec<_>>(),
            vec!["reservation-lease.runtime.1", "reservation-lease.runtime.2"]
        );
        assert_eq!(
            plan.runtime_host_request.batch_execution_request_id,
            plan.batch_execution_request_id
        );
        assert_eq!(
            plan.runtime_host_request.anchor_execution_request_id,
            "workflow-runtime-batch-member.assignment.1"
        );
        assert_eq!(
            plan.runtime_host_request
                .members
                .iter()
                .map(|member| member.assignment_id.as_str())
                .collect::<Vec<_>>(),
            vec!["assignment.1", "assignment.2"]
        );
        assert_eq!(
            plan.runtime_host_request
                .members
                .iter()
                .map(|member| prompt_text_from_runtime_host_member_request(member))
                .collect::<Vec<_>>(),
            vec![
                "prompt owned by run.2026-05-22.001".to_string(),
                "prompt owned by run.2026-05-22.002".to_string(),
            ]
        );
        assert!(plan.runtime_host_request.members.iter().all(|member| {
            member.failure_policy == RuntimeHostBatchMemberFailurePolicy::Retryable
                && member.reservation_policy
                    == RuntimeHostBatchMemberReservationPolicy::DeferToScheduler
                && member.timeout_ms == Some(30_000)
        }));
        assert_eq!(
            responder_fan_out.observed_assignment_ids(),
            vec![vec!["assignment.1".to_string(), "assignment.2".to_string()]]
        );
    }

    #[test]
    fn runtime_branch_batch_execution_owner_applies_completed_response_mutations() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let members = active_batch_members(&service);
        let plan = owner
            .prepare_claimed_batch(&service, batch_claim_outcome(&members))
            .expect("claimed batch execution plan");
        assert_eq!(
            service
                .scheduler_task_orchestrator
                .active_task_lifecycle_handle_count()
                .expect("active task handle count"),
            2
        );
        let response = runtime_host_batch_response_from_plan(&plan);

        let outcome = owner
            .apply_batch_response_mutations(&service, &plan, &response)
            .expect("apply batch response mutations");

        assert_eq!(
            outcome
                .member_outcomes
                .iter()
                .map(|outcome| outcome.state)
                .collect::<Vec<_>>(),
            vec![
                WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Completed,
                WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Completed,
            ]
        );
        assert_eq!(
            service
                .scheduler_task_orchestrator
                .active_task_lifecycle_handle_count()
                .expect("active task handle count"),
            0
        );
        let mut store = service.session_store_guard().expect("session store");
        for member in &plan.members {
            let results = store
                .active_run_scheduler_task_results(&member.session_id, &member.workflow_run_id)
                .expect("active run task results");
            assert!(results.iter().any(|result| {
                result.task_id == member.scheduler_task_id
                    && result.status == WorkflowSchedulerTaskResultStatus::Completed
            }));
        }
    }

    #[test]
    fn runtime_branch_batch_execution_owner_rejects_response_missing_member_before_mutation() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let members = active_batch_members(&service);
        let plan = owner
            .prepare_claimed_batch(&service, batch_claim_outcome(&members))
            .expect("claimed batch execution plan");
        let mut response = runtime_host_batch_response_from_plan(&plan);
        response.members.pop();

        let failure = owner
            .apply_batch_response_mutations(&service, &plan, &response)
            .expect_err("missing response member must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberResponseMissing
        );
        assert_eq!(
            service
                .scheduler_task_orchestrator
                .active_task_lifecycle_handle_count()
                .expect("active task handle count"),
            2
        );
        let mut store = service.session_store_guard().expect("session store");
        for member in &plan.members {
            let results = store
                .active_run_scheduler_task_results(&member.session_id, &member.workflow_run_id)
                .expect("active run task results");
            assert!(!results
                .iter()
                .any(|result| result.task_id == member.scheduler_task_id));
        }
    }

    #[test]
    fn runtime_branch_batch_execution_owner_fails_closed_when_member_lacks_task_attempt_fact() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let mut claim_outcome = batch_claim_outcome(&static_batch_members());
        claim_outcome.assignments[1].task_attempt_fact = None;

        let failure = owner
            .prepare_claimed_batch(&service, claim_outcome)
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
        let mut claim_outcome = batch_claim_outcome(&static_batch_members());
        claim_outcome.assignments[1]
            .task_attempt_fact
            .as_mut()
            .expect("second assignment task-attempt fact")
            .workflow_run_id = "run.unrelated".to_string();

        let failure = owner
            .prepare_claimed_batch(&service, claim_outcome)
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
    fn runtime_branch_batch_execution_owner_rejects_missing_active_run_state() {
        let service = WorkflowService::new();
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );

        let failure = owner
            .prepare_claimed_batch(&service, batch_claim_outcome(&static_batch_members()))
            .expect_err("missing active run state must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskStateUnavailable
        );
        assert_eq!(failure.member_outcomes.len(), 1);
        assert_eq!(
            failure.member_outcomes[0].assignment_id.as_str(),
            "assignment.1"
        );
        assert!(
            responder_fan_out.observed_assignment_ids().is_empty(),
            "fan-out must not be consulted before active-run state rehydration"
        );
    }

    #[test]
    fn runtime_branch_batch_execution_owner_rejects_stale_active_attempt_facts() {
        let service = WorkflowService::new();
        let members = active_batch_members(&service);
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );
        let mut claim_outcome = batch_claim_outcome(&members);
        claim_outcome.assignments[0].scheduler_task_attempt_id =
            "scheduler-task-attempt.stale".to_string();
        claim_outcome.assignments[0]
            .task_attempt_fact
            .as_mut()
            .expect("first assignment task-attempt fact")
            .scheduler_task_attempt_id = "scheduler-task-attempt.stale".to_string();

        let failure = owner
            .prepare_claimed_batch(&service, claim_outcome)
            .expect_err("stale scheduler attempt fact must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ActiveRunTaskAttemptMismatch
        );
        assert_eq!(failure.member_outcomes.len(), 1);
        assert_eq!(
            failure.member_outcomes[0].assignment_id.as_str(),
            "assignment.1"
        );
        assert!(
            responder_fan_out.observed_assignment_ids().is_empty(),
            "fan-out must not be consulted after active attempt mismatch"
        );
    }

    #[test]
    fn runtime_branch_batch_execution_owner_rejects_missing_materialized_member_inputs() {
        let service = WorkflowService::new();
        let members = active_batch_members(&service);
        {
            let mut store = service.session_store_guard().expect("session store");
            store
                .set_active_run_scheduler_task_results(
                    &members[0].session_id,
                    &members[0].workflow_run_id,
                    Vec::new(),
                )
                .expect("clear first member results");
        }
        let responder_fan_out = RecordingResponderFanOut::default();
        let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
            &service.scheduler_task_orchestrator,
            &responder_fan_out,
        );

        let failure = owner
            .prepare_claimed_batch(&service, batch_claim_outcome(&members))
            .expect_err("missing materialized input must fail closed");

        assert_eq!(
            failure.diagnostics[0].code,
            WorkflowRuntimeBranchBatchExecutionDiagnosticCode::RuntimeHostBatchMemberInputMappingInvalid
        );
        assert_eq!(failure.member_outcomes.len(), 1);
        assert_eq!(
            failure.member_outcomes[0].assignment_id.as_str(),
            "assignment.1"
        );
        assert!(
            responder_fan_out.observed_assignment_ids().is_empty(),
            "fan-out must not be consulted after input materialization fails"
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
        let members = active_batch_members(&service);

        let failure = owner
            .prepare_claimed_batch(&service, batch_claim_outcome(&members))
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
    struct DispatchAssignmentFixtureSpec {
        assignment_id: &'static str,
        runtime_branch_event_id: &'static str,
        workflow_run_id: &'static str,
        reservation_lease_id: &'static str,
    }

    impl DispatchAssignmentFixtureSpec {
        fn first() -> Self {
            Self {
                assignment_id: "assignment.1",
                runtime_branch_event_id:
                    "runtime-branch-task-event.run.2026-05-22.001.task.image_generation.001",
                workflow_run_id: "run.2026-05-22.001",
                reservation_lease_id: "reservation-lease.runtime.1",
            }
        }

        fn second() -> Self {
            Self {
                assignment_id: "assignment.2",
                runtime_branch_event_id:
                    "runtime-branch-task-event.run.2026-05-22.002.task.image_generation.001",
                workflow_run_id: "run.2026-05-22.002",
                reservation_lease_id: "reservation-lease.runtime.2",
            }
        }
    }

    #[derive(Debug, Clone)]
    struct DispatchAssignmentFixtureMember {
        assignment_id: String,
        runtime_branch_event_id: String,
        session_id: String,
        workflow_run_id: String,
        scheduler_task_attempt_id: String,
        scheduler_task_attempt_started_at_ms: u64,
        reservation_lease_id: String,
    }

    fn active_batch_members(service: &WorkflowService) -> Vec<DispatchAssignmentFixtureMember> {
        [
            DispatchAssignmentFixtureSpec::first(),
            DispatchAssignmentFixtureSpec::second(),
        ]
        .into_iter()
        .map(|spec| active_batch_member(service, &spec))
        .collect()
    }

    fn active_batch_member(
        service: &WorkflowService,
        spec: &DispatchAssignmentFixtureSpec,
    ) -> DispatchAssignmentFixtureMember {
        let prompt_text = format!("prompt owned by {}", spec.workflow_run_id);
        let session_id = prepare_active_runtime_run(service, spec.workflow_run_id, &prompt_text);
        let task_attempt = service
            .session_store_guard()
            .expect("session store")
            .active_run_scheduler_task_attempt_read_facts(&session_id, spec.workflow_run_id)
            .expect("task attempt facts")
            .get("task.image_generation.001")
            .expect("runtime task attempt")
            .clone();
        DispatchAssignmentFixtureMember {
            assignment_id: spec.assignment_id.to_string(),
            runtime_branch_event_id: spec.runtime_branch_event_id.to_string(),
            session_id,
            workflow_run_id: spec.workflow_run_id.to_string(),
            scheduler_task_attempt_id: task_attempt.attempt_id.as_str().to_string(),
            scheduler_task_attempt_started_at_ms: task_attempt.started_at_ms,
            reservation_lease_id: spec.reservation_lease_id.to_string(),
        }
    }

    fn static_batch_members() -> Vec<DispatchAssignmentFixtureMember> {
        vec![
            static_batch_member(
                &DispatchAssignmentFixtureSpec::first(),
                "session.image.1",
                "attempt.image.1",
                100,
            ),
            static_batch_member(
                &DispatchAssignmentFixtureSpec::second(),
                "session.image.2",
                "attempt.image.2",
                101,
            ),
        ]
    }

    fn static_batch_member(
        spec: &DispatchAssignmentFixtureSpec,
        session_id: &str,
        attempt_id: &str,
        started_at_ms: u64,
    ) -> DispatchAssignmentFixtureMember {
        DispatchAssignmentFixtureMember {
            assignment_id: spec.assignment_id.to_string(),
            runtime_branch_event_id: spec.runtime_branch_event_id.to_string(),
            session_id: session_id.to_string(),
            workflow_run_id: spec.workflow_run_id.to_string(),
            scheduler_task_attempt_id: attempt_id.to_string(),
            scheduler_task_attempt_started_at_ms: started_at_ms,
            reservation_lease_id: spec.reservation_lease_id.to_string(),
        }
    }

    fn batch_claim_outcome(
        members: &[DispatchAssignmentFixtureMember],
    ) -> WorkflowRuntimeDispatchAssignmentBatchClaimOutcome {
        let mut repository = InMemoryWorkflowRuntimeDispatchAssignmentRepository::new();
        let first = repository
            .create(assignment_request_for_member(&members[0]))
            .expect("first assignment");
        let second = repository
            .create(assignment_request_for_member(&members[1]))
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
        member: &DispatchAssignmentFixtureMember,
    ) -> WorkflowRuntimeDispatchAssignmentRequest {
        let readiness_proof = readiness_proof_for_member(member);
        let selected_candidate_fact = selected_candidate_fact_for_member(member);
        let reservation_lease_id = selected_candidate_fact.reservations[0]
            .reservation_lease_id
            .clone();
        let selected_candidate_id = Some(selected_candidate_fact.candidate_id.clone());
        WorkflowRuntimeDispatchAssignmentRequest {
            assignment_id: WorkflowRuntimeDispatchAssignmentId::parse(&member.assignment_id)
                .expect("assignment id"),
            runtime_branch_event_id: WorkflowRuntimeBranchTaskEventId::parse(
                &member.runtime_branch_event_id,
            )
            .expect("event id"),
            session_id: member.session_id.clone(),
            workflow_id: "workflow.image_generation".to_string(),
            workflow_run_id: member.workflow_run_id.clone(),
            scheduler_task_id: "task.image_generation.001".to_string(),
            scheduler_task_attempt_id: member.scheduler_task_attempt_id.clone(),
            scheduler_task_attempt_started_at_ms: member.scheduler_task_attempt_started_at_ms,
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

    fn prepare_active_runtime_run(
        service: &WorkflowService,
        workflow_run_id: &str,
        prompt_text: &str,
    ) -> String {
        let mut store = service.session_store_guard().expect("session store");
        let session_id = store
            .create_session(
                "workflow.image_generation".to_string(),
                None,
                None,
                vec!["pytorch".to_string()],
                vec!["stable-diffusion-xl".to_string()],
                true,
            )
            .expect("create session");
        let queued_run_id = store
            .enqueue_run_with_id(
                &session_id,
                &run_request(&session_id, prompt_text),
                workflow_run_id.to_string(),
            )
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &queued_run_id)
            .expect("begin run")
            .expect("dequeued run");
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                workflow_run_id,
                active_runtime_task_graph(workflow_run_id),
                vec![ready_runtime_task_record(workflow_run_id)],
            )
            .expect("set scheduler task state");
        store
            .set_active_run_scheduler_task_results(
                &session_id,
                workflow_run_id,
                vec![prompt_task_result(workflow_run_id, prompt_text)],
            )
            .expect("set scheduler task results");
        let _started = store
            .start_active_run_scheduler_task_attempt(
                &session_id,
                workflow_run_id,
                WorkflowSchedulerTaskAttemptId::new(),
                running_runtime_task_transition(workflow_run_id),
            )
            .expect("start runtime task attempt");
        session_id
    }

    fn run_request(session_id: &str, prompt_text: &str) -> WorkflowExecutionSessionRunRequest {
        WorkflowExecutionSessionRunRequest {
            session_id: session_id.to_string(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "prompt.input".to_string(),
                port_id: "text".to_string(),
                value: serde_json::Value::String(prompt_text.to_string()),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image.output".to_string(),
                port_id: "image".to_string(),
            }]),
            override_selection: None,
            timeout_ms: Some(30_000),
            priority: None,
        }
    }

    fn active_runtime_task_graph(workflow_run_id: &str) -> WorkflowSchedulerTaskGraph {
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            tasks: vec![
                WorkflowSchedulerTask {
                    workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                        .expect("workflow id"),
                    workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
                        .expect("run id"),
                    node_id: SchedulerNodeId::parse("prompt.input").expect("node id"),
                    task_id: SchedulerTaskId::parse("prompt.input").expect("task id"),
                    node_type: "source-input".to_string(),
                    execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
                    dependency_task_ids: Vec::new(),
                    input_bindings: Vec::new(),
                    schedulable_intent: None,
                    schedulable_intent_template: None,
                    non_runtime_task_template: None,
                    source_input_task_template: Some(WorkflowSchedulerSourceInputTemplate::Text {
                        port_id: "text".to_string(),
                    }),
                    inference_descriptor_fingerprint: None,
                    runtime_source_context: None,
                    diagnostics: Vec::new(),
                },
                WorkflowSchedulerTask {
                    workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                        .expect("workflow id"),
                    workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
                        .expect("run id"),
                    node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
                    task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
                    node_type: "llm-inference".to_string(),
                    execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                    dependency_task_ids: vec![
                        SchedulerTaskId::parse("prompt.input").expect("source task id")
                    ],
                    input_bindings: vec![WorkflowSchedulerTaskInputBinding {
                        source_node_id: SchedulerNodeId::parse("prompt.input")
                            .expect("source node id"),
                        source_task_id: SchedulerTaskId::parse("prompt.input")
                            .expect("source task id"),
                        source_port_id: "text".to_string(),
                        target_port_id: "prompt".to_string(),
                    }],
                    schedulable_intent: Some(task_intent_for_run(workflow_run_id)),
                    schedulable_intent_template: None,
                    non_runtime_task_template: None,
                    source_input_task_template: None,
                    inference_descriptor_fingerprint: None,
                    runtime_source_context: Some(runtime_source_context()),
                    diagnostics: Vec::new(),
                },
            ],
        }
    }

    fn ready_runtime_task_record(workflow_run_id: &str) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
            task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
            state: SchedulerTaskState::Ready {
                execution_intent: runtime_execution_intent(workflow_run_id),
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse(format!(
                "transition.ready.{workflow_run_id}"
            ))
            .expect("transition id"),
        }
    }

    fn running_runtime_task_transition(workflow_run_id: &str) -> SchedulerTaskStateTransition {
        SchedulerTaskStateTransition {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            transition_id: SchedulerTaskStateTransitionId::parse(format!(
                "transition.running.{workflow_run_id}"
            ))
            .expect("transition id"),
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
            task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
            expected_previous_state: Some(SchedulerTaskStateKind::Ready),
            next_state: SchedulerTaskState::Running {
                execution_intent: runtime_execution_intent(workflow_run_id),
            },
        }
    }

    fn runtime_execution_intent(workflow_run_id: &str) -> SchedulerTaskExecutionIntent {
        SchedulerTaskExecutionIntent::Runtime {
            task_intent: task_intent_for_run(workflow_run_id),
        }
    }

    fn prompt_task_result(workflow_run_id: &str, prompt_text: &str) -> WorkflowSchedulerTaskResult {
        WorkflowSchedulerTaskResult {
            schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            workflow_id: "workflow.image_generation".to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            node_id: "prompt.input".to_string(),
            task_id: "prompt.input".to_string(),
            status: WorkflowSchedulerTaskResultStatus::Completed,
            outputs: vec![WorkflowSchedulerTaskResultOutput {
                port_id: "text".to_string(),
                value: WorkflowSchedulerTaskResultValue::String(prompt_text.to_string()),
            }],
            diagnostics: Vec::new(),
            terminal_metadata: None,
        }
    }

    fn prompt_text_from_materialized_results(
        materialized_results: &[WorkflowSchedulerTaskResult],
    ) -> String {
        let result = materialized_results
            .iter()
            .find(|result| result.task_id == "prompt.input")
            .expect("prompt result");
        let output = result
            .outputs
            .iter()
            .find(|output| output.port_id == "text")
            .expect("prompt output");
        match &output.value {
            WorkflowSchedulerTaskResultValue::String(value) => value.clone(),
            other => panic!("unexpected prompt output value: {other:?}"),
        }
    }

    fn prompt_text_from_runtime_host_member_request(
        member: &RuntimeHostBatchExecutionMemberRequest,
    ) -> String {
        let input = member
            .materialized_inputs
            .iter()
            .find(|input| input.port_id == "prompt")
            .expect("prompt input");
        match &input.value {
            RuntimeHostExecutionInputValue::String(value) => value.clone(),
            other => panic!("unexpected runtime-host prompt input value: {other:?}"),
        }
    }

    fn runtime_host_batch_response_from_plan(
        plan: &WorkflowRuntimeBranchBatchExecutionPlan,
    ) -> RuntimeHostBatchExecutionResponse {
        RuntimeHostBatchExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: plan.batch_execution_request_id.clone(),
            state: RuntimeHostBatchExecutionState::Completed,
            diagnostics: Vec::new(),
            members: plan
                .runtime_host_request
                .members
                .iter()
                .map(|member| RuntimeHostBatchExecutionMemberResponse {
                    execution_request_id: member.execution_request_id.clone(),
                    assignment_id: member.assignment_id.clone(),
                    workflow_id: member.handoff.workflow_id.clone(),
                    workflow_run_id: member.handoff.workflow_run_id.clone(),
                    node_id: member.handoff.node_id.clone(),
                    task_id: member.handoff.task_id.clone(),
                    state: RuntimeHostBatchExecutionMemberState::Completed,
                    retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                    reservation_disposition:
                        RuntimeHostBatchMemberReservationDisposition::RetainedForRuntimeReuse,
                    outputs: Vec::new(),
                    diagnostics: Vec::new(),
                    terminal_metadata: None,
                })
                .collect(),
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
        member: &DispatchAssignmentFixtureMember,
        readiness_proof: DependencyReadinessProofEnvelope,
        reservation_lease_id: pantograph_scheduler::SchedulerReservationLeaseId,
    ) -> SchedulerRuntimeHandoff {
        let intent = task_intent_for_member(member);
        let environment_ref = environment_ref();
        let workflow_run_id =
            SchedulerWorkflowRunId::parse(&member.workflow_run_id).expect("run id");
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

    fn task_intent_for_member(member: &DispatchAssignmentFixtureMember) -> SchedulableTaskIntent {
        task_intent_for_run(&member.workflow_run_id)
    }

    fn task_intent_for_run(workflow_run_id: &str) -> SchedulableTaskIntent {
        SchedulableTaskIntent {
            contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image_generation")
                .expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
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
        member: &DispatchAssignmentFixtureMember,
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
                    &member.reservation_lease_id,
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
        member: &DispatchAssignmentFixtureMember,
    ) -> DependencyReadinessProofEnvelope {
        let mut proof = readiness_proof();
        proof.execution_context.workflow_run_id =
            DependencyReadinessWorkflowRunId::parse(&member.workflow_run_id)
                .expect("readiness workflow run id");
        proof.readiness_proof_id =
            DependencyReadinessProofId::parse(format!("readiness-proof.{}", member.assignment_id))
                .expect("readiness proof id");
        proof.validate().expect("readiness proof");
        proof
    }
}
