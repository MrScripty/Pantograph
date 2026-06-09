use pantograph_scheduler::SchedulerTaskStateRecord;

use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventRecord,
    WorkflowRuntimeBranchTaskEventState,
};
use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;
use super::runtime_task_attempt_fact::{
    WorkflowRuntimeTaskAttemptFactDiagnostic, WorkflowRuntimeTaskAttemptSourceContext,
    WorkflowRuntimeTaskAttemptSourceContextRequest,
};
use super::{
    workflow_scheduler_task_run_summary, WorkflowExecutionSessionSummary,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskRunSummary, WorkflowService,
};
use crate::scheduler::{
    WorkflowExecutionSessionActiveRunContext, WorkflowSchedulerTaskAttemptId,
    WorkflowSchedulerTaskAttemptReadFact,
};

#[derive(Debug, Clone)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchRehydratedContext {
    pub(super) session: WorkflowExecutionSessionSummary,
    pub(super) active_run: WorkflowExecutionSessionActiveRunContext,
    pub(super) task_graph: WorkflowSchedulerTaskGraph,
    pub(super) task_records: Vec<SchedulerTaskStateRecord>,
    pub(super) task_run_summary: WorkflowSchedulerTaskRunSummary,
    pub(super) runtime_task_id: String,
    pub(super) task_attempt_source_context: WorkflowRuntimeTaskAttemptSourceContext,
    pub(super) scheduler_task_attempt_id: WorkflowSchedulerTaskAttemptId,
    pub(super) scheduler_task_attempt_started_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchRehydrationDiagnostic {
    pub(super) code: WorkflowRuntimeBranchRehydrationDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeBranchRehydrationDiagnosticCode {
    ClaimMismatch,
    ActiveRunUnavailable,
    TaskStateUnavailable,
    TaskRunSummaryInvalid,
    RuntimeTaskUnavailable,
    TaskAttemptUnavailable,
    TaskAttemptSourceContextInvalid,
    CorrelationMismatch,
}

pub(super) fn rehydrate_runtime_branch_execution_context(
    service: &WorkflowService,
    record: &WorkflowRuntimeBranchTaskEventRecord,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
) -> Result<WorkflowRuntimeBranchRehydratedContext, WorkflowRuntimeBranchRehydrationDiagnostic> {
    validate_claimed_event(record, claim)?;

    let store = service.session_store_guard().map_err(|error| {
        WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::ActiveRunUnavailable,
            format!("runtime branch rehydration could not read session store: {error}"),
        )
    })?;
    let session = store.session_summary(&record.session_id).map_err(|error| {
        WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::ActiveRunUnavailable,
            format!("runtime branch session summary is unavailable: {error}"),
        )
    })?;
    let active_run = store
        .active_run_context(&record.session_id, &record.workflow_run_id)
        .map_err(|error| {
            WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::ActiveRunUnavailable,
                format!("runtime branch active run context is unavailable: {error}"),
            )
        })?;
    validate_active_run_correlation(record, &session, &active_run)?;

    let (task_graph, task_records) = store
        .active_run_scheduler_task_state(&record.session_id, &record.workflow_run_id)
        .map_err(|error| {
            WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskStateUnavailable,
                format!("runtime branch scheduler task state is unavailable: {error}"),
            )
        })?
        .ok_or_else(|| {
            WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskStateUnavailable,
                "runtime branch active run has no scheduler task state",
            )
        })?;
    validate_runtime_task(record, &task_graph)?;
    let task_run_summary = workflow_scheduler_task_run_summary(&task_graph, &task_records)
        .map_err(|error| {
            WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskRunSummaryInvalid,
                format!("runtime branch scheduler task run summary is invalid: {error}"),
            )
        })?;
    let task_attempt_facts = store
        .active_run_scheduler_task_attempt_read_facts(&record.session_id, &record.workflow_run_id)
        .map_err(|error| {
            WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptUnavailable,
                format!("runtime branch scheduler task attempt facts are unavailable: {error}"),
            )
        })?;
    let task_attempt = task_attempt_facts
        .get(&record.scheduler_task_id)
        .ok_or_else(|| {
            WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptUnavailable,
                "runtime branch scheduler task has no active attempt fact",
            )
        })?;
    validate_scheduler_task_attempt_correlation(record, task_attempt)?;
    let task_attempt_source_context = rehydrate_task_attempt_source_context(record)?;

    Ok(WorkflowRuntimeBranchRehydratedContext {
        session,
        active_run,
        task_graph,
        task_records,
        task_run_summary,
        runtime_task_id: record.scheduler_task_id.clone(),
        task_attempt_source_context,
        scheduler_task_attempt_id: task_attempt.attempt_id.clone(),
        scheduler_task_attempt_started_at_ms: task_attempt.started_at_ms,
    })
}

impl WorkflowRuntimeBranchRehydrationDiagnostic {
    fn new(
        code: WorkflowRuntimeBranchRehydrationDiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

fn validate_claimed_event(
    record: &WorkflowRuntimeBranchTaskEventRecord,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
) -> Result<(), WorkflowRuntimeBranchRehydrationDiagnostic> {
    if !matches!(
        record.state,
        WorkflowRuntimeBranchTaskEventState::Claimed
            | WorkflowRuntimeBranchTaskEventState::Dispatching
    ) {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::ClaimMismatch,
            "runtime branch rehydration requires an active claimed or dispatching task event",
        ));
    }
    if record.claim.as_ref() != Some(claim) {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::ClaimMismatch,
            "runtime branch rehydration claim does not match the claimed task event",
        ));
    }
    Ok(())
}

fn validate_active_run_correlation(
    record: &WorkflowRuntimeBranchTaskEventRecord,
    session: &WorkflowExecutionSessionSummary,
    active_run: &WorkflowExecutionSessionActiveRunContext,
) -> Result<(), WorkflowRuntimeBranchRehydrationDiagnostic> {
    if session.workflow_id != record.workflow_id || active_run.workflow_id != record.workflow_id {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
            "runtime branch event workflow id does not match backend session or active run",
        ));
    }
    if active_run.timeout_ms != record.timeout_ms {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
            "runtime branch event timeout does not match backend active run timeout",
        ));
    }
    if active_run.output_targets != record.output_targets {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
            "runtime branch event output targets do not match backend active run targets",
        ));
    }
    Ok(())
}

fn validate_runtime_task(
    record: &WorkflowRuntimeBranchTaskEventRecord,
    task_graph: &WorkflowSchedulerTaskGraph,
) -> Result<(), WorkflowRuntimeBranchRehydrationDiagnostic> {
    let Some(task) = task_graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == record.scheduler_task_id)
    else {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::RuntimeTaskUnavailable,
            "runtime branch event scheduler task id is not present in backend task graph",
        ));
    };
    if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::RuntimeTaskUnavailable,
            "runtime branch event scheduler task is not a runtime inference task",
        ));
    }
    Ok(())
}

fn validate_scheduler_task_attempt_correlation(
    record: &WorkflowRuntimeBranchTaskEventRecord,
    task_attempt: &WorkflowSchedulerTaskAttemptReadFact,
) -> Result<(), WorkflowRuntimeBranchRehydrationDiagnostic> {
    if let Some(expected_attempt_id) = &record.scheduler_task_attempt_id {
        if expected_attempt_id != task_attempt.attempt_id.as_str() {
            return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
                "runtime branch event scheduler task attempt id does not match active scheduler attempt",
            ));
        }
    }
    if task_attempt.started_at_ms == 0 {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptUnavailable,
            "runtime branch scheduler task active attempt has invalid start timestamp",
        ));
    }
    Ok(())
}

fn rehydrate_task_attempt_source_context(
    record: &WorkflowRuntimeBranchTaskEventRecord,
) -> Result<WorkflowRuntimeTaskAttemptSourceContext, WorkflowRuntimeBranchRehydrationDiagnostic> {
    let runtime_branch_profile = record.batch_eligibility.clone().ok_or_else(|| {
        WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptSourceContextInvalid,
            "runtime branch task event is missing batch eligibility profile for task-attempt facts",
        )
    })?;
    let selected_candidate_fact = record.selected_candidate_fact.clone().ok_or_else(|| {
        WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptSourceContextInvalid,
            "runtime branch task event is missing selected candidate evidence for task-attempt facts",
        )
    })?;
    validate_selected_candidate_correlation(record, &selected_candidate_fact)?;

    WorkflowRuntimeTaskAttemptSourceContext::new(WorkflowRuntimeTaskAttemptSourceContextRequest {
        workflow_id: record.workflow_id.clone(),
        workflow_run_id: record.workflow_run_id.clone(),
        scheduler_task_id: record.scheduler_task_id.clone(),
        task_attempt_generation: record.attempt_generation,
        timeout_ms: record.timeout_ms,
        runtime_branch_profile,
        selected_candidate_fact,
    })
    .map_err(task_attempt_source_context_diagnostic)
}

fn validate_selected_candidate_correlation(
    record: &WorkflowRuntimeBranchTaskEventRecord,
    fact: &WorkflowRuntimeDispatchCandidateFact,
) -> Result<(), WorkflowRuntimeBranchRehydrationDiagnostic> {
    if fact.resource_fit_assessment.workflow_run_id.as_str() != record.workflow_run_id {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
            "runtime branch selected candidate resource-fit workflow run does not match task event",
        ));
    }
    if fact.resource_fit_assessment.task_id.as_str() != record.scheduler_task_id {
        return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
            "runtime branch selected candidate resource-fit task id does not match task event",
        ));
    }
    for reservation in &fact.reservations {
        if reservation.workflow_run_id.as_str() != record.workflow_run_id {
            return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
                "runtime branch selected candidate reservation workflow run does not match task event",
            ));
        }
        if reservation.task_id.as_str() != record.scheduler_task_id {
            return Err(WorkflowRuntimeBranchRehydrationDiagnostic::new(
                WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch,
                "runtime branch selected candidate reservation task id does not match task event",
            ));
        }
    }
    Ok(())
}

fn task_attempt_source_context_diagnostic(
    error: WorkflowRuntimeTaskAttemptFactDiagnostic,
) -> WorkflowRuntimeBranchRehydrationDiagnostic {
    WorkflowRuntimeBranchRehydrationDiagnostic::new(
        WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptSourceContextInvalid,
        format!(
            "runtime branch task-attempt source context is invalid: {}",
            error.message
        ),
    )
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{
        DependencyEnvironmentId, DependencyEnvironmentRef, DependencyTaskId, DeviceIntentId,
        PumasModelRef,
    };
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerDispatchCandidateId, SchedulerNodeId,
        SchedulerReservationLeaseId, SchedulerResourceFitAssessment, SchedulerResourceFitState,
        SchedulerResourceKind, SchedulerResourceReservation, SchedulerRuntimeDeviceConstraints,
        SchedulerRuntimeVariantId, SchedulerTaskExecutionIntent, SchedulerTaskId,
        SchedulerTaskState, SchedulerTaskStateKind, SchedulerTaskStateRecord,
        SchedulerTaskStateTransition, SchedulerTaskStateTransitionId, SchedulerWorkflowId,
        SchedulerWorkflowRunId, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };

    use crate::scheduler::WorkflowSchedulerTaskAttemptId;

    use super::super::runtime_branch_task_event::{
        WorkflowRuntimeBranchBatchEligibilityProfile, WorkflowRuntimeBranchTaskEventClaimOwnerId,
        WorkflowRuntimeBranchTaskEventId, WorkflowRuntimeBranchTaskEventRecord,
        WorkflowRuntimeBranchTaskEventRequest,
    };
    use super::super::runtime_dispatch_selection::WorkflowRuntimeDispatchLoadState;
    use super::super::{
        WorkflowExecutionSessionRunRequest, WorkflowOutputTarget, WorkflowPortBinding,
        WorkflowSchedulerTask, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    };
    use super::*;

    #[test]
    fn runtime_branch_rehydration_loads_backend_active_run_context() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.rehydrate", Some(750));
        let claimed = claimed_runtime_branch_record(&session_id, "run.rehydrate", Some(750), 100);
        let mut record = claimed.record;
        let claim = claimed.claim;

        let context =
            rehydrate_runtime_branch_execution_context(&service, &record, &claim).expect("context");

        assert_eq!(context.session.session_id, session_id);
        assert_eq!(context.active_run.workflow_id, "workflow-image-plan");
        assert_eq!(context.active_run.timeout_ms, Some(750));
        assert_eq!(context.task_graph.workflow_run_id.as_str(), "run.rehydrate");
        assert_eq!(context.task_records.len(), 1);
        assert_eq!(context.runtime_task_id, "image-task");
        assert!(context.task_run_summary.has_runtime_inference());
        assert_eq!(
            context.task_attempt_source_context.workflow_id,
            "workflow-image-plan"
        );
        assert_eq!(
            context.task_attempt_source_context.workflow_run_id,
            "run.rehydrate"
        );
        assert_eq!(
            context.task_attempt_source_context.scheduler_task_id,
            "image-task"
        );
        assert_eq!(
            context.task_attempt_source_context.task_attempt_generation,
            1
        );
        assert_eq!(
            context
                .task_attempt_source_context
                .selected_candidate_fact
                .candidate_id
                .as_str(),
            "candidate.diffusers.cuda0"
        );
        assert!(context
            .scheduler_task_attempt_id
            .as_str()
            .starts_with("scheduler-task-attempt."));
        assert!(context.scheduler_task_attempt_started_at_ms > 0);

        record.timeout_ms = Some(1_000);
        let diagnostic = rehydrate_runtime_branch_execution_context(&service, &record, &claim)
            .expect_err("timeout mismatch fails");
        assert_eq!(
            diagnostic.code,
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch
        );
    }

    #[test]
    fn runtime_branch_rehydration_accepts_dispatching_task_event() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.dispatching", Some(750));
        let claimed = claimed_runtime_branch_record(&session_id, "run.dispatching", Some(750), 100);
        let dispatching = claimed
            .record
            .mark_dispatching(&claimed.claim, 110)
            .expect("event marks dispatching");

        let context =
            rehydrate_runtime_branch_execution_context(&service, &dispatching, &claimed.claim)
                .expect("dispatching context");

        assert_eq!(context.session.session_id, session_id);
        assert_eq!(context.active_run.workflow_id, "workflow-image-plan");
        assert_eq!(context.runtime_task_id, "image-task");
        assert_eq!(
            context.task_attempt_source_context.workflow_run_id,
            "run.dispatching"
        );
    }

    #[test]
    fn runtime_branch_rehydration_rejects_missing_task_state() {
        let service = WorkflowService::new();
        let session_id = prepare_active_run_without_task_state(&service, "run.no-state", None);
        let claimed = ready_runtime_branch_record(&session_id, "run.no-state", None)
            .claim(owner_id(), 100, 1_000)
            .expect("event claims");

        let diagnostic =
            rehydrate_runtime_branch_execution_context(&service, &claimed.record, &claimed.claim)
                .expect_err("missing state fails");

        assert_eq!(
            diagnostic.code,
            WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskStateUnavailable
        );
    }

    #[test]
    fn runtime_branch_rehydration_rejects_missing_selected_candidate_evidence() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.no-evidence", Some(750));
        let claimed = ready_runtime_branch_record(&session_id, "run.no-evidence", Some(750))
            .claim(owner_id(), 100, 1_000)
            .expect("event claims");

        let diagnostic =
            rehydrate_runtime_branch_execution_context(&service, &claimed.record, &claimed.claim)
                .expect_err("missing selected candidate evidence fails");

        assert_eq!(
            diagnostic.code,
            WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptSourceContextInvalid
        );
        assert!(
            diagnostic.message.contains("selected candidate evidence"),
            "unexpected diagnostic: {}",
            diagnostic.message
        );
    }

    #[test]
    fn runtime_branch_rehydration_rejects_mismatched_scheduler_attempt_id() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.attempt-mismatch", Some(750));
        let claimed =
            claimed_runtime_branch_record(&session_id, "run.attempt-mismatch", Some(750), 100);
        let mut record = claimed.record;
        record.scheduler_task_attempt_id = Some("scheduler-task-attempt.other".to_string());

        let diagnostic =
            rehydrate_runtime_branch_execution_context(&service, &record, &claimed.claim)
                .expect_err("attempt id mismatch fails");

        assert_eq!(
            diagnostic.code,
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch
        );
        assert!(
            diagnostic.message.contains("scheduler task attempt id"),
            "unexpected diagnostic: {}",
            diagnostic.message
        );
    }

    #[test]
    fn runtime_branch_rehydration_rejects_mismatched_selected_candidate_task_scope() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.candidate-mismatch", Some(750));
        let claimed = ready_runtime_branch_record(&session_id, "run.candidate-mismatch", Some(750))
            .claim(owner_id(), 100, 1_000)
            .expect("event claims");
        let mut selected_candidate_fact = selected_candidate_fact("run.other");
        selected_candidate_fact.resource_fit_assessment.task_id =
            SchedulerTaskId::parse("image-task").expect("task id");
        let record = claimed
            .record
            .record_selected_candidate_fact(&claimed.claim, selected_candidate_fact)
            .expect("profile-compatible candidate evidence records");

        let diagnostic =
            rehydrate_runtime_branch_execution_context(&service, &record, &claimed.claim)
                .expect_err("candidate workflow run mismatch fails");

        assert_eq!(
            diagnostic.code,
            WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch
        );
        assert!(
            diagnostic.message.contains("resource-fit workflow run"),
            "unexpected diagnostic: {}",
            diagnostic.message
        );
    }

    #[test]
    fn runtime_branch_rehydration_requires_current_claim() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.claim", None);
        let claimed = ready_runtime_branch_record(&session_id, "run.claim", None)
            .claim(owner_id(), 100, 1_000)
            .expect("event claims");
        let other_claim = ready_runtime_branch_record(&session_id, "run.claim", None)
            .claim(
                WorkflowRuntimeBranchTaskEventClaimOwnerId::parse("worker.other")
                    .expect("owner id"),
                100,
                1_000,
            )
            .expect("other event claims")
            .claim;

        let diagnostic =
            rehydrate_runtime_branch_execution_context(&service, &claimed.record, &other_claim)
                .expect_err("stale claim fails");

        assert_eq!(
            diagnostic.code,
            WorkflowRuntimeBranchRehydrationDiagnosticCode::ClaimMismatch
        );
    }

    fn prepare_active_runtime_run(
        service: &WorkflowService,
        workflow_run_id: &str,
        timeout_ms: Option<u64>,
    ) -> String {
        let session_id =
            prepare_active_run_without_task_state(service, workflow_run_id, timeout_ms);
        let mut store = service.session_store_guard().expect("session store");
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                workflow_run_id,
                scheduler_task_graph(workflow_run_id),
                vec![scheduler_record(workflow_run_id)],
            )
            .expect("set task state");
        let (_apply_result, _attempt_id, _started_at_ms) = store
            .start_active_run_scheduler_task_attempt(
                &session_id,
                workflow_run_id,
                WorkflowSchedulerTaskAttemptId::new(),
                running_transition(workflow_run_id),
            )
            .expect("start scheduler attempt");
        session_id
    }

    fn prepare_active_run_without_task_state(
        service: &WorkflowService,
        workflow_run_id: &str,
        timeout_ms: Option<u64>,
    ) -> String {
        let mut store = service.session_store_guard().expect("session store");
        let session_id = store
            .create_session(
                "workflow-image-plan".to_string(),
                None,
                None,
                vec!["pytorch".to_string()],
                vec!["stable-diffusion-xl".to_string()],
                true,
            )
            .expect("create session");
        let mut request = run_request(&session_id);
        request.timeout_ms = timeout_ms;
        let queued_run_id = store
            .enqueue_run_with_id(&session_id, &request, workflow_run_id.to_string())
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &queued_run_id)
            .expect("begin run")
            .expect("dequeued run");
        session_id
    }

    fn run_request(session_id: &str) -> WorkflowExecutionSessionRunRequest {
        WorkflowExecutionSessionRunRequest {
            session_id: session_id.to_string(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: vec![WorkflowPortBinding {
                node_id: "prompt".to_string(),
                port_id: "text".to_string(),
                value: serde_json::Value::String("a mountain at sunset".to_string()),
            }],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
            }]),
            override_selection: None,
            timeout_ms: None,
            priority: None,
        }
    }

    fn ready_runtime_branch_record(
        session_id: &str,
        workflow_run_id: &str,
        timeout_ms: Option<u64>,
    ) -> WorkflowRuntimeBranchTaskEventRecord {
        WorkflowRuntimeBranchTaskEventRecord::ready(WorkflowRuntimeBranchTaskEventRequest {
            event_id: WorkflowRuntimeBranchTaskEventId::parse(format!(
                "runtime-branch-task-event.{workflow_run_id}.image-task"
            ))
            .expect("event id"),
            session_id: session_id.to_string(),
            workflow_id: "workflow-image-plan".to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            scheduler_task_id: "image-task".to_string(),
            scheduler_task_attempt_id: None,
            attempt_generation: 1,
            queued_input_keys: vec!["prompt:text".to_string()],
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
            }]),
            timeout_ms,
            batching_key: Some("runtime-branch-task.workflow-image-plan.image-task".to_string()),
            batch_eligibility: Some(batch_profile()),
            ready_at_ms: 1,
        })
        .expect("runtime branch task event")
    }

    fn claimed_runtime_branch_record(
        session_id: &str,
        workflow_run_id: &str,
        timeout_ms: Option<u64>,
        claimed_at_ms: u64,
    ) -> super::super::runtime_branch_task_event::WorkflowRuntimeBranchTaskEventClaimOutcome {
        let claimed = ready_runtime_branch_record(session_id, workflow_run_id, timeout_ms)
            .claim(owner_id(), claimed_at_ms, 1_000)
            .expect("event claims");
        let selected_candidate_fact = selected_candidate_fact(workflow_run_id);
        let record = claimed
            .record
            .record_selected_candidate_fact(&claimed.claim, selected_candidate_fact)
            .expect("selected candidate fact records");
        super::super::runtime_branch_task_event::WorkflowRuntimeBranchTaskEventClaimOutcome {
            record,
            claim: claimed.claim,
        }
    }

    fn scheduler_task_graph(workflow_run_id: &str) -> WorkflowSchedulerTaskGraph {
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            tasks: vec![WorkflowSchedulerTask {
                workflow_id: SchedulerWorkflowId::parse("workflow-image-plan")
                    .expect("workflow id"),
                workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
                node_id: SchedulerNodeId::parse("image-task").expect("node id"),
                task_id: SchedulerTaskId::parse("image-task").expect("task id"),
                node_type: "llm-inference".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: None,
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: None,
                inference_descriptor_fingerprint: None,
                diagnostics: Vec::new(),
            }],
        }
    }

    fn scheduler_record(workflow_run_id: &str) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            task_id: SchedulerTaskId::parse("image-task").expect("task id"),
            state: SchedulerTaskState::Ready {
                execution_intent: SchedulerTaskExecutionIntent::Runtime {
                    task_intent: task_intent(workflow_run_id),
                },
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse("transition.ready")
                .expect("transition id"),
        }
    }

    fn running_transition(workflow_run_id: &str) -> SchedulerTaskStateTransition {
        SchedulerTaskStateTransition {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            transition_id: SchedulerTaskStateTransitionId::parse(format!(
                "transition.running.{workflow_run_id}"
            ))
            .expect("transition id"),
            workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            task_id: SchedulerTaskId::parse("image-task").expect("task id"),
            expected_previous_state: Some(SchedulerTaskStateKind::Ready),
            next_state: SchedulerTaskState::Running {
                execution_intent: SchedulerTaskExecutionIntent::Runtime {
                    task_intent: task_intent(workflow_run_id),
                },
            },
        }
    }

    fn task_intent(workflow_run_id: &str) -> SchedulableTaskIntent {
        SchedulableTaskIntent {
            contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            task_id: SchedulerTaskId::parse("image-task").expect("task id"),
            fairness_key: None,
            task_type: DependencyTaskId::parse("image_generation").expect("task type"),
            model_ref: PumasModelRef {
                model_id: "image/example/tiny-diffusion".to_string(),
                revision: None,
                selected_artifact_id: Some("diffusers-bundle".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            constraints: SchedulerRuntimeDeviceConstraints::default(),
            trait_settings: Vec::new(),
            dependency_override_patches: Vec::new(),
            estimate_hints: Vec::new(),
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

    fn selected_candidate_fact(workflow_run_id: &str) -> WorkflowRuntimeDispatchCandidateFact {
        let workflow_run_id: SchedulerWorkflowRunId =
            workflow_run_id.parse().expect("workflow run id");
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

    fn owner_id() -> WorkflowRuntimeBranchTaskEventClaimOwnerId {
        WorkflowRuntimeBranchTaskEventClaimOwnerId::parse("worker.rehydrate").expect("owner id")
    }
}
