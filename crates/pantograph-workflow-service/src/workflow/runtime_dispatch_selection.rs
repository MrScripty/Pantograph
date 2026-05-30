use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
use pantograph_scheduler::{
    SchedulerDispatchCandidate, SchedulerDispatchSelectionRequest, SchedulerTaskStateRecord,
    ValidatedSchedulerDispatchSelectionRequest, SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};
use thiserror::Error;

use super::{WorkflowSchedulerTask, WorkflowServiceError};

/// Workflow-service provider boundary for runtime dispatch candidates.
///
/// Implementations gather already-canonical runtime, resource, and model facts.
/// Scheduler policy still owns selection and ranking.
pub(crate) trait WorkflowRuntimeDispatchCandidateProvider: Send + Sync {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<Vec<SchedulerDispatchCandidate>, WorkflowRuntimeDispatchCandidateProviderError>;
}

#[derive(Debug, Default)]
pub(crate) struct NoRuntimeDispatchCandidatesProvider;

impl WorkflowRuntimeDispatchCandidateProvider for NoRuntimeDispatchCandidatesProvider {
    fn runtime_dispatch_candidates(
        &self,
        _task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        _readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<Vec<SchedulerDispatchCandidate>, WorkflowRuntimeDispatchCandidateProviderError>
    {
        Ok(Vec::new())
    }
}

pub(crate) fn runtime_dispatch_selection_request(
    task: &WorkflowSchedulerTask,
    readiness_proof: DependencyReadinessProofEnvelope,
    candidates: Vec<SchedulerDispatchCandidate>,
) -> Result<ValidatedSchedulerDispatchSelectionRequest, WorkflowRuntimeDispatchSelectionError> {
    let Some(task_intent) = task.schedulable_intent.clone() else {
        return Err(WorkflowRuntimeDispatchSelectionError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{}' is missing a schedulable task intent",
                task.task_id.as_str()
            )),
        ));
    };
    let Some(environment_ref) = readiness_proof.preflight_result.environment_ref.clone() else {
        return Err(WorkflowRuntimeDispatchSelectionError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{}' has no dependency environment reference for dispatch selection",
                task.task_id.as_str()
            )),
        ));
    };
    SchedulerDispatchSelectionRequest {
        contract_version: SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
        task_intent,
        readiness_proof,
        environment_ref,
        candidates,
        diagnostics: Vec::new(),
    }
    .try_into()
    .map_err(WorkflowRuntimeDispatchSelectionError::SchedulerContract)
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowRuntimeDispatchCandidateProviderError {
    #[error("runtime dispatch candidate provider failed: {message}")]
    #[allow(dead_code)]
    Failed { message: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowRuntimeDispatchSelectionError {
    #[error("workflow service operation failed")]
    WorkflowService(WorkflowServiceError),
    #[error("scheduler dispatch-selection contract validation failed")]
    SchedulerContract(#[from] pantograph_scheduler::SchedulerContractError),
}
