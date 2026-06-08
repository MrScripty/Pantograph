use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use pantograph_dependency_planning::{
    DependencyReadinessProofEnvelope, PumasModelRef, RuntimeIntentId,
};
use pantograph_runtime_registry::{
    RuntimeRegistryStatus, RuntimeReservationRequirements, RuntimeReservationResourceClaim,
    RuntimeRetentionHint,
};
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerDispatchSelectionDiagnostic,
    SchedulerDispatchSelectionDiagnosticCode, SchedulerDispatchSelectionDiagnosticSeverity,
    SchedulerEstimateHintKind, SchedulerResourceFitAssessment, SchedulerResourceFitState,
    SchedulerTaskStateRecord,
};
use pantograph_workflow_service::workflow::{
    ValidatedWorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateFact,
    WorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateProvider,
    WorkflowRuntimeDispatchCandidateProviderError, WorkflowRuntimeDispatchCandidateSet,
};
use pantograph_workflow_service::WorkflowSchedulerTask;

use crate::inference_resource_estimator::conservative_loaded_runtime_memory_estimate_bytes;
use crate::pumas_dispatch_package_facts::{
    PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsDiagnostic,
    PumasDispatchPackageFactsDiagnosticCode, PumasDispatchPackageFactsProjection,
};
use crate::runtime_dispatch_capability_facts::{
    RuntimeDispatchCapabilityFactsDiagnostic, RuntimeDispatchCapabilityFactsOutcome,
    RuntimeDispatchCapabilityFactsProjection, RuntimeDispatchRuntimeCapabilityFacts,
};
use crate::runtime_dispatch_evidence::{
    RuntimeDispatchEvidenceDiagnostic, RuntimeDispatchEvidenceLoadState,
    RuntimeDispatchEvidenceRecord, RuntimeDispatchEvidenceRequest,
};
use crate::runtime_dispatch_resource_facts::{
    RuntimeDispatchResourceFactsDiagnostic, RuntimeDispatchResourceFactsOutcome,
    RuntimeDispatchResourceFactsRequest, RuntimeDispatchResourceFactsSource,
};
use crate::runtime_dispatch_source_snapshot::{
    EmbeddedRuntimeDispatchCandidateSourceSnapshot, EmbeddedRuntimeDispatchSourceFactSnapshotStore,
    EmbeddedRuntimeDispatchSourceSnapshotDiagnostic,
    EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode,
};

const MISSING_PUMAS_PACKAGE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_pumas_package_facts";
const MISSING_RUNTIME_CAPABILITY_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_capability_facts";
const MISSING_RUNTIME_RESOURCE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_resource_facts";
const MISSING_DEPENDENCY_ENVIRONMENT_REF_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_dependency_environment_ref";
const MISSING_SELECTED_DEVICE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_selected_device_facts";
const PATH_CARRYING_MODEL_REF_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.path_carrying_model_ref";
const INCOMPATIBLE_RUNTIME_BACKEND_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.incompatible_runtime_backend";
const MISSING_RUNTIME_DISPATCH_EVIDENCE_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_dispatch_evidence";

#[derive(Clone)]
pub(crate) struct EmbeddedRuntimeDispatchCandidateProvider {
    source_snapshot: EmbeddedRuntimeDispatchCandidateSource,
    resource_facts_source: Option<RuntimeDispatchResourceFactsSource>,
}

#[derive(Clone)]
enum EmbeddedRuntimeDispatchCandidateSource {
    Snapshot(EmbeddedRuntimeDispatchCandidateSourceSnapshot),
    Store(EmbeddedRuntimeDispatchSourceFactSnapshotStore),
}

impl Default for EmbeddedRuntimeDispatchCandidateSource {
    fn default() -> Self {
        Self::Snapshot(EmbeddedRuntimeDispatchCandidateSourceSnapshot::default())
    }
}

impl Default for EmbeddedRuntimeDispatchCandidateProvider {
    fn default() -> Self {
        Self {
            source_snapshot: EmbeddedRuntimeDispatchCandidateSource::default(),
            resource_facts_source: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmbeddedRuntimeDispatchCandidateDraft {
    candidate_id: SchedulerDispatchCandidateId,
    selected_runtime_id: RuntimeIntentId,
    selected_backend_key: String,
    selected_model_ref: PumasModelRef,
    loaded_runtime_memory_estimate_bytes: Option<u64>,
    runtime_status: RuntimeRegistryStatus,
    runtime_instance_id: Option<String>,
}

impl EmbeddedRuntimeDispatchCandidateProvider {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn with_source_snapshot(
        source_snapshot: EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    ) -> Self {
        Self {
            source_snapshot: EmbeddedRuntimeDispatchCandidateSource::Snapshot(source_snapshot),
            resource_facts_source: None,
        }
    }

    #[must_use]
    pub(crate) fn with_source_snapshot_store(
        source_snapshot_store: EmbeddedRuntimeDispatchSourceFactSnapshotStore,
    ) -> Self {
        Self {
            source_snapshot: EmbeddedRuntimeDispatchCandidateSource::Store(source_snapshot_store),
            resource_facts_source: None,
        }
    }

    #[must_use]
    pub(crate) fn with_resource_facts_source(
        mut self,
        resource_facts_source: RuntimeDispatchResourceFactsSource,
    ) -> Self {
        self.resource_facts_source = Some(resource_facts_source);
        self
    }
}

impl WorkflowRuntimeDispatchCandidateProvider for EmbeddedRuntimeDispatchCandidateProvider {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        let source_snapshot = self
            .source_snapshot
            .snapshot_for_dispatch(&readiness_proof.preflight_result.identity_key.model_ref);
        if let Some(resource_facts_source) = &self.resource_facts_source {
            return resource_backed_candidate_set(
                &source_snapshot,
                resource_facts_source,
                task,
                readiness_proof,
            );
        }
        Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: Vec::new(),
            diagnostics: fail_closed_diagnostics(
                &source_snapshot,
                &readiness_proof.preflight_result.identity_key.model_ref,
            ),
        })
    }
}

impl EmbeddedRuntimeDispatchCandidateSource {
    fn snapshot_for_dispatch(
        &self,
        model_ref: &PumasModelRef,
    ) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
        match self {
            Self::Snapshot(snapshot) => snapshot.clone(),
            Self::Store(store) => store.snapshot_for_dispatch(model_ref, current_time_ms()),
        }
    }
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn resource_backed_candidate_set(
    source_snapshot: &EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    resource_facts_source: &RuntimeDispatchResourceFactsSource,
    task: &WorkflowSchedulerTask,
    readiness_proof: &DependencyReadinessProofEnvelope,
) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError> {
    let model_ref = &readiness_proof.preflight_result.identity_key.model_ref;
    if model_ref.selected_artifact_path.is_some() {
        return Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: Vec::new(),
            diagnostics: vec![provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                "runtime dispatch candidate provider rejected a path-carrying Pumas model ref",
                PATH_CARRYING_MODEL_REF_HINT,
            )],
        });
    }

    let mut diagnostics = Vec::new();
    diagnostics.extend(pumas_package_diagnostics(
        source_snapshot.pumas_package_facts.as_ref(),
    ));
    diagnostics.extend(runtime_capability_diagnostics(
        source_snapshot.runtime_capability_facts.as_ref(),
    ));
    diagnostics.extend(source_snapshot_diagnostics(&source_snapshot.diagnostics));
    let (candidate_drafts, draft_diagnostics) = candidate_drafts(source_snapshot);
    diagnostics.extend(draft_diagnostics);

    let Some(task_intent) = task.schedulable_intent.as_ref() else {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            "runtime dispatch candidate provider requires a schedulable task intent",
            MISSING_RUNTIME_RESOURCE_FACTS_HINT,
        ));
        return Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: Vec::new(),
            diagnostics,
        });
    };
    let Some(environment_ref) = readiness_proof.preflight_result.environment_ref.clone() else {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            "runtime dispatch candidate provider requires a dependency environment ref",
            MISSING_DEPENDENCY_ENVIRONMENT_REF_HINT,
        ));
        return Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: Vec::new(),
            diagnostics,
        });
    };
    let Some(selected_device_id) = task_intent.constraints.requested_device_id.clone() else {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::IncompatibleDeviceRequirement,
            "runtime dispatch candidate provider requires an explicit selected device until runtime capability facts expose device candidates",
            MISSING_SELECTED_DEVICE_FACTS_HINT,
        ));
        return Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: Vec::new(),
            diagnostics,
        });
    };

    let mut facts = Vec::new();
    for draft in candidate_drafts {
        if let Err(evidence_diagnostic) =
            pre_reservation_evidence_check(&draft, task_intent, selected_device_id.clone())
        {
            diagnostics.push(runtime_dispatch_evidence_diagnostic(
                &draft.candidate_id,
                evidence_diagnostic,
            ));
            continue;
        }
        match resource_facts_source.reserve(resource_facts_request(
            &draft,
            task_intent,
            selected_device_id.clone(),
        )) {
            RuntimeDispatchResourceFactsOutcome::Reserved {
                facts: resource_facts,
                diagnostics: resource_diagnostics,
            } => {
                diagnostics.extend(resource_source_diagnostics(
                    &draft.candidate_id,
                    &resource_diagnostics,
                ));
                facts.push(WorkflowRuntimeDispatchCandidateFact {
                    candidate_id: draft.candidate_id,
                    selected_runtime_id: draft.selected_runtime_id,
                    selected_runtime_variant_id: None,
                    selected_device_ids: vec![selected_device_id.clone()],
                    selected_model_ref: draft.selected_model_ref,
                    runtime_trait_settings: task_intent.trait_settings.clone(),
                    environment_ref: environment_ref.clone(),
                    reservations: resource_facts.reservations,
                    resource_fit_assessment: resource_facts.fit_assessment,
                    batching_group_id: None,
                });
            }
            RuntimeDispatchResourceFactsOutcome::Unavailable {
                diagnostics: resource_diagnostics,
                ..
            } => diagnostics.extend(resource_source_diagnostics(
                &draft.candidate_id,
                &resource_diagnostics,
            )),
        }
    }

    if facts.is_empty() && diagnostics.is_empty() {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
            "runtime dispatch candidate provider produced no resource-backed candidates",
            MISSING_RUNTIME_RESOURCE_FACTS_HINT,
        ));
    }

    let bundle =
        ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            WorkflowRuntimeDispatchCandidateFactBundle {
                contract_version: pantograph_workflow_service::workflow::WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
                facts,
                diagnostics,
            },
        )
        .map_err(|error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
            message: format!("runtime dispatch candidate facts failed validation: {error}"),
        })?;
    Ok(WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle))
}

fn fail_closed_diagnostics(
    source_snapshot: &EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    model_ref: &PumasModelRef,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    let mut diagnostics = Vec::new();
    if model_ref.selected_artifact_path.is_some() {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            "runtime dispatch candidate provider rejected a path-carrying Pumas model ref",
            PATH_CARRYING_MODEL_REF_HINT,
        ));
        return diagnostics;
    }

    diagnostics.extend(pumas_package_diagnostics(
        source_snapshot.pumas_package_facts.as_ref(),
    ));
    diagnostics.extend(runtime_capability_diagnostics(
        source_snapshot.runtime_capability_facts.as_ref(),
    ));
    diagnostics.extend(source_snapshot_diagnostics(&source_snapshot.diagnostics));
    let (_candidate_drafts, draft_diagnostics) = candidate_drafts(source_snapshot);
    diagnostics.extend(draft_diagnostics);
    diagnostics.push(provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        "runtime dispatch candidate provider has no staged runtime resource facts",
        MISSING_RUNTIME_RESOURCE_FACTS_HINT,
    ));
    diagnostics
}

fn source_snapshot_diagnostics(
    diagnostics: &[EmbeddedRuntimeDispatchSourceSnapshotDiagnostic],
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            provider_diagnostic(
                source_snapshot_diagnostic_code(diagnostic.code),
                &diagnostic.message,
                source_snapshot_diagnostic_hint(diagnostic.code),
            )
        })
        .collect()
}

fn source_snapshot_diagnostic_code(
    code: EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode,
) -> SchedulerDispatchSelectionDiagnosticCode {
    match code {
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::PathCarryingModelRef
        | EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::InvalidContractVersion
        | EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::ModelRefMismatch => {
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        }
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::MissingSnapshot
        | EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::StaleSnapshot => {
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates
        }
    }
}

fn source_snapshot_diagnostic_hint(
    code: EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode,
) -> &'static str {
    match code {
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::MissingSnapshot => {
            "embedded_runtime_dispatch_candidate_provider.source_snapshot.missing"
        }
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::StaleSnapshot => {
            "embedded_runtime_dispatch_candidate_provider.source_snapshot.stale"
        }
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::ModelRefMismatch => {
            "embedded_runtime_dispatch_candidate_provider.source_snapshot.model_ref_mismatch"
        }
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::InvalidContractVersion => {
            "embedded_runtime_dispatch_candidate_provider.source_snapshot.invalid_contract_version"
        }
        EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::PathCarryingModelRef => {
            "embedded_runtime_dispatch_candidate_provider.source_snapshot.path_carrying_model_ref"
        }
    }
}

fn pumas_package_diagnostics(
    outcome: Option<&PumasDispatchPackageFactsBridgeOutcome>,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    match outcome {
        Some(PumasDispatchPackageFactsBridgeOutcome::Projected { diagnostics, .. }) => diagnostics
            .iter()
            .map(pumas_package_source_diagnostic)
            .collect(),
        Some(PumasDispatchPackageFactsBridgeOutcome::Unavailable { diagnostics }) => diagnostics
            .iter()
            .map(pumas_package_source_diagnostic)
            .collect(),
        None => vec![provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
            "runtime dispatch candidate provider has no staged Pumas package facts",
            MISSING_PUMAS_PACKAGE_FACTS_HINT,
        )],
    }
}

fn pumas_package_source_diagnostic(
    diagnostic: &PumasDispatchPackageFactsDiagnostic,
) -> SchedulerDispatchSelectionDiagnostic {
    provider_diagnostic(
        pumas_package_diagnostic_code(diagnostic.code),
        &diagnostic.message,
        pumas_package_diagnostic_hint(diagnostic.code),
    )
}

fn pumas_package_diagnostic_code(
    code: PumasDispatchPackageFactsDiagnosticCode,
) -> SchedulerDispatchSelectionDiagnosticCode {
    match code {
        PumasDispatchPackageFactsDiagnosticCode::InvalidModelRef
        | PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef => {
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        }
        PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess
        | PumasDispatchPackageFactsDiagnosticCode::UnsupportedSelectorAccessRole
        | PumasDispatchPackageFactsDiagnosticCode::PackageFactsLookupFailed
        | PumasDispatchPackageFactsDiagnosticCode::PackageFactsDecodeFailed
        | PumasDispatchPackageFactsDiagnosticCode::StalePackageFactsContract
        | PumasDispatchPackageFactsDiagnosticCode::SelectedArtifactMismatch
        | PumasDispatchPackageFactsDiagnosticCode::MissingLogicalSizeFacts
        | PumasDispatchPackageFactsDiagnosticCode::PathFactsStripped => {
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates
        }
    }
}

fn pumas_package_diagnostic_hint(code: PumasDispatchPackageFactsDiagnosticCode) -> &'static str {
    match code {
        PumasDispatchPackageFactsDiagnosticCode::InvalidModelRef => {
            "embedded_runtime_dispatch_candidate_provider.pumas.invalid_model_ref"
        }
        PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef => {
            "embedded_runtime_dispatch_candidate_provider.pumas.path_carrying_model_ref"
        }
        PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess => {
            "embedded_runtime_dispatch_candidate_provider.pumas.missing_selector_access"
        }
        PumasDispatchPackageFactsDiagnosticCode::UnsupportedSelectorAccessRole => {
            "embedded_runtime_dispatch_candidate_provider.pumas.unsupported_selector_access_role"
        }
        PumasDispatchPackageFactsDiagnosticCode::PackageFactsLookupFailed => {
            "embedded_runtime_dispatch_candidate_provider.pumas.package_facts_lookup_failed"
        }
        PumasDispatchPackageFactsDiagnosticCode::PackageFactsDecodeFailed => {
            "embedded_runtime_dispatch_candidate_provider.pumas.package_facts_decode_failed"
        }
        PumasDispatchPackageFactsDiagnosticCode::StalePackageFactsContract => {
            "embedded_runtime_dispatch_candidate_provider.pumas.stale_package_facts_contract"
        }
        PumasDispatchPackageFactsDiagnosticCode::SelectedArtifactMismatch => {
            "embedded_runtime_dispatch_candidate_provider.pumas.selected_artifact_mismatch"
        }
        PumasDispatchPackageFactsDiagnosticCode::MissingLogicalSizeFacts => {
            "embedded_runtime_dispatch_candidate_provider.pumas.missing_logical_size_facts"
        }
        PumasDispatchPackageFactsDiagnosticCode::PathFactsStripped => {
            "embedded_runtime_dispatch_candidate_provider.pumas.path_facts_stripped"
        }
    }
}

fn runtime_capability_diagnostics(
    outcome: Option<&RuntimeDispatchCapabilityFactsOutcome>,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    match outcome {
        Some(RuntimeDispatchCapabilityFactsOutcome::Projected { diagnostics, .. }) => diagnostics
            .iter()
            .map(runtime_capability_source_diagnostic)
            .collect(),
        Some(RuntimeDispatchCapabilityFactsOutcome::Unavailable { diagnostics }) => diagnostics
            .iter()
            .map(runtime_capability_source_diagnostic)
            .collect(),
        None => vec![provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
            "runtime dispatch candidate provider has no staged runtime capability facts",
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT,
        )],
    }
}

fn runtime_capability_source_diagnostic(
    diagnostic: &RuntimeDispatchCapabilityFactsDiagnostic,
) -> SchedulerDispatchSelectionDiagnostic {
    provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        &diagnostic.message,
        "embedded_runtime_dispatch_candidate_provider.runtime_capability.source_diagnostic",
    )
}

fn candidate_drafts(
    source_snapshot: &EmbeddedRuntimeDispatchCandidateSourceSnapshot,
) -> (
    Vec<EmbeddedRuntimeDispatchCandidateDraft>,
    Vec<SchedulerDispatchSelectionDiagnostic>,
) {
    let Some(PumasDispatchPackageFactsBridgeOutcome::Projected {
        facts: package_facts,
        ..
    }) = source_snapshot.pumas_package_facts.as_ref()
    else {
        return (Vec::new(), Vec::new());
    };
    let Some(RuntimeDispatchCapabilityFactsOutcome::Projected {
        facts: capability_facts,
        ..
    }) = source_snapshot.runtime_capability_facts.as_ref()
    else {
        return (Vec::new(), Vec::new());
    };

    candidate_drafts_from_projected_facts(package_facts, capability_facts)
}

fn candidate_drafts_from_projected_facts(
    package_facts: &PumasDispatchPackageFactsProjection,
    capability_facts: &RuntimeDispatchCapabilityFactsProjection,
) -> (
    Vec<EmbeddedRuntimeDispatchCandidateDraft>,
    Vec<SchedulerDispatchSelectionDiagnostic>,
) {
    let backend_hint_keys = backend_hint_keys(&package_facts.backend_hints);
    if backend_hint_keys.is_empty() {
        return (
            Vec::new(),
            vec![provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                "Pumas package facts contain no accepted backend hints for runtime dispatch",
                INCOMPATIBLE_RUNTIME_BACKEND_HINT,
            )],
        );
    }

    let drafts = capability_facts
        .runtimes
        .iter()
        .flat_map(|runtime| runtime_candidate_drafts(package_facts, runtime, &backend_hint_keys))
        .collect::<Vec<_>>();

    if drafts.is_empty() {
        return (
            drafts,
            vec![provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement,
                "no runtime registry capability facts match the Pumas package backend hints",
                INCOMPATIBLE_RUNTIME_BACKEND_HINT,
            )],
        );
    }

    (drafts, Vec::new())
}

fn runtime_candidate_drafts(
    package_facts: &PumasDispatchPackageFactsProjection,
    runtime: &RuntimeDispatchRuntimeCapabilityFacts,
    backend_hint_keys: &BTreeSet<String>,
) -> Vec<EmbeddedRuntimeDispatchCandidateDraft> {
    runtime
        .backend_keys
        .iter()
        .filter_map(|backend_key| {
            let normalized_backend_key = normalize_backend_key(backend_key);
            backend_hint_keys
                .contains(&normalized_backend_key)
                .then(|| runtime_candidate_draft(package_facts, runtime, normalized_backend_key))
        })
        .collect()
}

fn runtime_candidate_draft(
    package_facts: &PumasDispatchPackageFactsProjection,
    runtime: &RuntimeDispatchRuntimeCapabilityFacts,
    selected_backend_key: String,
) -> EmbeddedRuntimeDispatchCandidateDraft {
    EmbeddedRuntimeDispatchCandidateDraft {
        candidate_id: SchedulerDispatchCandidateId::parse(format!(
            "runtime.{}",
            runtime.runtime_id
        ))
        .expect("runtime registry ids should be scheduler-safe"),
        selected_runtime_id: RuntimeIntentId::parse(&runtime.runtime_id)
            .expect("runtime registry ids should be runtime-intent safe"),
        selected_backend_key,
        selected_model_ref: package_facts.model_ref.clone(),
        loaded_runtime_memory_estimate_bytes: conservative_loaded_runtime_memory_estimate_bytes(
            &package_facts.logical_size,
        ),
        runtime_status: runtime.status,
        runtime_instance_id: runtime.runtime_instance_id.clone(),
    }
}

fn pre_reservation_evidence_check(
    draft: &EmbeddedRuntimeDispatchCandidateDraft,
    task_intent: &pantograph_scheduler::SchedulableTaskIntent,
    selected_device_id: pantograph_dependency_planning::DeviceIntentId,
) -> Result<RuntimeDispatchEvidenceRecord, RuntimeDispatchEvidenceDiagnostic> {
    RuntimeDispatchEvidenceRecord::new(RuntimeDispatchEvidenceRequest {
        selected_backend_key: draft.selected_backend_key.clone(),
        runtime_family: String::new(),
        resolved_load_target: String::new(),
        runtime_residency_key: String::new(),
        loaded_runtime_memory_estimate_bytes: draft
            .loaded_runtime_memory_estimate_bytes
            .unwrap_or_default(),
        runtime_load_state: Some(runtime_dispatch_evidence_load_state(draft.runtime_status)),
        runtime_instance_id: draft.runtime_instance_id.clone(),
        selected_runtime_id: draft.selected_runtime_id.clone(),
        selected_model_ref: draft.selected_model_ref.clone(),
        selected_device_id,
        reservations: Vec::new(),
        resource_fit_assessment: SchedulerResourceFitAssessment {
            workflow_run_id: task_intent.workflow_run_id.clone(),
            task_id: task_intent.task_id.clone(),
            state: SchedulerResourceFitState::Fits,
            diagnostics: Vec::new(),
        },
    })
}

fn runtime_dispatch_evidence_load_state(
    status: RuntimeRegistryStatus,
) -> RuntimeDispatchEvidenceLoadState {
    match status {
        RuntimeRegistryStatus::Stopped => RuntimeDispatchEvidenceLoadState::NotLoaded,
        RuntimeRegistryStatus::Warming => RuntimeDispatchEvidenceLoadState::Loading,
        RuntimeRegistryStatus::Ready => RuntimeDispatchEvidenceLoadState::Loaded,
        RuntimeRegistryStatus::Busy => RuntimeDispatchEvidenceLoadState::Busy,
        RuntimeRegistryStatus::Unhealthy | RuntimeRegistryStatus::Failed => {
            RuntimeDispatchEvidenceLoadState::Failed
        }
        RuntimeRegistryStatus::Stopping => RuntimeDispatchEvidenceLoadState::Unloading,
    }
}

fn runtime_dispatch_evidence_diagnostic(
    candidate_id: &SchedulerDispatchCandidateId,
    diagnostic: RuntimeDispatchEvidenceDiagnostic,
) -> SchedulerDispatchSelectionDiagnostic {
    SchedulerDispatchSelectionDiagnostic {
        severity: SchedulerDispatchSelectionDiagnosticSeverity::Error,
        code: SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
        message: diagnostic.message,
        candidate_id: Some(candidate_id.clone()),
        hint: Some(format!(
            "{MISSING_RUNTIME_DISPATCH_EVIDENCE_HINT}:{}",
            diagnostic.field_path
        )),
    }
}

fn resource_facts_request(
    draft: &EmbeddedRuntimeDispatchCandidateDraft,
    task_intent: &pantograph_scheduler::SchedulableTaskIntent,
    selected_device_id: pantograph_dependency_planning::DeviceIntentId,
) -> RuntimeDispatchResourceFactsRequest {
    RuntimeDispatchResourceFactsRequest {
        runtime_id: draft.selected_runtime_id.clone(),
        selected_device_id,
        workflow_id: task_intent.workflow_id.as_str().to_string(),
        workflow_run_id: task_intent.workflow_run_id.clone(),
        task_id: task_intent.task_id.clone(),
        reservation_owner_id: format!(
            "{}:{}",
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str()
        ),
        model_id: Some(draft.selected_model_ref.model_id.clone()),
        usage_profile: Some(task_intent.task_type.as_str().to_string()),
        requirements: reservation_requirements_from_estimate_hints(task_intent),
        retention_hint: RuntimeRetentionHint::Ephemeral,
    }
}

fn reservation_requirements_from_estimate_hints(
    task_intent: &pantograph_scheduler::SchedulableTaskIntent,
) -> RuntimeReservationRequirements {
    let mut peak_ram_bytes = None;
    let mut peak_vram_bytes = None;
    for hint in &task_intent.estimate_hints {
        match hint.kind {
            SchedulerEstimateHintKind::PeakRamBytes => {
                peak_ram_bytes = Some(peak_ram_bytes.unwrap_or(0).max(hint.value));
            }
            SchedulerEstimateHintKind::PeakVramBytes => {
                peak_vram_bytes = Some(peak_vram_bytes.unwrap_or(0).max(hint.value));
            }
            _ => {}
        }
    }
    let mut claims = Vec::new();
    if let Some(bytes) = peak_ram_bytes {
        claims.push(RuntimeReservationResourceClaim::ram_bytes(bytes));
    }
    if let Some(bytes) = peak_vram_bytes {
        claims.push(RuntimeReservationResourceClaim::vram_bytes(bytes));
    }
    RuntimeReservationRequirements::from_claims(claims)
}

fn resource_source_diagnostics(
    candidate_id: &SchedulerDispatchCandidateId,
    diagnostics: &[RuntimeDispatchResourceFactsDiagnostic],
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| SchedulerDispatchSelectionDiagnostic {
            severity: SchedulerDispatchSelectionDiagnosticSeverity::Error,
            code: SchedulerDispatchSelectionDiagnosticCode::ResourceFitRejected,
            message: diagnostic.message.clone(),
            candidate_id: Some(candidate_id.clone()),
            hint: Some(MISSING_RUNTIME_RESOURCE_FACTS_HINT.to_string()),
        })
        .collect()
}

fn backend_hint_keys(backend_hints: &inference::BackendHintFacts) -> BTreeSet<String> {
    backend_hints
        .accepted
        .iter()
        .map(|hint| normalize_backend_key(backend_hint_label_key(*hint)))
        .collect()
}

fn backend_hint_label_key(label: inference::BackendHintLabel) -> &'static str {
    match label {
        inference::BackendHintLabel::Transformers => "transformers",
        inference::BackendHintLabel::LlamaCpp => "llama.cpp",
        inference::BackendHintLabel::Vllm => "vllm",
        inference::BackendHintLabel::Mlx => "mlx",
        inference::BackendHintLabel::Candle => "candle",
        inference::BackendHintLabel::Diffusers => "diffusers",
        inference::BackendHintLabel::OnnxRuntime => "onnxruntime",
    }
}

fn normalize_backend_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn provider_diagnostic(
    code: SchedulerDispatchSelectionDiagnosticCode,
    message: &str,
    hint: &str,
) -> SchedulerDispatchSelectionDiagnostic {
    SchedulerDispatchSelectionDiagnostic {
        severity: SchedulerDispatchSelectionDiagnosticSeverity::Error,
        code,
        message: message.to_string(),
        candidate_id: None,
        hint: Some(hint.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pantograph_dependency_planning::DependencyReadinessProofEnvelope;
    use pantograph_runtime_registry::{
        RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeRegistry, RuntimeTransition,
    };
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerEstimateHint, SchedulerFairnessKey, SchedulerNodeId,
        SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTaskState,
        SchedulerTaskStateRecord, SchedulerTaskStateTransitionId, SchedulerTraitValue,
        SchedulerWorkflowId, SchedulerWorkflowRunId, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };
    use pantograph_workflow_service::workflow::WorkflowSchedulerTaskExecutionClass;
    use serde_json::json;

    use super::*;
    use crate::runtime_dispatch_capability_facts::{
        RuntimeDispatchCapabilityFactsProjection, RuntimeDispatchRuntimeCapabilityFacts,
    };

    #[test]
    fn fail_closed_provider_reports_missing_source_facts() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot::default(),
            &path_free_model_ref(),
        );

        assert_eq!(diagnostics.len(), 3);
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.candidate_id.is_none()));
        assert!(has_hint(&diagnostics, MISSING_PUMAS_PACKAGE_FACTS_HINT));
        assert!(has_hint(
            &diagnostics,
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT
        ));
        assert!(has_hint(&diagnostics, MISSING_RUNTIME_RESOURCE_FACTS_HINT));
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code == SchedulerDispatchSelectionDiagnosticCode::NoCandidates
                && diagnostic.severity == SchedulerDispatchSelectionDiagnosticSeverity::Error
        }));
    }

    #[test]
    fn fail_closed_provider_rejects_path_carrying_model_refs() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot::default(),
            &path_carrying_model_ref(),
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(
            diagnostic.code,
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(PATH_CARRYING_MODEL_REF_HINT)
        );
    }

    #[test]
    fn fail_closed_provider_projects_staged_source_diagnostics() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot {
                pumas_package_facts: Some(PumasDispatchPackageFactsBridgeOutcome::Unavailable {
                    diagnostics: vec![PumasDispatchPackageFactsDiagnostic {
                        code: PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess,
                        message: "Pumas owner access is unavailable".to_string(),
                    }],
                }),
                runtime_capability_facts: Some(RuntimeDispatchCapabilityFactsOutcome::Unavailable {
                    diagnostics: vec![RuntimeDispatchCapabilityFactsDiagnostic {
                        code: crate::runtime_dispatch_capability_facts::RuntimeDispatchCapabilityFactsDiagnosticCode::NoRegisteredRuntimes,
                        runtime_id: None,
                        message: "runtime registry has no runtimes".to_string(),
                    }],
                }),
                ..EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
            },
            &path_free_model_ref(),
        );

        assert_eq!(diagnostics.len(), 3);
        assert!(!has_hint(&diagnostics, MISSING_PUMAS_PACKAGE_FACTS_HINT));
        assert!(!has_hint(
            &diagnostics,
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT
        ));
        assert!(has_hint(&diagnostics, MISSING_RUNTIME_RESOURCE_FACTS_HINT));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "Pumas owner access is unavailable"));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "runtime registry has no runtimes"));
    }

    #[test]
    fn fail_closed_provider_projects_snapshot_lifecycle_diagnostics() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot {
                diagnostics: vec![EmbeddedRuntimeDispatchSourceSnapshotDiagnostic {
                    code: EmbeddedRuntimeDispatchSourceSnapshotDiagnosticCode::StaleSnapshot,
                    message: "runtime dispatch source-fact snapshot is stale".to_string(),
                }],
                ..EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
            },
            &path_free_model_ref(),
        );

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.hint.as_deref()
                == Some("embedded_runtime_dispatch_candidate_provider.source_snapshot.stale")
                && diagnostic.code == SchedulerDispatchSelectionDiagnosticCode::NoCandidates
        }));
    }

    #[test]
    fn provider_reads_snapshot_store_at_dispatch_time() {
        let store = EmbeddedRuntimeDispatchSourceFactSnapshotStore::new(
            crate::pumas_dispatch_package_facts::PumasDispatchPackageFactsSource::new(None),
            crate::runtime_dispatch_capability_facts::RuntimeDispatchCapabilityFactsSource::new(
                Arc::new(RuntimeRegistry::new()),
            ),
            crate::runtime_dispatch_load_target_facts::RuntimeDispatchLoadTargetFactsSource::new(
                None,
            ),
            100,
        );
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot_store(store);

        let candidate_set = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("missing source snapshot should be a typed diagnostic");

        assert!(candidate_set.candidates.is_empty());
        assert!(candidate_set.diagnostics.iter().any(|diagnostic| {
            diagnostic.hint.as_deref()
                == Some("embedded_runtime_dispatch_candidate_provider.source_snapshot.missing")
                && diagnostic.code == SchedulerDispatchSelectionDiagnosticCode::NoCandidates
        }));
    }

    #[test]
    fn candidate_drafts_match_pumas_backend_hints_to_runtime_capabilities() {
        let (drafts, diagnostics) = candidate_drafts_from_projected_facts(
            &pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
            &runtime_capability_facts(vec![runtime_capability("pytorch", vec!["diffusers"])]),
        );

        assert!(diagnostics.is_empty());
        assert_eq!(drafts.len(), 1);
        let draft = &drafts[0];
        assert_eq!(draft.candidate_id.as_str(), "runtime.pytorch");
        assert_eq!(draft.selected_runtime_id.as_str(), "pytorch");
        assert_eq!(draft.selected_backend_key, "diffusers");
        assert_eq!(draft.selected_model_ref.model_id, "pumas.model.sdxl");
        assert_eq!(draft.selected_model_ref.selected_artifact_path, None);
        assert_eq!(draft.loaded_runtime_memory_estimate_bytes, Some(23_856));
    }

    #[test]
    fn candidate_drafts_report_backend_hint_mismatch() {
        let (drafts, diagnostics) = candidate_drafts_from_projected_facts(
            &pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
            &runtime_capability_facts(vec![runtime_capability("llama.cpp", vec!["llama.cpp"])]),
        );

        assert!(drafts.is_empty());
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(
            diagnostic.code,
            SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(INCOMPATIBLE_RUNTIME_BACKEND_HINT)
        );
    }

    #[test]
    fn candidate_drafts_do_not_synthesize_memory_estimate_from_weak_size_facts() {
        let mut package_facts = pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]);
        package_facts.logical_size.value_source = inference::PackageFactValueSource::FilenameWeak;

        let (drafts, diagnostics) = candidate_drafts_from_projected_facts(
            &package_facts,
            &runtime_capability_facts(vec![runtime_capability("pytorch", vec!["diffusers"])]),
        );

        assert!(diagnostics.is_empty());
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].loaded_runtime_memory_estimate_bytes, None);
    }

    #[test]
    fn provider_fails_closed_when_runtime_dispatch_evidence_is_missing() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["diffusers".to_string()])
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::ram_bytes(Some(16 * mib())),
                    RuntimeAdmissionResourceBudget::vram_bytes(Some(8 * mib())),
                ])),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime.pytorch.001".to_string()),
                },
            )
            .expect("runtime ready");
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
            EmbeddedRuntimeDispatchCandidateSourceSnapshot {
                pumas_package_facts: Some(PumasDispatchPackageFactsBridgeOutcome::Projected {
                    facts: pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
                    diagnostics: Vec::new(),
                }),
                runtime_capability_facts: Some(RuntimeDispatchCapabilityFactsOutcome::Projected {
                    facts: runtime_capability_facts(vec![runtime_capability(
                        "pytorch",
                        vec!["diffusers"],
                    )]),
                    diagnostics: Vec::new(),
                }),
                ..EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
            },
        )
        .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));

        let candidate_set = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("missing runtime dispatch evidence should be a typed diagnostic");

        assert!(candidate_set.candidates.is_empty());
        assert!(candidate_set.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .candidate_id
                .as_ref()
                .is_some_and(|candidate_id| candidate_id.as_str() == "runtime.pytorch")
                && diagnostic.code
                    == SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
                && diagnostic
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.starts_with(MISSING_RUNTIME_DISPATCH_EVIDENCE_HINT))
        }));
        assert_eq!(registry.snapshot().reservations.len(), 0);
    }

    #[test]
    fn provider_does_not_emit_candidate_without_explicit_device_fact() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["diffusers".to_string()]),
        );
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
            EmbeddedRuntimeDispatchCandidateSourceSnapshot {
                pumas_package_facts: Some(PumasDispatchPackageFactsBridgeOutcome::Projected {
                    facts: pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
                    diagnostics: Vec::new(),
                }),
                runtime_capability_facts: Some(RuntimeDispatchCapabilityFactsOutcome::Projected {
                    facts: runtime_capability_facts(vec![runtime_capability(
                        "pytorch",
                        vec!["diffusers"],
                    )]),
                    diagnostics: Vec::new(),
                }),
                ..EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
            },
        )
        .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry));

        let candidate_set = provider
            .runtime_dispatch_candidates(&workflow_task(None), &ready_record(), &readiness_proof())
            .expect("missing selected device should be a typed diagnostic");

        assert!(candidate_set.candidates.is_empty());
        assert!(has_hint(
            &candidate_set.diagnostics,
            MISSING_SELECTED_DEVICE_FACTS_HINT
        ));
        assert!(candidate_set.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == SchedulerDispatchSelectionDiagnosticCode::IncompatibleDeviceRequirement
        }));
    }

    fn has_hint(diagnostics: &[SchedulerDispatchSelectionDiagnostic], hint: &str) -> bool {
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.hint.as_deref() == Some(hint))
    }

    fn path_free_model_ref() -> PumasModelRef {
        PumasModelRef {
            model_id: "pumas.model.sdxl".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }
    }

    fn path_carrying_model_ref() -> PumasModelRef {
        PumasModelRef {
            selected_artifact_path: Some("/models/sdxl".to_string()),
            ..path_free_model_ref()
        }
    }

    fn pumas_package_facts(
        accepted_backend_hints: Vec<inference::BackendHintLabel>,
    ) -> PumasDispatchPackageFactsProjection {
        PumasDispatchPackageFactsProjection {
            model_ref: path_free_model_ref(),
            artifact_kind: inference::ModelArtifactKind::DiffusersBundle,
            validation_state: inference::ModelValidationState::Valid,
            task: inference::TaskEvidence {
                pipeline_tag: Some("text-to-image".to_string()),
                task_type_primary: Some("image-generation".to_string()),
                input_modalities: vec!["text".to_string()],
                output_modalities: vec!["image".to_string()],
            },
            backend_hints: inference::BackendHintFacts {
                accepted: accepted_backend_hints,
                raw: Vec::new(),
                unsupported: Vec::new(),
            },
            requires_custom_code: false,
            logical_size: inference::PackageLogicalSizeFacts {
                total_size_bytes: Some(7952),
                value_source: inference::PackageFactValueSource::ComponentLayout,
                files: vec![inference::PackageFileSizeFact {
                    relative_path: "unet/diffusion_pytorch_model.safetensors".to_string(),
                    size_bytes: Some(4096),
                    status: inference::PackageFactStatus::Present,
                    value_source: inference::PackageFactValueSource::FilesystemMetadata,
                    role: Some(inference::PackageSizeRole::Weight),
                }],
                diagnostics: Vec::new(),
            },
            diffusers: None,
        }
    }

    fn runtime_capability_facts(
        runtimes: Vec<RuntimeDispatchRuntimeCapabilityFacts>,
    ) -> RuntimeDispatchCapabilityFactsProjection {
        RuntimeDispatchCapabilityFactsProjection {
            generated_at_ms: 1,
            runtimes,
        }
    }

    fn runtime_capability(
        runtime_id: &str,
        backend_keys: Vec<&str>,
    ) -> RuntimeDispatchRuntimeCapabilityFacts {
        RuntimeDispatchRuntimeCapabilityFacts {
            runtime_id: runtime_id.to_string(),
            backend_keys: backend_keys.into_iter().map(str::to_string).collect(),
            runtime_family: "diffusers".to_string(),
            runtime_residency_key: format!("runtime.diffusers.{runtime_id}.shared"),
            status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some(format!("{runtime_id}.instance")),
            loaded_model_ids: Vec::new(),
            active_reservation_ids: Vec::new(),
            has_admission_budget: true,
        }
    }

    fn workflow_task(requested_device_id: Option<&str>) -> WorkflowSchedulerTask {
        let intent = schedulable_intent(requested_device_id);
        WorkflowSchedulerTask {
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: intent.node_id.clone(),
            task_id: intent.task_id.clone(),
            node_type: "inference".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            dependency_task_ids: Vec::new(),
            input_bindings: Vec::new(),
            schedulable_intent: Some(intent),
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            diagnostics: Vec::new(),
        }
    }

    fn schedulable_intent(requested_device_id: Option<&str>) -> SchedulableTaskIntent {
        SchedulableTaskIntent {
            contract_version: 1,
            workflow_id: SchedulerWorkflowId::parse("workflow.image").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run.image.001")
                .expect("workflow run id"),
            node_id: SchedulerNodeId::parse("node.inference").expect("node id"),
            task_id: SchedulerTaskId::parse("task.inference.001").expect("task id"),
            fairness_key: Some(SchedulerFairnessKey::parse("user.local").expect("fairness key")),
            task_type: "image_generation".parse().expect("task type"),
            model_ref: path_free_model_ref(),
            constraints: SchedulerRuntimeDeviceConstraints {
                requested_runtime_id: Some(RuntimeIntentId::parse("pytorch").expect("runtime id")),
                requested_device_id: requested_device_id
                    .map(|device_id| device_id.parse().expect("device id")),
            },
            trait_settings: vec![pantograph_scheduler::SchedulerTraitSetting {
                trait_id: "denoiser.scheduler".parse().expect("trait id"),
                value: SchedulerTraitValue::String("euler".to_string()),
            }],
            dependency_override_patches: Vec::new(),
            estimate_hints: vec![
                SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakRamBytes,
                    value: mib(),
                },
                SchedulerEstimateHint {
                    kind: SchedulerEstimateHintKind::PeakVramBytes,
                    value: 2 * mib(),
                },
            ],
        }
    }

    fn ready_record() -> SchedulerTaskStateRecord {
        let intent = schedulable_intent(Some("cuda:0"));
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: intent.workflow_id,
            workflow_run_id: intent.workflow_run_id,
            node_id: intent.node_id,
            task_id: intent.task_id,
            state: SchedulerTaskState::Ready {
                execution_intent: pantograph_scheduler::SchedulerTaskExecutionIntent::Runtime {
                    task_intent: schedulable_intent(Some("cuda:0")),
                },
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse("transition.ready")
                .expect("transition id"),
        }
    }

    fn readiness_proof() -> DependencyReadinessProofEnvelope {
        serde_json::from_value(json!({
            "contract_version": 1,
            "execution_context": {
                "contract_version": 1,
                "workflow_id": "workflow.image",
                "workflow_run_id": "run.image.001",
                "scheduler_task_id": "task.inference.001",
                "node_id": "node.inference",
                "graph_revision": "graph.revision.001",
                "validation_session_id": "validation.session.001",
                "validation_snapshot_id": "validation.snapshot.001",
                "descriptor_fingerprint": "descriptor.image.001",
                "dependency_requirements_id": "deps.image",
                "correlation_id": "correlation.image.001"
            },
            "preflight_result": {
                "contract_version": 1,
                "identity_key": {
                    "model_ref": {
                        "model_id": "pumas.model.sdxl",
                        "revision": "main",
                        "selected_artifact_id": "diffusers"
                    },
                    "task_id": "image_generation"
                },
                "readiness_state": "ready",
                "dependency_requirements_id": "deps.image",
                "environment_ref": {
                    "environment_id": "env.image"
                }
            },
            "readiness_proof_id": "readiness.proof.image.001",
            "readiness_proof_version": 1
        }))
        .expect("readiness proof")
    }

    fn mib() -> u64 {
        1024 * 1024
    }
}
