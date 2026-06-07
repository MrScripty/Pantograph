use pantograph_scheduler::SchedulerTaskStateRecord;

use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventRecord,
    WorkflowRuntimeBranchTaskEventState,
};
use super::{
    workflow_scheduler_task_run_summary, WorkflowExecutionSessionSummary,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskRunSummary, WorkflowService,
};
use crate::scheduler::WorkflowExecutionSessionActiveRunContext;

#[derive(Debug, Clone)]
#[must_use]
pub(super) struct WorkflowRuntimeBranchRehydratedContext {
    pub(super) session: WorkflowExecutionSessionSummary,
    pub(super) active_run: WorkflowExecutionSessionActiveRunContext,
    pub(super) task_graph: WorkflowSchedulerTaskGraph,
    pub(super) task_records: Vec<SchedulerTaskStateRecord>,
    pub(super) task_run_summary: WorkflowSchedulerTaskRunSummary,
    pub(super) runtime_task_id: String,
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

    Ok(WorkflowRuntimeBranchRehydratedContext {
        session,
        active_run,
        task_graph,
        task_records,
        task_run_summary,
        runtime_task_id: record.scheduler_task_id.clone(),
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

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerNodeId, SchedulerRuntimeDeviceConstraints,
        SchedulerTaskExecutionIntent, SchedulerTaskId, SchedulerTaskState,
        SchedulerTaskStateRecord, SchedulerTaskStateTransitionId, SchedulerWorkflowId,
        SchedulerWorkflowRunId, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };

    use super::super::runtime_branch_task_event::{
        WorkflowRuntimeBranchTaskEventClaimOwnerId, WorkflowRuntimeBranchTaskEventId,
        WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventRequest,
    };
    use super::super::{
        WorkflowExecutionSessionRunRequest, WorkflowOutputTarget, WorkflowPortBinding,
        WorkflowSchedulerTask, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    };
    use super::*;

    #[test]
    fn runtime_branch_rehydration_loads_backend_active_run_context() {
        let service = WorkflowService::new();
        let session_id = prepare_active_runtime_run(&service, "run.rehydrate", Some(750));
        let mut record = ready_runtime_branch_record(&session_id, "run.rehydrate", Some(750))
            .claim(owner_id(), 100, 1_000)
            .expect("event claims")
            .record;
        let claim = record.claim.clone().expect("claim present");

        let context =
            rehydrate_runtime_branch_execution_context(&service, &record, &claim).expect("context");

        assert_eq!(context.session.session_id, session_id);
        assert_eq!(context.active_run.workflow_id, "workflow-image-plan");
        assert_eq!(context.active_run.timeout_ms, Some(750));
        assert_eq!(context.task_graph.workflow_run_id.as_str(), "run.rehydrate");
        assert_eq!(context.task_records.len(), 1);
        assert_eq!(context.runtime_task_id, "image-task");
        assert!(context.task_run_summary.has_runtime_inference());

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
        let claimed = ready_runtime_branch_record(&session_id, "run.dispatching", Some(750))
            .claim(owner_id(), 100, 1_000)
            .expect("event claims");
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
            ready_at_ms: 1,
        })
        .expect("runtime branch task event")
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

    fn owner_id() -> WorkflowRuntimeBranchTaskEventClaimOwnerId {
        WorkflowRuntimeBranchTaskEventClaimOwnerId::parse("worker.rehydrate").expect("owner id")
    }
}
