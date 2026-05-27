use pantograph_dependency_planning::{
    DependencyPlanningCallerContext, DependencyPlanningContractError,
    DependencyPlanningIdentityKey, DependencyPlanningRequest, DependencyPreflightResult,
    DependencyReadinessPolicy, DependencyReadinessRequest, DependencyTraitIntent,
    DependencyTraitIntentId, DependencyTraitIntentValue, SchedulerIntent,
    ValidatedDependencyReadinessRequest,
};
use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord, SchedulerTraitValue};
use thiserror::Error;

use crate::workflow::{
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph, WorkflowServiceError,
};

use super::{
    WorkflowExecutionSessionStore, WorkflowSchedulerTaskOrchestrator,
    WorkflowSchedulerTaskOrchestratorError,
};

/// Provider boundary used by workflow-service to obtain dependency readiness.
///
/// Implementations may call Pumas/package-manager/runtime-environment services,
/// but they must return the path-free dependency preflight proof owned by
/// `pantograph-dependency-planning`.
#[allow(dead_code)]
pub(crate) trait WorkflowDependencyReadinessProvider {
    fn resolve_dependency_readiness(
        &self,
        request: &ValidatedDependencyReadinessRequest,
    ) -> Result<Option<DependencyPreflightResult>, WorkflowDependencyReadinessProviderError>;
}

/// Scheduler-owned workflow-service lifecycle for dependency readiness proof.
///
/// This first lifecycle slice is synchronous and owns no background tasks. A
/// later async provider shell may be added here if dependency checks need I/O,
/// but spawned work must stay owned by this lifecycle.
#[derive(Clone)]
#[must_use]
#[allow(dead_code)]
pub(crate) struct WorkflowDependencyReadinessLifecycle {
    orchestrator: WorkflowSchedulerTaskOrchestrator,
}

#[allow(dead_code)]
impl WorkflowDependencyReadinessLifecycle {
    pub(crate) fn new(orchestrator: WorkflowSchedulerTaskOrchestrator) -> Self {
        Self { orchestrator }
    }

    pub(crate) fn readiness_request_for_active_runtime_task(
        &self,
        store: &WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
        policy: DependencyReadinessPolicy,
    ) -> Result<ValidatedDependencyReadinessRequest, WorkflowDependencyReadinessLifecycleError>
    {
        let (task_graph, records) =
            active_scheduler_task_state(store, session_id, workflow_run_id)?;
        let task_context = runtime_task_context(&task_graph, &records, workflow_run_id, task_id)?;
        let planning_request = dependency_planning_request_from_task_context(&task_context)?;
        let identity_key = DependencyPlanningIdentityKey::from_planning_request(&planning_request)
            .map_err(WorkflowDependencyReadinessLifecycleError::DependencyPlanning)?;
        DependencyReadinessRequest {
            contract_version: 1,
            identity_key,
            planning_request,
            policy,
        }
        .try_into()
        .map_err(WorkflowDependencyReadinessLifecycleError::DependencyPlanning)
    }

    pub(crate) fn resolve_and_admit_active_runtime_task<P>(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        provider: &P,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
        policy: DependencyReadinessPolicy,
    ) -> Result<SchedulerTaskStateRecord, WorkflowDependencyReadinessLifecycleError>
    where
        P: WorkflowDependencyReadinessProvider,
    {
        let request = self.readiness_request_for_active_runtime_task(
            store,
            session_id,
            workflow_run_id,
            task_id,
            policy.clone(),
        )?;
        let preflight_result = provider
            .resolve_dependency_readiness(&request)
            .map_err(WorkflowDependencyReadinessLifecycleError::Provider)?;
        self.orchestrator
            .apply_runtime_dependency_readiness_admission(
                store,
                session_id,
                workflow_run_id,
                task_id,
                policy,
                preflight_result,
            )
            .map_err(WorkflowDependencyReadinessLifecycleError::Orchestrator)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum WorkflowDependencyReadinessProviderError {
    #[error("dependency readiness provider failed: {message}")]
    Failed { message: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum WorkflowDependencyReadinessLifecycleError {
    #[error("workflow service operation failed")]
    WorkflowService(WorkflowServiceError),
    #[error("dependency planning contract validation failed")]
    DependencyPlanning(DependencyPlanningContractError),
    #[error("dependency readiness provider failed")]
    Provider(WorkflowDependencyReadinessProviderError),
    #[error("scheduler task orchestration failed")]
    Orchestrator(WorkflowSchedulerTaskOrchestratorError),
}

#[allow(dead_code)]
struct RuntimeTaskContext<'a> {
    task_graph: &'a WorkflowSchedulerTaskGraph,
    task_id: &'a str,
    node_type: &'a str,
    record: &'a SchedulerTaskStateRecord,
}

#[allow(dead_code)]
fn active_scheduler_task_state(
    store: &WorkflowExecutionSessionStore,
    session_id: &str,
    workflow_run_id: &str,
) -> Result<
    (WorkflowSchedulerTaskGraph, Vec<SchedulerTaskStateRecord>),
    WorkflowDependencyReadinessLifecycleError,
> {
    store
        .active_run_scheduler_task_state(session_id, workflow_run_id)
        .map_err(WorkflowDependencyReadinessLifecycleError::WorkflowService)?
        .ok_or_else(|| {
            WorkflowDependencyReadinessLifecycleError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "active workflow run '{}' has no scheduler task graph",
                    workflow_run_id
                )),
            )
        })
}

#[allow(dead_code)]
fn runtime_task_context<'a>(
    task_graph: &'a WorkflowSchedulerTaskGraph,
    records: &'a [SchedulerTaskStateRecord],
    workflow_run_id: &str,
    task_id: &'a str,
) -> Result<RuntimeTaskContext<'a>, WorkflowDependencyReadinessLifecycleError> {
    let task = task_graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == task_id)
        .ok_or_else(|| {
            invalid_request(format!(
                "scheduler task '{}' is not in active workflow run '{}'",
                task_id, workflow_run_id
            ))
        })?;
    if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
        return Err(invalid_request(format!(
            "scheduler task '{}' is not a runtime inference task",
            task_id
        )));
    }
    let record = records
        .iter()
        .find(|record| record.task_id.as_str() == task_id)
        .ok_or_else(|| {
            invalid_request(format!(
                "scheduler task '{}' has no active task-state record",
                task_id
            ))
        })?;
    if record.state.kind() != SchedulerTaskStateKind::WaitingDependencyReadiness {
        return Err(invalid_request(format!(
            "scheduler task '{}' must be waiting for dependency readiness before readiness request construction",
            task_id
        )));
    }
    Ok(RuntimeTaskContext {
        task_graph,
        task_id,
        node_type: &task.node_type,
        record,
    })
}

#[allow(dead_code)]
fn dependency_planning_request_from_task_context(
    context: &RuntimeTaskContext<'_>,
) -> Result<DependencyPlanningRequest, WorkflowDependencyReadinessLifecycleError> {
    let Some(execution_intent) = context.record.state.execution_intent() else {
        return Err(invalid_request(format!(
            "scheduler task '{}' has no runtime execution intent",
            context.task_id
        )));
    };
    let Some(task_intent) = execution_intent.runtime_task_intent() else {
        return Err(invalid_request(format!(
            "scheduler task '{}' does not carry runtime task intent",
            context.task_id
        )));
    };

    let request = DependencyPlanningRequest {
        model_ref: task_intent.model_ref.clone(),
        task_id: task_intent.task_type.clone(),
        task_type: Some(task_intent.task_type.clone()),
        expected_artifact_kind: None,
        scheduler_intent: SchedulerIntent {
            requested_runtime_id: task_intent.constraints.requested_runtime_id.clone(),
            requested_device_id: task_intent.constraints.requested_device_id.clone(),
        },
        platform_context: None,
        selected_binding_ids: Vec::new(),
        dependency_override_patches: task_intent.dependency_override_patches.clone(),
        trait_intents: task_intent
            .trait_settings
            .iter()
            .map(dependency_trait_intent_from_scheduler_trait)
            .collect::<Result<Vec<_>, _>>()?,
        caller_context: DependencyPlanningCallerContext {
            source_node_type: Some(
                pantograph_dependency_planning::DependencyNodeTypeId::parse(context.node_type)
                    .map_err(WorkflowDependencyReadinessLifecycleError::DependencyPlanning)?,
            ),
            workflow_id: Some(context.task_graph.workflow_id.as_str().to_string()),
            node_id: Some(task_intent.node_id.as_str().to_string()),
            port_id: None,
            run_id: Some(context.task_graph.workflow_run_id.as_str().to_string()),
        },
    };
    request
        .validate()
        .map_err(WorkflowDependencyReadinessLifecycleError::DependencyPlanning)?;
    Ok(request)
}

#[allow(dead_code)]
fn dependency_trait_intent_from_scheduler_trait(
    setting: &pantograph_scheduler::SchedulerTraitSetting,
) -> Result<DependencyTraitIntent, WorkflowDependencyReadinessLifecycleError> {
    Ok(DependencyTraitIntent {
        trait_id: DependencyTraitIntentId::parse(setting.trait_id.as_str())
            .map_err(WorkflowDependencyReadinessLifecycleError::DependencyPlanning)?,
        value: dependency_trait_value_from_scheduler_value(&setting.value)?,
    })
}

#[allow(dead_code)]
fn dependency_trait_value_from_scheduler_value(
    value: &SchedulerTraitValue,
) -> Result<DependencyTraitIntentValue, WorkflowDependencyReadinessLifecycleError> {
    match value {
        SchedulerTraitValue::String(value) => Ok(DependencyTraitIntentValue::Text(value.clone())),
        SchedulerTraitValue::Bool(value) => Ok(DependencyTraitIntentValue::Boolean(*value)),
        SchedulerTraitValue::I64(value) => Ok(DependencyTraitIntentValue::Integer(*value)),
        SchedulerTraitValue::U64(value) => i64::try_from(*value)
            .map(DependencyTraitIntentValue::Integer)
            .map_err(|_| {
                WorkflowDependencyReadinessLifecycleError::DependencyPlanning(
                    DependencyPlanningContractError::InvalidField {
                        field: "dependency_trait_intent.value",
                        reason: "unsigned scheduler trait values must fit dependency planning integer values",
                    },
                )
            }),
    }
}

#[allow(dead_code)]
fn invalid_request(message: String) -> WorkflowDependencyReadinessLifecycleError {
    WorkflowDependencyReadinessLifecycleError::WorkflowService(
        WorkflowServiceError::InvalidRequest(message),
    )
}

#[cfg(test)]
#[path = "readiness_lifecycle_tests.rs"]
mod tests;
