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

use super::{WorkflowSchedulerTask, WorkflowServiceError};

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
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkflowRuntimeDispatchCandidateSet {
    pub candidates: Vec<SchedulerDispatchCandidate>,
    pub diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>,
    pub candidate_evidence_context: WorkflowRuntimeDispatchCandidateEvidenceContext,
}

impl WorkflowRuntimeDispatchCandidateSet {
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
        }
    }

    pub fn from_diagnostics(diagnostics: Vec<SchedulerDispatchSelectionDiagnostic>) -> Self {
        Self {
            candidates: Vec::new(),
            diagnostics,
            candidate_evidence_context: WorkflowRuntimeDispatchCandidateEvidenceContext::default(),
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
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowRuntimeDispatchSelectionRequest {
    pub(crate) selection_request: ValidatedSchedulerDispatchSelectionRequest,
    pub(crate) candidate_evidence_context: WorkflowRuntimeDispatchCandidateEvidenceContext,
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
            "run.dispatch-facts".parse().expect("workflow run id");
        let task_id: SchedulerTaskId = "infer".parse().expect("task id");
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
}
