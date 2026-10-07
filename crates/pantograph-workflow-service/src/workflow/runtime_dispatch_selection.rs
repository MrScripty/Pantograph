use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet};

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

use crate::scheduler::task_orchestrator::{
    SelectedRuntimeTaskDispatch, StartedRuntimeTaskExecution,
};
use crate::scheduler::{WorkflowSchedulerTaskOrchestrator, WorkflowSchedulerTaskOrchestratorError};

use super::{WorkflowSchedulerTask, WorkflowService, WorkflowServiceError};

pub const WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION: u16 = 2;

/// Workflow-service pre-dispatch refresh boundary for runtime dispatch sources.
///
/// Implementations may refresh already-owned source snapshots before the
/// synchronous candidate provider is called. Workflow-service owns the
/// orchestration point, but not Pumas/runtime-registry source ownership.
#[async_trait]
pub trait WorkflowRuntimeDispatchSourceRefresher: Send + Sync {
    async fn refresh_runtime_dispatch_sources(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError>;
}

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

    /// Native opt-in only; existing providers keep their original collection path.
    fn requires_materialized_inputs(&self) -> bool {
        false
    }

    fn runtime_dispatch_candidates_with_inputs(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
        _inputs: Option<&[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        self.runtime_dispatch_candidates(task, ready_record, readiness_proof)
    }
}

/// Runtime-owned rollback custody travels with prepared facts until binding.
///
/// Implementations own cleanup for every provisional lease in the accompanying
/// candidate set. Dropping an untransferred token must roll back those claims;
/// workflow-service skips ordinary unselected-lease release while custody owns
/// rollback. Transfer occurs only after the selected task's in-memory cleanup
/// intent is bound. This boundary does not promise process-abort recovery.
pub trait WorkflowRuntimeDispatchReservationCustody: std::fmt::Debug + Send + Sync {
    fn transfer(self: Box<Self>) -> Result<(), WorkflowRuntimeDispatchCandidateProviderError>;
}

#[derive(Debug, Default)]
pub struct WorkflowRuntimeDispatchCandidateSet {
    pub candidates: Vec<SchedulerDispatchCandidate>,
    pub diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
    pub candidate_evidence_context: WorkflowRuntimeDispatchCandidateEvidenceContext,
    reservation_custody: Option<Box<dyn WorkflowRuntimeDispatchReservationCustody>>,
}

impl WorkflowRuntimeDispatchCandidateSet {
    pub fn with_reservation_custody(
        mut self,
        custody: Box<dyn WorkflowRuntimeDispatchReservationCustody>,
    ) -> Self {
        self.reservation_custody = Some(custody);
        self
    }
    pub fn from_candidate_fact_bundle(
        bundle: ValidatedWorkflowRuntimeDispatchCandidateFactBundle,
    ) -> Self {
        let bundle = bundle.into_inner();
        let facts = bundle.facts;
        Self {
            candidates: facts.iter().cloned().map(dispatch_candidate).collect(),
            diagnostics: bundle.diagnostics,
            candidate_evidence_context:
                WorkflowRuntimeDispatchCandidateEvidenceContext::from_validated_facts(facts),
            reservation_custody: None,
        }
    }

    pub fn from_diagnostics(diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>) -> Self {
        Self {
            candidates: Vec::new(),
            diagnostics,
            candidate_evidence_context: WorkflowRuntimeDispatchCandidateEvidenceContext::default(),
            reservation_custody: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowRuntimeDispatchCandidateEvidenceContext {
    facts_by_candidate_id: BTreeMap<String, WorkflowRuntimeDispatchCandidateFact>,
}

impl WorkflowRuntimeDispatchCandidateEvidenceContext {
    #[must_use]
    pub fn candidate_fact(
        &self,
        candidate_id: &SchedulerDispatchCandidateId,
    ) -> Option<&WorkflowRuntimeDispatchCandidateFact> {
        self.facts_by_candidate_id.get(candidate_id.as_str())
    }

    #[must_use]
    pub fn contains_candidate_id(&self, candidate_id: &SchedulerDispatchCandidateId) -> bool {
        self.facts_by_candidate_id
            .contains_key(candidate_id.as_str())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.facts_by_candidate_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts_by_candidate_id.is_empty()
    }

    fn from_validated_facts(facts: Vec<WorkflowRuntimeDispatchCandidateFact>) -> Self {
        Self {
            facts_by_candidate_id: facts
                .into_iter()
                .map(|fact| (fact.candidate_id.as_str().to_string(), fact))
                .collect(),
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
    pub selected_backend_key: String,
    pub runtime_family: String,
    pub resolved_load_target: String,
    pub runtime_residency_key: String,
    pub loaded_runtime_memory_estimate_bytes: u64,
    pub runtime_load_state: WorkflowRuntimeDispatchLoadState,
    pub runtime_instance_id: Option<String>,
    pub selected_device_ids: Vec<DeviceIntentId>,
    pub selected_model_ref: PumasModelRef,
    pub runtime_trait_settings: Vec<SchedulerTraitSetting>,
    pub environment_ref: DependencyEnvironmentRef,
    pub reservations: Vec<SchedulerResourceReservation>,
    pub resource_fit_assessment: SchedulerResourceFitAssessment,
    pub batching_group_id: Option<SchedulerBatchingGroupId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRuntimeDispatchLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Busy,
    Unloading,
    Failed,
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

#[derive(Debug, Default)]
pub(crate) struct NoRuntimeDispatchCandidatesProvider;

#[derive(Debug, Default)]
pub(crate) struct NoRuntimeDispatchSourceRefresher;

#[async_trait]
impl WorkflowRuntimeDispatchSourceRefresher for NoRuntimeDispatchSourceRefresher {
    async fn refresh_runtime_dispatch_sources(
        &self,
        _task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        _readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
        Ok(())
    }
}

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
) -> Result<WorkflowRuntimeDispatchSelectionRequest, WorkflowRuntimeDispatchSelectionError> {
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
    let selection_request = SchedulerDispatchSelectionRequest {
        contract_version: SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
        task_intent,
        readiness_proof,
        environment_ref,
        candidates: candidate_set.candidates,
        diagnostics: candidate_set.diagnostics,
    }
    .try_into()
    .map_err(WorkflowRuntimeDispatchSelectionError::SchedulerContract)?;
    Ok(WorkflowRuntimeDispatchSelectionRequest {
        selection_request,
        candidate_evidence_context: candidate_set.candidate_evidence_context,
        reservation_custody: candidate_set.reservation_custody,
    })
}

#[derive(Debug)]
pub(crate) struct WorkflowRuntimeDispatchSelectionRequest {
    pub(crate) selection_request: ValidatedSchedulerDispatchSelectionRequest,
    pub(crate) candidate_evidence_context: WorkflowRuntimeDispatchCandidateEvidenceContext,
    reservation_custody: Option<Box<dyn WorkflowRuntimeDispatchReservationCustody>>,
}

pub(crate) struct WorkflowRuntimeDispatchSelectionBoundary<'a> {
    source_refresher: &'a dyn WorkflowRuntimeDispatchSourceRefresher,
    candidate_provider: &'a dyn WorkflowRuntimeDispatchCandidateProvider,
    scheduler_task_orchestrator: &'a WorkflowSchedulerTaskOrchestrator,
}

impl<'a> WorkflowRuntimeDispatchSelectionBoundary<'a> {
    pub(crate) fn from_service(service: &'a WorkflowService) -> Self {
        Self {
            source_refresher: service.runtime_dispatch_source_refresher.as_ref(),
            candidate_provider: service.runtime_dispatch_candidate_provider.as_ref(),
            scheduler_task_orchestrator: &service.scheduler_task_orchestrator,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(
        source_refresher: &'a dyn WorkflowRuntimeDispatchSourceRefresher,
        candidate_provider: &'a dyn WorkflowRuntimeDispatchCandidateProvider,
        scheduler_task_orchestrator: &'a WorkflowSchedulerTaskOrchestrator,
    ) -> Self {
        Self {
            source_refresher,
            candidate_provider,
            scheduler_task_orchestrator,
        }
    }

    #[cfg(test)]
    pub(crate) async fn prepare_ready_runtime_task_dispatch(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchSelectionRequest, WorkflowRuntimeDispatchPreselectionError>
    {
        self.prepare_ready_runtime_task_dispatch_with_inputs(
            task,
            ready_record,
            readiness_proof,
            None,
        )
        .await
    }

    pub(crate) async fn prepare_ready_runtime_task_dispatch_with_inputs(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: DependencyReadinessProofEnvelope,
        inputs: Option<&[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>,
    ) -> Result<WorkflowRuntimeDispatchSelectionRequest, WorkflowRuntimeDispatchPreselectionError>
    {
        self.source_refresher
            .refresh_runtime_dispatch_sources(task, ready_record, &readiness_proof)
            .await
            .map_err(WorkflowRuntimeDispatchPreselectionError::SourceRefresh)?;
        let candidate_set = self
            .candidate_provider
            .runtime_dispatch_candidates_with_inputs(task, ready_record, &readiness_proof, inputs)
            .map_err(WorkflowRuntimeDispatchPreselectionError::CandidateCollection)?;
        runtime_dispatch_selection_request(task, readiness_proof, candidate_set)
            .map_err(WorkflowRuntimeDispatchPreselectionError::SelectionRequest)
    }

    pub(crate) async fn select_prepared_started_runtime_task_dispatch(
        &self,
        started_runtime_task: &StartedRuntimeTaskExecution,
        prepared_selection: WorkflowRuntimeDispatchSelectionRequest,
    ) -> Result<WorkflowRuntimeDispatchPreselection, WorkflowRuntimeDispatchPreselectionError> {
        let selection_request = prepared_selection.selection_request;
        let candidate_evidence_context = prepared_selection.candidate_evidence_context;
        let reservation_custody = prepared_selection.reservation_custody;
        let selection = if reservation_custody.is_some() {
            self.scheduler_task_orchestrator
                .select_runtime_task_dispatch_with_custody(
                    started_runtime_task.task(),
                    selection_request,
                )
                .await
        } else {
            self.scheduler_task_orchestrator
                .select_runtime_task_dispatch(started_runtime_task.task(), selection_request)
                .await
        };
        let selected_dispatch =
            selection.map_err(WorkflowRuntimeDispatchPreselectionError::SchedulerSelection)?;
        let selected_candidate_fact = selected_runtime_dispatch_candidate_fact(
            selected_dispatch.candidate_id(),
            &candidate_evidence_context,
        )
        .map_err(WorkflowRuntimeDispatchPreselectionError::SelectedCandidateEvidence)?;
        Ok(WorkflowRuntimeDispatchPreselection {
            selected_dispatch,
            selected_candidate_fact,
            reservation_custody,
        })
    }
}

#[derive(Debug)]
#[must_use]
pub(crate) struct WorkflowRuntimeDispatchPreselection {
    pub(crate) selected_dispatch: SelectedRuntimeTaskDispatch,
    pub(crate) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    reservation_custody: Option<Box<dyn WorkflowRuntimeDispatchReservationCustody>>,
}

impl WorkflowRuntimeDispatchPreselection {
    pub(crate) fn transfer_reservation_custody(&mut self) -> Result<(), WorkflowServiceError> {
        if let Some(custody) = self.reservation_custody.take() {
            custody.transfer().map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "runtime reservation custody transfer failed: {error}"
                ))
            })?;
        }
        Ok(())
    }
}

pub(crate) fn selected_runtime_dispatch_candidate_fact(
    candidate_id: Option<&SchedulerDispatchCandidateId>,
    candidate_evidence_context: &WorkflowRuntimeDispatchCandidateEvidenceContext,
) -> Result<WorkflowRuntimeDispatchCandidateFact, WorkflowRuntimeDispatchSelectionError> {
    let Some(candidate_id) = candidate_id else {
        return Err(WorkflowRuntimeDispatchSelectionError::MissingSelectedCandidateId);
    };
    candidate_evidence_context
        .candidate_fact(candidate_id)
        .cloned()
        .ok_or_else(
            || WorkflowRuntimeDispatchSelectionError::MissingSelectedCandidateFact {
                candidate_id: candidate_id.as_str().to_string(),
            },
        )
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkflowRuntimeDispatchCandidateProviderError {
    #[error("runtime dispatch candidate provider failed: {message}")]
    Failed { message: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowRuntimeDispatchSourceRefreshError {
    #[error("runtime dispatch source refresh failed: {message}")]
    Failed { message: String },
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
    #[error("runtime dispatch candidate fact '{candidate_id}' is missing selected evidence field '{field_path}'")]
    MissingSelectedEvidence {
        candidate_id: String,
        field_path: &'static str,
    },
    #[error("runtime dispatch candidate fact '{candidate_id}' has invalid memory estimate")]
    InvalidMemoryEstimate { candidate_id: String },
    #[error("runtime dispatch candidate fact '{candidate_id}' has invalid runtime instance fact")]
    InvalidRuntimeInstanceFact { candidate_id: String },
    #[error("runtime dispatch candidate fact '{candidate_id}' has no reservations")]
    MissingReservation { candidate_id: String },
    #[error(
        "runtime dispatch candidate fact '{candidate_id}' has reservations with mixed lease ids"
    )]
    MixedReservationLease { candidate_id: String },
    #[error("runtime dispatch candidate fact '{candidate_id}' has reservation for unselected device '{device_id}'")]
    ReservationDeviceNotSelected {
        candidate_id: String,
        device_id: String,
    },
    #[error("runtime dispatch candidate fact '{candidate_id}' has zero-byte reservation for device '{device_id}'")]
    EmptyReservationBytes {
        candidate_id: String,
        device_id: String,
    },
    #[error("runtime dispatch candidate fact '{candidate_id}' has duplicate reservation claim for device '{device_id}' and resource '{resource_kind}'")]
    DuplicateReservationClaim {
        candidate_id: String,
        device_id: String,
        resource_kind: String,
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
    #[error("scheduler selected runtime dispatch without a candidate id")]
    MissingSelectedCandidateId,
    #[error("scheduler selected candidate '{candidate_id}' has no retained workflow-service candidate fact")]
    MissingSelectedCandidateFact { candidate_id: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowRuntimeDispatchPreselectionError {
    #[error("runtime dispatch source refresh failed: {0}")]
    SourceRefresh(WorkflowRuntimeDispatchSourceRefreshError),
    #[error("runtime dispatch candidate collection failed: {0}")]
    CandidateCollection(WorkflowRuntimeDispatchCandidateProviderError),
    #[error("runtime dispatch selection request failed: {0}")]
    SelectionRequest(WorkflowRuntimeDispatchSelectionError),
    #[error("scheduler runtime dispatch selection failed: {0}")]
    SchedulerSelection(WorkflowSchedulerTaskOrchestratorError),
    #[error("runtime dispatch selected candidate evidence failed: {0}")]
    SelectedCandidateEvidence(WorkflowRuntimeDispatchSelectionError),
}

impl WorkflowRuntimeDispatchPreselectionError {
    pub(crate) fn scheduler_selection_error(
        &self,
    ) -> Option<&WorkflowSchedulerTaskOrchestratorError> {
        match self {
            Self::SchedulerSelection(error) => Some(error),
            _ => None,
        }
    }
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
    let mut candidate_ids = BTreeSet::new();
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
    let mut device_ids = BTreeSet::new();
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
    validate_selected_evidence(fact, candidate_id)?;
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
    let Some(first_reservation) = fact.reservations.first() else {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::MissingReservation {
                candidate_id: candidate_id.to_string(),
            },
        );
    };
    let mut reservation_claims = BTreeSet::new();
    for reservation in &fact.reservations {
        if reservation.reservation_lease_id != first_reservation.reservation_lease_id {
            return Err(
                WorkflowRuntimeDispatchCandidateFactBundleError::MixedReservationLease {
                    candidate_id: candidate_id.to_string(),
                },
            );
        }
        if !fact
            .selected_device_ids
            .iter()
            .any(|device_id| device_id == &reservation.device_id)
        {
            return Err(
                WorkflowRuntimeDispatchCandidateFactBundleError::ReservationDeviceNotSelected {
                    candidate_id: candidate_id.to_string(),
                    device_id: reservation.device_id.as_str().to_string(),
                },
            );
        }
        if reservation.reserved_bytes == 0 {
            return Err(
                WorkflowRuntimeDispatchCandidateFactBundleError::EmptyReservationBytes {
                    candidate_id: candidate_id.to_string(),
                    device_id: reservation.device_id.as_str().to_string(),
                },
            );
        }
        if !reservation_claims.insert((
            reservation.device_id.as_str(),
            reservation.resource_kind.clone(),
        )) {
            return Err(
                WorkflowRuntimeDispatchCandidateFactBundleError::DuplicateReservationClaim {
                    candidate_id: candidate_id.to_string(),
                    device_id: reservation.device_id.as_str().to_string(),
                    resource_kind: format!("{:?}", reservation.resource_kind),
                },
            );
        }
    }
    Ok(())
}

fn validate_selected_evidence(
    fact: &WorkflowRuntimeDispatchCandidateFact,
    candidate_id: &str,
) -> Result<(), WorkflowRuntimeDispatchCandidateFactBundleError> {
    validate_required_evidence_field(
        candidate_id,
        "selected_backend_key",
        &fact.selected_backend_key,
    )?;
    validate_required_evidence_field(candidate_id, "runtime_family", &fact.runtime_family)?;
    validate_required_evidence_field(
        candidate_id,
        "resolved_load_target",
        &fact.resolved_load_target,
    )?;
    validate_required_evidence_field(
        candidate_id,
        "runtime_residency_key",
        &fact.runtime_residency_key,
    )?;
    if fact.loaded_runtime_memory_estimate_bytes == 0 {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidMemoryEstimate {
                candidate_id: candidate_id.to_string(),
            },
        );
    }
    if fact
        .runtime_instance_id
        .as_ref()
        .is_some_and(|runtime_instance_id| runtime_instance_id.trim().is_empty())
    {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidRuntimeInstanceFact {
                candidate_id: candidate_id.to_string(),
            },
        );
    }
    if matches!(
        fact.runtime_load_state,
        WorkflowRuntimeDispatchLoadState::Loaded | WorkflowRuntimeDispatchLoadState::Busy
    ) && fact.runtime_instance_id.is_none()
    {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidRuntimeInstanceFact {
                candidate_id: candidate_id.to_string(),
            },
        );
    }
    Ok(())
}

fn validate_required_evidence_field(
    candidate_id: &str,
    field_path: &'static str,
    value: &str,
) -> Result<(), WorkflowRuntimeDispatchCandidateFactBundleError> {
    if value.trim().is_empty() {
        return Err(
            WorkflowRuntimeDispatchCandidateFactBundleError::MissingSelectedEvidence {
                candidate_id: candidate_id.to_string(),
                field_path,
            },
        );
    }
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

fn dispatch_candidate(fact: WorkflowRuntimeDispatchCandidateFact) -> SchedulerDispatchCandidate {
    SchedulerDispatchCandidate {
        candidate_id: fact.candidate_id,
        selected_runtime_id: fact.selected_runtime_id,
        selected_runtime_variant_id: fact.selected_runtime_variant_id,
        selected_device_ids: fact.selected_device_ids,
        selected_model_ref: fact.selected_model_ref,
        runtime_trait_settings: fact.runtime_trait_settings,
        reservations: fact.reservations,
        resource_fit_assessment: Some(fact.resource_fit_assessment),
        batching_group_id: fact.batching_group_id,
        candidate_source_diagnostics: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use pantograph_dependency_planning::{DependencyEnvironmentId, DependencyEnvironmentRef};
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionPort,
        RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
        SchedulerRuntimeHostDispatcher,
    };
    use pantograph_scheduler::{
        SchedulerDispatchSelectionDiagnosticCode, SchedulerDispatchSelectionDiagnosticSeverity,
        SchedulerNodeId, SchedulerReservationLeaseId, SchedulerResourceFitAssessment,
        SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
        SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateRecord,
        SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };

    use super::*;

    #[derive(Debug)]
    struct RecordingCustody {
        rollbacks: Arc<std::sync::atomic::AtomicUsize>,
        transfers: Arc<std::sync::atomic::AtomicUsize>,
        transferred: bool,
    }

    impl Drop for RecordingCustody {
        fn drop(&mut self) {
            if !self.transferred {
                self.rollbacks
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    impl WorkflowRuntimeDispatchReservationCustody for RecordingCustody {
        fn transfer(
            mut self: Box<Self>,
        ) -> Result<(), WorkflowRuntimeDispatchCandidateProviderError> {
            self.transferred = true;
            self.transfers
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    struct CustodyProvider {
        rollbacks: Arc<std::sync::atomic::AtomicUsize>,
        transfers: Arc<std::sync::atomic::AtomicUsize>,
    }

    impl WorkflowRuntimeDispatchCandidateProvider for CustodyProvider {
        fn runtime_dispatch_candidates(
            &self,
            _task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            _proof: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
                candidate_fact_bundle(vec![candidate_fact()]),
            )
            .unwrap();
            Ok(
                WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle)
                    .with_reservation_custody(Box::new(RecordingCustody {
                        rollbacks: self.rollbacks.clone(),
                        transfers: self.transfers.clone(),
                        transferred: false,
                    })),
            )
        }
    }

    #[tokio::test]
    async fn cancelling_prepared_dispatch_before_start_drops_unbound_custody() {
        let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transfers = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let boundary = preselection_boundary(
            RecordingRuntimeDispatchSourceRefresher,
            CustodyProvider {
                rollbacks: rollbacks.clone(),
                transfers: transfers.clone(),
            },
        );
        let task = runtime_task_fixture();
        let prepared = boundary
            .prepare_ready_runtime_task_dispatch(
                &task,
                &ready_record_fixture(&task),
                runtime_dispatch_readiness_proof_fixture(),
            )
            .await
            .unwrap();
        assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 0);
        drop(prepared);
        assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(transfers.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn invalid_prepared_selection_releases_custody_acquired_by_provider() {
        let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transfers = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let boundary = preselection_boundary(
            RecordingRuntimeDispatchSourceRefresher,
            CustodyProvider {
                rollbacks: rollbacks.clone(),
                transfers,
            },
        );
        let mut task = runtime_task_fixture();
        task.schedulable_intent = None;
        assert!(boundary
            .prepare_ready_runtime_task_dispatch(
                &task,
                &ready_record_fixture(&task),
                runtime_dispatch_readiness_proof_fixture()
            )
            .await
            .is_err());
        assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn start_failure_cancelled_binding_and_successful_binding_have_one_custody_owner() {
        for scenario in [
            "start_failure",
            "cancel_before_bind",
            "selection_rejected",
            "bound",
            "snapshot_version_changed",
            "snapshot_inputs_changed",
            "snapshot_proof_changed",
        ] {
            let rollbacks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let transfers = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let boundary = preselection_boundary(
                RecordingRuntimeDispatchSourceRefresher,
                CustodyProvider {
                    rollbacks: rollbacks.clone(),
                    transfers: transfers.clone(),
                },
            );
            let mut task = runtime_task_fixture();
            if scenario == "snapshot_inputs_changed" {
                task.input_bindings
                    .push(super::super::WorkflowSchedulerTaskInputBinding {
                        source_node_id: "node.source".parse().unwrap(),
                        source_task_id: "task.source".parse().unwrap(),
                        source_port_id: "text".into(),
                        target_port_id: "prompt".into(),
                    });
            }
            let run_id = task.workflow_run_id.as_str();
            let mut store = crate::scheduler::WorkflowExecutionSessionStore::new(1, 1);
            let session = store
                .create_session(
                    task.workflow_id.to_string(),
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                    true,
                )
                .unwrap();
            let request = super::super::WorkflowExecutionSessionRunRequest {
                session_id: session.clone(),
                workflow_semantic_version: "1.0.0".into(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            };
            let queued = store
                .enqueue_run_with_id(&session, &request, run_id.to_owned())
                .unwrap();
            store.begin_queued_run(&session, &queued).unwrap().unwrap();
            let graph = super::super::WorkflowSchedulerTaskGraph {
                schema_version: super::super::WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
                workflow_id: task.workflow_id.clone(),
                workflow_run_id: task.workflow_run_id.clone(),
                tasks: vec![task.clone()],
            };
            let source_result = |text: &str| super::super::WorkflowSchedulerTaskResult {
                schema_version: super::super::WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
                workflow_id: task.workflow_id.to_string(),
                workflow_run_id: run_id.into(),
                node_id: "node.source".into(),
                task_id: "task.source".into(),
                status: super::super::WorkflowSchedulerTaskResultStatus::Completed,
                outputs: vec![super::super::WorkflowSchedulerTaskResultOutput {
                    port_id: "text".into(),
                    value: super::super::WorkflowSchedulerTaskResultValue::String(text.into()),
                }],
                diagnostics: vec![],
                terminal_metadata: None,
            };
            if scenario == "snapshot_inputs_changed" {
                store
                    .record_active_run_scheduler_task_result(
                        &session,
                        run_id,
                        source_result("original"),
                    )
                    .unwrap();
            }
            let orchestrator = boundary.scheduler_task_orchestrator;
            orchestrator
                .initialize_active_run_task_state(&mut store, &session, run_id, graph)
                .unwrap();
            orchestrator
                .apply_runtime_dependency_readiness_admission(
                    &mut store,
                    &session,
                    run_id,
                    task.task_id.as_str(),
                    pantograph_dependency_planning::DependencyReadinessPolicy::CheckOnly,
                    Some(runtime_dispatch_readiness_proof_fixture()),
                )
                .unwrap();
            let (original_graph, original_records) = store
                .active_run_scheduler_task_state(&session, run_id)
                .unwrap()
                .unwrap();
            let ready = original_records
                .iter()
                .find(|r| r.task_id == task.task_id)
                .unwrap();
            let original_inputs = store.active_run_completion_inputs(&session, run_id, &task);
            let proof = runtime_dispatch_readiness_proof_fixture();
            store
                .validate_ready_dispatch_snapshot(
                    &session,
                    run_id,
                    &task,
                    ready,
                    &proof,
                    original_inputs.as_deref(),
                    true,
                )
                .unwrap();
            let mut prepared = boundary
                .prepare_ready_runtime_task_dispatch(&task, ready, proof.clone())
                .await
                .unwrap();
            if scenario.starts_with("snapshot_") {
                match scenario {
                    "snapshot_version_changed" => {
                        let mut changed = original_records.clone();
                        changed[0].state_version += 1;
                        store
                            .set_active_run_scheduler_task_state(
                                &session,
                                run_id,
                                original_graph,
                                changed,
                            )
                            .unwrap();
                    }
                    "snapshot_inputs_changed" => store
                        .record_active_run_scheduler_task_result(
                            &session,
                            run_id,
                            source_result("replacement"),
                        )
                        .unwrap(),
                    "snapshot_proof_changed" => {
                        let mut changed = proof.clone();
                        changed.execution_context.correlation_id =
                            "changed-correlation".parse().unwrap();
                        store
                            .record_active_run_runtime_dispatch_readiness_proof(
                                &session,
                                run_id,
                                task.task_id.as_str(),
                                changed,
                            )
                            .unwrap();
                    }
                    _ => unreachable!(),
                }
                // This is the authoritative fence used by both real runner paths;
                // it runs before attempt creation while holding the start lock.
                assert!(store
                    .validate_ready_dispatch_snapshot(
                        &session,
                        run_id,
                        &task,
                        ready,
                        &proof,
                        original_inputs.as_deref(),
                        true
                    )
                    .is_err());
                assert!(store
                    .active_run_scheduler_task_attempt_read_facts(&session, run_id)
                    .unwrap()
                    .is_empty());
                drop(prepared);
                assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 1);
                assert_eq!(transfers.load(std::sync::atomic::Ordering::SeqCst), 0);
                continue;
            }
            if scenario == "selection_rejected" {
                let mut raw = prepared.selection_request.into_inner();
                raw.candidates[0].selected_runtime_id = "unrequested-runtime".parse().unwrap();
                prepared.selection_request = raw.try_into().unwrap();
            }
            let started = orchestrator
                .start_ready_runtime_task(&mut store, &session, run_id, task.task_id.as_str())
                .unwrap();
            if scenario == "start_failure" {
                assert!(orchestrator
                    .start_ready_runtime_task(&mut store, &session, run_id, task.task_id.as_str())
                    .is_err());
                drop(prepared);
                assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 1);
                continue;
            }
            if scenario == "selection_rejected" {
                let error = boundary
                    .select_prepared_started_runtime_task_dispatch(&started, prepared)
                    .await
                    .unwrap_err();
                assert!(matches!(
                    error,
                    WorkflowRuntimeDispatchPreselectionError::SchedulerSelection(
                        WorkflowSchedulerTaskOrchestratorError::RuntimeDispatchSelectionNoSelection(
                            _
                        )
                    )
                ));
                assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 1);
                assert_eq!(transfers.load(std::sync::atomic::Ordering::SeqCst), 0);
                continue;
            }
            let mut preselection = boundary
                .select_prepared_started_runtime_task_dispatch(&started, prepared)
                .await
                .unwrap();
            if scenario == "cancel_before_bind" {
                let mutation = orchestrator
                    .cancel_started_runtime_task_terminal_mutation(
                        &mut store,
                        &session,
                        run_id,
                        &started,
                        "controlled cancellation",
                    )
                    .unwrap();
                assert!(mutation.reservation_release_intent.is_none());
                assert!(orchestrator
                    .bind_started_runtime_task_reservation(
                        &mut store,
                        &session,
                        run_id,
                        &started,
                        &preselection.selected_dispatch
                    )
                    .is_err());
                drop(preselection);
                assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 1);
            } else {
                orchestrator
                    .bind_started_runtime_task_reservation(
                        &mut store,
                        &session,
                        run_id,
                        &started,
                        &preselection.selected_dispatch,
                    )
                    .unwrap();
                preselection.transfer_reservation_custody().unwrap();
                drop(preselection);
                let mutation = orchestrator
                    .cancel_started_runtime_task_terminal_mutation(
                        &mut store,
                        &session,
                        run_id,
                        &started,
                        "controlled cancellation",
                    )
                    .unwrap();
                assert!(mutation.reservation_release_intent.is_some());
                assert_eq!(rollbacks.load(std::sync::atomic::Ordering::SeqCst), 0);
                assert_eq!(transfers.load(std::sync::atomic::Ordering::SeqCst), 1);
            }
        }
    }

    #[tokio::test]
    async fn preselection_boundary_prepares_request_with_retained_candidate_evidence() {
        let candidate_fact = candidate_fact();
        let candidate_id = candidate_fact.candidate_id.clone();
        let task = runtime_task_fixture();
        let ready_record = ready_record_fixture(&task);
        let readiness_proof = runtime_dispatch_readiness_proof_fixture();
        let boundary = preselection_boundary(
            RecordingRuntimeDispatchSourceRefresher,
            StaticRuntimeDispatchCandidateProvider::from_facts(vec![candidate_fact]),
        );

        let prepared = boundary
            .prepare_ready_runtime_task_dispatch(&task, &ready_record, readiness_proof)
            .await
            .expect("selection request should prepare");

        assert!(prepared
            .candidate_evidence_context
            .contains_candidate_id(&candidate_id));
        assert_eq!(prepared.selection_request.as_ref().candidates.len(), 1);
    }

    #[tokio::test]
    async fn preselection_boundary_returns_typed_refresh_diagnostic() {
        let task = runtime_task_fixture();
        let ready_record = ready_record_fixture(&task);
        let readiness_proof = runtime_dispatch_readiness_proof_fixture();
        let boundary = preselection_boundary(
            FailingRuntimeDispatchSourceRefresher,
            StaticRuntimeDispatchCandidateProvider::from_facts(vec![candidate_fact()]),
        );

        let error = boundary
            .prepare_ready_runtime_task_dispatch(&task, &ready_record, readiness_proof)
            .await
            .expect_err("source refresh failure should fail closed");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchPreselectionError::SourceRefresh(_)
        ));
        assert!(error
            .to_string()
            .contains("runtime dispatch source refresh failed"));
    }

    #[tokio::test]
    async fn preselection_boundary_returns_typed_candidate_collection_diagnostic() {
        let task = runtime_task_fixture();
        let ready_record = ready_record_fixture(&task);
        let readiness_proof = runtime_dispatch_readiness_proof_fixture();
        let boundary = preselection_boundary(
            RecordingRuntimeDispatchSourceRefresher,
            FailingRuntimeDispatchCandidateProvider,
        );

        let error = boundary
            .prepare_ready_runtime_task_dispatch(&task, &ready_record, readiness_proof)
            .await
            .expect_err("candidate collection failure should fail closed");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchPreselectionError::CandidateCollection(_)
        ));
        assert!(error
            .to_string()
            .contains("runtime dispatch candidate collection failed"));
    }

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
    fn candidate_fact_bundle_rejects_missing_reservations() {
        let mut fact = candidate_fact();
        fact.reservations.clear();
        let bundle = candidate_fact_bundle(vec![fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("candidate facts must carry reservation evidence");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::MissingReservation { .. }
        ));
    }

    #[test]
    fn candidate_fact_bundle_rejects_missing_runtime_family_evidence() {
        let mut fact = candidate_fact();
        fact.runtime_family.clear();
        let bundle = candidate_fact_bundle(vec![fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("candidate facts must carry runtime family evidence");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::MissingSelectedEvidence {
                field_path: "runtime_family",
                ..
            }
        ));
    }

    #[test]
    fn candidate_fact_bundle_rejects_loaded_runtime_without_instance_id() {
        let mut fact = candidate_fact();
        fact.runtime_instance_id = None;
        let bundle = candidate_fact_bundle(vec![fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("loaded runtime candidates must carry runtime instance evidence");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidRuntimeInstanceFact { .. }
        ));
    }

    #[test]
    fn candidate_fact_bundle_rejects_zero_memory_estimate() {
        let mut fact = candidate_fact();
        fact.loaded_runtime_memory_estimate_bytes = 0;
        let bundle = candidate_fact_bundle(vec![fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("candidate facts must carry memory estimate evidence");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::InvalidMemoryEstimate { .. }
        ));
    }

    #[test]
    fn candidate_fact_bundle_rejects_mixed_reservation_leases() {
        let mut fact = candidate_fact();
        let mut reservation = fact.reservations[0].clone();
        reservation.reservation_lease_id =
            SchedulerReservationLeaseId::parse("reservation.dispatch-facts.other")
                .expect("reservation id");
        reservation.device_id = "cuda:1".parse().expect("device id");
        fact.selected_device_ids.push(reservation.device_id.clone());
        fact.reservations.push(reservation);
        let bundle = candidate_fact_bundle(vec![fact]);

        let error = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(bundle)
            .expect_err("candidate facts must not mix reservation leases");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchCandidateFactBundleError::MixedReservationLease { .. }
        ));
    }

    #[test]
    fn candidate_fact_bundle_maps_path_free_facts_to_scheduler_candidates() {
        let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            candidate_fact_bundle(vec![candidate_fact()]),
        )
        .expect("candidate fact bundle");

        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle);

        assert_eq!(candidate_set.candidates.len(), 1);
        assert_eq!(candidate_set.diagnostics.len(), 1);
        let candidate = &candidate_set.candidates[0];
        assert_eq!(candidate.candidate_id.as_str(), "candidate.diffusers.cuda0");
        assert_eq!(candidate.selected_runtime_id.as_str(), "diffusers-pytorch");
        assert_eq!(candidate.selected_device_ids[0].as_str(), "cuda:0");
        assert_eq!(candidate.reservations.len(), 1);
        assert!(candidate.resource_fit_assessment.is_some());
        assert!(candidate.candidate_source_diagnostics.is_empty());
    }

    #[test]
    fn candidate_fact_bundle_retains_workflow_service_evidence_context() {
        let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            candidate_fact_bundle(vec![candidate_fact()]),
        )
        .expect("candidate fact bundle");

        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle);
        let candidate = &candidate_set.candidates[0];
        let fact = candidate_set
            .candidate_evidence_context
            .candidate_fact(&candidate.candidate_id)
            .expect("candidate evidence context should retain validated fact");

        assert_eq!(candidate_set.candidate_evidence_context.len(), 1);
        assert!(candidate_set
            .candidate_evidence_context
            .contains_candidate_id(&candidate.candidate_id));
        assert_eq!(fact.candidate_id, candidate.candidate_id);
        assert_eq!(fact.selected_backend_key, "diffusers");
        assert_eq!(
            fact.resolved_load_target,
            "pumas:image/example/tiny-diffusion:diffusers"
        );
        assert_eq!(
            fact.runtime_instance_id.as_deref(),
            Some("runtime.diffusers-pytorch.001")
        );
        assert_eq!(
            candidate.selected_runtime_id.as_str(),
            fact.selected_runtime_id.as_str()
        );
        assert!(candidate.candidate_source_diagnostics.is_empty());
    }

    #[test]
    fn diagnostics_only_candidate_set_has_no_evidence_context() {
        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_diagnostics(vec![
            SchedulerDispatchSelectionDiagnostic {
                severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
                code: SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                message: "candidate source unavailable".to_string(),
                candidate_id: None,
                hint: None,
            },
        ]);

        assert!(candidate_set.candidates.is_empty());
        assert_eq!(candidate_set.diagnostics.len(), 1);
        assert!(candidate_set.candidate_evidence_context.is_empty());
    }

    #[test]
    fn selected_candidate_fact_resolves_from_retained_evidence_context() {
        let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            candidate_fact_bundle(vec![candidate_fact()]),
        )
        .expect("candidate fact bundle");
        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle);
        let candidate_id = candidate_set.candidates[0].candidate_id.clone();

        let selected_fact = selected_runtime_dispatch_candidate_fact(
            Some(&candidate_id),
            &candidate_set.candidate_evidence_context,
        )
        .expect("selected candidate fact should resolve");

        assert_eq!(selected_fact.candidate_id, candidate_id);
        assert_eq!(selected_fact.selected_backend_key, "diffusers");
        assert_eq!(
            selected_fact.runtime_instance_id.as_deref(),
            Some("runtime.diffusers-pytorch.001")
        );
    }

    #[test]
    fn selected_candidate_fact_rejects_missing_selected_candidate_id() {
        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_diagnostics(Vec::new());

        let error = selected_runtime_dispatch_candidate_fact(
            None,
            &candidate_set.candidate_evidence_context,
        )
        .expect_err("missing selected candidate id must fail closed");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchSelectionError::MissingSelectedCandidateId
        ));
    }

    #[test]
    fn selected_candidate_fact_rejects_stale_selected_candidate_id() {
        let candidate_set = WorkflowRuntimeDispatchCandidateSet::from_diagnostics(Vec::new());
        let stale_candidate_id: SchedulerDispatchCandidateId =
            "candidate.stale".parse().expect("candidate id");

        let error = selected_runtime_dispatch_candidate_fact(
            Some(&stale_candidate_id),
            &candidate_set.candidate_evidence_context,
        )
        .expect_err("stale selected candidate id must fail closed");

        assert!(matches!(
            error,
            WorkflowRuntimeDispatchSelectionError::MissingSelectedCandidateFact { candidate_id }
                if candidate_id == "candidate.stale"
        ));
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
            "run.2026-05-22.001".parse().expect("workflow run id");
        let task_id: SchedulerTaskId = "task.image_generation.001".parse().expect("task id");
        let device_id: DeviceIntentId = "cuda:0".parse().expect("device id");
        WorkflowRuntimeDispatchCandidateFact {
            candidate_id: "candidate.diffusers.cuda0".parse().expect("candidate id"),
            selected_runtime_id: "diffusers-pytorch".parse().expect("runtime id"),
            selected_runtime_variant_id: Some(
                "diffusers-pytorch.cuda".parse().expect("variant id"),
            ),
            selected_backend_key: "diffusers".to_string(),
            runtime_family: "diffusers".to_string(),
            resolved_load_target: "pumas:image/example/tiny-diffusion:diffusers".to_string(),
            runtime_residency_key: "runtime.diffusers.diffusers-pytorch.shared".to_string(),
            loaded_runtime_memory_estimate_bytes: 8 * 1024 * 1024,
            runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
            runtime_instance_id: Some("runtime.diffusers-pytorch.001".to_string()),
            selected_device_ids: vec![device_id.clone()],
            selected_model_ref: PumasModelRef {
                model_id: "pumas://models/juggernaut-xl-v10".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("diffusers-bundle".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            runtime_trait_settings: Vec::new(),
            environment_ref: DependencyEnvironmentRef {
                environment_id: DependencyEnvironmentId::parse("env.dispatch-facts")
                    .expect("environment id"),
                manifest_id: None,
            },
            reservations: vec![SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(
                    "reservation.dispatch-facts",
                )
                .expect("reservation id"),
                workflow_run_id: workflow_run_id.clone(),
                task_id: task_id.clone(),
                device_id,
                resource_kind: SchedulerResourceKind::DeviceVram,
                reserved_bytes: 1,
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

    fn preselection_boundary<
        R: WorkflowRuntimeDispatchSourceRefresher + 'static,
        P: WorkflowRuntimeDispatchCandidateProvider + 'static,
    >(
        source_refresher: R,
        candidate_provider: P,
    ) -> WorkflowRuntimeDispatchSelectionBoundary<'static> {
        let source_refresher = Box::leak(Box::new(source_refresher));
        let candidate_provider = Box::leak(Box::new(candidate_provider));
        let scheduler_task_orchestrator =
            Box::leak(Box::new(WorkflowSchedulerTaskOrchestrator::new(
                SchedulerRuntimeHostDispatcher::new(Arc::new(UnusedRuntimeHostPort)),
            )));
        WorkflowRuntimeDispatchSelectionBoundary::new(
            source_refresher,
            candidate_provider,
            scheduler_task_orchestrator,
        )
    }

    fn runtime_task_fixture() -> WorkflowSchedulerTask {
        let request = runtime_host_request_fixture();
        let workflow_id: SchedulerWorkflowId =
            "workflow.image_generation".parse().expect("workflow id");
        let workflow_run_id: SchedulerWorkflowRunId =
            "run.2026-05-22.001".parse().expect("workflow run id");
        WorkflowSchedulerTask {
            workflow_id,
            workflow_run_id,
            node_id: SchedulerNodeId::parse("node.image_generation").expect("node id"),
            task_id: SchedulerTaskId::parse("task.image_generation.001").expect("task id"),
            node_type: "image.generate".to_string(),
            execution_class: super::super::WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            dependency_task_ids: Vec::new(),
            input_bindings: Vec::new(),
            schedulable_intent: Some(request.handoff.task_intent),
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: None,
            diagnostics: Vec::new(),
        }
    }

    fn ready_record_fixture(task: &WorkflowSchedulerTask) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: task.workflow_id.clone(),
            workflow_run_id: task.workflow_run_id.clone(),
            node_id: task.node_id.clone(),
            task_id: task.task_id.clone(),
            state: SchedulerTaskState::AwaitingInputs {
                diagnostics: Vec::new(),
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse(
                "initial:task.image_generation.001",
            )
            .expect("transition id"),
        }
    }

    fn runtime_dispatch_readiness_proof_fixture() -> DependencyReadinessProofEnvelope {
        runtime_host_request_fixture().handoff.readiness_proof
    }

    fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host request fixture")
    }

    struct RecordingRuntimeDispatchSourceRefresher;

    #[async_trait]
    impl WorkflowRuntimeDispatchSourceRefresher for RecordingRuntimeDispatchSourceRefresher {
        async fn refresh_runtime_dispatch_sources(
            &self,
            _task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            _readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
            Ok(())
        }
    }

    struct FailingRuntimeDispatchSourceRefresher;

    #[async_trait]
    impl WorkflowRuntimeDispatchSourceRefresher for FailingRuntimeDispatchSourceRefresher {
        async fn refresh_runtime_dispatch_sources(
            &self,
            _task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            _readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<(), WorkflowRuntimeDispatchSourceRefreshError> {
            Err(WorkflowRuntimeDispatchSourceRefreshError::Failed {
                message: "source ledger unavailable".to_string(),
            })
        }
    }

    struct StaticRuntimeDispatchCandidateProvider {
        candidate_set: WorkflowRuntimeDispatchCandidateSet,
    }

    impl StaticRuntimeDispatchCandidateProvider {
        fn from_facts(facts: Vec<WorkflowRuntimeDispatchCandidateFact>) -> Self {
            let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
                candidate_fact_bundle(facts),
            )
            .expect("candidate fact bundle");
            Self {
                candidate_set: WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(
                    bundle,
                ),
            }
        }
    }

    impl WorkflowRuntimeDispatchCandidateProvider for StaticRuntimeDispatchCandidateProvider {
        fn runtime_dispatch_candidates(
            &self,
            _task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            _readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            Ok(WorkflowRuntimeDispatchCandidateSet {
                candidates: self.candidate_set.candidates.clone(),
                diagnostics: self.candidate_set.diagnostics.clone(),
                candidate_evidence_context: self.candidate_set.candidate_evidence_context.clone(),
                reservation_custody: None,
            })
        }
    }

    struct FailingRuntimeDispatchCandidateProvider;

    impl WorkflowRuntimeDispatchCandidateProvider for FailingRuntimeDispatchCandidateProvider {
        fn runtime_dispatch_candidates(
            &self,
            _task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            _readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            Err(WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: "candidate provider unavailable".to_string(),
            })
        }
    }

    struct UnusedRuntimeHostPort;

    #[async_trait]
    impl RuntimeHostExecutionPort for UnusedRuntimeHostPort {
        async fn execute_runtime_host_request(
            &self,
            _request: RuntimeHostExecutionRequest,
            _cancellation: RuntimeHostExecutionCancellationHandle,
        ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
            Err(RuntimeHostExecutionPortError::ExecutionFailed {
                message: "runtime host should not be used by preselection boundary tests"
                    .to_string(),
            })
        }
    }
}
