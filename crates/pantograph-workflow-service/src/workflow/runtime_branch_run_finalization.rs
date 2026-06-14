use std::time::Instant;

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    SchedulerTaskAttemptExecutionClass, SchedulerTaskAttemptLifecycleChangedPayload,
    SchedulerTaskAttemptLifecycleTransition,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId,
};
use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord};

use crate::scheduler::{unix_timestamp_ms, WorkflowSchedulerTaskTerminalMutation};

use super::io_contract::validate_workflow_io;
use super::validation::{
    validate_host_output_bindings, validate_output_targets_against_io,
    validate_requested_outputs_produced,
};
use super::{
    project_scheduler_task_results_to_outputs, WorkflowHost, WorkflowOutputTarget,
    WorkflowRunResponse, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph, WorkflowService, WorkflowServiceError,
};
use crate::scheduler::task_orchestrator::SelectedRuntimeTaskDispatch;

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowSchedulerTaskAttemptDiagnosticAttribution {
    pub(super) client_id: Option<ClientId>,
    pub(super) client_session_id: Option<ClientSessionId>,
    pub(super) bucket_id: Option<BucketId>,
}

#[derive(Debug)]
pub(super) struct WorkflowSchedulerTaskAttemptTerminalDiagnosticRequest<'a> {
    pub(super) task: &'a WorkflowSchedulerTask,
    pub(super) attempt_id: &'a str,
    pub(super) started_at_ms: u64,
    pub(super) transition: SchedulerTaskAttemptLifecycleTransition,
    pub(super) reason: &'a str,
    pub(super) error_summary: Option<String>,
    pub(super) selected_dispatch: Option<&'a SelectedRuntimeTaskDispatch>,
    pub(super) terminal_mutation: Option<&'a WorkflowSchedulerTaskTerminalMutation>,
    pub(super) attribution: WorkflowSchedulerTaskAttemptDiagnosticAttribution,
}

impl WorkflowSchedulerTaskAttemptDiagnosticAttribution {
    pub(super) fn none() -> Self {
        Self {
            client_id: None,
            client_session_id: None,
            bucket_id: None,
        }
    }
}

pub(super) fn scheduler_task_attempt_terminal_diagnostic_event(
    request: WorkflowSchedulerTaskAttemptTerminalDiagnosticRequest<'_>,
) -> Result<DiagnosticEventAppendRequest, WorkflowServiceError> {
    let ended_at_ms = unix_timestamp_ms();
    scheduler_task_attempt_terminal_diagnostic_event_at(request, ended_at_ms)
}

pub(super) async fn completed_scheduler_run_response<H: WorkflowHost + ?Sized>(
    service: &WorkflowService,
    host: &H,
    session_id: &str,
    workflow_run_id: &str,
    workflow_id: &str,
    output_targets: Option<&[WorkflowOutputTarget]>,
    started_at: Instant,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    let (task_graph, records) =
        active_run_scheduler_task_state_required(service, session_id, workflow_run_id)?;
    ensure_all_scheduler_tasks_completed(&records)?;
    let results = {
        let mut store = service.session_store_guard()?;
        store.active_run_scheduler_task_results(session_id, workflow_run_id)?
    };
    let targets = scheduler_output_targets_for_run(host, workflow_id, output_targets).await?;
    let outputs = project_scheduler_task_results_to_outputs(&task_graph, &results, &targets)
        .map_err(|error| {
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler task output projection failed: {error}"
            ))
        })?;
    validate_host_output_bindings(&outputs, "outputs")?;
    validate_requested_outputs_produced(&targets, &outputs)?;
    Ok(WorkflowRunResponse {
        workflow_run_id: workflow_run_id.to_string(),
        outputs,
        timing_ms: started_at.elapsed().as_millis(),
    })
}

fn scheduler_task_attempt_terminal_diagnostic_event_at(
    request: WorkflowSchedulerTaskAttemptTerminalDiagnosticRequest<'_>,
    ended_at_ms: u64,
) -> Result<DiagnosticEventAppendRequest, WorkflowServiceError> {
    let duration_ms = ended_at_ms
        .checked_sub(request.started_at_ms)
        .ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "scheduler task '{}' terminal time preceded attempt start time",
                request.task.task_id.as_str()
            ))
        })?;
    let started_at_ms = i64::try_from(request.started_at_ms).map_err(|_| {
        WorkflowServiceError::Internal(format!(
            "scheduler task '{}' start time exceeded diagnostics ledger timestamp range",
            request.task.task_id.as_str()
        ))
    })?;
    let ended_at_ms = i64::try_from(ended_at_ms).map_err(|_| {
        WorkflowServiceError::Internal(format!(
            "scheduler task '{}' terminal time exceeded diagnostics ledger timestamp range",
            request.task.task_id.as_str()
        ))
    })?;
    let selected_decision = request
        .selected_dispatch
        .and_then(|dispatch| dispatch.dispatch_decision());
    let selected_runtime_id =
        selected_decision.map(|dispatch| dispatch.selected_runtime_id.as_str().to_string());
    let selected_runtime_variant_id = selected_decision.and_then(|dispatch| {
        dispatch
            .selected_runtime_variant_id
            .as_ref()
            .map(|runtime_variant_id| runtime_variant_id.as_str().to_string())
    });
    let selected_device_id = selected_decision.and_then(|dispatch| {
        dispatch
            .selected_device_ids
            .first()
            .map(|device_id| device_id.as_str().to_string())
    });
    let reservation_id =
        reservation_id_from_terminal_context(request.terminal_mutation, request.selected_dispatch);

    Ok(DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::Scheduler,
        source_instance_id: Some("workflow-session-scheduler".to_string()),
        occurred_at_ms: ended_at_ms,
        workflow_run_id: Some(WorkflowRunId::try_from(
            request.task.workflow_run_id.as_str().to_string(),
        )?),
        workflow_id: Some(WorkflowId::try_from(
            request.task.workflow_id.as_str().to_string(),
        )?),
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: Some(request.task.node_id.as_str().to_string()),
        node_type: Some(request.task.node_type.clone()),
        node_version: None,
        runtime_id: selected_runtime_id.clone(),
        runtime_version: None,
        model_id: None,
        model_version: None,
        client_id: request.attribution.client_id,
        client_session_id: request.attribution.client_session_id,
        bucket_id: request.attribution.bucket_id,
        scheduler_policy_id: Some("priority_then_fifo".to_string()),
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::SchedulerTaskAttemptLifecycleChanged(
            SchedulerTaskAttemptLifecycleChangedPayload {
                scheduler_task_id: request.task.task_id.as_str().to_string(),
                scheduler_attempt_id: request.attempt_id.to_string(),
                execution_class: scheduler_task_attempt_execution_class(request.task)?,
                transition: request.transition,
                started_at_ms: Some(started_at_ms),
                ended_at_ms: Some(ended_at_ms),
                duration_ms: Some(duration_ms),
                selected_runtime_id,
                selected_runtime_variant_id,
                selected_backend_key: None,
                selected_device_class: None,
                selected_device_id,
                selected_network_node_id: None,
                reservation_id,
                reason: Some(request.reason.to_string()),
                error_summary: request.error_summary,
                canonical_error_event_id: None,
            },
        ),
    })
}

fn active_run_scheduler_task_state_required(
    service: &WorkflowService,
    session_id: &str,
    workflow_run_id: &str,
) -> Result<(WorkflowSchedulerTaskGraph, Vec<SchedulerTaskStateRecord>), WorkflowServiceError> {
    let store = service.session_store_guard()?;
    store
        .active_run_scheduler_task_state(session_id, workflow_run_id)?
        .ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "active workflow run '{}' has no scheduler task state",
                workflow_run_id
            ))
        })
}

fn ensure_all_scheduler_tasks_completed(
    records: &[SchedulerTaskStateRecord],
) -> Result<(), WorkflowServiceError> {
    if let Some(record) = records
        .iter()
        .find(|record| record.state.kind() != SchedulerTaskStateKind::Completed)
    {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' did not complete; final state was {:?}",
            record.task_id.as_str(),
            record.state.kind()
        )));
    }
    Ok(())
}

async fn scheduler_output_targets_for_run<H: WorkflowHost + ?Sized>(
    host: &H,
    workflow_id: &str,
    output_targets: Option<&[WorkflowOutputTarget]>,
) -> Result<Vec<WorkflowOutputTarget>, WorkflowServiceError> {
    let io = host.workflow_io(workflow_id).await?;
    validate_workflow_io(&io)?;
    if let Some(targets) = output_targets {
        validate_output_targets_against_io(targets, &io)?;
        return Ok(targets.to_vec());
    }
    Ok(io
        .outputs
        .iter()
        .flat_map(|node| {
            node.ports.iter().map(|port| WorkflowOutputTarget {
                node_id: node.node_id.clone(),
                port_id: port.port_id.clone(),
            })
        })
        .collect())
}

fn reservation_id_from_terminal_context(
    terminal_mutation: Option<&WorkflowSchedulerTaskTerminalMutation>,
    selected_dispatch: Option<&SelectedRuntimeTaskDispatch>,
) -> Option<String> {
    terminal_mutation
        .and_then(|mutation| mutation.reservation_release_intent.as_ref())
        .map(|release_intent| release_intent.reservation_lease_id.as_str().to_string())
        .or_else(|| {
            selected_dispatch.map(|dispatch| dispatch.reservation_lease_id().as_str().to_string())
        })
}

fn scheduler_task_attempt_execution_class(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskAttemptExecutionClass, WorkflowServiceError> {
    match task.execution_class {
        WorkflowSchedulerTaskExecutionClass::RuntimeInference => {
            Ok(SchedulerTaskAttemptExecutionClass::Runtime)
        }
        WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine => {
            Ok(SchedulerTaskAttemptExecutionClass::NonRuntimeNodeEngine)
        }
        other => Err(WorkflowServiceError::Internal(format!(
            "scheduler task '{}' has unsupported started-attempt execution class {:?}",
            task.task_id.as_str(),
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use pantograph_diagnostics_ledger::DiagnosticEventPayload;
    use pantograph_scheduler::{
        SchedulerNodeId, SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateRecord,
        SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };

    use super::*;

    #[test]
    fn terminal_diagnostic_event_preserves_scheduler_task_attempt_scope() {
        let task = runtime_task();
        let event = scheduler_task_attempt_terminal_diagnostic_event_at(
            WorkflowSchedulerTaskAttemptTerminalDiagnosticRequest {
                task: &task,
                attempt_id: "attempt.runtime.1",
                started_at_ms: 100,
                transition: SchedulerTaskAttemptLifecycleTransition::Completed,
                reason: "scheduler runtime task attempt completed",
                error_summary: None,
                selected_dispatch: None,
                terminal_mutation: None,
                attribution: WorkflowSchedulerTaskAttemptDiagnosticAttribution::none(),
            },
            145,
        )
        .expect("terminal diagnostic event");

        assert_eq!(event.occurred_at_ms, 145);
        assert_eq!(event.node_id.as_deref(), Some("node.image"));
        assert_eq!(event.node_type.as_deref(), Some("image_generation"));
        let DiagnosticEventPayload::SchedulerTaskAttemptLifecycleChanged(payload) = event.payload
        else {
            panic!("expected scheduler task-attempt lifecycle payload");
        };
        assert_eq!(payload.scheduler_task_id, "task.image");
        assert_eq!(payload.scheduler_attempt_id, "attempt.runtime.1");
        assert_eq!(
            payload.execution_class,
            SchedulerTaskAttemptExecutionClass::Runtime
        );
        assert_eq!(
            payload.transition,
            SchedulerTaskAttemptLifecycleTransition::Completed
        );
        assert_eq!(payload.started_at_ms, Some(100));
        assert_eq!(payload.ended_at_ms, Some(145));
        assert_eq!(payload.duration_ms, Some(45));
        assert_eq!(
            payload.reason.as_deref(),
            Some("scheduler runtime task attempt completed")
        );
        assert!(payload.error_summary.is_none());
        assert!(payload.reservation_id.is_none());
    }

    #[test]
    fn completed_scheduler_run_response_rejects_incomplete_scheduler_task_state() {
        let record = scheduler_task_state_record(
            "task.image",
            SchedulerTaskState::TerminalFailed {
                diagnostics: Vec::new(),
            },
        );
        let error = ensure_all_scheduler_tasks_completed(&[record])
            .expect_err("incomplete scheduler task state should be rejected");

        assert!(matches!(
            error,
            WorkflowServiceError::InvalidRequest(message)
                if message.contains("scheduler task 'task.image' did not complete")
                    && message.contains("TerminalFailed")
        ));
    }

    fn runtime_task() -> WorkflowSchedulerTask {
        WorkflowSchedulerTask {
            workflow_id: SchedulerWorkflowId::parse("workflow.image").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run.image.1").expect("run id"),
            node_id: SchedulerNodeId::parse("node.image").expect("node id"),
            task_id: SchedulerTaskId::parse("task.image").expect("task id"),
            node_type: "image_generation".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            dependency_task_ids: Vec::new(),
            input_bindings: Vec::new(),
            schedulable_intent: None,
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: None,
            diagnostics: Vec::new(),
        }
    }

    fn scheduler_task_state_record(
        task_id: &str,
        state: SchedulerTaskState,
    ) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow.image").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run.image.1").expect("run id"),
            node_id: SchedulerNodeId::parse("node.image").expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            state,
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse("transition.terminal")
                .expect("transition id"),
        }
    }
}
