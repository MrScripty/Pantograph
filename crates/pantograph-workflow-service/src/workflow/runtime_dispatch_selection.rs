use async_trait::async_trait;
use pantograph_dependency_planning::{
    DependencyEnvironmentRef, DependencyPlanningContractError, DependencyReadinessProofEnvelope,
    DeviceIntentId, PumasModelRef, RuntimeIntentId,
};
use pantograph_scheduler::{
    SchedulerBatchingGroupId, SchedulerDispatchCandidate, SchedulerDispatchCandidateId,
    SchedulerDispatchSelectionDiagnostic, SchedulerDispatchSelectionRequest,
    SchedulerResourceFitAssessment, SchedulerResourceReservation, SchedulerRuntimeVariantId,
    SchedulerTaskStateRecord, SchedulerTraitSetting, ValidatedSchedulerDispatchSelectionRequest,
    SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};
use thiserror::Error;

use super::{WorkflowSchedulerTask, WorkflowServiceError};

pub const WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION: u16 = 1;

/// Workflow-service provider boundary for runtime dispatch candidates.
///
/// Implementations gather already-canonical runtime, resource, and model facts.
/// Scheduler policy still owns selection and ranking.
pub trait WorkflowRuntimeDispatchCandidateProvider: Send + Sync {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowRuntimeDispatchCandidateSet {
    pub candidates: Vec<SchedulerDispatchCandidate>,
    pub diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
}

impl WorkflowRuntimeDispatchCandidateSet {
    pub fn from_candidate_fact_bundle(
        bundle: ValidatedWorkflowRuntimeDispatchCandidateFactBundle,
    ) -> Self {
        let bundle = bundle.into_inner();
        Self {
            candidates: Vec::new(),
            diagnostics: bundle.diagnostics,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRuntimeDispatchCandidateFactBundle {
    pub contract_version: u16,
    pub facts: Vec<WorkflowRuntimeDispatchCandidateFact>,
    pub diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRuntimeDispatchCandidateFact {
    pub candidate_id: SchedulerDispatchCandidateId,
    pub selected_runtime_id: RuntimeIntentId,
    pub selected_runtime_variant_id: Option<SchedulerRuntimeVariantId>,
    pub selected_device_ids: Vec<DeviceIntentId>,
    pub selected_model_ref: PumasModelRef,
    pub runtime_trait_settings: Vec<SchedulerTraitSetting>,
    pub environment_ref: DependencyEnvironmentRef,
    pub reservation: SchedulerResourceReservation,
    pub resource_fit_assessment: SchedulerResourceFitAssessment,
    pub batching_group_id: Option<SchedulerBatchingGroupId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedWorkflowRuntimeDispatchCandidateFactBundle(
    WorkflowRuntimeDispatchCandidateFactBundle,
);

impl ValidatedWorkflowRuntimeDispatchCandidateFactBundle {
    pub fn into_inner(self) -> WorkflowRuntimeDispatchCandidateFactBundle {
        self.0
    }
}

impl TryFrom<WorkflowRuntimeDispatchCandidateFactBundle>
    for ValidatedWorkflowRuntimeDispatchCandidateFactBundle
{
    type Error = WorkflowRuntimeDispatchCandidateFactBundleError;

    fn try_from(value: WorkflowRuntimeDispatchCandidateFactBundle) -> Result<Self, Self::Error> {
        validate_candidate_fact_bundle(&value)?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRuntimeDispatchCandidateFactSourceKind {
    PumasPackageFacts,
    RuntimeCapabilityFacts,
    ResourceReservationFacts,
}

#[async_trait]
pub trait WorkflowRuntimeDispatchCandidateFactSource: Send + Sync {
    fn source_kind(&self) -> WorkflowRuntimeDispatchCandidateFactSourceKind;

    async fn collect_candidate_facts(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<
        ValidatedWorkflowRuntimeDispatchCandidateFactBundle,
        WorkflowRuntimeDispatchCandidateFactSourceError,
    >;
}

#[derive(Debug, Default)]
pub(crate) struct NoRuntimeDispatchCandidatesProvider;

impl WorkflowRuntimeDispatchCandidateProvider for NoRuntimeDispatchCandidatesProvider {
    fn runtime_dispatch_candidates(
        &self,
        _task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        _readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        Ok(WorkflowRuntimeDispatchCandidateSet::default())
    }
}

pub(crate) fn runtime_dispatch_selection_request(
    task: &WorkflowSchedulerTask,
    readiness_proof: DependencyReadinessProofEnvelope,
    candidate_set: WorkflowRuntimeDispatchCandidateSet,
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
        candidates: candidate_set.candidates,
        diagnostics: candidate_set.diagnostics,
    }
    .try_into()
    .map_err(WorkflowRuntimeDispatchSelectionError::SchedulerContract)
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkflowRuntimeDispatchCandidateProviderError {
    #[error("runtime dispatch candidate provider failed: {message}")]
    Failed { message: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkflowRuntimeDispatchCandidateFactSourceError {
    #[error("runtime dispatch candidate fact source failed: {message}")]
    Failed { message: String },
    #[error("runtime dispatch candidate fact source returned invalid facts")]
    Contract(#[from] WorkflowRuntimeDispatchCandidateFactBundleError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowRuntimeDispatchCandidateFactBundleError {
    #[error("unsupported runtime dispatch candidate fact bundle contract version {0}")]
    UnsupportedContractVersion(u16),
    #[error("runtime dispatch candidate fact bundle contains duplicate candidate id '{0}'")]
    DuplicateCandidateId(String),
    #[error("runtime dispatch candidate fact '{candidate_id}' has no selected devices")]
    MissingSelectedDevice { candidate_id: String },
    #[error("runtime dispatch candidate fact '{candidate_id}' contains duplicate selected device '{device_id}'")]
    DuplicateSelectedDevice {
        candidate_id: String,
        device_id: String,
    },
    #[error("runtime dispatch candidate fact '{candidate_id}' carries a path-shaped model ref")]
    PathCarryingModelRef { candidate_id: String },
    #[error("runtime dispatch candidate fact '{candidate_id}' has invalid model ref")]
    InvalidModelRef {
        candidate_id: String,
        source: DependencyPlanningContractError,
    },
    #[error("runtime dispatch candidate fact source diagnostic is invalid: {message}")]
    InvalidSourceDiagnostic { message: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowRuntimeDispatchSelectionError {
    #[error("workflow service operation failed")]
    WorkflowService(WorkflowServiceError),
    #[error("scheduler dispatch-selection contract validation failed")]
    SchedulerContract(#[from] pantograph_scheduler::SchedulerContractError),
}

fn validate_candidate_fact_bundle(
    bundle: &WorkflowRuntimeDispatchCandidateFactBundle,
) -> Result<(), WorkflowRuntimeDispatchCandidateFactBundleError> {
    if bundle.contract_version != WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::UnsupportedContractVersion(
                bundle.contract_version,
            ),
        );
    }
    let mut candidate_ids = std::collections::BTreeSet::new();
    for fact in &bundle.facts {
        let candidate_id = fact.candidate_id.as_str();
        if !candidate_ids.insert(candidate_id) {
            return Err(
                WorkflowRuntimeDispatchCandidateFactBundleError::DuplicateCandidateId(
                    candidate_id.to_string(),
                ),
            );
        }
        validate_candidate_fact(fact)?;
    }
    for diagnostic in &bundle.diagnostics {
        validate_source_diagnostic(diagnostic)?;
    }
    Ok(())
}

fn validate_candidate_fact(
    fact: &WorkflowRuntimeDispatchCandidateFact,
) -> Result<(), WorkflowRuntimeDispatchCandidateFactBundleError> {
    let candidate_id = fact.candidate_id.as_str();
    if fact.selected_device_ids.is_empty() {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::MissingSelectedDevice {
                candidate_id: candidate_id.to_string(),
            },
        );
    }
    let mut device_ids = std::collections::BTreeSet::new();
    for device_id in &fact.selected_device_ids {
        if !device_ids.insert(device_id.as_str()) {
            return Err(
                WorkflowRuntimeDispatchCandidateFactBundleError::DuplicateSelectedDevice {
                    candidate_id: candidate_id.to_string(),
                    device_id: device_id.as_str().to_string(),
                },
            );
        }
    }
    if fact.selected_model_ref.selected_artifact_path.is_some() {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::PathCarryingModelRef {
                candidate_id: candidate_id.to_string(),
            },
        );
    }
    fact.selected_model_ref.validate().map_err(|source| {
        WorkflowRuntimeDispatchCandidateFactBundleError::InvalidModelRef {
            candidate_id: candidate_id.to_string(),
            source,
        }
    })?;
    Ok(())
}

fn validate_source_diagnostic(
    diagnostic: &SchedulerDispatchSelectionDiagnostic,
) -> Result<(), WorkflowRuntimeDispatchCandidateFactBundleError> {
    if diagnostic.message.trim().is_empty() {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidSourceDiagnostic {
                message: "diagnostic message must not be blank".to_string(),
            },
        );
    }
    if diagnostic
        .hint
        .as_ref()
        .is_some_and(|hint| hint.trim().is_empty())
    {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidSourceDiagnostic {
                message: "diagnostic hint must not be blank".to_string(),
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{DependencyEnvironmentId, DependencyEnvironmentRef};
    use pantograph_scheduler::{
        SchedulerDispatchSelectionDiagnosticCode, SchedulerDispatchSelectionDiagnosticSeverity,
        SchedulerReservationLeaseId, SchedulerResourceFitAssessment, SchedulerResourceFitState,
        SchedulerResourceKind, SchedulerResourceReservation, SchedulerTaskId,
        SchedulerWorkflowRunId,
    };

    use super::*;

    #[test]
    fn candidate_fact_bundle_validates_path_free_dispatch_facts() {
        let bundle = candidate_fact_bundle(vec![candidate_fact()]);

        let validated = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect("path-free candidate facts should validate");

        assert_eq!(validated.into_inner().facts.len(), 1);
    }

    #[test]
    fn candidate_fact_bundle_rejects_path_carrying_model_ref() {
        let mut fact = candidate_fact();
        fact.selected_model_ref.selected_artifact_path =
            Some("/models/juggernaut/model.safetensors".to_string());
        let bundle = candidate_fact_bundle(vec![fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("path-shaped model facts must be rejected");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::PathCarryingModelRef { .. }
        ));
    }

    #[test]
    fn candidate_fact_bundle_rejects_duplicate_candidate_ids() {
        let fact = candidate_fact();
        let bundle = candidate_fact_bundle(vec![fact.clone(), fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("candidate ids must be unique before scheduler selection");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::DuplicateCandidateId(_)
        ));
    }

    #[test]
    fn candidate_fact_bundle_does_not_emit_candidates_before_mapping_slice() {
        let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            candidate_fact_bundle(vec![candidate_fact()]),
        )
        .expect("candidate fact bundle");

        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle);

        assert!(candidate_set.candidates.is_empty());
        assert_eq!(candidate_set.diagnostics.len(), 1);
    }

    fn candidate_fact_bundle(
        facts: Vec<WorkflowRuntimeDispatchCandidateFact>,
    ) -> WorkflowRuntimeDispatchCandidateFactBundle {
        WorkflowRuntimeDispatchCandidateFactBundle {
            contract_version: WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
            facts,
            diagnostics: vec![SchedulerDispatchSelectionDiagnostic {
                severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
                code: SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                message: "candidate facts collected but production mapping is not wired"
                    .to_string(),
                candidate_id: None,
                hint: None,
            }],
        }
    }

    fn candidate_fact() -> WorkflowRuntimeDispatchCandidateFact {
        let workflow_run_id: SchedulerWorkflowRunId =
            "run.dispatch-facts".parse().expect("workflow run id");
        let task_id: SchedulerTaskId = "infer".parse().expect("task id");
        let device_id: DeviceIntentId = "cuda:0".parse().expect("device id");
        WorkflowRuntimeDispatchCandidateFact {
            candidate_id: "candidate.diffusers.cuda0".parse().expect("candidate id"),
            selected_runtime_id: "diffusers-pytorch".parse().expect("runtime id"),
            selected_runtime_variant_id: Some(
                "diffusers-pytorch.cuda".parse().expect("variant id"),
            ),
            selected_device_ids: vec![device_id.clone()],
            selected_model_ref: PumasModelRef {
                model_id: "image/example/tiny-diffusion".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            runtime_trait_settings: Vec::new(),
            environment_ref: DependencyEnvironmentRef {
                environment_id: DependencyEnvironmentId::parse("env.dispatch-facts")
                    .expect("environment id"),
                manifest_id: None,
            },
            reservation: SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(
                    "reservation.dispatch-facts",
                )
                .expect("reservation id"),
                workflow_run_id: workflow_run_id.clone(),
                task_id: task_id.clone(),
                device_id,
                resource_kind: SchedulerResourceKind::DeviceVram,
                reserved_bytes: 1,
            },
            resource_fit_assessment: SchedulerResourceFitAssessment {
                workflow_run_id,
                task_id,
                state: SchedulerResourceFitState::Fits,
                diagnostics: Vec::new(),
            },
            batching_group_id: None,
        }
    }
}
